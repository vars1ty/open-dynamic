use crate::{
    globals::{CONTEXT_PTR, DELTA_TIME, IS_CURSOR_IN_UI},
    mod_cores::base_core::BaseCore,
    ui::community::CommunityWindow,
    utils::{
        colorutils::ColorUtils,
        eguiutils::{CustomTexture, CustomTextureType, ImGuiUtils},
        extensions::{OptionExt, ResultExtensions},
    },
    winutils::WinUtils,
};
use hudhook::{
    imgui::{self, Condition, Context, Style, StyleColor, TreeNodeFlags},
    ImguiRenderLoop, RenderContext,
};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc, OnceLock},
};
use windows::Win32::Foundation::POINT;

/// Simple basic ImGui windows, responsible for also drawing custom windows.
pub struct DX11UI {
    /// `BaseCore` instance.
    base_core: Arc<RwLock<BaseCore>>,

    /// The code editor input.
    code_editor_input: String,

    /// Script Name.
    script_name: String,

    /// The community window.
    community_window: OnceLock<CommunityWindow>,

    /// Current cursor point.
    point: POINT,

    /// Should the UI be displayed?
    display_ui: bool,

    /// Can we toggle the UI on/off?
    can_toggle_ui: bool,

    /// Current CrossCom channel.
    crosscom_channel: String,

    /// Invalid textures that should be removed the next frame.
    invalid_textures: Vec<String>,

    /// Default dynamic ImGui style.
    default_style: Style,

    /// UI Colors preset input field.
    ui_colors_preset: String,
}

impl DX11UI {
    /// Returns an instance to `Self`.
    pub fn new(base_core: Arc<RwLock<BaseCore>>) -> Self {
        let reader = base_core
            .try_read()
            .unwrap_or_crash(zencstr!("[ERROR] Failed reading BaseCore"));
        let crosscom = reader.get_crosscom();
        let crosscom = crosscom
            .try_read()
            .unwrap_or_crash(zencstr!("[ERROR] Failed reading CrossCom"));
        let crosscom_channel = crosscom
            .get_current_channel()
            .try_borrow()
            .dynamic_expect(zencstr!("Failed borrowing crosscom.current_channel"))
            .to_owned();
        drop(reader);

        Self {
            base_core,
            code_editor_input: String::default(),
            script_name: String::with_capacity(24),
            community_window: OnceLock::new(),
            point: POINT::default(),
            display_ui: true,
            can_toggle_ui: true,
            crosscom_channel,
            invalid_textures: Vec::with_capacity(4),
            default_style: Style::default(),
            ui_colors_preset: String::default(),
        }
    }

    /// Executes Rune code.
    fn execute_rune_code(&self) {
        let Some(base_core_reader) = self.base_core.try_read() else {
            return;
        };

        // Execute the script.
        // CrossCom disabled for now when it comes to buttons, might be changed in the future.
        base_core_reader.get_script_core().execute(
            self.code_editor_input.to_owned(),
            Arc::clone(&self.base_core),
            false,
            false,
        );
    }

    /// Draws Save and Load management.
    fn draw_script_management(&mut self, ui: &imgui::Ui) {
        ui.input_text(zencstr!("󰈞 Script Name"), &mut self.script_name)
            .build();

        if self.script_name.is_empty() {
            return;
        }

        if button!(ui, "󰈝 Save Rune Script") {
            self.base_core
                .read()
                .get_config()
                .save_to_file(&self.script_name, &self.code_editor_input);
        }

        ui.same_line();
        if button!(ui, "󱇧 Load Rune Script")
            && self
                .base_core
                .read()
                .get_config()
                .get_file_content(&self.script_name, &mut self.code_editor_input)
        {
            self.script_name.clear();
        }
    }

