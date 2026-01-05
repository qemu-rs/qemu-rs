# QEMU-PLUGIN-SYS

Low level auto-generated FFI bindings to the QEMU Plugin API (`qemu-plugin.h`). This
crate should not be used directly, check out the `qemu-plugin` crate for the idiomatic
high-level bindings.

## Versioning

As of QEMU 8.2.4, the QEMU plugin API has more than a single version. This enables some
great features like register inspection and conditional callbacks. Versioning is
implemented in the `qemu-plugin-sys` crate via compile-time features, because a dynamic
library can only be compatible with one version at a time. To choose a version, set a
listing like:

```toml
qemu-plugin-sys = { version = "9.0.0-v0", features = ["plugin-api-v2"], default-features = false }
```

The `qemu-plugin-sys` crate's default plugin version is set to the latest version that
is officially released in QEMU. If you need a different version, you *must* set `default-features = false`
and explicitly set the `plugin-api-vX` value in your `Cargo.toml` file.
