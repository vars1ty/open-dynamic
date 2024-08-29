use crate::{utils::extensions::OptionExt, winutils::WinUtils};
use retour::static_detour;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

static_detour! {
    /// WinAPI SetCursorPos.
    static SetCursorPosHook: unsafe extern "system" fn(i32, i32) -> bool;
}

/// Generic hooks which are enabled for all games.
pub struct GenericHoooks {
    /// If true, hinders all `SetCursorPos` calls from being accepted.
    pub disable_set_cursor_pos: Arc<AtomicBool>,
}

impl GenericHoooks {
    /// Initializes the hooks.
    pub fn init() -> Self {
        let disable_set_cursor_pos = Arc::new(AtomicBool::new(false));
        let disable_set_cursor_pos_clone = Arc::clone(&disable_set_cursor_pos);

        enable_hook!(
            SetCursorPosHook,
            WinUtils::get_module_symbol_address(zencstr!("user32.dll"), c"SetCursorPos")
                .unwrap_or_crash(zencstr!(
                "[ERROR] Can't get the user32 SetCursorPos address, what have you done to your PC?"
            )),
            move |x, y| Self::set_cursor_pos(x, y, Arc::clone(&disable_set_cursor_pos_clone)),
            "user32 SetCursorPos"
        );

        Self {
            disable_set_cursor_pos,
        }
    }

    /// SetCursorPos hook.
    /// TODO: Migrate over to RawDetour or GenericDetour.
    fn set_cursor_pos(x: i32, y: i32, disable_cursor_pos: Arc<AtomicBool>) -> bool {
        unsafe {
            if disable_cursor_pos.load(Ordering::SeqCst) {
                return false;
            }

            SetCursorPosHook.call(x, y)
        }
    }
}
