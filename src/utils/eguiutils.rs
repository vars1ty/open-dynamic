use super::crosscom::CrossCom;
use crate::{
    globals::LOGGED_MESSAGES,
    utils::{
        colorutils::ColorUtils,
        config::Config,
        extensions::{OptionExt, ResultExtensions},
    },
    winutils::WinUtils,
};
use dashmap::DashMap;
use hudhook::imgui::{self, internal::DataTypeKind, sys::*, *};
use parking_lot::RwLock;
use std::{
    fs::File,
    sync::{Arc, LazyLock},
};
use windows::Win32::Foundation::POINT;

#[derive(Clone, Copy)]
pub struct CustomTexture {
    pub texture_type: CustomTextureType,
    pub texture_id: Option<TextureId>,
}

/// Type for `CustomTexture`.
#[derive(Clone, Copy, PartialEq)]
pub enum CustomTextureType {
    Singular,
    Gif,
    GifFrame,
}

#[derive(Default)]
pub struct ContentFrameData {
    /// Logged titles of content frames for the current category.
    pub titles: Vec<String>,
}

/// Highly experimental ImGui Utils.
pub struct ImGuiUtils {
    /// Enable side messages?
    pub enable_side_messages: bool,

    /// A map to keep track of the custom-added fonts, so we can use the relative path to identify
    /// them.
    pub fonts: DashMap<Arc<String>, usize>,
}

impl ImGuiUtils {
    pub fn new() -> Self {
        Self {
            enable_side_messages: true,
            fonts: DashMap::new(),
        }
    }

    /// Applies a custom font.
    fn apply_font(
        &self,
        ctx: &mut imgui::Context,
        config: &Config,
        crosscom: Arc<RwLock<CrossCom>>,
    ) {
        let (normal_font_bytes, bold_font_bytes) = crosscom
            .try_read()
            .unwrap_or_crash(zencstr!(
                "[ERROR] CrossCom is locked, fonts can't be loaded from server!"
            ))
            .get_fonts();

        // https://github.com/ryanoasis/nerd-fonts/wiki/Glyph-Sets-and-Code-Points
        let glyph_ranges = imgui::FontGlyphRanges::from_slice(&[0xf0001, 0xf1af0, 0x1, 0x1FFFF, 0]);
        let (oversample_h, oversample_v) = (4, 4);

        ctx.fonts().add_font(&[FontSource::TtfData {
            data: &normal_font_bytes,
            size_pixels: config.get_main_font_size(),
            config: Some(imgui::FontConfig {
                oversample_h,
                oversample_v,
                glyph_ranges: glyph_ranges.to_owned(),
                ..imgui::FontConfig::default()
            }),
        }]);
        ctx.fonts().add_font(&[FontSource::TtfData {
            data: &normal_font_bytes,
            size_pixels: config.get_header_font_size(),
            config: Some(imgui::FontConfig {
                oversample_h,
                oversample_v,
                glyph_ranges: glyph_ranges.to_owned(),
                ..imgui::FontConfig::default()
            }),
        }]);
        ctx.fonts().add_font(&[FontSource::TtfData {
            data: &bold_font_bytes,
            size_pixels: config.get_main_font_size(),
            config: Some(imgui::FontConfig {
                oversample_h,
                oversample_v,
                glyph_ranges: glyph_ranges.to_owned(),
                ..imgui::FontConfig::default()
            }),
        }]);

        drop(normal_font_bytes);
        drop(bold_font_bytes);

        // Load custom fonts if any are defined.
        let Some(custom_fonts) = config.get_fonts().take() else {
            return;
        };

        for (relative_font_path, font_size) in custom_fonts {
            ctx.fonts().add_font(&[FontSource::TtfData {
                data: &config
                    .get_file_content_bytes(relative_font_path)
                    .dynamic_expect(zencstr!(
                        "Failed reading Font Data from config.jsonc -> fonts -> ",
                        relative_font_path
                    )),
                size_pixels: font_size,
                config: Some(imgui::FontConfig {
                    oversample_h: 4,
                    oversample_v: 4,
                    glyph_ranges: glyph_ranges.to_owned(),
                    ..imgui::FontConfig::default()
                }),
            }]);

            log!(
                "[FONTS] Installed Font from relative path \"",
                relative_font_path,
                "\", size: ",
                font_size,
                "."
            );
            self.fonts.insert(
                Arc::new(relative_font_path.to_owned()),
                ctx.fonts().fonts().len() - 1,
            );
        }
    }

