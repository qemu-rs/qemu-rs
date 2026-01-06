//! High level idiomatic Rust bindings to the QEMU plugin API
//!
//! `qemu-plugin` makes it easy to write QEMU plugins in Rust.
//!
//! # Example
//!
//! Below is a minimal plugin example for a plugin which prints the execution trace of the
//! program running in QEMU. Notice that all we do is register a struct which implements
//! `Plugin`, and the library takes care of the rest.
//!
//! ```rust,ignore
//! use qemu_plugin::{
//!     HasCallbacks, Plugin, Register, register, Error, Result, PluginId,
//!     TranslationBlock,
//! };
//!
//! struct TinyTrace;
//!
//! impl Register for TinyTrace {}
//!
//! impl HasCallbacks for TinyTrace {
//!     fn on_translation_block_translate(
//!         &mut self,
//!         _id: PluginId,
//!         tb: TranslationBlock,
//!     ) -> Result<()> {
//!         tb.instructions().enumerate().try_for_each(|(idx, insn)| {
//!             if idx == 0 {
//!                 println!("====TB: {:08x}", insn.vaddr());
//!             }
//!
//!             println!("{:08x}: {}", insn.vaddr(), insn.disas()?);
//!             Ok::<(), Error>(())
//!         })?;
//!
//!         Ok(())
//!     }
//! }
//!
//! register!(TinyTrace);
//! ```
//!
//! The above `src/lib.rs` in a Cargo project with the following `Cargo.toml` will compile to
//! `libtiny.so`, which can be loaded in QEMU by running `qemu-system-ARCH -plugin ./libtiny.so`.
//!
//! ```toml
//! [package]
//! name = "tiny"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! qemu-plugin = "10.1.0-v2"
//! ```

#![deny(missing_docs)]
#![cfg_attr(feature = "num-traits", feature(generic_const_exprs))]

#[cfg(windows)]
mod win_link_hook;

use crate::sys::{
    qemu_plugin_cb_flags, qemu_plugin_id_t, qemu_plugin_insn, qemu_plugin_mem_rw,
    qemu_plugin_meminfo_t, qemu_plugin_op, qemu_plugin_simple_cb_t, qemu_plugin_tb,
    qemu_plugin_udata_cb_t, qemu_plugin_vcpu_simple_cb_t, qemu_plugin_vcpu_syscall_cb_t,
    qemu_plugin_vcpu_syscall_ret_cb_t, qemu_plugin_vcpu_tb_trans_cb_t,
};
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use crate::sys::{qemu_plugin_reg_descriptor, qemu_plugin_u64};
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
use qemu_plugin_sys::qemu_plugin_cond;
#[cfg(not(feature = "plugin-api-v0"))]
use std::{ffi::CStr, path::PathBuf};
use std::{
    ffi::{CString, c_uint, c_void},
    sync::{Mutex, OnceLock},
};

pub mod error;
#[allow(unused_imports)]
pub use error::*;
pub mod install;
pub use install::*;
pub mod plugin;
pub use plugin::*;
pub mod instruction;
pub mod sys;
pub use instruction::*;
pub mod translation_block;
pub use translation_block::*;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
pub mod register;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
pub use register::*;
pub mod memory;
pub use memory::*;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
pub mod scoreboard;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
pub use scoreboard::*;
pub mod glib;
pub(crate) use glib::*;

/// The index of a vCPU
pub type VCPUIndex = c_uint;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// u64 member of an entry in a scoreboard, allows access to a specific u64 member in
/// one given entry, located at a specified offset. Inline operations expect this as an
/// entry.
pub type PluginU64 = qemu_plugin_u64;
/// Flags for callbacks
pub type CallbackFlags = qemu_plugin_cb_flags;
/// Memory read/write flags
pub type MemRW = qemu_plugin_mem_rw;
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
/// A condition for a callback to be run
pub type PluginCondition = qemu_plugin_cond;
/// Plugin operations for inline operations
pub type PluginOp = qemu_plugin_op;
/// A plugin ID
pub type PluginId = qemu_plugin_id_t;

