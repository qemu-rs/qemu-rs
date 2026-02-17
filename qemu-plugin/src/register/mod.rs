//! Register-related functionality for QEMU plugins
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use crate::{
    Error, Result,
    sys::{qemu_plugin_read_register, qemu_plugin_reg_descriptor, qemu_plugin_register},
};
#[cfg(all(
    not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")),
    feature = "num-traits"
))]
use num_traits::{FromBytes, PrimInt};
#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
use std::{
    ffi::CStr,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

#[cfg(not(any(
    feature = "plugin-api-v0",
    feature = "plugin-api-v1",
    feature = "plugin-api-v2",
    feature = "plugin-api-v3",
    feature = "plugin-api-v4"
)))]
use crate::sys::{GByteArray, qemu_plugin_write_register};

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
#[derive(Clone)]
/// Wrapper structure for a `qemu_plugin_register_descriptor`
///
/// # Safety
///
/// This structure is safe to use as long as the pointer is valid. The pointer is
/// always opaque, and therefore may not be dereferenced.
pub struct RegisterDescriptor<'a> {
    /// Opaque handle to the register for retrieving the value with
    /// qemu_plugin_read_register
    handle: usize,
    /// The register name
    pub name: String,
    /// Optional feature descriptor
    pub feature: Option<String>,
    marker: PhantomData<&'a ()>,
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a> From<qemu_plugin_reg_descriptor> for RegisterDescriptor<'a> {
    fn from(descriptor: qemu_plugin_reg_descriptor) -> Self {
        let name = unsafe { CStr::from_ptr(descriptor.name) }
            .to_str()
            .expect("Register name is not valid UTF-8")
            .to_string();

        let feature = if descriptor.feature.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(descriptor.feature) }
                    .to_str()
                    .expect("Register feature is not valid UTF-8")
                    .to_string(),
            )
        };

        Self {
            handle: descriptor.handle as usize,
            name,
            feature,
            marker: PhantomData,
        }
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a> Debug for RegisterDescriptor<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisterDescriptor")
            .field("name", &self.name)
            .field("feature", &self.feature)
            .finish()
    }
}

#[cfg(not(any(feature = "plugin-api-v0", feature = "plugin-api-v1")))]
impl<'a> RegisterDescriptor<'a> {
    /// Read a register value
    ///
    /// This must only be called in a callback which has been registered with
    /// `CallbackFlags::QEMU_PLUGIN_CB_R_REGS` or
    /// `CallbackFlags::QEMU_PLUGIN_CB_RW_REGS`, otherwise it will fail.
    pub fn read(&self) -> Result<Vec<u8>> {
        use crate::g_byte_array_free;

        let byte_array = unsafe {
            use crate::g_byte_array_new;
            g_byte_array_new()
        };

        let result = unsafe {
            qemu_plugin_read_register(self.handle as *mut qemu_plugin_register, byte_array)
        };

        // Return type of read_register() was changed in this commit, from int to bool
        // https://github.com/qemu/qemu/commit/c22ea55b3be01b1798bc6b9cf1311d9a5e58c68b
        #[cfg(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5"
        ))]
        let error_value: int = -1;

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5"
        )))]
        let error_value: bool = false;

        if result == error_value {
            return Err(Error::RegisterReadError {
                name: self.name.clone(),
            });
        }

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4"
        )))]
        let mut data = Vec::new();
        data.extend_from_slice(unsafe {
            std::slice::from_raw_parts((*byte_array).data, (*byte_array).len as usize)
        });

        assert_eq!(
            unsafe { g_byte_array_free(byte_array, true) },
            std::ptr::null_mut(),
            "g_byte_array_free must return NULL"
        );

        Ok(data)
    }

    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4"
    )))]
    /// Read a register value
    ///
    /// This must only be called in a callback which has been registered with
    /// `CallbackFlags::QEMU_PLUGIN_CB_RW_REGS`, otherwise it will fail.
    pub fn write(&self, data: &mut [u8]) -> Result<()> {
        let mut buf = GByteArray {
            data: data.as_mut_ptr(),
            len: data.len() as u32,
        };

        let result = unsafe {
            qemu_plugin_write_register(self.handle as *mut _, &mut buf as *mut GByteArray)
        };

        // Return type of write_register() was changed in this commit, from int to bool
        // https://github.com/qemu/qemu/commit/c22ea55b3be01b1798bc6b9cf1311d9a5e58c68b
        #[cfg(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5"
        ))]
        let error_value: int = -1;

        #[cfg(not(any(
            feature = "plugin-api-v0",
            feature = "plugin-api-v1",
            feature = "plugin-api-v2",
            feature = "plugin-api-v3",
            feature = "plugin-api-v4",
            feature = "plugin-api-v5"
        )))]
        let error_value: bool = false;

        if result == error_value {
            Err(Error::RegisterWriteError {
                name: self.name.clone(),
            })
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "num-traits")]
    /// Read a register value into a numeric type in big-endian byte order
    ///
    /// This must only be called in a callback which has been registered with
    /// `CallbackFlags::QEMU_PLUGIN_CB_R_REGS` or
    /// `CallbackFlags::QEMU_PLUGIN_CB_RW_REGS`.
    pub fn read_be<T>(&self) -> Result<T>
    where
        T: PrimInt + FromBytes + Sized,
        T: FromBytes<Bytes = [u8; std::mem::size_of::<T>()]>,
    {
        let data = self.read()?;
        let mut bytes = [0; std::mem::size_of::<T>()];
        bytes.copy_from_slice(&data);
        Ok(T::from_be_bytes(&bytes))
    }

    #[cfg(feature = "num-traits")]
    /// Read a register value into a numeric type in little-endian byte order
    ///
    /// This must only be called in a callback which has been registered with
    /// `CallbackFlags::QEMU_PLUGIN_CB_R_REGS` or
    /// `CallbackFlags::QEMU_PLUGIN_CB_RW_REGS`.
    pub fn read_le<T>(&self) -> Result<T>
    where
        T: PrimInt + FromBytes + Sized,
        T: FromBytes<Bytes = [u8; std::mem::size_of::<T>()]>,
    {
        let data = self.read()?;
        let mut bytes = [0; std::mem::size_of::<T>()];
        bytes.copy_from_slice(&data);
        Ok(T::from_le_bytes(&bytes))
    }
}
