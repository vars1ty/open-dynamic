use std::ffi::CStr;

use hudhook::imgui::TextureId;

use crate::utils::colorutils::ColorUtils;

/// Exposed ImGui functions for use from memory function calling.
/// No safety is promised, use at your own risk. All functions are C-Style.
pub struct MemGui;

type UI = hudhook::imgui::Ui;

impl MemGui {
    /// Tries to get the C-String from `ptr` as a `&'static str`.
    /// **Highly unsafe and may crash!**
    fn cstr_to_str(ptr: *const u8) -> Option<&'static str> {
        unsafe { CStr::from_ptr(ptr as _).to_str() }.ok()
    }

    #[no_mangle]
    pub extern "C" fn ui_text(ui: &UI, text: *const u8) {
        let Some(text) = Self::cstr_to_str(text) else {
            return;
        };

        ui.text(text);
    }

    #[no_mangle]
    pub extern "C" fn ui_add_rect(
        ui: &UI,
        surface_type: u8,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        filled: bool,
    ) {
        const WINDOW_LIST: u8 = 0;
        const BACKGROUND_LIST: u8 = 1;
        const FOREGROUND_LIST: u8 = 2;

        // No support for calling with a f32/f64 parameter, hence casting.
        let from = [from_x as f32, from_y as f32];
        let to = [to_x as f32, to_y as f32];
        let color = ColorUtils::rgba_to_frgba([r, g, b, a]);
        match surface_type {
            WINDOW_LIST => ui.get_window_draw_list().add_rect(from, to, color).filled(filled).build(),
            BACKGROUND_LIST => ui
                .get_background_draw_list()
                .add_rect(from, to, color).filled(filled)
                .build(),
            FOREGROUND_LIST => ui.get_foreground_draw_list().add_rect(from, to, color).filled(filled).build(),
            _ => crash!("[ERROR] Tried calling ui_add_rect with surface_type ", surface_type, ". Only 0 [WINDOW_LIST], 1 [BACKGROUND_LIST] and 2 [FOREGROUND_LIST] are supported!"),
        }
    }

    #[no_mangle]
    pub extern "C" fn ui_add_text(
        ui: &UI,
        surface_type: u8,
        text: *const u8,
        x: i32,
        y: i32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) {
        let Some(text) = Self::cstr_to_str(text) else {
            return;
        };

        const WINDOW_LIST: u8 = 0;
        const BACKGROUND_LIST: u8 = 1;
        const FOREGROUND_LIST: u8 = 2;

        // No support for calling with a f32/f64 parameter, hence casting.
        let pos = [x as f32, y as f32];
        let color = ColorUtils::rgba_to_frgba([r, g, b, a]);
        match surface_type {
            WINDOW_LIST => ui.get_window_draw_list().add_text(pos, color, text),
            BACKGROUND_LIST => ui
                .get_background_draw_list()
                .add_text(pos, color, text),
            FOREGROUND_LIST => ui.get_foreground_draw_list().add_text(pos, color, text),
            _ => crash!("[ERROR] Tried calling ui_add_text with surface_type ", surface_type, ". Only 0 [WINDOW_LIST], 1 [BACKGROUND_LIST] and 2 [FOREGROUND_LIST] are supported!"),
        }
    }

    #[no_mangle]
    pub extern "C" fn ui_cursor_pos_x(ui: &UI) -> i32 {
        ui.cursor_pos()[0] as i32
    }

    #[no_mangle]
    pub extern "C" fn ui_cursor_pos_y(ui: &UI) -> i32 {
        ui.cursor_pos()[1] as i32
    }

    #[no_mangle]
    pub extern "C" fn ui_set_cursor_pos(ui: &UI, x: i32, y: i32) {
        ui.set_cursor_pos([x as f32, y as f32]);
    }

    #[no_mangle]
    pub extern "C" fn ui_cursor_screen_pos_x(ui: &UI) -> i32 {
        ui.cursor_screen_pos()[0] as i32
    }

    #[no_mangle]
    pub extern "C" fn ui_cursor_screen_pos_y(ui: &UI) -> i32 {
        ui.cursor_screen_pos()[1] as i32
    }

    #[no_mangle]
    pub extern "C" fn ui_set_cursor_screen_pos(ui: &UI, x: i32, y: i32) {
        ui.set_cursor_screen_pos([x as f32, y as f32]);
    }

    #[no_mangle]
    pub extern "C" fn ui_button(ui: &UI, text: *const u8) -> bool {
        let size = [0.0, 0.0];
        let text_color =
            ui.push_style_color(hudhook::imgui::StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
        let result = unsafe { hudhook::imgui::sys::igButton(text as _, size.into()) };
        text_color.pop();
        result
    }

    #[no_mangle]
    pub extern "C" fn ui_add_image(
        ui: &UI,
        surface_type: u8,
        texture_id: usize,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    ) {
        let texture_id = TextureId::new(texture_id);
        let min = [min_x as f32, min_y as f32];
        let max = [max_x as f32, max_y as f32];

        const WINDOW_LIST: u8 = 0;
        const BACKGROUND_LIST: u8 = 1;
        const FOREGROUND_LIST: u8 = 2;

        match surface_type {
            WINDOW_LIST => ui.get_window_draw_list().add_image(texture_id, min, max).build(),
            BACKGROUND_LIST => ui.get_background_draw_list().add_image(texture_id, min, max).build(),
            FOREGROUND_LIST => ui.get_foreground_draw_list().add_image(texture_id, min, max).build(),
            _ => crash!("[ERROR] Tried calling ui_add_image with surface_type ", surface_type, ". Only 0 [WINDOW_LIST], 1 [BACKGROUND_LIST] and 2 [FOREGROUND_LIST] are supported!"),
        }
    }

    #[no_mangle]
    pub extern "C" fn ui_same_line(ui: &UI) {
        ui.same_line();
    }
}