/// A callback that can be called many times, each time a vCPU is initialized
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that was initialized
pub type VCPUInitCallback = qemu_plugin_vcpu_simple_cb_t;

/// A callback that can be called many times, each time a vCPU exits
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that exited
pub type VCPUExitCallback = qemu_plugin_vcpu_simple_cb_t;

/// A callback that can be called many times, each time a vCPU idles
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that idled
pub type VCPUIdleCallback = qemu_plugin_vcpu_simple_cb_t;

/// A callback that can be called many times, each time a vCPU is resumed
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that resumed
pub type VCPUResumeCallback = qemu_plugin_vcpu_simple_cb_t;

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4",
    feature = "plugin-api-v5"
)))]
/// A callback that can be called many times, each time a discontinuity occurs
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU on which the discontinuity occurred
/// - `type`: The type of discontinuity in execution
/// - `from_pc`: The source of the discontinuity, e.g. the PC before the
///   transition
/// - `to_pc`: The PC pointing to the next instruction to be executed
pub type VCPUDisconCallback = sys::qemu_plugin_vcpu_discon_cb_t;

/// A callback that can be called many times, each time a translation occurs.  The
/// callback is passed an opaque `qemu_plugin_tb` pointer, which can be queried for
/// additional information including the list of translated instructions.  The callback
/// can register further callbacks to be triggered when the block or individual
/// instructions execute.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `tb`: The translated block
pub type VCPUTranslationBlockTranslationCallback = qemu_plugin_vcpu_tb_trans_cb_t;

/// A callback called on flush.
///
/// # Arguments
///
/// - `id`: The plugin ID
pub type FlushCallback = qemu_plugin_simple_cb_t;

/// A callback called on Syscall entry
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that executed the instruction
/// - `num`: The syscall number
/// - `a1`: The first syscall argument
/// - `a2`: The second syscall argument
/// - `a3`: The third syscall argument
/// - `a4`: The fourth syscall argument
/// - `a5`: The fifth syscall argument
/// - `a6`: The sixth syscall argument
/// - `a7`: The seventh syscall argument
/// - `a8`: The eighth syscall argument
pub type SyscallCallback = qemu_plugin_vcpu_syscall_cb_t;

/// A callback called on Syscall return
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `vcpu_index`: The index of the vCPU that executed the instruction
/// - `num`: The syscall number
/// - `ret`: The syscall return value
pub type SyscallReturnCallback = qemu_plugin_vcpu_syscall_ret_cb_t;

/// A callback called when execution has finished and the plugin should free its resources
///
/// # Arguments
///
/// - `id`: The plugin ID
pub type AtExitCallback = qemu_plugin_udata_cb_t;

// NOTE: Box<Box< is not strictly necessary here because the pointer is never sent via
// FFI which means we never downcast to an 8-byte pointer from fat, but it is best not
// to rely on that.

#[allow(clippy::type_complexity)]
/// A callback which will run once removal and uninstallation of the plugin is finalized. This callback
/// can only be set once, by calling the `qemu_plugin_uninstall` function.
static UNINSTALL_CALLBACK: OnceLock<
    Mutex<Option<Box<Box<dyn FnOnce(qemu_plugin_id_t) + Send + Sync + 'static>>>>,
> = OnceLock::new();

#[allow(clippy::type_complexity)]
/// A callback which will run once the plugin is reset. This callback is set by calling the
/// `qemu_plugin_reset` function.
static RESET_CALLBACK: OnceLock<
    Mutex<Option<Box<Box<dyn FnOnce(qemu_plugin_id_t) + Send + Sync + 'static>>>>,
> = OnceLock::new();

/// Handle the invocation of the uninstall callback by calling the stored
/// callback closure, if one exists.
extern "C" fn handle_qemu_plugin_uninstall_callback(id: qemu_plugin_id_t) {
    if let Some(callback) = UNINSTALL_CALLBACK.get()
        && let Ok(mut callback) = callback.lock()
        && let Some(callback) = callback.take()
    {
        callback(id);
    }
    // NOTE: An error here is ignored, and exceedingly fatal
}

