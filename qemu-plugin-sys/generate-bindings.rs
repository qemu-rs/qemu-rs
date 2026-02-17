#!/usr/bin/env -S cargo +nightly-gnu -Z script
---
[package]
edition = "2024"
[dependencies]
anyhow = "*"
bindgen = "*"
cargo_metadata = "*"
reqwest = { version = "*", features = ["blocking"] }
syn = "*"
zip = "*"
[lints.rust]
non_snake_case = "allow"
---

use anyhow::{Result, anyhow};
use bindgen::{
    AliasVariation, EnumVariation, FieldVisibilityKind, MacroTypeVariation, NonCopyUnionStyle,
    builder,
};
use cargo_metadata::MetadataCommand;
use reqwest::blocking::get;
use std::{
    fs::{File, OpenOptions, create_dir_all, read_to_string, write},
    io::copy,
    path::{Path, PathBuf},
};
use syn::{File as RustFile, ForeignItem, ForeignItemFn, Item, ItemForeignMod, parse_str};
use zip::ZipArchive;

const QEMU_GITHUB_URL_BASE: &str = "https://github.com/qemu/qemu/";

const QEMU_VERSIONS: &[&str] = &[
    // Plugin V0 is from 4.2.0
    "b0ca999a43a22b38158a222233d3f5881648bb4f",
    // Plugin V1 is up until 8.2.4
    "1332b8dd434674480f0feb2cdf3bbaebb85b4240",
    // Plugin V2 is from 9.0.0
    "c25df57ae8f9fe1c72eee2dab37d76d904ac382e",
    // Plugin V3 is from 9.1.0
    "7de77d37880d7267a491cb32a1b2232017d1e545",
    // Plugin V4 is from 9.2.0
    "595cd9ce2ec9330882c991a647d5bc2a5640f380",
    // Plugin V5 is from 10.1.0
    "f8b2f64e2336a28bf0d50b6ef8a7d8c013e9bcf3",
    // Plugin V6 is from 10.2.1+ era
    // https://github.com/qemu/qemu/commit/c22ea55b3be01b1798bc6b9cf1311d9a5e58c68b
    "c22ea55b3be01b1798bc6b9cf1311d9a5e58c68b",
    // Plugins .h moved in this commit, but not yet a new V7 released with that change.
    // Uncomment to test future/V7 generation, from header file in new location.
    // https://github.com/qemu/qemu/commit/cab15547f1fe300fec91966eec05932212d6d6e1
    // "cab15547f1fe300fec91966eec05932212d6d6e1",
];

fn qemu_git_url(hash: &str) -> String {
    format!("{}/archive/{}.zip", QEMU_GITHUB_URL_BASE, hash)
}

/// Download a URL to a destination, using a blocking request
fn download(url: &str, destination: &Path) -> Result<()> {
    let mut response = get(url)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(destination)?;
    response.copy_to(&mut file)?;
    Ok(())
}

/// Extract a zip file at a path to a destination (stripping the root)
fn extract_zip(archive: &Path, destination: &Path) -> Result<()> {
    let archive = File::open(archive)?;
    let mut archive = ZipArchive::new(archive)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.mangled_name();

        let components: Vec<_> = file_path.components().collect();

        if components.len() <= 1 {
            continue;
        }

        let new_path = components[1..].iter().collect::<PathBuf>();
        let out_path = destination.join(new_path);

        if file.is_dir() {
            create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                create_dir_all(parent)?;
            }
            let mut out_file = File::create(&out_path)?;
            copy(&mut file, &mut out_file)?;
        }
    }
    Ok(())
}

