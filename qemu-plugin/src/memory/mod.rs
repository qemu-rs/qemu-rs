//! Memory-related functionality for QEMU plugins

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
)))]
use crate::Error;
#[cfg(not(feature = "plugin-api-v0"))]
use crate::Result;
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
use crate::glib::{g_byte_array_free, g_byte_array_new};
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
use crate::sys::qemu_plugin_hwaddr_operation_result;
#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
use crate::sys::{GByteArray, qemu_plugin_mem_value, qemu_plugin_mem_value_type};
use crate::sys::{qemu_plugin_hwaddr, qemu_plugin_meminfo_t};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
/// Wrapper structure for a `qemu_plugin_meminfo_t`
///
/// # Safety
///
/// This structure is safe to use during the invocation of the callback which receives it as an
/// argument. The structure is always opaque, and therefore may not be accessed directly.
pub struct MemoryInfo<'a> {
    memory_info: qemu_plugin_meminfo_t,
    marker: PhantomData<&'a ()>,
}

impl<'a> From<qemu_plugin_meminfo_t> for MemoryInfo<'a> {
    fn from(info: qemu_plugin_meminfo_t) -> Self {
        Self {
            memory_info: info,
            marker: PhantomData,
        }
    }
}

impl<'a> MemoryInfo<'a> {
    /// Returns the size of the access in base-2, e.g. 0 for byte, 1 for 16-bit, 2 for
    /// 32-bit, etc.
    pub fn size_shift(&self) -> usize {
        (unsafe { crate::sys::qemu_plugin_mem_size_shift(self.memory_info) }) as usize
    }

    /// Returns whether the access was sign extended
    pub fn sign_extended(&self) -> bool {
        unsafe { crate::sys::qemu_plugin_mem_is_sign_extended(self.memory_info) }
    }

    /// Returns whether the access was big-endian
    pub fn big_endian(&self) -> bool {
        unsafe { crate::sys::qemu_plugin_mem_is_big_endian(self.memory_info) }
    }

    /// Returns whether the access was a store
    pub fn is_store(&self) -> bool {
        unsafe { crate::sys::qemu_plugin_mem_is_store(self.memory_info) }
    }