    /// Applies the custom theme.
    pub fn apply_style(
        &self,
        ctx: &mut imgui::Context,
        default_style: &mut Style,
        config: &Config,
        crosscom: Arc<RwLock<CrossCom>>,
    ) {
        self.apply_font(ctx, config, crosscom);
        let style = ctx.style_mut();
        style.window_title_align = [0.5, 0.5]; // Center
        style.window_rounding = 4.0;
        style.frame_rounding = 6.0;

        const MAIN_DARK: [f32; 4] = ColorUtils::rgba_to_frgba([10, 10, 10, 255]);
        const MAIN_DARK_GRAY: [f32; 4] = ColorUtils::rgba_to_frgba([20, 20, 20, 255]);
        const MAIN_DARKISH_GRAY: [f32; 4] = ColorUtils::rgba_to_frgba([25, 25, 25, 255]);

        const WHITE_FULL: [f32; 4] = ColorUtils::rgba_to_frgba([255, 255, 255, 255]);
        const WHITE_ALMOST_FULL: [f32; 4] = ColorUtils::rgba_to_frgba([255, 255, 255, 230]);
        const WHITE_HINT: [f32; 4] = ColorUtils::rgba_to_frgba([255, 255, 255, 220]);
        const WHITE_SLIGHT_HINT: [f32; 4] = ColorUtils::rgba_to_frgba([255, 255, 255, 200]);

        let mut colors = style.colors;
        // Main canvas
        colors[ImGuiCol_WindowBg as usize] = MAIN_DARK;

        // Title background
        colors[ImGuiCol_TitleBg as usize] = MAIN_DARK;
        colors[ImGuiCol_TitleBgActive as usize] = MAIN_DARKISH_GRAY;
        colors[ImGuiCol_TitleBgCollapsed as usize] = MAIN_DARKISH_GRAY;

        // Frame
        colors[ImGuiCol_FrameBg as usize] = MAIN_DARK_GRAY;
        colors[ImGuiCol_FrameBgHovered as usize] = WHITE_HINT;
        colors[ImGuiCol_FrameBgActive as usize] = WHITE_SLIGHT_HINT;

        // Scrollbar
        colors[ImGuiCol_ScrollbarGrab as usize] = WHITE_HINT;
        colors[ImGuiCol_ScrollbarGrabHovered as usize] = WHITE_ALMOST_FULL;
        colors[ImGuiCol_ScrollbarGrabActive as usize] = WHITE_FULL;

        // Button
        colors[ImGuiCol_Button as usize] = WHITE_HINT;
        colors[ImGuiCol_ButtonHovered as usize] = WHITE_ALMOST_FULL;
        colors[ImGuiCol_ButtonActive as usize] = WHITE_FULL;

        // Tab
        colors[ImGuiCol_Tab as usize] = WHITE_HINT;
        colors[ImGuiCol_TabHovered as usize] = WHITE_ALMOST_FULL;

        // Checkmark
        colors[ImGuiCol_CheckMark as usize] = WHITE_HINT;

        // Slider
        colors[ImGuiCol_SliderGrab as usize] = WHITE_HINT;
        colors[ImGuiCol_SliderGrabActive as usize] = WHITE_SLIGHT_HINT;

        // Resize
        colors[ImGuiCol_ResizeGrip as usize] = MAIN_DARKISH_GRAY;
        colors[ImGuiCol_ResizeGripHovered as usize] = MAIN_DARK_GRAY;
        colors[ImGuiCol_ResizeGripActive as usize] = MAIN_DARK;

        // Drop Down
        colors[ImGuiCol_Header as usize] = WHITE_HINT;
        colors[ImGuiCol_HeaderHovered as usize] = WHITE_ALMOST_FULL;
        colors[ImGuiCol_HeaderActive as usize] = WHITE_FULL;

        style.colors = colors;
        *default_style = *style;
    }

