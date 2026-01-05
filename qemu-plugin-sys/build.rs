use anyhow::Result;
#[cfg(windows)]
use anyhow::anyhow;
#[cfg(windows)]
use std::{env::var, path::PathBuf, process::Command, str::FromStr};

#[cfg(feature = "plugin-api-v0")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v0.def";
#[cfg(feature = "plugin-api-v1")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v1.def";
#[cfg(feature = "plugin-api-v2")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v2.def";
#[cfg(feature = "plugin-api-v3")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v3.def";
#[cfg(feature = "plugin-api-v4")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v4.def";
#[cfg(feature = "plugin-api-v5")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v5.def";
#[cfg(feature = "plugin-api-v6")]
pub const PLUGIN_API_DEF_FILE_NAME: &str = "qemu_plugin_api_v6.def";

#[cfg(windows)]
fn out_dir() -> Result<PathBuf> {
    Ok(PathBuf::from(
        var("OUT_DIR").map_err(|e| anyhow!("OUT_DIR not set: {e}"))?,
    ))
}

fn main() -> Result<()> {
    #[cfg(windows)]
    {
        let out_dir = out_dir()?;
        let def_file = PathBuf::from_str(&format!("src/{PLUGIN_API_DEF_FILE_NAME}"))?;
        let def_file_str = def_file.to_string_lossy();
        let lib_file = out_dir.join("qemu_plugin_api.lib");
        let lib_file_str = lib_file.to_string_lossy();
        let ch = Command::new("dlltool")
            .args([
                "--input-def",
                &def_file_str,
                "--output-delaylib",
                &lib_file_str,
                "--dllname",
                "qemu.exe",
            ])
            .spawn()?
            .wait()?;
        if !ch.success() {
            return Err(anyhow!("dlltool failed"));
        }
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rustc-link-lib=qemu_plugin_api");
    }

    #[cfg(all(target_family = "unix", not(target_os = "macos")))]
    {
        println!("cargo:rustc-link-arg-cdylib=-Wl,-z,undefs")
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo::rustc-cdylib-link-arg=-undefined");
        println!("cargo::rustc-cdylib-link-arg=dynamic_lookup");
    }

    Ok(())
}
