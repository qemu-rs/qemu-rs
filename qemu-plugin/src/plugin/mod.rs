//! Traits and helpers enabling idiomatic QEMU plugin implementation

use std::sync::{Mutex, OnceLock};

use crate::{
    Args, Error, Info, PluginId, Result, TranslationBlock, VCPUIndex,
    qemu_plugin_register_atexit_cb, qemu_plugin_register_flush_cb,
    qemu_plugin_register_vcpu_exit_cb, qemu_plugin_register_vcpu_idle_cb,
    qemu_plugin_register_vcpu_init_cb, qemu_plugin_register_vcpu_resume_cb,
    qemu_plugin_register_vcpu_syscall_cb, qemu_plugin_register_vcpu_syscall_ret_cb,
    qemu_plugin_register_vcpu_tb_trans_cb,
};

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4",
    feature = "plugin-api-v5"
)))]
use crate::{qemu_plugin_discon_type, qemu_plugin_register_vcpu_discon_cb};

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_init_cb`
/// function. These callbacks are called when a vCPU is initialized in QEMU (in softmmu
/// mode only) and notify us which vCPU index is newly initialized.
extern "C" fn handle_qemu_plugin_register_vcpu_init_cb(id: PluginId, vcpu_id: VCPUIndex) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_init(id, vcpu_id)
        .expect("Failed running callback on_vcpu_init");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_exit_cb`
/// function. These callbacks are called when a vCPU exits in QEMU (in softmmu mode
/// only) and notify us which vCPU index is exiting.
extern "C" fn handle_qemu_plugin_register_vcpu_exit_cb(id: PluginId, vcpu_id: VCPUIndex) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_exit(id, vcpu_id)
        .expect("Failed running callback on_vcpu_exit");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_idle_cb`
/// function. These callbacks are called when a vCPU goes idle in QEMU (in softmmu mode
/// only) and notify us which vCPU index is going idle.
extern "C" fn handle_qemu_plugin_register_vcpu_idle_cb(id: PluginId, vcpu_id: VCPUIndex) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_idle(id, vcpu_id)
        .expect("Failed running callback on_vcpu_idle");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_resume_cb`
/// function. These callbacks are called when a vCPU resumes in QEMU (in softmmu mode
/// only) and notify us which vCPU index is resuming.
extern "C" fn handle_qemu_plugin_register_vcpu_resume_cb(id: PluginId, vcpu_id: VCPUIndex) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_resume(id, vcpu_id)
        .expect("Failed running callback on_vcpu_resume");
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4",
    feature = "plugin-api-v5"
)))]
/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_discon_cb`
/// function for interrupts.
extern "C" fn handle_qemu_plugin_register_vcpu_interrupt(
    id: PluginId,
    vcpu_id: VCPUIndex,
    _: qemu_plugin_discon_type,
    from_pc: u64,
    to_pc: u64,
) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_interrupt(id, vcpu_id, from_pc, to_pc)
        .expect("Failed running callback on_vcpu_interrupt");
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4",
    feature = "plugin-api-v5"
)))]
/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_discon_cb`
/// function for exceptions.
extern "C" fn handle_qemu_plugin_register_vcpu_exception(
    id: PluginId,
    vcpu_id: VCPUIndex,
    _: qemu_plugin_discon_type,
    from_pc: u64,
    to_pc: u64,
) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_vcpu_exception(id, vcpu_id, from_pc, to_pc)
        .expect("Failed running callback on_vcpu_exception");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_tb_trans_cb`
/// function. These callbacks are called when a translation block is translated in QEMU
/// and pass an opaque pointer to the translation block.
extern "C" fn handle_qemu_plugin_register_vcpu_tb_trans_cb(
    id: PluginId,
    tb: *mut crate::sys::qemu_plugin_tb,
) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    let tb = TranslationBlock::from(tb);

    plugin
        .on_translation_block_translate(id, tb)
        .expect("Failed running callback on_translation_block_translate");
}

/// Handler for callbacks registered via the `qemu_plugin_register_flush_cb`
/// function. These callbacks are called when QEMU flushes all TBs, which is
/// roughly equivalent to a TLB flush to invalidate all cached instructions.
extern "C" fn handle_qemu_plugin_register_flush_cb(id: PluginId) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_flush(id)
        .expect("Failed running callback on_flush");
}

/// Handler for callbacks registered via the `qemu_plugin_register_atexit_cb`
/// function. These callbacks are called when execution has finished and plugins
/// should free their resources.
extern "C" fn handle_qemu_plugin_register_atexit_cb(id: PluginId, _udata: *mut std::ffi::c_void) {
    // We don't actually need the userdata here, and we pass std::ptr::null_mut() when registering.
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin.on_exit(id).expect("Failed running callback on_exit");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_syscall_cb`
/// function. These callbacks are called when a syscall is made in QEMU and pass the
/// syscall number and its arguments.
extern "C" fn handle_qemu_plugin_register_syscall_cb(
    id: PluginId,
    vcpu_index: VCPUIndex,
    num: i64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    a8: u64,
) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_syscall(id, vcpu_index, num, a1, a2, a3, a4, a5, a6, a7, a8)
        .expect("Failed running callback on_syscall");
}