    /// Draws a virtual software cursor.
    pub fn render_software_cursor(ui: &imgui::Ui, point: &mut POINT) {
        if !ui.io().want_capture_mouse {
            return;
        }

        static WHITE: [f32; 4] = [1.0; 4];

        WinUtils::get_cursor_pos_recycle(point);
        let draw_list = ui.get_foreground_draw_list();
        let mouse_pos = [point.x as f32, point.y as f32];
        draw_list
            .add_rect(mouse_pos, [mouse_pos[0] + 5.0, mouse_pos[1] + 5.0], WHITE)
            .filled(true)
            .rounding(5.0)
            .build();
    }

    /// Draws widgets centered using a `Group` widget.
    /// Returns the group size which you should cache.
    pub fn draw_centered_widgets<F: FnOnce()>(
        ui: &imgui::Ui,
        center_around: [f32; 2],
        group_size: &[f32; 2],
        custom_y: Option<f32>,
        draw_widgets: F,
    ) -> [f32; 2] {
        // Y Position.
        // If custom_y is specified, that value is used.
        // If not, the center of the window is used.
        let y_pos = custom_y.unwrap_or_else(|| center_around[1] / 2.0);

        // Set to draw at the center, with the group box X-coordinate taken into
        // consideration after it has been rendered and cached.
        ui.set_cursor_pos([(center_around[0] - group_size[0]) / 2.0, y_pos]);

        // Draw the group widget.
        ui.group(draw_widgets);

        // Store the size so that we can use it in the next pass, where we then position the group
        // widget.
        ui.item_rect_size()
    }

