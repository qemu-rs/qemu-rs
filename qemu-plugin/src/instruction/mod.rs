//! Instruction-related functionality for QEMU plugins

use crate::{
    CallbackFlags, Error, MemRW, MemoryInfo, Result, TranslationBlock, VCPUIndex, g_free,
    handle_qemu_plugin_register_vcpu_insn_exec_cb, handle_qemu_plugin_register_vcpu_mem_cb,
    sys::qemu_plugin_insn,
};
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
use crate::{PluginCondition, scoreboard::PluginU64};
use std::{
    ffi::{CStr, c_void},
    marker::PhantomData,
};

#[derive(Debug, Clone)]
/// Wrapper structure for a `qemu_plugin_insn *`
///
/// # Safety
///
/// This structure is safe to use as long as the pointer is valid. The pointer is
/// always opaque, and therefore may not be dereferenced.
///
/// # Example
///
/// ```
/// struct MyPlugin;
///
/// impl qemu_plugin::plugin::Register for MyPlugin {}
///
/// impl qemu_plugin::plugin::HasCallbacks for MyPlugin {
///     fn on_translation_block_translate(
///         &mut self,
///         id: qemu_plugin::PluginId,
///         tb: qemu_plugin::TranslationBlock,
///     ) -> Result<()> {
///         for insn in tb.instructions() {
///             let vaddr = insn.vaddr();
///             let disas = insn.disas()?;
///             // Register a callback to be run on execution of this instruction
///             insn.register_execute_callback(move |vcpu_index| {
///                 println!("{vcpu_index}@{vaddr:#x}: {disas}");
///             });
///         }
///         Ok(())
///     }
/// }
/// ```
pub struct Instruction<'a> {
    #[allow(unused)]
    // NOTE: This field may be useful in the future
    translation_block: &'a TranslationBlock<'a>,
    pub(crate) instruction: usize,
    marker: PhantomData<&'a ()>,
}

impl<'a> Instruction<'a> {
    pub(crate) fn new(
        translation_block: &'a TranslationBlock<'a>,
        insn: *mut qemu_plugin_insn,
    ) -> Self {
        Self {
            translation_block,
            instruction: insn as usize,
            marker: PhantomData,
        }
    }
}