/// Handler for callbacks registered via the `qemu_plugin_register_vcpu_syscall_ret_cb`
/// function. These callbacks are called when a syscall returns in QEMU and pass the
/// syscall number and its return value.
extern "C" fn handle_qemu_plugin_register_syscall_ret_cb(
    id: PluginId,
    vcpu_index: VCPUIndex,
    num: i64,
    ret: i64,
) {
    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .on_syscall_return(id, vcpu_index, num, ret)
        .expect("Failed running callback on_syscall_return");
}

/// Trait which implemenents registering the callbacks implemented on a struct which
/// `HasCallbacks` with QEMU
///
/// # Example
///
/// Using default registration, you can simply declare an empty `impl` block for
/// `Register` on your type. Then, on plugin load, any callbacks you implement in
/// `HasCallbacks` will be automatically registered with QEMU. Callback events you don't
/// implement will default to no-ops. The only drawback of this approach is a small
/// performance penalty for events even when there is no callback registered for them.
///
/// ```
/// struct MyPlugin;
///
/// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {
///     fn on_translation_block_translate(&mut self, _: qemu_plugin::PluginId, tb: qemu_plugin::TranslationBlock) -> qemu_plugin::Result<()> {
///         println!("Translation block translated");
///         Ok(())
///     }
/// }
///
/// impl qemu_plugin::plugin::Register for MyPlugin {}
/// ```
///
/// For more granular control or to register your own callback handlers, you can
/// implement the `register` method yourself.
///
/// ```
/// struct MyPlugin;
///
/// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {}
/// impl qemu_plugin::plugin::Register for MyPlugin {
///    fn register(&mut self, id: qemu_plugin::PluginId, args: &qemu_plugin::install::Args, info: &qemu_plugin::install::Info) -> Result<(), qemu_plugin::Error> {
///       // Custom registration logic here
///       Ok(())
///    }
/// }
/// ```
///
/// Finally, if you want to override the default registration behavior, you can
/// implement `register_default` yourself. This allows you to circumvent any minor
/// performance penalties.
///
/// ```
/// struct MyPlugin;
///
/// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {}
/// impl qemu_plugin::plugin::Register for MyPlugin {
///     fn register_default(
///        &mut self,   
///        id: qemu_plugin::PluginId,
///        args: &qemu_plugin::install::Args,
///        info: &qemu_plugin::install::Info,
///     ) -> Result<()> {
///         // Custom registration logic here, maybe registering a different
///         // function as a callback rather than using `HasCallbacks`
///         Ok(())
///    }
/// }
/// ```
///
pub trait Register: HasCallbacks + Send + Sync + 'static {
    #[allow(unused)]
    /// Called by QEMU when registering the plugin. This method should only be overridden if no
    /// default callbacks are desired, and will require re-implementing handlers which is not
    /// recommended.
    fn register_default(&mut self, id: PluginId, args: &Args, info: &Info) -> Result<()> {
        qemu_plugin_register_vcpu_init_cb(id, Some(handle_qemu_plugin_register_vcpu_init_cb))?;

        qemu_plugin_register_vcpu_exit_cb(id, Some(handle_qemu_plugin_register_vcpu_exit_cb))?;

        qemu_plugin_register_vcpu_idle_cb(id, Some(handle_qemu_plugin_register_vcpu_idle_cb))?;

        qemu_plugin_register_vcpu_resume_cb(id, Some(handle_qemu_plugin_register_vcpu_resume_cb))?;

        qemu_plugin_register_vcpu_tb_trans_cb(
            id,
            Some(handle_qemu_plugin_register_vcpu_tb_trans_cb),
        )?;

        qemu_plugin_register_flush_cb(id, Some(handle_qemu_plugin_register_flush_cb));

        qemu_plugin_register_atexit_cb(id, |id| {
            handle_qemu_plugin_register_atexit_cb(id, std::ptr::null_mut());
        });

        qemu_plugin_register_vcpu_syscall_cb(id, Some(handle_qemu_plugin_register_syscall_cb));

        qemu_plugin_register_vcpu_syscall_ret_cb(
            id,
            Some(handle_qemu_plugin_register_syscall_ret_cb),
        );

        self.register(id, args, info)?;

        Ok(())
    }

    #[allow(unused)]
    /// Called when registering the plugin. User definition of on-registration behavior should
    /// be implemented here.
    fn register(&mut self, id: PluginId, args: &Args, info: &Info) -> Result<()> {
        Ok(())
    }
}