/// Handle the invocation of the reset callback by calling the stored
/// callback closure, if one exists.
extern "C" fn handle_qemu_plugin_reset_callback(id: qemu_plugin_id_t) {
    if let Some(callback) = UNINSTALL_CALLBACK.get()
        && let Ok(mut callback) = callback.lock()
        && let Some(callback) = callback.take()
    {
        callback(id);
    }
    // NOTE: An error here is ignored, and exceedingly fatal
}

/// Uninstall a plugin.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: A callback function that will be called when the plugin has been
///   uninstalled.
///
/// # Safety
///
/// Do NOT assume that the plugin has been uninstalled once this function returns.
/// Plugins are uninstalled asynchronously, and therefore the given plugin receives
/// callbacks until cb is called. This function must not be called from
/// `qemu_plugin_install`.
pub fn qemu_plugin_uninstall<F>(id: qemu_plugin_id_t, cb: F) -> Result<()>
where
    F: FnOnce(qemu_plugin_id_t) + Send + Sync + 'static,
{
    UNINSTALL_CALLBACK
        .set(Mutex::new(Some(Box::new(Box::new(cb)))))
        .map_err(|_| Error::ConcurrentPluginUninstallCallbackSet)?;

    unsafe { crate::sys::qemu_plugin_uninstall(id, Some(handle_qemu_plugin_uninstall_callback)) };

    Ok(())
}

/// Reset a plugin
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: A callback function that will be called when the plugin has been reset.
///
/// # Safety
///
/// Do NOT assume that the plugin has been reset once this function returns. Plugins
/// are reset asynchronously, and therefore the given plugin receives callbacks until
/// cb is called.
pub fn qemu_plugin_reset<F>(id: qemu_plugin_id_t, cb: F) -> Result<()>
where
    F: FnOnce(qemu_plugin_id_t) + Send + Sync + 'static,
{
    if let Some(callback) = RESET_CALLBACK.get() {
        let Ok(mut callback) = callback.lock() else {
            return Err(Error::PluginResetCallbackState);
        };
        let _ = callback.replace(Box::new(Box::new(cb)));
    } else {
        RESET_CALLBACK
            .set(Mutex::new(Some(Box::new(Box::new(cb)))))
            .map_err(|_| Error::ConcurrentPluginResetCallbackSet)?;
    }

    unsafe { crate::sys::qemu_plugin_reset(id, Some(handle_qemu_plugin_reset_callback)) };

    Ok(())
}

/// Register a callback to be called when a vCPU is initialized. The callback does not receive
/// user data, so it is not possible to register it via closure.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_init_cb(id: qemu_plugin_id_t, cb: VCPUInitCallback) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_register_vcpu_init_cb(id, cb) };
    Ok(())
}

/// Register a callback to be called when a vCPU exits. The callback does not receive
/// user data, so it is not possible to register it via closure.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_exit_cb(id: qemu_plugin_id_t, cb: VCPUExitCallback) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_register_vcpu_exit_cb(id, cb) };
    Ok(())
}

/// Register a callback to be called when a vCPU idles. The callback does not receive
/// user data, so it is not possible to register it via closure.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_idle_cb(id: qemu_plugin_id_t, cb: VCPUIdleCallback) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_register_vcpu_idle_cb(id, cb) };
    Ok(())
}

/// Register a callback to be called when a vCPU resumes. The callback does not receive
/// user data, so it is not possible to register it via closure.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_resume_cb(
    id: qemu_plugin_id_t,
    cb: VCPUResumeCallback,
) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_register_vcpu_resume_cb(id, cb) };
    Ok(())
}

/// Register a callback to be called when a translation block is translated. The callback
/// receives a pointer to a `qemu_plugin_tb` structure, which can be queried for additional
/// information including the list of translated instructions. The callback can register
/// further callbacks to be triggered when the block or individual instructions execute.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_tb_trans_cb(
    id: qemu_plugin_id_t,
    cb: VCPUTranslationBlockTranslationCallback,
) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_register_vcpu_tb_trans_cb(id, cb) };
    Ok(())
}