    /// Toggles the UI on/off when the user press F5.
    fn on_toggle_ui(&mut self) {
        if WinUtils::is_key_down(&zencstr!("F5").data) {
            if !self.can_toggle_ui {
                return;
            }

            self.display_ui = !self.display_ui;
            if !self.display_ui {
                IS_CURSOR_IN_UI.store(false, Ordering::Relaxed);
            }

            self.can_toggle_ui = false;
            return;
        }

        self.can_toggle_ui = true;
    }

    /// Caches uninitialized textures for custom windows.
    fn load_unitialized_textures(&mut self, render_context: &mut dyn RenderContext) {
        let Some(cached_images) = self
            .base_core
            .try_read()
            .map(|reader| reader.get_custom_window_utils().get_cached_images())
        else {
            log!("[ERROR] Failed reading BaseCore as it's currently busy, cannot load uninitialized textures!");
            return;
        };

        let Ok(mut cached_textures) = cached_images.try_borrow_mut() else {
            log!("[ERROR] Cached textures is in use, cannot load uninitialized textures!");
            return;
        };

        if cached_textures.is_empty() {
            if !self.invalid_textures.is_empty() {
                self.invalid_textures.clear();
            }

            return;
        }

        // Only get the uninitialized textures.
        let uninitialized_textures = cached_textures.iter_mut().filter(|entry| {
            entry.1.texture_type != CustomTextureType::GifFrame && entry.1.texture_id.is_none()
        });

        let mut gif_image_path = None;

        for (image_path, custom_texture) in uninitialized_textures {
            let image_path = image_path.to_owned();
            let texture_type = custom_texture.texture_type;

            match texture_type {
                CustomTextureType::Gif => {
                    log!("[Texture Loader] Attempting to load GIF texture...");

                    // Save path for after this for-loop, as otherwise we risk deadlocks due to
                    // held-on resources.
                    gif_image_path = Some(image_path);
                }
                CustomTextureType::GifFrame => {
                    crash!("[ERROR] Attempted processing CustomTextureType::GifFrame, escaping the filter!");
                }
                CustomTextureType::Singular => {
                    let image = image::open(&image_path);
                    let Ok(image) = image else {
                        log!(
                            "[ERROR] Failed loading image at path \"",
                            image_path,
                            "\" into memory, error: ",
                            image.unwrap_err()
                        );
                        self.invalid_textures.push(image_path.to_owned());
                        break;
                    };

                    let loaded_texture_id = render_context.load_texture(
                        &image.to_rgba8().into_raw(),
                        image.width(),
                        image.height(),
                    );
                    let Ok(loaded_texture_id) = loaded_texture_id else {
                        log!(
                            "[ERROR] Failed loading image at path \"",
                            image_path,
                            "\" into the GPU, error: ",
                            loaded_texture_id.unwrap_err()
                        );
                        self.invalid_textures.push(image_path.to_owned());
                        break;
                    };

                    custom_texture.texture_id = Some(loaded_texture_id);
                }
            }
        }

        if let Some(image_path) = gif_image_path.take() {
            self.load_gif_frames(image_path, &mut cached_textures, render_context);
        }

        for invalid_texture in &self.invalid_textures {
            cached_textures.remove(invalid_texture);
        }

        self.invalid_textures.clear();
    }

