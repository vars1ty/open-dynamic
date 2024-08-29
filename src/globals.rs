use ahash::AHashMap;
use atomic_float::AtomicF32;
use parking_lot::{Mutex, RwLock};
use std::sync::OnceLock;
use windows::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32;
use zstring::ZString;

/// Safe wrapper around MODULEENTRY32.
pub struct SafeMODULEENTRY32(pub MODULEENTRY32);
thread_safe_structs!(SafeMODULEENTRY32);

/// Cached process modules.
pub static MODULES: OnceLock<AHashMap<String, SafeMODULEENTRY32>> = OnceLock::new();

// Keys that should be used with loops in scripts. If a value is false, the loop should stop.
#[deprecated = "This shouldn't be static, it should be moved to ScriptCore."]
pub static SCRIPTING_THREAD_KEYS: OnceLock<RwLock<AHashMap<String, bool>>> = OnceLock::new();

/// Logged screen (and stdout) messages.
pub static LOGGED_MESSAGES: OnceLock<Mutex<ZString>> = OnceLock::new();

/// Last-set delta time.
pub static DELTA_TIME: AtomicF32 = AtomicF32::new(0.0);