    /// Activates a font by its index, panics if not found.
    /// Returns the font stack token, which you have to `pop()` after you are done using the font.
    pub fn activate_font(ui: &imgui::Ui, font_id: usize) -> Option<FontStackToken<'_>> {
        let fonts = ui.fonts();
        let font = fonts.get_font(fonts.fonts()[font_id])?;
        Some(ui.push_font(font.id()))
    }

    /// Tries to find the custom font by its relative path, returning the index to be used with
    /// `Self::activate_font`.
    pub fn get_cfont_from_rpath(&self, relative_font_path: Arc<String>) -> usize {
        *self
            .fonts
            .get(&relative_font_path)
            .unwrap_or_crash(zencstr!(
                "[ERROR] No Font has been instantiated with the relative path of \"",
                relative_font_path,
                "\"!"
            ))
    }

    /// Draws the top-left screen messages.
    pub fn draw_screen_messages(&self, ui: &imgui::Ui) {
        if !self.enable_side_messages {
            return;
        }

        let Ok(logged_messages) = LOGGED_MESSAGES.try_borrow() else {
            return;
        };

        let draw = ui.get_background_draw_list();
        static DRAW_POS: [f32; 2] = [0.0, 100.0];
        static WHITE: [f32; 4] = [1.0; 4];
        draw.add_text(DRAW_POS, WHITE, &*logged_messages);
    }

    /// Draws an image onto the UI in form of a `image_button` without any styling but the image
    /// itself.
    /// Returns `true` if pressed.
    pub fn draw_image<S: AsRef<str>>(
        ui: &imgui::Ui,
        identifier: S,
        width: f32,
        height: f32,
        image_texture_id: TextureId,
    ) -> bool {
        static TRANSPARENT: [f32; 4] = [0.0; 4];

        // Erase the button colors just for this one occasion.
        let button_style = ui.push_style_color(StyleColor::Button, TRANSPARENT);
        let button_active_style = ui.push_style_color(StyleColor::ButtonActive, TRANSPARENT);
        let button_hovered_style = ui.push_style_color(StyleColor::ButtonHovered, TRANSPARENT);

        // Draw button, the only colors present are inherited from the image.
        let res = ui.image_button(identifier, image_texture_id, [width, height]);

        // Restore colors.
        button_style.pop();
        button_active_style.pop();
        button_hovered_style.pop();

        res
    }

    /// Adds a slider to the `ui` which automatically clamps the value between `min` and `max`,
    /// which `ui.slider()` does not do by default.
    pub fn slider<S: AsRef<str>, N: Default + Clone + Copy + DataTypeKind + PartialOrd>(
        ui: &imgui::Ui,
        text: S,
        min: N,
        max: N,
        output: &mut N,
    ) -> bool {
        let result = ui.slider(text, min, max, output);

        // Clamp the output value.
        if *output < min {
            *output = min;
        } else if *output > max {
            *output = max;
        }

        result
    }

    /// Draws a frame with a title and content within it. Then a border surrounding the content.
    pub fn draw_content_frame<C: FnOnce()>(
        ui: &imgui::Ui,
        title: &str,
        content_frame_data: &mut ContentFrameData,
        content: C,
    ) {
        // A vector of pre-allocated Strings, so we don't always re-allocate every frame for
        // ui.columns.
        static INDEX_STRINGS: LazyLock<Vec<String>> = LazyLock::new(|| {
            let mut vec = Vec::with_capacity(32);
            for i in 0..vec.capacity() {
                vec.insert(i, i.to_string());
            }

            vec
        });
        static FRAMES_PER_COLUMN: usize = 3;

        // This padding is applied both left and right of the content group. If the rows are
        // stacked, it's then also used to add a bit of vertical space between them.
        static PADDING: f32 = 5.0;

        {
            let title_heap = title.to_owned();
            if !content_frame_data.titles.contains(&title_heap) {
                content_frame_data.titles.push(title_heap);
            }
        }

        let current_index = content_frame_data
            .titles
            .iter()
            .position(|found_title| *found_title == title)
            .unwrap_or_default();

        if current_index % FRAMES_PER_COLUMN == 0 {
            // If it isn't the first item, don't end the column as there is none prior to that.
            if current_index != 0 {
                ui.columns(1, "", false);
            }

            ui.columns(
                FRAMES_PER_COLUMN as i32,
                &INDEX_STRINGS[current_index],
                false,
            );
        }

        ui.group(|| {
            // Future Note: If centered title text is of interest, bring back
            // content_frame_data and cache the group size so we can properly center out label.
            ui.text(title);

            let current_pos = ui.cursor_pos();
            ui.set_cursor_pos([current_pos[0] + PADDING, current_pos[1]]);
            ui.group(|| {
                content();
                ui.dummy([0.0, 1.0]);
            });
        });

        let width = ui.item_rect_size()[0] + 20.0;
        let max = ui.item_rect_max();
        ui.get_window_draw_list()
            .add_rect(ui.item_rect_min(), [max[0] + PADDING, max[1]], [1.0; 4])
            .filled(false)
            .rounding(3.0)
            .build();

        ui.set_column_width(ui.current_column_index(), width);
        ui.next_column();
    }

    /// Syncs the Windows clipboard back to ImGui.
    /// This is a hack and only needed due to Hudhook being broken with the clipboard.
    /// For Linux, this won't work with content that has been copied from outside a Wine
    /// application.
    /// **Note**: This should be called every frame and will **only** sync the clipboard if CTRL+V
    /// has been pressed.
    pub fn sync_clipboard(ui: &imgui::Ui) {
        if !ui.is_key_down(Key::V) || !ui.io().key_ctrl {
            return;
        }

        let Ok(text) = clipboard_win::get_clipboard_string() else {
            return;
        };

        ui.set_clipboard_text(text);
    }

    /// Extracts the frames of a GIF into a `Vec<gif::Frame>`.
    pub fn extract_gif_frames(path: &str) -> Vec<gif::Frame> {
        let input = File::open(path).dynamic_expect(zencstr!("Failed opening GIF"));
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = options.read_info(input).unwrap();
        let mut frames = vec![];
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frames.push(frame.to_owned());
        }

        frames
    }
}