/// Trait implemented by structs which have callbacks which should be registered with QEMU.
///
/// # Example
///
/// ```
/// struct MyPlugin;
///
/// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {
///     // This callback will be registered on plugin load
///     fn on_translation_block_translate(&mut self, _: qemu_plugin::PluginId, tb: qemu_plugin::TranslationBlock) -> qemu_plugin::Result<()> {
///         println!("Translation block translated");
///         Ok(())
///     }
/// }
///
/// impl qemu_plugin::plugin::Register for MyPlugin {}
/// ```
pub trait HasCallbacks: Send + Sync + 'static {
    #[allow(unused)]
    /// Callback triggered on vCPU init
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    ///
    /// # Example
    ///
    /// ```
    /// struct MyPlugin;
    ///
    /// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {
    ///     fn on_vcpu_init(&mut self, id: qemu_plugin::PluginId, vcpu_id: qemu_plugin::VCPUIndex) -> Result<(), qemu_plugin::Error> {
    ///         println!("vCPU {} initialized for plugin {}", vcpu_id, id);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    /// struct MyPlugin;
    fn on_vcpu_init(&mut self, id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on vCPU exit
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    ///
    /// # Example
    fn on_vcpu_exit(&mut self, id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on vCPU idle
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    fn on_vcpu_idle(&mut self, id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on vCPU resume
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    fn on_vcpu_resume(&mut self, id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5"
    )))]
    /// Callback triggered on vCPU interrupts
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    /// * `from_pc` - The PC of the instruction that would have been executed
    ///               next, had the interrupt not occurred.
    /// * `to_pc` - The PC of the instruction that the interrupt trapped to
    fn on_vcpu_interrupt(
        &mut self,
        id: PluginId,
        vcpu_id: VCPUIndex,
        from_pc: u64,
        to_pc: u64,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4",
        feature = "plugin-api-v5"
    )))]
    /// Callback triggered on vCPU exceptions
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_id` - The ID of the vCPU
    /// * `from_pc` - The PC of the instruction that caused the exception
    /// * `to_pc` - The PC of the instruction that the exception trapped to
    fn on_vcpu_exception(
        &mut self,
        id: PluginId,
        vcpu_id: VCPUIndex,
        from_pc: u64,
        to_pc: u64,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on translation block translation
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `tb` - The translation block
    fn on_translation_block_translate(&mut self, id: PluginId, tb: TranslationBlock) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on flush
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    fn on_flush(&mut self, id: PluginId) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on exit
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    fn on_exit(&mut self, id: PluginId) -> Result<()> {
        Ok(())
    }

    #[allow(unused, clippy::too_many_arguments)]
    /// Callback triggered on syscall
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_index` - The ID of the vCPU
    /// * `num` - The syscall number
    /// * `a1` - The first syscall argument
    /// * `a2` - The second syscall argument
    /// * `a3` - The third syscall argument
    /// * `a4` - The fourth syscall argument
    /// * `a5` - The fifth syscall argument
    /// * `a6` - The sixth syscall argument
    /// * `a7` - The seventh syscall argument
    /// * `a8` - The eighth syscall argument
    fn on_syscall(
        &mut self,
        id: PluginId,
        vcpu_index: VCPUIndex,
        num: i64,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused)]
    /// Callback triggered on syscall return
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the plugin
    /// * `vcpu_index` - The ID of the vCPU
    /// * `num` - The syscall number
    /// * `ret` - The return value of the syscall
    fn on_syscall_return(
        &mut self,
        id: PluginId,
        vcpu_index: VCPUIndex,
        num: i64,
        ret: i64,
    ) -> Result<()> {
        Ok(())
    }
}

/// Trait implemented by structs which are QEMU plugin contexts
pub trait Plugin: Register + HasCallbacks {}

impl<T> Plugin for T where T: Register + HasCallbacks {}

#[doc(hidden)]
/// The global plugin item
pub static PLUGIN: OnceLock<Mutex<Box<dyn Plugin>>> = OnceLock::new();

#[doc(hidden)]
#[inline(never)]
pub fn register_plugin(plugin: impl Plugin) {
    PLUGIN
        .set(Mutex::new(Box::new(plugin)))
        .map_err(|_| Error::PluginInstanceSetError)
        .expect("Failed to set plugin");
}

#[macro_export]
/// Register a plugin
macro_rules! register {
    ($plugin:expr) => {
        #[cfg_attr(target_os = "linux", unsafe(link_section = ".text.startup"))]
        extern "C" fn __plugin_ctor() {
            $crate::plugin::register_plugin($plugin);
        }

        #[used]
        // .init_array.XXXXX sections are processed in lexicographical order
        #[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
        // But there is no way to specify such an ordering on MacOS, even with
        // __TEXT,__init_offsets
        #[cfg_attr(
            target_os = "macos",
            unsafe(link_section = "__DATA,__mod_init_func,mod_init_funcs")
        )]
        // On Windows, it's from .CRT$XCA to .CRT$XCZ, where usually XCU =
        // early, XCT = middle, XCL = late
        #[cfg_attr(windows, unsafe(link_section = ".CRT$XCU"))]
        static __PLUGIN_CTOR: unsafe extern "C" fn() = __plugin_ctor;
    };
}