extern "C" fn handle_qemu_plugin_register_vcpu_tb_exec_cb<F>(
    vcpu_index: VCPUIndex,
    userdata: *mut c_void,
) where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    let mut cb: Box<Box<F>> = unsafe { Box::from_raw(userdata as *mut _) };
    cb(vcpu_index);
    Box::leak(cb);
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register a callback to be called when a translation block is executed.
///
/// # Arguments
///
/// - `tb`: The translated block to register the execution callback for
/// - `cb`: The callback to be called when the block `tb` is executed
///
/// # Safety
///
/// This function is safe when the pointer `tb` is a valid pointer to a `qemu_plugin_tb`
/// structure, which is always opaque.
pub fn qemu_plugin_register_vcpu_tb_exec_cb<F>(tb: TranslationBlock, cb: F, flags: CallbackFlags)
where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    tb.register_execute_callback_flags(cb, flags);
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
/// Register a callback to be conditionally called when a translation block is executed.
///
/// # Arguments
///
/// - `tb`: The translated block to register the execution callback for
/// - `cb`: The callback to be called when the block `tb` is executed
/// - `cond`: The condition to be met for the callback to be called
/// - `entry`: The entry to be passed to the callback
/// - `immediate`: The immediate value to be passed to the callback
///
/// # Safety
///
/// This function is safe when the pointer `tb` is a valid pointer to a `qemu_plugin_tb`
/// structure, which is always opaque.
pub fn qemu_plugin_register_vcpu_tb_exec_cond_cb<F>(
    tb: TranslationBlock,
    cb: F,
    flags: CallbackFlags,
    cond: PluginCondition,
    entry: PluginU64,
    immediate: u64,
) where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    tb.register_conditional_execute_callback_flags(cb, flags, cond, entry, immediate);
}

#[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback to be called when a translation block is executed.
///
/// # Arguments
///
/// - `tb`: The translated block to register the execution callback for
/// - `op`: The operation to be performed
/// - `ptr`: The pointer to the data to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_tb_exec_inline(
    tb: TranslationBlock,
    op: PluginOp,
    ptr: *mut c_void,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_tb_exec_inline(
            tb.translation_block as *mut qemu_plugin_tb,
            op,
            ptr,
            imm,
        );
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback to be called when a translation block is executed.
///
/// # Arguments
///
/// - `tb`: The translated block to register the execution callback for
/// - `op`: The operation to be performed
/// - `entry`: The entry to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_tb_exec_inline_per_vcpu(
    tb: TranslationBlock,
    op: PluginOp,
    entry: PluginU64,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_tb_exec_inline_per_vcpu(
            tb.translation_block as *mut qemu_plugin_tb,
            op,
            entry,
            imm,
        );
    }
}

extern "C" fn handle_qemu_plugin_register_vcpu_insn_exec_cb<F>(
    vcpu_index: VCPUIndex,
    userdata: *mut c_void,
) where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    let mut cb: Box<Box<F>> = unsafe { Box::from_raw(userdata as *mut _) };
    cb(vcpu_index);
    // NOTE: This memory will be freed on plugin exit
    Box::leak(cb);
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register a callback to be called when an instruction is executed.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_insn_exec_cb<F>(insn: Instruction, cb: F, flags: CallbackFlags)
where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    insn.register_execute_callback_flags(cb, flags);
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2"
)))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register a callback to be conditionally called when an instruction is executed.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `cb`: The callback to be called
/// - `cond`: The condition to be met for the callback to be called
/// - `entry`: The entry to be passed to the callback
/// - `immediate`: The immediate value to be passed to the callback
pub fn qemu_plugin_register_vcpu_insn_exec_cond_cb<F>(
    insn: Instruction,
    cb: F,
    flags: CallbackFlags,
    cond: PluginCondition,
    entry: PluginU64,
    immediate: u64,
) where
    F: FnMut(VCPUIndex) + Send + Sync + 'static,
{
    insn.register_conditional_execute_callback_flags(cb, flags, cond, entry, immediate);
}

