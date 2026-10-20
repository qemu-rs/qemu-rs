//! Translation Block-related functionality for QEMU plugins

use crate::{
    CallbackFlags, Error, Instruction, Result, VCPUIndex,
    handle_qemu_plugin_register_vcpu_tb_exec_cb, sys::qemu_plugin_tb,
};
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
use crate::{PluginCondition, scoreboard::PluginU64};
use std::{ffi::c_void, marker::PhantomData};

#[derive(Debug, Clone)]
/// Wrapper structure for a `qemu_plugin_tb *`
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
///             println!("{:08x}: {}", insn.vaddr(), insn.disas()?);
///         }   
///         Ok(())
///     }
/// }
/// ```
pub struct TranslationBlock<'a> {
    pub(crate) translation_block: usize,
    marker: PhantomData<&'a ()>,
}

impl<'a> From<*mut qemu_plugin_tb> for TranslationBlock<'a> {
    fn from(tb: *mut qemu_plugin_tb) -> Self {
        Self {
            translation_block: tb as usize,
            marker: PhantomData,
        }
    }
}

impl<'a> TranslationBlock<'a> {
    /// Returns the number of instructions in the translation block
    pub fn size(&self) -> usize {
        unsafe { crate::sys::qemu_plugin_tb_n_insns(self.translation_block as *mut qemu_plugin_tb) }
    }

    /// Returns the virtual address for the start of a translation block
    pub fn vaddr(&self) -> u64 {
        unsafe { crate::sys::qemu_plugin_tb_vaddr(self.translation_block as *mut qemu_plugin_tb) }
    }

    /// Returns the instruction in the translation block at `index`. If the index is out of bounds,
    /// an error is returned.
    ///
    /// # Arguments
    ///
    /// - `index`: The index of the instruction to return
    pub fn instruction(&'a self, index: usize) -> Result<Instruction<'a>> {
        let size = self.size();

        if index >= size {
            Err(Error::InvalidInstructionIndex { index, size })
        } else {
            Ok(Instruction::new(self, unsafe {
                crate::sys::qemu_plugin_tb_get_insn(
                    self.translation_block as *mut qemu_plugin_tb,
                    index,
                )
            }))
        }
    }

    /// Returns an iterator over the instructions in the translation block
    pub fn instructions(&'a self) -> TranslationBlockIterator<'a> {
        TranslationBlockIterator { tb: self, index: 0 }
    }

    /// Register a callback to be run on execution of this translation block
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    pub fn register_execute_callback<F>(&self, cb: F)
    where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        self.register_execute_callback_flags(cb, CallbackFlags::QEMU_PLUGIN_CB_NO_REGS);
    }

    /// Register a callback to be run on execution of this translation block
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    pub fn register_execute_callback_flags<F>(&self, cb: F, flags: CallbackFlags)
    where
        F: FnMut(VCPUIndex) + Send + Sync + 'static,
    {
        let callback = Box::new(cb);
        let callback_box = Box::new(callback);
        let userdata = Box::into_raw(callback_box) as *mut c_void;

        unsafe {
            crate::sys::qemu_plugin_register_vcpu_tb_exec_cb(
                self.translation_block as *mut qemu_plugin_tb,
                Some(handle_qemu_plugin_register_vcpu_tb_exec_cb::<F>),
                flags,
                userdata,
            )
        };
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    /// Register a callback to be conditionally run on execution of this translation
    /// block
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `cond`: The condition for the callback to be run
    /// - `entry` The entry to increment the scoreboard for
    /// - `immediate`: The immediate value to use for the callback
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

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2"
    )))]
    /// Register a callback to be conditionally run on execution of this translation
    /// block
    ///
    /// # Arguments
    ///
    /// - `cb`: The callback to be run
    /// - `flags`: The flags for the callback
    /// - `cond`: The condition for the callback to be run
    /// - `entry`: The entry to increment the scoreboard for
    /// - `immediate`: The immediate value to use for the callback
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
            crate::sys::qemu_plugin_register_vcpu_tb_exec_cond_cb(
                self.translation_block as *mut qemu_plugin_tb,
                Some(handle_qemu_plugin_register_vcpu_tb_exec_cb::<F>),
                flags,
                cond,
                entry.inner,
                immediate,
                userdata,
            )
        };
    }
}

/// An iterator over the instructions of a translation block
pub struct TranslationBlockIterator<'a> {
    tb: &'a TranslationBlock<'a>,
    index: usize,
}

impl<'a> Iterator for TranslationBlockIterator<'a> {
    type Item = Instruction<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.tb.size();

        if self.index >= size {
            None
        } else {
            let insn = self.tb.instruction(self.index).ok();
            self.index += 1;
            insn
        }
    }
}
