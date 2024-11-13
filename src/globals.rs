use ahash::AHashMap;
use atomic_float::AtomicF32;
use parking_lot::Mutex;
use std::sync::{atomic::AtomicBool, OnceLock};
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

/// Is the cursor inside of an UI window?
/// Global because tracking this across Rune will turn into a mess.
pub static IS_CURSOR_IN_UI: AtomicBool = AtomicBool::new(true);
