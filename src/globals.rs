use ahash::AHashMap;
use atomic_float::AtomicF32;
use parking_lot::Mutex;
use std::sync::OnceLock;
use windows::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32;
use zstring::ZString;

/// Safe wrapper around MODULEENTRY32.
pub struct SafeMODULEENTRY32(pub MODULEENTRY32);
thread_safe_structs!(SafeMODULEENTRY32);

/// Cached process modules.
pub static MODULES: OnceLock<AHashMap<String, SafeMODULEENTRY32>> = OnceLock::new();

/// Logged screen (and stdout) messages.
pub static LOGGED_MESSAGES: OnceLock<Mutex<ZString>> = OnceLock::new();

/// Last-set delta time.
pub static DELTA_TIME: AtomicF32 = AtomicF32::new(0.0);
