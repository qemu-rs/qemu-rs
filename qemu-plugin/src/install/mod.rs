//! Installation for the QEMU plugin

use crate::qemu_plugin_bool_parse;
use qemu_plugin_sys::{
    QEMU_PLUGIN_VERSION, qemu_info_t, qemu_info_t__bindgen_ty_1,
    qemu_info_t__bindgen_ty_2__bindgen_ty_1, qemu_plugin_id_t,
};
use std::{
    collections::HashMap,
    ffi::{CStr, c_char, c_int},
};

use crate::{error::Error, plugin::PLUGIN};

#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
/// The version of the plugin API that this plugin is compatible with
pub static qemu_plugin_version: c_int = QEMU_PLUGIN_VERSION as c_int;

/// Code returned from `qemu_plugin_install` to indicate successful installation
pub const PLUGIN_INSTALL_SUCCESS: c_int = 0;

#[derive(Debug, Clone)]
/// A value passed to a QEMU plugin via the command line, either as a boolean,
/// integer, or string. Booleans are parsed using the `qemu_plugin_bool_parse`
/// function, integers are parsed from strings, and strings are taken as-is.
pub enum Value {
    /// A boolean argument to a QEMU plugin, for example `val=true` or `val=on`
    /// see https://www.qemu.org/docs/master/devel/tcg-plugins.html#c.qemu_plugin_bool_parse
    Bool(bool),
    /// An integer argument to a QEMU plugin, for example `val=1`
    Integer(i64),
    /// A string argument to a QEMU plugin, for example `val=foo`
    String(String),
}

impl Value {
    fn new(key: &str, value: &str) -> Result<Self, Error> {
        if let Ok(maybe_bool) = qemu_plugin_bool_parse(key, value) {
            Ok(Self::Bool(maybe_bool))
        } else if let Ok(int) = value.parse::<i64>() {
            Ok(Self::Integer(int))
        } else {
            Ok(Self::String(value.to_string()))
        }
    }
}

#[derive(Debug, Clone)]
/// Arguments to QEMU as passed to `qemu_plugin_install`. `qemu_plugin_install`
/// takes a comma-separated list of key=value pairs, such as
/// `val1=foo,val2=bar`.
pub struct Args {
    /// Arguments to the QEMU plugin as passed in by QEMU. Each entry is a
    /// key=value pair where the key is the name of the argument and the value
    /// is the value of the argument.
    pub raw: Vec<String>,
    /// Arguments to the QEMU plugin, parsed into valid argument types and value
    /// types. Each key is the name of the argument and the value is a `Value`
    /// enum which can be a boolean, integer, or string.
    pub parsed: HashMap<String, Value>,
}

impl Args {
    /// Create a new QEMU `Args` container from the raw arguments passed to the plugin on the
    /// command line
    fn new(argc: c_int, value: *const *const c_char) -> Result<Self, Error> {
        Ok(Self {
            raw: (0..argc)
                .map(|i| unsafe { CStr::from_ptr(*value.offset(i as isize)) })
                .map(|cstr| cstr.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            parsed: (0..argc)
                .map(|i| unsafe { CStr::from_ptr(*value.offset(i as isize)) })
                .map(|cstr| cstr.to_string_lossy().into_owned())
                .map(|argument| {
                    let mut split = argument.splitn(2, '=');
                    let Some(key) = split.next() else {
                        return Err(Error::MissingArgKey { argument });
                    };
                    let Some(value) = split.next() else {
                        return Err(Error::MissingArgValue { argument });
                    };
                    Ok((key.to_string(), Value::new(key, value)?))
                })
                .collect::<Result<Vec<(_, _)>, Error>>()?
                .into_iter()
                .collect::<HashMap<_, _>>(),
        })
    }
}

#[derive(Debug, Clone)]
/// The version specification of the QEMU plugin API
pub struct Version {
    /// Current plugin API version
    pub current: i64,
    /// Minimum plugin API version
    pub minimum: i64,
}

impl From<&qemu_info_t__bindgen_ty_1> for Version {
    fn from(value: &qemu_info_t__bindgen_ty_1) -> Self {
        Self {
            current: value.cur as i64,
            minimum: value.min as i64,
        }
    }
}

#[derive(Debug, Clone)]
/// Information about the virtualized system, present if the emulator is running
/// in full system emulation mode
pub struct System {
    /// The maximum number of virtual CPUs supported by the system
    pub max_vcpus: i64,
    /// The number of virtual CPUs currently configured
    pub smp_vcpus: i64,
}

impl From<&qemu_info_t__bindgen_ty_2__bindgen_ty_1> for System {
    fn from(value: &qemu_info_t__bindgen_ty_2__bindgen_ty_1) -> Self {
        Self {
            max_vcpus: value.max_vcpus as i64,
            smp_vcpus: value.smp_vcpus as i64,
        }
    }
}

#[derive(Debug, Clone)]
/// Information about the simulation, including the target name, version, and virtual
/// system information
pub struct Info {
    /// The target name of the simulation (e.g. `x86_64-softmmu`)
    pub target_name: String,
    /// The minimum and current plugin API version
    pub version: Version,
    /// Information about the system, if the emulator is running in full system
    /// emulation mode. If `None`, the emulator is running in user mode
    pub system: Option<System>,
}

impl Info {
    /// # Safety
    ///
    /// This method should only called by QEMU inside the `qemu_plugin_install` function
    /// when the plugin is loaded. The `value` pointer is a valid pointer to a
    /// `qemu_info_t` struct which is live for the duration of the `qemu_plugin_install`
    /// function.
    unsafe fn try_from(value: *const qemu_info_t) -> Result<Self, Error> {
        let target_name = unsafe { CStr::from_ptr((*value).target_name) }
            .to_str()
            .map_err(Error::from)?
            .to_string();
        let version = Version::from(unsafe { &(*value).version });
        let system_emulation = unsafe { (*value).system_emulation };
        let system = if system_emulation {
            // NOTE: This is safe because `system_emulation` is true, which means the
            // `system` field is valid
            Some(System::from(unsafe { &(*value).__bindgen_anon_1.system }))
        } else {
            None
        };

        Ok(Self {
            target_name,
            version,
            system,
        })
    }
}

#[unsafe(no_mangle)]
/// Called by QEMU when the plugin is loaded
///
/// # Safety
///
/// This function is called by QEMU when the plugin is loaded, and should not be called
/// by dependent code. The `info` pointer is valid for the duration of the function
/// call, and must not be accessed after the function returns. `argv` remains valid for
/// the duration of the plugin's lifetime.
pub unsafe extern "C" fn qemu_plugin_install(
    id: qemu_plugin_id_t,
    info: *const qemu_info_t,
    argc: c_int,
    argv: *const *const c_char,
) -> c_int {
    let args = Args::new(argc, argv).expect("Failed to parse arguments");
    let info = unsafe { Info::try_from(info) }.expect("Failed to convert qemu_info_t");

    let Some(plugin) = PLUGIN.get() else {
        panic!("Plugin not set");
    };

    let Ok(mut plugin) = plugin.lock() else {
        panic!("Failed to lock plugin");
    };

    plugin
        .register_default(id, &args, &info)
        .expect("Failed to register plugin");

    PLUGIN_INSTALL_SUCCESS
}
