use crate::winutils::WinUtils;
use ahash::AHashMap;
use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use std::sync::{
    atomic::{AtomicBool, AtomicI64},
    LazyLock,
};
use windows::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32;
use zstring::ZString;

/// Safe wrapper around MODULEENTRY32.
pub struct SafeMODULEENTRY32(pub MODULEENTRY32);
thread_safe_structs!(SafeMODULEENTRY32);

/// ImGui mutable context pointer.
/// Safety doesn't exist, it's only intended for accessing the colors slice, nothing else.
pub static CONTEXT_PTR: AtomicI64 = AtomicI64::new(0);

/// Cached process modules.
pub static MODULES: LazyLock<AHashMap<String, SafeMODULEENTRY32>> =
    LazyLock::new(WinUtils::get_modules_no_cache);

/// Logged screen (and stdout) messages.
pub static LOGGED_MESSAGES: LazyLock<AtomicRefCell<ZString>> = LazyLock::new(Default::default);

/// Last-set delta time.
pub static DELTA_TIME: AtomicF32 = AtomicF32::new(0.0);

/// Is the cursor inside of an UI window?
/// Global because tracking this across Rune will turn into a mess.
pub static IS_CURSOR_IN_UI: AtomicBool = AtomicBool::new(false);
