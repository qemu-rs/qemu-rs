# qemu-rs

This repository provides tools for building [QEMU](https://www.qemu.org)
[TCG](https://www.qemu.org/docs/master/devel/index-tcg.html) plugins in Rust!

If you're unfamiliar with TCG plugins, they provide the ability to get callbacks
on a range of events:

- VCPU Initialize (QEMU system/softmmu only)
- VCPU Exit (QEMU system/softmmu only)
- VCPU Idle (QEMU system/softmmu only)
- Translation Block Cache Flush
- Translation Block Translation: The TCG translated native code to TCG instructions
- Translation Block Executed: A block of translated instructions executes
- Instruction Executed: A specific translated instruction executes
- Instruction Memory Access: An instruction executes that accesses memory
- Syscall Executed (QEMU user mode only)
- Syscall Return (QEMU user mode only)

They also allow you to read and write registers, virtual memory, and physical
memory. This provides the building blocks for a number of analyses and tools
from profilers to fuzzers to tracers and beyond.

## Quickstart

To build a plugin on qemu-rs, all you need to do is:

1. Create a new crate: `cargo new --lib myplugin`
2. Make it a `cdylib` crate type and add features to toggle
   between support for different versions of the QEMU API (see [versions](#versions))

```toml
cat <<EOF >> myplugin/Cargo.toml
[lib]
crate-type = ["cdylib"]

[features]
default = ["plugin-api-v5"]
plugin-api-v0 = ["qemu-plugin/plugin-api-v0"]
plugin-api-v1 = ["qemu-plugin/plugin-api-v1"]
plugin-api-v2 = ["qemu-plugin/plugin-api-v2"]
plugin-api-v3 = ["qemu-plugin/plugin-api-v3"]
plugin-api-v4 = ["qemu-plugin/plugin-api-v4"]
plugin-api-v5 = ["qemu-plugin/plugin-api-v5"]
EOF
```
3. Add dependencies: `cargo -C myplugin add qemu-plugin anyhow`
4. Create a new `lib.rs` that declares a plugin:

```rust
cat <<EOF > myplugin/src/lib.rs
use anyhow::Result;
use qemu_plugin::{
    HasCallbacks, Register, PluginId, TranslationBlock, CallbackFlags, register
};

struct QemuPlugin;

impl Register for QemuPlugin {}
impl HasCallbacks for QemuPlugin {
    fn on_translation_block_translate(
        &mut self,
        _id: PluginId,
        tb: TranslationBlock
    ) -> Result<()> {
        tb.instructions().try_for_each(|insn| {
            let insn_disas = insn.disas()?;
            insn.register_execute_callback_flags<F>(|vcpu_index| {
                    println!("[{vcpu_index}]: {insn_disas}");
                },
                CallbackFlags::QEMU_PLUGIN_CB_NO_REGS
            )
        })
    }
}

register!(QemuPlugin);
EOF
```

5. Build your plugin: `cargo build -r`
6. Make sure you have a `qemu` built with plugin support: `qemu-x86_64 -h | grep qemu`
7. Run your plugin: `qemu-x86_64 -plugin target/release/libmyplugin.so /bin/ls`

## Versions

QEMU versions its plugin API --- the API of each version for the plugins are very similar
but they are *not* 100% stable between versions. They are mostly forward compatible but
are not backward compatible. It is *STRONGLY* encouraged to target the exact version of
QEMU for correctness. Some discussions here on [the QEMU mailing list](https://lore.kernel.org/qemu-devel/dfb3afd3-af50-4a14-afe4-fddd8adc5040@linaro.org/) around the V5 to V6 compatibility topic.

The following QEMU versions introduce the corresponding plugin API versions.

| QEMU Version | Plugin API Version |
| ------------ | ------------------ |
| 4.2.0        | 0                  |
| 6.0.0        | 1                  |
| 9.0.0        | 2                  |
| 9.1.0        | 3                  |
| 9.2.0        | 4                  |
| 10.1.0       | 5                  |
| 11.0         | 6 (Incompatible with V5)|
