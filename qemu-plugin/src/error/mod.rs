//! Errors that can occur in the qemu-plugin crate

#[derive(thiserror::Error, Debug)]
/// An error from the qemu-plugin crate
pub enum Error {
    #[error("Missing key for argument {argument}")]
    /// Error when an argument is missing a key
    MissingArgKey {
        /// The argument string a key is missing for
        argument: String,
    },
    #[error("Missing value for argument {argument}")]
    /// Error when an argument is missing a value
    MissingArgValue {
        /// The argument string a value is missing for
        argument: String,
    },
    #[error("Invalid boolean value {name} ({val})")]
    /// Error when a boolean argument is invalid
    InvalidBool {
        /// The name of the key-value argument pair which does not correctly parse as boolean
        name: String,
        /// The value of the key-value argument pair which does not correctly parse as boolean
        val: String,
    },
    #[error(
        "Setting the QEMU plugin uninstall callback was attempted concurrently and this attempt failed."
    )]
    /// Error when the QEMU plugin uninstall callback is set concurrently
    ConcurrentPluginUninstallCallbackSet,
    #[error(
        "Setting the QEMU plugin reset callback was attempted concurrently and this attempt failed."
    )]
    /// Error when the QEMU plugin reset callback is set concurrently
    ConcurrentPluginResetCallbackSet,
    #[error("Invalid state for plugin reset callback")]
    /// Error when the plugin reset callback is in an invalid state
    PluginResetCallbackState,
    #[error("Invalid instruction index {index} for translation block of size {size}")]
    /// Error when an instruction index is invalid
    InvalidInstructionIndex {
        /// The index into the translation block that is invalid
        index: usize,
        /// The size of the translation block
        size: usize,
    },
    #[error("No disassembly string available for instruction")]
    /// Error when no disassembly string is available for an instruction (i.e. NULL string
    NoDisassemblyString,
    #[error("Invalid size {size} for read of register {name}")]
    /// Error when the size of a register read is invalid
    InvalidRegisterReadSize {
        /// The register name
        name: String,
        /// The size of the attempted read
        size: usize,
    },
    #[error("Error while reading register {name}")]
    /// Error when reading a register fails
    RegisterReadError {
        /// The register name
        name: String,
    },
    #[error("Error while writing register {name}")]
    /// Error when writing a register fails
    RegisterWriteError {
        /// The register name
        name: String,
    },
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3"
    )))]
    #[error("Error while reading {len} bytes from virtual address {addr:#x}")]
    /// Error when reading memory from a virtual address fails
    VaddrReadError {
        /// The address read from
        addr: u64,
        /// The number of bytes read
        len: u32,
    },
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4"
    )))]
    #[error("Error while writing {len} bytes to virtual address {addr:#x}")]
    /// Error when writing memory from a virtual address fails
    VaddrWriteError {
        /// The address written to
        addr: u64,
        /// The number of bytes written
        len: u32,
    },
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4"
    )))]
    #[error("Error while reading {len} bytes from hardware address {addr:#x}: {result}")]
    /// Error when reading memory from a hardware address fails
    HwaddrReadError {
        /// The address read from
        addr: u64,
        /// The number of bytes read
        len: u32,
        /// The operation result
        result: crate::HwaddrOperationResult,
    },
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4"
    )))]
    #[error("Error while writing {len} bytes to hardware address {addr:#x}: {result}")]
    /// Error when writing memory from a hardware address fails
    HwaddrWriteError {
        /// The address written to
        addr: u64,
        /// The number of bytes written
        len: u32,
        /// The operation result
        result: crate::HwaddrOperationResult,
    },
    #[cfg(not(any(
        feature = "plugin-api-v0",
        feature = "plugin-api-v1",
        feature = "plugin-api-v2",
        feature = "plugin-api-v3",
        feature = "plugin-api-v4"
    )))]
    #[error("Error while translating virtual address {vaddr:#x} to hardware address")]
    /// Error when translating a virtual address to a hardware address fails
    VaddrTranslateError {
        /// The virtual address that failed to translate
        vaddr: u64,
    },
    #[error(
        "Attempt to address element at offset {offset} in score board entry of size {entry_size}"
    )]
    /// Error when attempting to create a scoreboard entry with an illegal offset
    InvalidScoreBoardEntryOffset {
        /// Offset into the entry
        offset: usize,
        /// Size of the entry
        entry_size: usize,
    },
    #[error("Error while setting global plugin instance")]
    /// Error when setting the global plugin instance fails
    PluginInstanceSetError,
    #[error(transparent)]
    /// A transparently wrapped `std::str::Utf8Error`
    StdUtf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    /// A transparently wrapped `core::convert::Infallible`
    Infallible(#[from] core::convert::Infallible),
    #[error(transparent)]
    /// A transparently wrapped `std::alloc::LayoutError`
    LayoutError(#[from] std::alloc::LayoutError),
    #[error(transparent)]
    /// A transparently wrapped `std::array::TryFromSliceError`
    TryFromSliceError(#[from] std::array::TryFromSliceError),
    #[error(transparent)]
    /// A transparently wrapped `std::cell::BorrowError`
    BorrowError(#[from] std::cell::BorrowError),
    #[error(transparent)]
    /// A transparently wrapped `std::cell::BorrowMutError`
    BorrowMutError(#[from] std::cell::BorrowMutError),
    #[error(transparent)]
    /// A transparently wrapped `std::char::CharTryFromError`
    CharTryFromError(#[from] std::char::CharTryFromError),
    #[error(transparent)]
    /// A transparently wrapped `std::char::DecodeUtf16Error`
    DecodeUtf16Error(#[from] std::char::DecodeUtf16Error),
    #[error(transparent)]
    /// A transparently wrapped `std::char::ParseCharError`
    ParseCharError(#[from] std::char::ParseCharError),
    #[error(transparent)]
    /// A transparently wrapped `std::char::TryFromCharError`
    TryFromCharError(#[from] std::char::TryFromCharError),
    #[error(transparent)]
    /// A transparently wrapped `std::collections::TryReserveError`
    TryReserveError(#[from] std::collections::TryReserveError),
    #[error(transparent)]
    /// A transparently wrapped `std::env::JoinPathsError`
    JoinPathsError(#[from] std::env::JoinPathsError),
    #[error(transparent)]
    /// A transparently wrapped `std::env::VarError`
    VarError(#[from] std::env::VarError),
    #[error(transparent)]
    /// A transparently wrapped `std::ffi::FromBytesUntilNulError`
    FromBytesUntilNulError(#[from] std::ffi::FromBytesUntilNulError),
    #[error(transparent)]
    /// A transparently wrapped `std::ffi::FromBytesWithNulError`
    FromBytesWithNulError(#[from] std::ffi::FromBytesWithNulError),
    #[error(transparent)]
    /// A transparently wrapped `std::ffi::FromVecWithNulError`
    FromVecWithNulError(#[from] std::ffi::FromVecWithNulError),
    #[error(transparent)]
    /// A transparently wrapped `std::ffi::IntoStringError`
    IntoStringError(#[from] std::ffi::IntoStringError),
    #[error(transparent)]
    /// A transparently wrapped `std::ffi::NulError`
    NulError(#[from] std::ffi::NulError),
    #[error(transparent)]
    /// A transparently wrapped `std::fmt::Error`
    FmtError(#[from] std::fmt::Error),
    #[error(transparent)]
    /// A transparently wrapped `std::fs::TryLockError`
    FsTryLockError(#[from] std::fs::TryLockError),
    #[error(transparent)]
    /// A transparently wrapped `std::io::Error`
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    /// A transparently wrapped `std::net::AddrParseError`
    AddrParseError(#[from] std::net::AddrParseError),
    #[error(transparent)]
    /// A transparently wrapped `std::num::ParseFloatError`
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    /// A transparently wrapped `std::num::ParseIntError`
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    /// A transparently wrapped `std::num::TryFromIntError`
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    /// A transparently wrapped `std::path::StripPrefixError`
    StripPrefixError(#[from] std::path::StripPrefixError),
    #[error(transparent)]
    /// A transparently wrapped `std::str::ParseBoolError`
    ParseBoolError(#[from] std::str::ParseBoolError),
    #[error(transparent)]
    /// A transparently wrapped `std::string::FromUtf8Error`
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    /// A transparently wrapped `std::string::FromUtf16Error`
    FromUtf16Error(#[from] std::string::FromUtf16Error),
    #[error(transparent)]
    /// A transparently wrapped `std::sync::mpsc::RecvError`
    RecvError(#[from] std::sync::mpsc::RecvError),
    #[error(transparent)]
    /// A transparently wrapped `std::sync::mpsc::RecvTimeoutError`
    RecvTimeoutError(#[from] std::sync::mpsc::RecvTimeoutError),
    #[error(transparent)]
    /// A transparently wrapped `std::sync::mpsc::TryRecvError`
    TryRecvError(#[from] std::sync::mpsc::TryRecvError),
    #[error(transparent)]
    /// A transparently wrapped `std::thread::AccessError`
    AccessError(#[from] std::thread::AccessError),
    #[error(transparent)]
    /// A transparently wrapped `std::time::SystemTimeError`
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    /// A transparently wrapped `std::time::TryFromFloatSecsError`
    TryFromFloatSecsError(#[from] std::time::TryFromFloatSecsError),
    #[cfg(windows)]
    #[error(transparent)]
    /// A transparently wrapped `std::os::windows::io::InvalidHandleError`
    InvalidHandleError(#[from] std::os::windows::io::InvalidHandleError),
    #[cfg(windows)]
    #[error(transparent)]
    /// A transparently wrapped `std::os::windows::io::NullHandleError`
    NullHandleError(#[from] std::os::windows::io::NullHandleError),
    #[cfg(feature = "anyhow")]
    #[error(transparent)]
    /// A transparently wrapped `anyhow::Error`
    AnyhowError(#[from] anyhow::Error),
    #[error(transparent)]
    /// A transparently wrapped `Box<dyn std::error::Error>`
    BoxedError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[allow(dead_code)]
/// Assert that Error is Send + Sync
fn _assert_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
}

/// Result type for the qemu-plugin crate
pub type Result<T> = std::result::Result<T, Error>;