#[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback to be called when an instruction is executed.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `op`: The operation to be performed
/// - `ptr`: The pointer to the data to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_insn_exec_inline(
    insn: Instruction,
    op: PluginOp,
    ptr: *mut c_void,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_insn_exec_inline(
            insn.instruction as *mut qemu_plugin_insn,
            op,
            ptr,
            imm,
        );
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback to be called when an instruction is executed.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `op`: The operation to be performed
/// - `entry`: The entry to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_insn_exec_inline_per_vcpu(
    insn: Instruction,
    op: PluginOp,
    entry: PluginU64,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_insn_exec_inline_per_vcpu(
            insn.instruction as *mut qemu_plugin_insn,
            op,
            entry,
            imm,
        );
    }
}

extern "C" fn handle_qemu_plugin_register_vcpu_mem_cb<F>(
    vcpu_index: VCPUIndex,
    meminfo: qemu_plugin_meminfo_t,
    vaddr: u64,
    userdata: *mut c_void,
) where
    F: for<'a> FnMut(VCPUIndex, MemoryInfo<'a>, u64) + Send + Sync + 'static,
{
    let mut cb: Box<Box<F>> = unsafe { Box::from_raw(userdata as *mut _) };
    let meminfo = MemoryInfo::from(meminfo);
    cb(vcpu_index, meminfo, vaddr);
    // NOTE: This memory will be freed on plugin exit
    Box::leak(cb);
}

/// Register a callback for every memory transaction of a particular instruction. If the
/// instruction is executed multiple times, the callback will be called multiple times.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `cb`: The callback to be called
/// - `rw`: Whether the callback should be called for reads, writes, or both
pub fn qemu_plugin_register_vcpu_mem_cb<F>(
    insn: Instruction,
    cb: F,
    flags: CallbackFlags,
    rw: MemRW,
) where
    F: for<'a> FnMut(VCPUIndex, MemoryInfo<'a>, u64) + Send + Sync + 'static,
{
    insn.register_memory_access_callback_flags(cb, rw, flags);
}

#[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback for every memory transaction of a particular instruction.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `rw`: Whether the callback should be called for reads, writes, or both
/// - `op`: The operation to be performed
/// - `ptr`: The pointer to the data to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_mem_inline(
    insn: Instruction,
    rw: MemRW,
    op: PluginOp,
    ptr: *mut c_void,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_mem_inline(
            insn.instruction as *mut qemu_plugin_insn,
            rw,
            op,
            ptr,
            imm,
        );
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Register an inline callback for every memory transaction of a particular instruction.
///
/// # Arguments
///
/// - `insn`: The instruction handle to register the callback for
/// - `rw`: Whether the callback should be called for reads, writes, or both
/// - `op`: The operation to be performed
/// - `entry`: The entry to be passed to the operation
/// - `imm`: The immediate value to be passed to the operation
pub fn qemu_plugin_register_vcpu_mem_inline_per_vcpu(
    insn: Instruction,
    rw: MemRW,
    op: PluginOp,
    entry: PluginU64,
    imm: u64,
) {
    unsafe {
        crate::sys::qemu_plugin_register_vcpu_mem_inline_per_vcpu(
            insn.instruction as *mut qemu_plugin_insn,
            rw,
            op,
            entry,
            imm,
        );
    }
}

extern "C" fn handle_qemu_plugin_register_atexit_cb<F>(id: qemu_plugin_id_t, userdata: *mut c_void)
where
    F: FnOnce(qemu_plugin_id_t) + Send + Sync + 'static,
{
    let cb: Box<Box<F>> = unsafe { Box::from_raw(userdata as *mut _) };
    cb(id);
    // NOTE: This memory is not leaked because this is the last callback to be called
    // and it can only be called once, so we allow it to drop
}

/// Register a callback to run once execution is finished. Plugins should be able to free all
/// their resources at this point.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_atexit_cb<F>(id: qemu_plugin_id_t, cb: F) -> Result<()>
where
    F: FnOnce(qemu_plugin_id_t) + Send + Sync + 'static,
{
    let callback = Box::new(cb);
    let callback_box = Box::new(callback);
    unsafe {
        crate::sys::qemu_plugin_register_atexit_cb(
            id,
            Some(handle_qemu_plugin_register_atexit_cb::<F>),
            Box::into_raw(callback_box) as *mut c_void,
        )
    };
    Ok(())
}