    /// Loads the frame of a GIF into the GPU and stores them as `image_path.frame_[frame_number]`.
    fn load_gif_frames(
        &mut self,
        image_path: String,
        cached_images: &mut HashMap<String, CustomTexture>,
        render_context: &mut dyn RenderContext,
    ) {
        let extracted_frames = ImGuiUtils::extract_gif_frames(&image_path);
        for (i, frame) in extracted_frames.iter().enumerate() {
            let i = i + 1; // Make the index be more human-readable and not count from 0 when
                           // displaying.

            let loaded_texture_id =
                render_context.load_texture(&frame.buffer, frame.width as u32, frame.height as u32);
            let Ok(loaded_texture_id) = loaded_texture_id else {
                log!(
                    "[ERROR] Failed uploading image at path \"",
                    image_path,
                    "\" to the GPU, frame ",
                    i,
                    ", error: ",
                    loaded_texture_id.unwrap_err()
                );
                self.invalid_textures.push(image_path.to_owned());
                return;
            };

            if i == 1 {
                // Not an elegant solution, the ideal way would be to remove the image_path entry,
                // in order to not keep 2 entries with the same resource (image_path & first
                // frame).
                cached_images.insert(
                    image_path.to_owned(),
                    CustomTexture {
                        texture_id: Some(loaded_texture_id),
                        texture_type: CustomTextureType::Gif,
                    },
                );
            }

            let frame_path = ozencstr!(image_path, ".frame_", i);
            log!(
                "[Texture Loader] Loaded frame ",
                i,
                ", ready at in-memory path: ",
                frame_path
            );
            cached_images.insert(
                frame_path,
                CustomTexture {
                    texture_id: Some(loaded_texture_id),
                    texture_type: CustomTextureType::GifFrame,
                },
            );
        }

        log!(
            "[Texture Loader] GIF frames loaded from \"",
            image_path,
            "\", frames: ",
            extracted_frames.len(),
            "!"
        );
    }

    /// Loads the UI Colors preset from `self.ui_colors_preset`.
    fn load_ui_colors_preset(&mut self) {
        if self.ui_colors_preset.is_empty() {
            return;
        }

        let base_core_reader = self
            .base_core
            .try_read()
            .unwrap_or_crash(zencstr!("[ERROR] BaseCore is locked!"));
        base_core_reader
            .get_config()
            .load_colors_from_file(&self.ui_colors_preset);
        self.ui_colors_preset.clear();
    }

    /// Displays 2 warning messages about colors.
    fn display_colors_warning(&self, ui: &imgui::Ui) {
        let text_color = ui.push_style_color(
            StyleColor::Text,
            ColorUtils::rgba_to_frgba([255, 105, 0, 255]),
        );
        label!(ui, "󱇎 Warning: Text Color on buttons and alike isn't directly derived from the UI Stylesheet, and is instead programmed into dynamic.");
        text_color.pop();
    }
}

impl ImguiRenderLoop for DX11UI {
    /// Called as the UI has been initialized.
    fn initialize<'a>(&'a mut self, ctx: &mut Context, _: &'a mut dyn RenderContext) {
        let base_core_reader = self
            .base_core
            .try_read()
            .unwrap_or_crash(zencstr!("[ERROR] BaseCore is locked!"));

        let imgui_utils = base_core_reader.get_imgui_utils();
        let imgui_utils_reader = imgui_utils
            .try_read()
            .unwrap_or_crash(zencstr!("[ERROR] ImGuiUtils is locked!"));