impl<'a> Instruction<'a> {
    #[cfg(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    ))]
    /// Returns the data for this instruction. This method may only be called inside the
    /// callback in which the instruction is obtained, but the resulting data is owned.
    pub fn data(&self) -> Vec<u8> {
        let size = self.size();
        let mut data = Vec::with_capacity(size);

        // NOTE: The name of this API doesn't change, but its parameters and return value *do*
        let insn_data =
            unsafe { crate::sys::qemu_plugin_insn_data(self.instruction as *mut qemu_plugin_insn) }
                as *mut u8;

        unsafe {
            data.set_len(size);
            std::ptr::copy_nonoverlapping(insn_data, data.as_mut_ptr(), size);
        }

        data
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    /// Reads the data for this instruction returning number of bytes read. This method may only be
    /// called inside the callback in which the instruction is obtained.
    pub fn read_data(&self, data: &mut [u8]) -> usize {
        // NOTE: The name of this API doesn't change, but its parameters and return value *do*
        unsafe {
            crate::sys::qemu_plugin_insn_data(
                self.instruction as *mut qemu_plugin_insn,
                data.as_mut_ptr() as *mut _,
                data.len(),
            )
        }
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    /// Returns the data for this instruction. This method may only be called inside the
    /// callback in which the instruction is obtained, but the resulting data is owned.
    pub fn data(&self) -> Vec<u8> {
        let size = self.size();
        let mut data = vec![0; size];

        let size = self.read_data(&mut data);

        data.truncate(size);

        data
    }

    /// Returns the size of the data for this instruction
    pub fn size(&self) -> usize {
        unsafe { crate::sys::qemu_plugin_insn_size(self.instruction as *mut qemu_plugin_insn) }
    }

    /// Returns the virtual address of this instruction
    pub fn vaddr(&self) -> u64 {
        unsafe { crate::sys::qemu_plugin_insn_vaddr(self.instruction as *mut qemu_plugin_insn) }
    }

    /// Returns the hardware (physical) address of this instruction
    pub fn haddr(&self) -> u64 {
        (unsafe { crate::sys::qemu_plugin_insn_haddr(self.instruction as *mut qemu_plugin_insn) })
            as usize as u64
    }

    /// Returns the textual disassembly of this instruction
    pub fn disas(&self) -> Result<String> {
        let disas = unsafe {
            crate::sys::qemu_plugin_insn_disas(self.instruction as *mut qemu_plugin_insn)
        };
        if disas.is_null() {
            Err(Error::NoDisassemblyString)
        } else {
            let disas_string = unsafe { CStr::from_ptr(disas) }.to_str()?.to_string();

            // NOTE: The string is allocated, so we free it
            unsafe { g_free(disas as *mut _) };

            Ok(disas_string)
        }
    }

    #[cfg(not(feature = "plugin-api-v0"))]
    /// Returns the symbol associated with this instruction, if one exists and the
    /// binary contains a symbol table
    pub fn symbol(&self) -> Result<Option<String>> {
        let symbol = unsafe {
            crate::sys::qemu_plugin_insn_symbol(self.instruction as *mut qemu_plugin_insn)
        };
        if symbol.is_null() {
            Ok(None)
        } else {
            let symbol_string = unsafe { CStr::from_ptr(symbol) }.to_str()?.to_string();
            // NOTE: The string is static, so we do not free it
            Ok(Some(symbol_string))
        }
    }

    /// Register a callback to be run on execution of this instruction with no
    /// capability to inspect registers
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    pub fn register_execute_callback<F>(&self, cb: F)
    where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        self.register_execute_callback_flags(cb, CallbackFlags::QEMU_PLUGIN_CB_NO_REGS)
    }

    /// Register a callback to be run on execution of this instruction with a choice of
    /// capability whether to inspect or modify registers or not
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `flags`: The flags for the callback specifying whether the callback needs
    ///   permission to read or write registers
    pub fn register_execute_callback_flags<F>(&self, cb: F, flags: CallbackFlags)
    where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        let callback = Box::new(cb);
        let callback_box = Box::new(callback);
        let userdata = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            crate::sys::qemu_plugin_register_vcpu_insn_exec_cb(
                self.instruction as *mut qemu_plugin_insn,
                Some(handle_qemu_plugin_register_vcpu_insn_exec_cb::<F>),
                flags,
                userdata,
            )
        };
    }

    /// Register a callback to be conditionally run on execution of this instruction
    /// with no capability to inspect registers
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `cond`: The condition for the callback to be run
    /// - `entry`: The entry to increment the scoreboard for
    /// - `immediate`: The immediate value to use for the callback
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    pub fn register_conditional_execute_callback<F>(
        &self,
        cb: F,
        cond: PluginCondition,
        entry: PluginU64,
        immediate: u64,
    ) where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        self.register_conditional_execute_callback_flags(
            cb,
            CallbackFlags::QEMU_PLUGIN_CB_NO_REGS,
            cond,
            entry,
            immediate,
        )
    }

    /// Register a callback to be conditionally run on execution of this instruction
    /// with a choice of capability whether to inspect or modify registers or not
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `flags`: The flags for the callback specifying whether the callback needs
    ///   permission to read or write registers
    /// - `cond`: The condition for the callback to be run
    /// - `entry`: The entry to increment the scoreboard for
    /// - `immediate`: The immediate value to use for the callback
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    pub fn register_conditional_execute_callback_flags<F>(
        &self,
        cb: F,
        flags: CallbackFlags,
        cond: PluginCondition,
        entry: PluginU64,
        immediate: u64,
    ) where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        let callback = Box::new(cb);
        let callback_box = Box::new(callback);
        let userdata = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            crate::sys::qemu_plugin_register_vcpu_insn_exec_cond_cb(
                self.instruction as *mut qemu_plugin_insn,
                Some(handle_qemu_plugin_register_vcpu_insn_exec_cb::<F>),
                flags,
                cond,
                entry.inner,
                immediate,
                userdata,
            )
        };
    }

    /// Register a callback to be run on memory access of this instruction
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `rw`: The type of memory access to trigger the callback on
    pub fn register_memory_access_callback<F>(&self, cb: F, rw: MemRW)
    where
        F: for<'b> FnMut(VCPUIndex, MemoryInfo<'b>, u64) + Send + Sync + 'static,
    {
        self.register_memory_access_callback_flags(cb, rw, CallbackFlags::QEMU_PLUGIN_CB_NO_REGS)
    }

    /// Register a callback to be run on memory access of this instruction
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `rw`: The type of memory access to trigger the callback on
    pub fn register_memory_access_callback_flags<F>(&self, cb: F, rw: MemRW, flags: CallbackFlags)
    where
        F: for<'b> FnMut(VCPUIndex, MemoryInfo<'b>, u64) + Send + Sync + 'static,
    {
        let callback = Box::new(cb);
        let callback_box = Box::new(callback);
        let userdata = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            crate::sys::qemu_plugin_register_vcpu_mem_cb(
                self.instruction as *mut qemu_plugin_insn,
                Some(handle_qemu_plugin_register_vcpu_mem_cb::<F>),
                flags,
                rw,
                userdata,
            )
        };
    }
}