/// Register a callback to run after QEMU flushes all translation blocks. This is
/// roughly equivalent to a TLB flush, where all instruction caches are invalidated.
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_flush_cb(id: qemu_plugin_id_t, cb: FlushCallback) {
    unsafe { crate::sys::qemu_plugin_register_flush_cb(id, cb) };
}

/// Register a callback to run on Syscall
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_syscall_cb(id: qemu_plugin_id_t, cb: SyscallCallback) {
    unsafe { crate::sys::qemu_plugin_register_vcpu_syscall_cb(id, cb) };
}

/// Register a callback to run on Syscall return
///
/// # Arguments
///
/// - `id`: The plugin ID
/// - `cb`: The callback to be called
pub fn qemu_plugin_register_vcpu_syscall_ret_cb(id: qemu_plugin_id_t, cb: SyscallReturnCallback) {
    unsafe { crate::sys::qemu_plugin_register_vcpu_syscall_ret_cb(id, cb) };
}

/// Output a string via the QEMU logging mechanism
pub fn qemu_plugin_outs<S>(string: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe {
        crate::sys::qemu_plugin_outs(CString::new(string.as_ref())?.as_ptr());
    }

    Ok(())
}

#[cfg(feature = "plugin-api-v0")]
/// Parse a boolean argument in the form of `=[on|yes|true|off|no|false]`.
/// returns true if the combination @name=@val parses correctly to a boolean
/// argument, and false otherwise. Note that in plugin API v0, this function is
/// not available, so we implement it ourselves. This may ostensibly lead to
/// different behavior.
///
/// # Arguments
///
/// - `name`: argument name, the part before the equals sign @val: argument
///   value, what’s after the equals sign @ret: output return value
/// - `val`: Argument value, what’s after the equals sign
///
pub fn qemu_plugin_bool_parse<S>(name: S, val: S) -> Result<bool>
where
    S: AsRef<str>,
{
    // We can't call sys::qemu_plugin_bool_parse directly because it doesn't exist in plugin-api-v0
    match val.as_ref() {
        "on" | "yes" | "true" => Ok(true),
        "off" | "no" | "false" => Ok(false),
        _ => Err(Error::InvalidBool {
            name: name.as_ref().to_string(),
            val: val.as_ref().to_string(),
        }),
    }
}

#[cfg(not(feature = "plugin-api-v0"))]
/// Parse a boolean argument in the form of `=[on|yes|true|off|no|false]`. returns true
/// if the combination @name=@val parses correctly to a boolean argument, and false
/// otherwise.
///
/// # Arguments
///
/// - `name`: argument name, the part before the equals sign @val: argument value, what’s
///   after the equals sign @ret: output return value
/// - `val`: Argument value, what’s after the equals sign
///
pub fn qemu_plugin_bool_parse<S>(name: S, val: S) -> Result<bool>
where
    S: AsRef<str>,
{
    let mut value = false;
    if unsafe {
        crate::sys::qemu_plugin_bool_parse(
            CString::new(name.as_ref())?.as_ptr(),
            CString::new(val.as_ref())?.as_ptr(),
            &mut value,
        )
    } {
        Ok(value)
    } else {
        Err(Error::InvalidBool {
            name: name.as_ref().to_string(),
            val: val.as_ref().to_string(),
        })
    }
}

#[cfg(not(feature = "plugin-api-v0"))]
/// Return the path to the binary file being executed if running in user mode,
/// or None if running in System mode. Return an error if the path cannot be
/// converted to a string.
pub fn qemu_plugin_path_to_binary() -> Result<Option<PathBuf>> {
    let path_str = unsafe { crate::sys::qemu_plugin_path_to_binary() };
    if path_str.is_null() {
        Ok(None)
    } else {
        let path = unsafe { PathBuf::from(CStr::from_ptr(path_str).to_str()?) };
        unsafe { g_free(path_str as *mut _) };
        Ok(Some(path))
    }
}

#[cfg(not(feature = "plugin-api-v0"))]
/// Return the start of the text segment of the binary file being executed if
/// running in user mode, or None if running in System mode. If not running in
/// system mode, `None` may be interpreted as zero by callers, but the caller
/// must take care to ensure the plugin is not running in a system mode context.
pub fn qemu_plugin_start_code() -> Option<u64> {
    let start = unsafe { crate::sys::qemu_plugin_start_code() };

    if start == 0 { None } else { Some(start) }
}