    /// Return a handle to query details about the physical address backing the virtual address
    /// in system emulation. In user-mode, this method always returns `None`.
    pub fn hwaddr(&'a self, vaddr: u64) -> Option<HwAddr<'a>> {
        let hwaddr = unsafe { crate::sys::qemu_plugin_get_hwaddr(self.memory_info, vaddr) };
        if hwaddr.is_null() {
            None
        } else {
            Some(HwAddr::from(hwaddr))
        }
    }

    /// Return last value loaded/stored
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3"
    )))]
    pub fn value(&self) -> MemValue {
        let qemu_mem_value = unsafe { crate::sys::qemu_plugin_mem_get_value(self.memory_info) };
        MemValue::from(qemu_mem_value)
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
#[derive(Clone)]
/// Memory value loaded/stored (in memory callback)
///
/// Wrapper structure for a `qemu_plugin_mem_value`
pub enum MemValue {
    /// 8-bit value
    U8(u8),
    /// 16-bit value
    U16(u16),
    /// 32-bit value
    U32(u32),
    /// 64-bit value
    U64(u64),
    /// 128-bit value
    U128(u128),
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
impl From<qemu_plugin_mem_value> for MemValue {
    fn from(value: qemu_plugin_mem_value) -> Self {
        unsafe {
            match value.type_ {
                qemu_plugin_mem_value_type::QEMU_PLUGIN_MEM_VALUE_U8 => Self::U8(value.data.u8_),
                qemu_plugin_mem_value_type::QEMU_PLUGIN_MEM_VALUE_U16 => Self::U16(value.data.u16_),
                qemu_plugin_mem_value_type::QEMU_PLUGIN_MEM_VALUE_U32 => Self::U32(value.data.u32_),
                qemu_plugin_mem_value_type::QEMU_PLUGIN_MEM_VALUE_U64 => Self::U64(value.data.u64_),
                qemu_plugin_mem_value_type::QEMU_PLUGIN_MEM_VALUE_U128 => {
                    let high = value.data.u128_.high as u128;
                    let low = value.data.u128_.low as u128;
                    Self::U128(high << 64 | low)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Wrapper structure for a `qemu_plugin_hwaddr *`
///
/// # Safety
///
/// This structure is safe to use as long as the pointer is valid. The pointer is
/// always opaque, and therefore may not be dereferenced.
pub struct HwAddr<'a> {
    hwaddr: usize,
    marker: PhantomData<&'a ()>,
}

impl<'a> From<*mut qemu_plugin_hwaddr> for HwAddr<'a> {
    fn from(hwaddr: *mut qemu_plugin_hwaddr) -> Self {
        Self {
            hwaddr: hwaddr as usize,
            marker: PhantomData,
        }
    }
}

impl<'a> HwAddr<'a> {
    /// Returns whether the memory operation is to MMIO. Returns false if the operation is to
    /// RAM.
    pub fn is_io(&self) -> bool {
        unsafe { crate::sys::qemu_plugin_hwaddr_is_io(self.hwaddr as *mut qemu_plugin_hwaddr) }
    }

    #[cfg(not(feature = "plugin-api-v0"))]
    /// Returns the physical address for the memory operation
    pub fn hwaddr(&self) -> u64 {
        unsafe { crate::sys::qemu_plugin_hwaddr_phys_addr(self.hwaddr as *mut qemu_plugin_hwaddr) }
    }

    #[cfg(not(feature = "plugin-api-v0"))]
    /// Returns a string representing the device
    pub fn device_name(&self) -> Result<Option<String>> {
        let device_name = unsafe {
            crate::sys::qemu_plugin_hwaddr_device_name(self.hwaddr as *mut qemu_plugin_hwaddr)
        };

        if device_name.is_null() {
            Ok(None)
        } else {
            let device_name_string = unsafe {
                use std::ffi::CStr;
                CStr::from_ptr(device_name)
            }
            .to_str()?
            .to_string();
            // NOTE: The string is static, so we do not free it
            Ok(Some(device_name_string))
        }
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3"
)))]
/// Read memory from a virtual address. The address must be valid and mapped.
pub fn qemu_plugin_read_memory_vaddr(addr: u64, buf: &mut [u8]) -> Result<()> {
    let mut g_buf = unsafe { g_byte_array_new() };
    if g_buf.is_null() {
        return Err(Error::VaddrReadError {
            addr,
            len: buf.len() as u32,
        });
    }

    let res = if unsafe { crate::sys::qemu_plugin_read_memory_vaddr(addr, g_buf, buf.len()) } {
        // Upon success, copy the gbuf contents to the output slice
        // SAFETY: qemu_plugin_read_memory_vaddr will ensure that the
        // input g_buf has the proper size. We also use `buf.len()` as
        // the upper bound, so worst case scenario it's uninit mem being
        // copied into the output buffer (but that shouldn't happen!).
        unsafe {
            assert_eq!((*g_buf).len as usize, buf.len());
            std::ptr::copy((*g_buf).data, buf.as_mut_ptr(), buf.len());
        }

        Ok(())
    } else {
        Err(Error::VaddrReadError {
            addr,
            len: buf.len() as u32,
        })
    };

    // SAFETY: `true` here will free the backing bytes if the refcount drops to zero,
    // but they've already been copied to the caller's buffer.
    unsafe {
        g_byte_array_free(g_buf, true);
    }

    res
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
/// Write memory to a virtual address. The address must be valid and mapped.
pub fn qemu_plugin_write_memory_vaddr(addr: u64, buf: &mut [u8]) -> Result<()> {
    let mut buf = GByteArray {
        data: buf.as_mut_ptr(),
        len: buf.len() as u32,
    };

    if unsafe { crate::sys::qemu_plugin_write_memory_vaddr(addr, &mut buf as *mut GByteArray) } {
        Ok(())
    } else {
        Err(Error::VaddrWriteError { addr, len: buf.len })
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
/// The result of a hardware operation
pub enum HwaddrOperationResult {
    #[error("Operation completed successfully")]
    /// Operation completed successfully
    Ok = 0,
    #[error("Unspecified error")]
    /// Unspecified error
    Error = 1,
    #[error("Device error")]
    /// Device error
    DeviceError = 2,
    #[error("Access denied")]
    /// Access denied
    AccessDenied = 3,
    /// Invalid address
    #[error("Invalid address")]
    InvalidAddress = 4,
    /// Invalid address space
    #[error("Invalid address space")]
    InvalidAddressSpace = 5,
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
impl From<qemu_plugin_hwaddr_operation_result> for HwaddrOperationResult {
    fn from(value: qemu_plugin_hwaddr_operation_result) -> Self {
        match value {
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_OK => Self::Ok,
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_ERROR => Self::Error,
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_DEVICE_ERROR => Self::DeviceError,
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_ACCESS_DENIED => Self::AccessDenied,
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_INVALID_ADDRESS => Self::InvalidAddress,
            qemu_plugin_hwaddr_operation_result::QEMU_PLUGIN_HWADDR_OPERATION_INVALID_ADDRESS_SPACE => Self::InvalidAddressSpace,
        }
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
/// Read memory from a hardware address. The address must be valid and mapped.
pub fn qemu_plugin_read_memory_hwaddr(addr: u64, buf: &mut [u8]) -> Result<()> {
    let mut buf = GByteArray {
        data: buf.as_mut_ptr(),
        len: buf.len() as u32,
    };

    match unsafe {
        crate::sys::qemu_plugin_read_memory_hwaddr(
            addr,
            &mut buf as *mut GByteArray,
            buf.len as usize,
        )
    }
    .into()
    {
        HwaddrOperationResult::Ok => Ok(()),
        error => Err(Error::HwaddrReadError {
            addr,
            len: buf.len,
            result: error,
        }),
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
/// Read memory from a virtual address. The address must be valid and mapped.
pub fn qemu_plugin_write_memory_hwaddr(addr: u64, buf: &mut [u8]) -> Result<()> {
    let mut buf = GByteArray {
        data: buf.as_mut_ptr(),
        len: buf.len() as u32,
    };

    match unsafe { crate::sys::qemu_plugin_write_memory_hwaddr(addr, &mut buf as *mut GByteArray) }
        .into()
    {
        HwaddrOperationResult::Ok => Ok(()),
        error => Err(Error::HwaddrWriteError {
            addr,
            len: buf.len,
            result: error,
        }),
    }
}

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
/// Translate a virtual address to a hardware address. If the address is not
/// mapped, an error is returned.
pub fn qemu_plugin_translate_vaddr(vaddr: u64) -> Result<u64> {
    let mut hwaddr: u64 = 0;
    if unsafe { crate::sys::qemu_plugin_translate_vaddr(vaddr, &mut hwaddr as *mut _) } {
        Ok(hwaddr)
    } else {
        Err(Error::VaddrTranslateError { vaddr })
    }
}