fn generate_bindings(
    qemu_plugin_header: &Path,
    bindings_path: &Path,
    def_path: &Path,
) -> Result<()> {
    let header_contents = read_to_string(qemu_plugin_header)?;
    let header_file_name = qemu_plugin_header
        .file_name()
        .ok_or_else(|| anyhow!("Failed to get file name"))?
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert file name to string"))?;
    let header_contents = header_contents.replace("#include <glib.h>", "");
    // Append `typedef GArray void;` and `typedef GByteArray void;` to the header. Otherwise, we
    // need to use pkg_config to find the glib-2.0 include paths and our bindings will be
    // massive.
    let header_contents = format!(
        "#include <stddef.h>\n{}\n{}\n{}\n",
        "typedef struct GArray { char *data; unsigned int len; } GArray;",
        "typedef struct GByteArray { unsigned char *data; unsigned int len; } GByteArray;",
        header_contents,
    );

    let rust_bindings = builder()
        .clang_arg("-fretain-comments-from-system-headers")
        .clang_arg("-fparse-all-comments")
        .clang_arg("-Wno-everything")
        .default_visibility(FieldVisibilityKind::Public)
        .default_alias_style(AliasVariation::TypeAlias)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .default_macro_constant_type(MacroTypeVariation::Unsigned)
        .default_non_copy_union_style(NonCopyUnionStyle::BindgenWrapper)
        .derive_default(true)
        .derive_hash(true)
        .derive_partialord(true)
        .derive_ord(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .generate_comments(true)
        .layout_tests(false)
        .header_contents(header_file_name, &header_contents)
        // Blocklist because we will define these items
        .blocklist_function("qemu_plugin_install")
        .blocklist_item("qemu_plugin_version")
        // ALlowlist all other qemu_plugin.* items
        .allowlist_item("qemu_plugin.*")
        .allowlist_item("QEMU_PLUGIN.*")
        .allowlist_item("G.*")
        .allowlist_item("g_.*")
        .generate()?;

    rust_bindings.write_to_file(bindings_path)?;

    let parsed: RustFile = parse_str(&rust_bindings.to_string())?;

    let mut export_names: Vec<String> = parsed
        .items
        .iter()
        .filter_map(|item| {
            if let Item::ForeignMod(ItemForeignMod { items, .. }) = item {
                Some(items)
            } else {
                None
            }
        })
        .flat_map(|items| {
            items.iter().filter_map(|item| {
                if let ForeignItem::Fn(ForeignItemFn { sig, .. }) = item {
                    Some(sig.ident.to_string())
                } else {
                    None
                }
            })
        })
        .collect();

    export_names.sort();

    // Write to file using a single buffer
    let mut output = String::from("EXPORTS\n");
    output.extend(export_names.into_iter().map(|name| format!("  {}\n", name)));

    write(&def_path, output)?;

    Ok(())
}

fn generate(tmp_dir: &Path, out_dir: &Path, version: usize) -> Result<()> {
    println!(
        "Generating bindings with tmp={:?} out={:?} version={}",
        tmp_dir, out_dir, version
    );
    let src_archive = tmp_dir.join(format!("qemu-{}.zip", QEMU_VERSIONS[version]));
    let src_dir = tmp_dir.join(format!("qemu-{}", QEMU_VERSIONS[version]));

    if !src_archive.exists() {
        let qemu_url = qemu_git_url(QEMU_VERSIONS[version]);
        println!("Downloading {} to {:?}", qemu_url, src_archive);
        download(&qemu_url, &src_archive)?;
    }

    if !src_dir.exists() {
        println!("Extracting {:?} to {:?}", src_archive, src_dir);
        extract_zip(&src_archive, &src_dir)?;
    }

    let old_plugin_h = src_dir.join("include").join("qemu").join("qemu-plugin.h");
    let new_plugin_h = src_dir
        .join("include")
        .join("plugins")
        .join("qemu-plugin.h");

    let plugin_h = if old_plugin_h.exists() {
        old_plugin_h
    } else {
        new_plugin_h
    };

    // TODO: change location of header file here for newer versions?
    generate_bindings(
        &plugin_h,
        &out_dir.join(&format!("bindings_v{}.rs", version)),
        &out_dir.join(&format!("qemu_plugin_api_v{}.def", version)),
    )?;

    Ok(())
}

fn main() -> Result<()> {
    let metadata = MetadataCommand::new().no_deps().exec()?;

    let search_package = "qemu-plugin-sys";
    let package = metadata
        .packages
        .iter()
        .find(|p| p.name == search_package)
        .ok_or_else(|| anyhow!("Failed to find package"))?;

    let out_dir = package
        .manifest_path
        .parent()
        .ok_or_else(|| anyhow!("Failed to get manifest path"))?
        .join("src")
        .into_std_path_buf();

    let tmp_dir = metadata.target_directory.join("tmp").into_std_path_buf();

    if !tmp_dir.exists() {
        create_dir_all(&tmp_dir)?;
    }

    for i in 0..QEMU_VERSIONS.len() {
        generate(&tmp_dir, &out_dir, i)?;
    }

    Ok(())
}