#[cfg(not(feature = "plugin-api-v0"))]
/// Return the end of the text segment of the binary file being executed if
/// running in user mode, or None if running in System mode. If not running in
/// system mode, `None` may be interpreted as zero by callers, but the caller
/// must take care to ensure the plugin is not running in a system mode context.
pub fn qemu_plugin_end_code() -> Option<u64> {
    let end = unsafe { crate::sys::qemu_plugin_end_code() };

    if end == 0 { None } else { Some(end) }
}

#[cfg(not(feature = "plugin-api-v0"))]
/// Return the start address for the module of the binary file being executed if
/// running in user mode, or None if running in System mode. If not running in
/// system mode, `None` may be interpreted as zero by callers, but the caller
/// must take care to ensure the plugin is not running in a system mode context.
pub fn qemu_plugin_entry_code() -> Option<u64> {
    let entry = unsafe { crate::sys::qemu_plugin_entry_code() };

    if entry == 0 { None } else { Some(entry) }
}

#[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
/// Return the number of vCPUs, if running in system mode
pub fn qemu_plugin_n_vcpus() -> Option<i32> {
    let vcpus = unsafe { crate::sys::qemu_plugin_n_vcpus() };

    if vcpus == -1 { None } else { Some(vcpus) }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Return the number of vCPUs, if running in system mode
pub fn qemu_plugin_num_vcpus() -> Option<i32> {
    let vcpus = unsafe { crate::sys::qemu_plugin_num_vcpus() };

    if vcpus == -1 { None } else { Some(vcpus) }
}

#[cfg(any(feature = "plugin-api-v0", feature = "plugin-api-v1"))]
/// Return the maximum number of vCPUs, if running in system mode
pub fn qemu_plugin_n_max_vcpus() -> Option<i32> {
    let max_cpus = unsafe { crate::sys::qemu_plugin_n_max_vcpus() };

    if max_cpus == -1 { None } else { Some(max_cpus) }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Returns a potentially empty list of registers. This should be used from a
/// qemu_plugin_register_vcpu_init_cb callback after the vcpu has been initialized.
pub fn qemu_plugin_get_registers<'a>() -> Result<Vec<RegisterDescriptor<'a>>> {
    use std::slice::from_raw_parts;

    let array = unsafe { crate::sys::qemu_plugin_get_registers() };

    let registers = unsafe {
        from_raw_parts(
            (*array).data as *mut qemu_plugin_reg_descriptor,
            (*array).len as usize,
        )
    }
    .iter()
    .map(|desc| RegisterDescriptor::from(*desc))
    .collect::<Vec<_>>();

    // Function notes say caller frees the array but not the strings in each entry
    assert_eq!(
        unsafe { g_array_free(array, true) },
        std::ptr::null_mut(),
        "g_array_free return value must be NULL"
    );

    Ok(registers)
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Add a value to a `PluginU64` for a given VCPU
pub fn qemu_plugin_u64_add(entry: PluginU64, vcpu_index: VCPUIndex, added: u64) -> Result<()> {
    unsafe { crate::sys::qemu_plugin_u64_add(entry, vcpu_index, added) };
    Ok(())
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Get the value of a `PluginU64` for a given VCPU
pub fn qemu_plugin_u64_get(entry: PluginU64, vcpu_index: VCPUIndex) -> u64 {
    unsafe { crate::sys::qemu_plugin_u64_get(entry, vcpu_index) }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Set the value of a `PluginU64` for a given VCPU
pub fn qemu_plugin_u64_set(entry: PluginU64, vcpu_index: VCPUIndex, value: u64) {
    unsafe { crate::sys::qemu_plugin_u64_set(entry, vcpu_index, value) }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
/// Get the sum of all VCPU entries in a scoreboard
pub fn qemu_plugin_scoreboard_sum(entry: PluginU64) -> u64 {
    unsafe { crate::sys::qemu_plugin_u64_sum(entry) }
}