        imgui_utils_reader.apply_style(
            ctx,
            &mut self.default_style,
            base_core_reader.get_config(),
            base_core_reader.get_crosscom(),
        )
    }

    /// Called before rendering the UI.
    fn before_render<'a>(
        &'a mut self,
        _ctx: &mut Context,
        _render_context: &'a mut dyn RenderContext,
    ) {
        CONTEXT_PTR.store(std::ptr::addr_of!(_ctx) as i64, Ordering::Relaxed);
        self.load_unitialized_textures(_render_context);
    }

    /// Renders the UI.
    fn render(&mut self, ui: &mut imgui::Ui, _render_context: &mut dyn RenderContext) {
        DELTA_TIME.store(ui.io().delta_time, Ordering::SeqCst);

        let base_core = Arc::clone(&self.base_core);
        let Some(base_core_reader) = base_core.try_read() else {
            return;
        };

        let script_core = base_core_reader.get_script_core();
        let config = base_core_reader.get_config();
        let imgui_utils = base_core_reader.get_imgui_utils();
        let Some(imgui_utils_reader) = imgui_utils.try_read() else {
            return;
        };

        imgui_utils_reader.draw_screen_messages(ui);
        drop(imgui_utils_reader);

        self.on_toggle_ui();
        script_core.call_frame_update_callbacks(None, None);
        if !self.display_ui {
            return;
        }

        IS_CURSOR_IN_UI.store(ui.io().want_capture_mouse, Ordering::Relaxed);
        self.load_unitialized_textures(_render_context);
        ImGuiUtils::sync_clipboard(ui);

        base_core_reader
            .get_custom_window_utils()
            .draw_custom_windows(ui, Arc::clone(&self.base_core));

        ui.window(zencstr!("󰅩 Code Editor"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .collapsed(true, Condition::Once)
            .build(|| {
                let available_size = ui.content_region_avail();
                ui.input_text_multiline(
                    " ",
                    &mut self.code_editor_input,
                    [available_size[0], available_size[1] - 25.0],
                )
                .build();
                if button!(ui, "󱐋 Execute") {
                    self.execute_rune_code();
                }

                self.draw_script_management(ui);
                ImGuiUtils::render_software_cursor(ui, &mut self.point);
            });

        ui.window(zencstr!("󰡉 Community"))
            .size([300.0, 100.0], Condition::FirstUseEver)
            .collapsed(true, Condition::Once)
            .build(|| {
                if let Some(community_window) = self.community_window.get_mut() {
                    community_window.draw(ui);
                    ImGuiUtils::render_software_cursor(ui, &mut self.point);
                } else {
                    self.community_window
                        .get_or_init(|| CommunityWindow::init(Arc::clone(&self.base_core)));
                }
            });
        ui.window(zencstr!("󱁤 Settings"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .collapsed(true, Condition::Once)
            .build(|| {
                let imgui_utils = base_core_reader.get_imgui_utils();
                let Some(mut imgui_utils_writer) = imgui_utils.try_write() else {
                    label!(ui, "ImGuiUtils is locked, settings unavailable!");
                    return;
                };

                let text_color = ui.push_style_color(StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
                if ui.collapsing_header(zencstr!("󱁤 System"), TreeNodeFlags::OPEN_ON_ARROW) {
                    text_color.pop();

                    ui.input_text(zencstr!("󰷔 CrossCom Channel"), &mut self.crosscom_channel)
                        .build();
                    ui.checkbox(
                        zencstr!("󰵅 Enable Side Messages"),
                        &mut imgui_utils_writer.enable_side_messages,
                    );
                    drop(imgui_utils_writer);
                    ui.separator();

                    if button!(ui, "󱘖 Join Channel") {
                        self.base_core
                            .try_read()
                            .unwrap_or_crash(zencstr!(
                                "[ERROR] Failed reading BaseCore, cannot continue!"
                            ))
                            .get_crosscom()
                            .try_read()
                            .unwrap_or_crash(zencstr!(
                                "[ERROR] Failed reading CrossCom, cannot continue!"
                            ))
                            .join_channel(&self.crosscom_channel);
                    }
                } else {
                    text_color.pop();
                }

                let text_color = ui.push_style_color(StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
                if ui.collapsing_header(zencstr!("󰉼 Colors"), TreeNodeFlags::OPEN_ON_ARROW) {
                    text_color.pop();

                    self.display_colors_warning(ui);
                    let text_color = ui.push_style_color(StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
                    if ui.collapsing_header(zencstr!("󰆓 Management"), TreeNodeFlags::OPEN_ON_ARROW)
                    {
                        text_color.pop();

                        ui.input_text(zencstr!("󰈔 Name"), &mut self.ui_colors_preset)
                            .build();
                        if button!(ui, "󱣪 Save") {
                            config.save_colors_to_file(ui, &self.ui_colors_preset);
                        }

                        ui.same_line();

                        if button!(ui, "󰦗 Load") {
                            self.load_ui_colors_preset();
                        }
                    } else {
                        text_color.pop();
                    }

                    ui.show_style_editor(&mut self.default_style);
                } else {
                    text_color.pop();
                }

                ImGuiUtils::render_software_cursor(ui, &mut self.point);
            });
    }
}
