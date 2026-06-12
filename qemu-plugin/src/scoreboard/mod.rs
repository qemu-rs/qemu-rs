//! Scoreboard-related functionality for QEMU plugins

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use crate::VCPUIndex;
use crate::error::Error;
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use crate::sys::{qemu_plugin_scoreboard, qemu_plugin_u64};
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use std::{marker::PhantomData, mem::MaybeUninit};

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
#[derive(Debug)]
/// A wrapper structure for a `qemu_plugin_scoreboard *`. This is a way of having one
/// entry per VCPU, the count of which is managed automatically by QEMU. Keep in mind
/// that additional entries *and* existing entries will be allocated and reallocated by
/// *qemu*, not by the plugin, so every use of a `T` should include a check for whether
/// it is initialized.
pub struct Scoreboard<'a, T>
where
    T: Sized,
{
    handle: usize,
    marker: PhantomData<&'a T>,
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a, T> Scoreboard<'a, T> {
    /// Allocate a new scoreboard object. This must be freed by calling
    /// `qemu_plugin_scoreboard_free` (or by being dropped).
    pub fn new() -> Self {
        let handle =
            unsafe { crate::sys::qemu_plugin_scoreboard_new(std::mem::size_of::<T>()) as usize };

        Self {
            handle,
            marker: PhantomData,
        }
    }

    /// Returns a reference to entry of a scoreboard matching a given vcpu index. This address
    /// is only valid until the next call to `get` or `set`.
    pub fn find<'b>(&mut self, vcpu_index: VCPUIndex) -> &'b mut MaybeUninit<T> {
        unsafe {
            &mut *(crate::sys::qemu_plugin_scoreboard_find(
                self.handle as *mut qemu_plugin_scoreboard,
                vcpu_index,
            ) as *mut MaybeUninit<T>)
        }
    }

    /// Create a [`PluginU64`] pointing to offset into this scoreboard's entries
    pub fn entry(&mut self, offset: usize) -> Result<PluginU64<'_>, Error> {
        let entry_size = std::mem::size_of::<T>();
        (offset.saturating_add(8) <= entry_size)
            .then_some(PluginU64 {
                inner: qemu_plugin_u64 {
                    score: self.handle as *mut qemu_plugin_scoreboard,
                    offset,
                },
                marker: Default::default(),
            })
            .ok_or(Error::InvalidScoreBoardEntryOffset { offset, entry_size })
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a, T> Default for Scoreboard<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a, T> Drop for Scoreboard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            crate::sys::qemu_plugin_scoreboard_free(self.handle as *mut qemu_plugin_scoreboard)
        }
    }
}

/// `u64` within a [`Scoreboard`]'s entries
///
/// Addresses an `u64` member of an entry in a scoreboard, allows access to a
/// specific u64 member in one given entry, located at a specified offset.
/// Inline operations expect this as an entry.
#[derive(Copy, Clone, Debug)]
pub struct PluginU64<'s> {
    pub(crate) inner: qemu_plugin_u64,
    marker: std::marker::PhantomData<&'s ()>,
}

impl PluginU64<'_> {
    /// Get the value for a given VCPU
    pub fn get(self, vcpu_index: VCPUIndex) -> u64 {
        unsafe { crate::sys::qemu_plugin_u64_get(self.inner, vcpu_index) }
    }

    /// Set the value for a given VCPU
    pub fn set(self, vcpu_index: VCPUIndex, value: u64) {
        unsafe { crate::sys::qemu_plugin_u64_set(self.inner, vcpu_index, value) }
    }

    /// Add a value for a given VCPU
    pub fn add(self, vcpu_index: VCPUIndex, value: u64) {
        unsafe { crate::sys::qemu_plugin_u64_add(self.inner, vcpu_index, value) }
    }

    /// Get the sum of all VCPU entries for this value
    pub fn sum(self) -> u64 {
        unsafe { crate::sys::qemu_plugin_u64_sum(self.inner) }
    }
}
