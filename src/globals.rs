use crate::utils::eguiutils::ImGuiUtils;
use ahash::AHashMap;
use parking_lot::{Mutex, RwLock};
use zstring::ZString;
use std::sync::{Arc, OnceLock};
use windows::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32;

/// Basic macro for generating a static mutable reference.
macro_rules! public_static_mut {
    ($identifier:ident, $type:ty, $value:expr, $doc:literal) => {
        #[doc=$doc]
        pub static mut $identifier: $type = $value;
    };
    ($identifier:ident, $type:ty, $doc:literal) => {
        #[doc=$doc]
        pub static mut $identifier: $type = None;
    };
}

/// Safe wrapper around MODULEENTRY32.
pub struct SafeMODULEENTRY32(pub MODULEENTRY32);
thread_safe_structs!(SafeMODULEENTRY32);

/// Global static `ImGuiUtils` instance.
pub static IMGUI_UTILS: OnceLock<Arc<RwLock<ImGuiUtils>>> = OnceLock::new();

/// Cached process modules.
pub static MODULES: OnceLock<AHashMap<String, SafeMODULEENTRY32>> = OnceLock::new();

// Keys that should be used with loops in scripts. If a value is false, the loop should stop.
pub static SCRIPTING_THREAD_KEYS: OnceLock<RwLock<AHashMap<String, bool>>> = OnceLock::new();

pub static LOGGED_MESSAGES: OnceLock<Mutex<ZString>> = OnceLock::new();

/// Legacy script parameters.
#[cfg(target_pointer_width = "32")]
pub static SSO_LEGACY_SCRIPT_PARMS: OnceLock<(i32, i32)> = OnceLock::new();

public_static_mut!(DELTA_TIME, f32, 0.0, "Current Delta Time.");
