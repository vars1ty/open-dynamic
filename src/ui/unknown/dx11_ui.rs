use crate::{
    globals::DELTA_TIME,
    mod_cores::base_core::BaseCore,
    ui::community::CommunityWindow,
    utils::{eguiutils::ImGuiUtils, extensions::OptionExt},
    winutils::WinUtils,
};
use hudhook::{
    imgui::{self, Condition, Context},
    ImguiRenderLoop, RenderContext,
};
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
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

    /// Disable SetCursorPos calls?
    disable_set_cursor_pos: Arc<AtomicBool>,

    /// Should the UI be displayed?
    display_ui: bool,

    /// Can we toggle the UI on/off?
    can_toggle_ui: bool,

    /// Current CrossCom channel.
    crosscom_channel: String,

    /// Invalid textures that should be removed the next frame.
    invalid_textures: Vec<String>,
}

impl DX11UI {
    /// Returns an instance to `Self`.
    pub fn new(base_core: Arc<RwLock<BaseCore>>, disable_set_cursor_pos: Arc<AtomicBool>) -> Self {
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
            .unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed borrowing crosscom.current_channel, error: ",
                    error
                )
            })
            .to_owned();
        drop(reader);

        Self {
            base_core,
            code_editor_input: String::default(),
            script_name: String::with_capacity(24),
            community_window: OnceLock::new(),
            point: POINT::default(),
            disable_set_cursor_pos,
            display_ui: true,
            can_toggle_ui: true,
            crosscom_channel,
            invalid_textures: Vec::with_capacity(4),
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
        ui.input_text(zencstr!("Script Name"), &mut self.script_name)
            .build();

        if self.script_name.is_empty() {
            return;
        }

        if button!(ui, "Save Rune Script") {
            self.base_core
                .read()
                .get_config()
                .save_to_file(&self.script_name, &self.code_editor_input);
        }

        ui.same_line();
        if button!(ui, "Load Rune Script")
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
                // Allow cursor input.
                self.disable_set_cursor_pos.store(false, Ordering::SeqCst);
            }

            self.can_toggle_ui = false;
            return;
        }

        self.can_toggle_ui = true;
    }

    /// Executes pending scripts that were received from custom windows.
    /// They aren't instantly executed from the window due to stability concerns.
    fn execute_pending_scripts(&self) {
        let Some(base_core_reader) = self.base_core.try_read() else {
            return;
        };

        let custom_window_utils = base_core_reader.get_custom_window_utils();
        let Some(pending_scripts) = custom_window_utils
            .get_pending_scripts()
            .try_borrow_mut()
            .ok()
            .and_then(|mut pending_scripts| pending_scripts.take())
        else {
            return;
        };

        let script_core = base_core_reader.get_script_core();
        pending_scripts.into_iter().for_each(|script| {
            script_core.execute(script, Arc::clone(&self.base_core), false, false)
        });
    }

    /// Caches uninitialized textures for custom windows.
    fn load_unitialized_textures(&mut self, render_context: &mut dyn RenderContext) {
        let Some(cached_images) = self
            .base_core
            .try_read()
            .map(|reader| reader.get_custom_window_utils().get_cached_images())
        else {
            return;
        };

        // Only get the uninitialized textures.
        let uninitialized_textures = cached_images
            .iter_mut()
            .filter(|entry| entry.value().is_none());

        for mut entry in uninitialized_textures {
            let image_path = entry.key();
            let image = image::open(image_path);

            let Ok(image) = image else {
                log!(
                    "[ERROR] Failed loading image at path \"",
                    image_path,
                    "\" into memory, error: ",
                    image.unwrap_err()
                );
                self.invalid_textures.push(image_path.to_owned());
                return;
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
                return;
            };

            *entry.value_mut() = Some(loaded_texture_id);
        }

        if self.invalid_textures.is_empty() {
            return;
        }

        for invalid_texture in &self.invalid_textures {
            cached_images.remove(invalid_texture);
        }

        self.invalid_textures.clear();
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
        self.load_unitialized_textures(_render_context);
    }

    /// Renders the UI.
    fn render(&mut self, ui: &mut imgui::Ui) {
        DELTA_TIME.store(ui.io().delta_time, Ordering::SeqCst);
        let base_core = Arc::clone(&self.base_core);
        let Some(base_core_reader) = base_core.try_read() else {
            return;
        };

        let imgui_utils = base_core_reader.get_imgui_utils();
        let Some(imgui_utils_reader) = imgui_utils.try_read() else {
            return;
        };

        imgui_utils_reader.draw_screen_messages(ui);
        drop(imgui_utils_reader);

        self.execute_pending_scripts();
        self.on_toggle_ui();
        if !self.display_ui {
            return;
        }

        self.disable_set_cursor_pos
            .store(ui.io().want_capture_mouse, Ordering::SeqCst);
        base_core_reader
            .get_custom_window_utils()
            .draw_custom_windows(ui, Arc::clone(&self.base_core));

        ui.window(zencstr!("󰅩 Code Editor"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .collapsed(true, Condition::Once)
            .build(|| {
                let available_size = ui.content_region_avail();
                static FREE_SPACE_FOR_BUTTON: f32 = 25.0;
                ui.input_text_multiline(
                    " ",
                    &mut self.code_editor_input,
                    [available_size[0], available_size[1] - FREE_SPACE_FOR_BUTTON],
                )
                .build();
                if button!(ui, "Execute") {
                    self.execute_rune_code();
                }

                self.draw_script_management(ui);
                ImGuiUtils::render_software_cursor(ui, &mut self.point);
            });

        ui.window(zencstr!("󰡉 Community"))
            .size([1280.0, 600.0], Condition::FirstUseEver)
            .size_constraints([1280.0, 200.0], [f32::INFINITY, f32::INFINITY])
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

                ui.input_text(zencstr!("󰷔 CrossCom Channel"), &mut self.crosscom_channel)
                    .build();
                ui.checkbox(
                    zencstr!("󰵅 Enable Side Messages"),
                    &mut imgui_utils_writer.enable_side_messages,
                );
                drop(imgui_utils_writer);
                ui.separator();

                if ui.button(zencstr!("󱘖 Join Channel")) {
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

                ImGuiUtils::render_software_cursor(ui, &mut self.point);
            });
    }
}
