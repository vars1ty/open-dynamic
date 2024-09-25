use crate::mod_cores::base_core::BaseCore;
use crate::utils::{dynwidget::WidgetType, eguiutils::ImGuiUtils};
use crate::winutils::WinUtils;
use atomic_refcell::AtomicRefCell;
use dashmap::DashMap;
use hudhook::imgui::{self, Condition, TextureId};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use rune::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};
use tinyapi32::tinyapi32::POINT;
use zstring::ZString;

/// # Safety
/// `WidgetsMap` is wrapped inside of `DashMap<T>` and uses `AtomicRefCell` with checks for
/// whether or not the value can be borrowed as mutable, or as a reference.
/// If it can't and assuming there is a check, the code returns and doesn't panic.
type WidgetsMap = IndexMap<String, Arc<AtomicRefCell<WidgetType>>>;

/// Custom window utilities for making custom windows easier to use, and supporting multiple
/// instances of them.
#[derive(Default)]
pub struct CustomWindowsUtils {
    /// Titles for each custom window.
    window_titles: RwLock<Vec<String>>,

    /// Widgets for each window.
    #[allow(clippy::type_complexity)] // Ignore because it's complex.
    window_widgets: DashMap<usize, WidgetsMap>,

    /// Current window index to be used for modifications.
    current_window_index: AtomicUsize,

    /// Window size constraints.
    window_size_constraints: Mutex<Vec<[f32; 4]>>,

    /// Cached GPU TextureIds, key being the path to the image.
    cached_images: DashMap<String, Option<TextureId>>,

    /// Current cursor point.
    /// Allowed to be non-thread-safe as it doesn't matter.
    point: Cell<POINT>,

    /// Widgets that should remain hidden.
    hidden_widgets: Mutex<Vec<String>>,

    /// If `Some(String)`, all widgets that get added next will be added into the defined centered
    /// widget parent/holder.
    add_into_centered: RefCell<Option<String>>,

    /// Pending scripts to be executed on the next UI draw call.
    pending_scripts: RefCell<Option<Vec<String>>>,
}

thread_safe_structs!(CustomWindowsUtils);

impl CustomWindowsUtils {
    /// Executes Rune code, intended for custom windows.
    fn execute_rune_code(&self, rune_code: &str) {
        let Ok(mut pending_scripts) = self.pending_scripts.try_borrow_mut() else {
            log!("[ERROR] pending_scripts cannot be borrowed as mutable, cancelling script execution.");
            return;
        };

        let rune_code = rune_code.to_owned();
        let Some(existing_pending_scripts) = pending_scripts.as_mut() else {
            *pending_scripts = Some(vec![rune_code]);
            return;
        };

        // Execute the script.
        // CrossCom disabled for now when it comes to buttons, might be changed in the future.
        existing_pending_scripts.push(rune_code);
    }

    /// Draws all of the custom windows.
    pub fn draw_custom_windows(&self, ui: &imgui::Ui, base_core: Arc<RwLock<BaseCore>>) {
        let Some(window_titles) = self.window_titles.try_read() else {
            return;
        };

        let Some(window_size_constraints) = self.window_size_constraints.try_lock() else {
            return;
        };

        static DEFAULT_SIZE: [f32; 2] = [600.0, 200.0];

        for (index, custom_window) in window_titles.iter().enumerate() {
            let Some(widgets) = window_titles
                .iter()
                .enumerate()
                .find(|(_, window_title)| *window_title == custom_window)
                .and_then(|(found_index, _)| self.window_widgets.get(&found_index))
            else {
                continue;
            };

            let Some(size_constraints) = window_size_constraints.get(index) else {
                continue;
            };

            self.call_ui_event_on_arctic(ui, custom_window, Arc::clone(&base_core), true);

            ui.window(&**custom_window)
                .size(DEFAULT_SIZE, Condition::FirstUseEver)
                .size_constraints(
                    [size_constraints[0], size_constraints[1]],
                    [size_constraints[2], size_constraints[3]],
                )
                .collapsed(true, Condition::Once)
                .build(|| {
                    self.call_ui_event_on_arctic(ui, custom_window, Arc::clone(&base_core), false);
                    self.draw_custom_window(Arc::clone(&base_core), &widgets, ui);
                    ImGuiUtils::render_software_cursor(ui, &mut self.point.get());
                });
        }
    }

    /// Draws a custom window.
    fn draw_custom_window(
        &self,
        base_core: Arc<RwLock<BaseCore>>,
        widgets: &WidgetsMap,
        ui: &imgui::Ui,
    ) {
        for (identifier, widget) in widgets {
            let Some(hidden_widgets) = self.hidden_widgets.try_lock() else {
                continue;
            };

            if hidden_widgets.contains(identifier) {
                continue;
            }

            drop(hidden_widgets);
            self.handle_widget(Arc::clone(&base_core), ui, identifier, widget);
        }
    }

    /// Handles all of the various widget types.
    fn handle_widget(
        &self,
        base_core: Arc<RwLock<BaseCore>>,
        ui: &imgui::Ui,
        identifier: &str,
        widget: &AtomicRefCell<WidgetType>,
    ) {
        let Ok(mut widget) = widget.try_borrow_mut() else {
            return;
        };

        match &mut *widget {
            WidgetType::Label(content, font_id) => {
                let Some(font_token) = ImGuiUtils::activate_font(ui, *font_id) else {
                    log!(
                        "[ERROR] Failed activating non-installed font at index ",
                        font_id,
                        "!"
                    );
                    return;
                };

                label!(ui, content);
                font_token.pop();
            }
            WidgetType::LabelCustomFont(content, relative_font_path) => {
                let Some(base_core_reader) = base_core.try_read() else {
                    return;
                };

                let imgui_utils = base_core_reader.get_imgui_utils();
                let Some(imgui_utils_reader) = imgui_utils.try_read() else {
                    return;
                };

                let Some(font_token) = ImGuiUtils::activate_font(
                    ui,
                    imgui_utils_reader.get_cfont_from_rpath(Arc::clone(relative_font_path)),
                ) else {
                    log!(
                        "[ERROR] Failed activating non-installed font from relative path at \"",
                        relative_font_path,
                        "\"!"
                    );
                    return;
                };

                label!(ui, content);
                font_token.pop();
            }
            WidgetType::Button(text, function, opt_param) => {
                if !button!(ui, text) {
                    return;
                };

                if let Err(error) = function
                    .call::<(Option<&Value>,), ()>((opt_param.as_ref(),))
                    .into_result()
                {
                    log!(
                        "[ERROR] Failed calling button function on \"",
                        identifier,
                        "\", error: ",
                        error
                    );
                }
            }
            WidgetType::LegacyButton(text, rune_code) => {
                if button!(ui, text) && !rune_code.is_empty() {
                    self.execute_rune_code(rune_code);
                }
            }
            WidgetType::Spacing(x, y) => ui.dummy([*x, *y]),
            WidgetType::Separator => ui.separator(),
            WidgetType::F32Slider(text, min, max, current_value) => {
                slider!(ui, text, *min, *max, *current_value);
            }
            WidgetType::I32Slider(text, min, max, current_value) => {
                slider!(ui, text, *min, *max, *current_value);
            }
            WidgetType::NextWidgetWidth(width) => ui.set_next_item_width(*width),
            WidgetType::SameLine => ui.same_line(),
            WidgetType::Image(image_path, width, height, overlay, background, rune_code) => {
                let Some(texture_id) =
                    self.get_texture_id(image_path, base_core.read().get_config().get_path())
                else {
                    return;
                };

                if *overlay {
                    let window_pos = ui.window_pos();
                    ui.get_foreground_draw_list()
                        .add_image(
                            texture_id,
                            window_pos,
                            [window_pos[0] + *width, window_pos[1] + *height],
                        )
                        .build();
                    return;
                }

                if *background {
                    let window_pos = ui.window_pos();
                    ui.get_window_draw_list()
                        .add_image(
                            texture_id,
                            window_pos,
                            [window_pos[0] + *width, window_pos[1] + *height],
                        )
                        .build();
                    return;
                }

                if ImGuiUtils::draw_image(ui, identifier, *width, *height, texture_id) {
                    self.execute_rune_code(rune_code);
                }
            }
            WidgetType::CenteredWidgets(widgets, custom_y, group_size) => {
                let new_group_size = ImGuiUtils::draw_centered_widgets(
                    ui,
                    ui.window_size(),
                    group_size,
                    *custom_y,
                    || {
                        let widgets = widgets.iter_mut().filter(|(identifier, _)| {
                            !self.hidden_widgets.lock().contains(identifier)
                        });

                        for (identifier, widget) in widgets {
                            self.handle_widget(
                                Arc::clone(&base_core),
                                ui,
                                identifier,
                                Arc::make_mut(widget),
                            );
                        }
                    },
                );

                *group_size = new_group_size;
            }
            WidgetType::InputTextMultiLine(label, text_input, width, height) => {
                ui.input_text_multiline(label, text_input, [*width, *height])
                    .build();
            }
        }
    }

    /// Changes the currently selected window.
    pub fn set_current_window_to(&self, window: String) {
        let Some(window_titles) = self.window_titles.try_read() else {
            log!("[ERROR] window_titles is locked, cannot swap window focus!");
            return;
        };

        let Some((index, _)) = window_titles
            .iter()
            .enumerate()
            .find(|(_, window_title)| **window_title == window)
        else {
            return;
        };

        self.current_window_index.store(index, Ordering::Relaxed);
    }

    /// Adds a new custom window.
    pub fn add_window(&self, title: String) {
        let Some(mut window_titles) = self.window_titles.try_write() else {
            return;
        };

        let Some(mut window_size_constraints) = self.window_size_constraints.try_lock() else {
            return;
        };

        if window_titles.contains(&title) {
            log!(
                "[WARN] There's already a window with the name of \"",
                title,
                "\"!"
            );
            return;
        }

        window_titles.push(title);
        window_size_constraints.push([0.0, 0.0, 9999.0, 9999.0]);

        let len = self.window_widgets.len();
        self.window_widgets.insert(len, Default::default());
    }

    /// Attempts to remove a custom window.
    pub fn remove_window(&self, title: String) {
        let Some(mut window_titles) = self.window_titles.try_write() else {
            return;
        };

        let Some(mut window_size_constraints) = self.window_size_constraints.try_lock() else {
            return;
        };

        if window_titles.is_empty() {
            log!("[ERROR] No windows are present!");
            return;
        }

        let Some((index, _)) = window_titles
            .iter()
            .enumerate()
            .find(|(_, window_title)| **window_title == title)
        else {
            return;
        };

        self.window_widgets.remove(&index);
        window_titles.remove(index);
        window_size_constraints.remove(index);

        if self.current_window_index.load(Ordering::Relaxed) == index {
            self.current_window_index.store(0, Ordering::Relaxed);
        }
    }

    /// Attempts to rename a custom window.
    pub fn rename_window(&self, from: String, to: String) {
        let Some(mut window_titles) = self.window_titles.try_write() else {
            return;
        };

        if !window_titles.contains(&from) {
            crash!(
                "[ERROR] The specified window by name \"",
                from,
                "\" does not exist, how do you want me to rename something that doesn't exist retard?"
            );
        }

        if window_titles.contains(&to) {
            log!(
                "[ERROR] There's already a window with the name of \"",
                to,
                "\", can't rename window to the name of an already-existing window!"
            );
            return;
        }

        let Some(window_title) = window_titles
            .iter_mut()
            .find(|window_title| **window_title == from)
        else {
            return;
        };

        *window_title = to;
    }

    /// Adds a widget to the currently selected custom window.
    pub fn add_widget(&self, identifier: String, widget_type: WidgetType) {
        if identifier.is_empty() {
            log!("[ERROR] A widget identifier is empty!");
            return;
        }

        // If auto-center is enabled and assuming the identifier for it isn't empty, center the
        // widget.
        if let Some(parent_identifier) = self.add_into_centered.borrow().as_ref() {
            self.add_into_centered_widget(parent_identifier, identifier, widget_type);
            return;
        }

        let Some(mut window_widgets) = self
            .window_widgets
            .get_mut(&self.current_window_index.load(Ordering::Relaxed))
        else {
            return;
        };

        #[allow(clippy::arc_with_non_send_sync)]
        window_widgets.insert(identifier, Arc::new(AtomicRefCell::new(widget_type)));
    }

    /// Adds a widget to a centered widget holder/parent.
    pub fn add_into_centered_widget(
        &self,
        parent_identifier: &str,
        identifier: String,
        widget_type: WidgetType,
    ) {
        for entry in &self.window_widgets {
            let widget_map = entry.value();
            let Some(entry) = widget_map
                .iter()
                .find(|(identifier, _)| *identifier == parent_identifier)
            else {
                continue;
            };

            let Ok(mut widget) = entry.1.try_borrow_mut() else {
                log!(
                    "[ERROR] Cannot borrow widget \"",
                    identifier,
                    "\" as mutable, cancelling!"
                );
                return;
            };

            let WidgetType::CenteredWidgets(widgets, _custom_y, _group_size) = &mut *widget else {
                continue;
            };

            // Check if the user is trying to nest centered widgets together.
            // If attempted, don't continue.
            if let WidgetType::CenteredWidgets(..) = widget_type {
                log!("[ERROR] Nesting centered group widgets isn't supported!");
                return;
            }

            #[allow(clippy::arc_with_non_send_sync)]
            widgets.insert(identifier, Arc::new(AtomicRefCell::new(widget_type)));
            return;
        }
    }

    /// Removes a widget from the currently selected custom window.
    pub fn remove_widget(&self, identifier: String) {
        let Some(mut window_widgets) = self
            .window_widgets
            .get_mut(&self.current_window_index.load(Ordering::Relaxed))
        else {
            return;
        };

        window_widgets.shift_remove(&identifier);

        // Search through all widgets for all CenteredWidgets, then try and remove all instances of
        // `identifier`.
        for (_, widget) in &*window_widgets {
            if let WidgetType::CenteredWidgets(widgets, _, _) = &mut *widget.borrow_mut() {
                widgets.shift_remove(&identifier);
            }
        }
    }

    /// Removes all widgets from the currently selected window.
    pub fn remove_all_widgets(&self) {
        let Some(mut window_widgets) = self
            .window_widgets
            .get_mut(&self.current_window_index.load(Ordering::Relaxed))
        else {
            return;
        };

        window_widgets.clear();
    }

    /// Gets a widget from a specific window.
    pub fn get_widget(&self, identifier: &str) -> Option<Arc<AtomicRefCell<WidgetType>>> {
        let window_widgets = self
            .window_widgets
            .get(&self.current_window_index.load(Ordering::Relaxed))?;
        for (widget_identifier, widget) in &*window_widgets {
            // If widget was found outside of a `CenteredWidget` widget, return it.
            if widget_identifier == identifier {
                return Some(Arc::clone(widget));
            }

            let Ok(widget) = widget.try_borrow() else {
                log!(
                    "[ERROR] Failed to borrow widget \"",
                    widget_identifier,
                    "\", cannot safely return widget!"
                );
                return None;
            };

            // Otherwise, check if the widget is CenteredWidgets and scan the widgets inside of
            // it. If found, return it.
            // This won't work with nested CenteredWidgets, but that's frowned upon and
            // shouldn't be accounted for regardless.
            let WidgetType::CenteredWidgets(widgets, _, _) = &*widget else {
                continue;
            };

            let Some(widget) = widgets.get(identifier) else {
                continue;
            };

            return Some(Arc::clone(widget));
        }

        None
    }

    /// Attempts to get the value of a f32-slider in the currently-active window.
    pub fn get_f32_slider_value(&self, identifier: String) -> f32 {
        let Some(widget) = self.get_widget(&identifier) else {
            log!(
                "[ERROR] Couldn't find any widgets named \"",
                identifier,
                "\"!"
            );
            return 0.0;
        };

        let WidgetType::F32Slider(_, _, _, current_value) = *widget.borrow() else {
            log!(
                "[ERROR] Couldn't find any f32-sliders with the name of \"",
                identifier,
                "\"!"
            );
            return 0.0;
        };

        current_value
    }

    /// Attempts to get the value of a i32-slider in the currently-active window.
    pub fn get_i32_slider_value(&self, identifier: String) -> i32 {
        let Some(widget) = self.get_widget(&identifier) else {
            log!(
                "[ERROR] Couldn't find any widgets named \"",
                identifier,
                "\"!"
            );
            return 0;
        };

        let WidgetType::I32Slider(_, _, _, current_value) = *widget.borrow() else {
            log!(
                "[ERROR] Couldn't find any i32-sliders with the name of \"",
                identifier,
                "\"!"
            );
            return 0;
        };

        current_value
    }

    /// Attempts to get the value of a i32-slider in the currently-active window.
    pub fn get_input_text_multiline_value(&self, identifier: String) -> String {
        let Some(widget) = self.get_widget(&identifier) else {
            log!(
                "[ERROR] Couldn't find any widgets named \"",
                identifier,
                "\"!"
            );
            return String::default();
        };

        let WidgetType::InputTextMultiLine(_, input, _, _) = &*widget.borrow() else {
            log!(
                "[ERROR] Couldn't find any multiline text input fields with the name of \"",
                identifier,
                "\"!"
            );
            return String::default();
        };

        input.to_owned()
    }

    /// Sets the window constraints for the currently focused window.
    pub fn set_active_window_size_constraints(&self, constraints: [f32; 4]) {
        if let Some(active_constraints) = self
            .window_size_constraints
            .lock()
            .get_mut(self.current_window_index.load(Ordering::Relaxed))
        {
            *active_constraints = constraints;
        }
    }

    /// Replaces the image in an existing widget with a new one.
    pub fn replace_image(
        &self,
        identifier: String,
        new_image_path: String,
        width_height: [f32; 2],
    ) {
        for entry in &self.window_widgets {
            let widget_map = entry.value();
            let Some(entry) = widget_map
                .iter()
                .find(|(widget_identifier, _)| **widget_identifier == identifier)
            else {
                continue;
            };

            let Ok(mut widget) = entry.1.try_borrow_mut() else {
                log!(
                    "[ERROR] Cannot borrow widget \"",
                    identifier,
                    "\" as mutable, cancelling!"
                );
                return;
            };

            if let WidgetType::Image(image_path, width, height, _overlay, _background, _rune_code) =
                &mut *widget
            {
                *image_path = new_image_path;
                *width = width_height[0];
                *height = width_height[1];
                return;
            }
        }
    }

    /// Clears all cached images.
    pub fn clear_cached_images(&self) {
        self.cached_images.clear();
    }

    /// Gets the Texture ID for an image. If it hasn't been cached already, then it's cached and
    /// returned.
    pub fn get_texture_id(&self, image_path: &str, config_dir_path: &str) -> Option<TextureId> {
        let mut full_image_path = ZString::new(String::with_capacity(
            config_dir_path.len() + image_path.len(),
        ));
        full_image_path.data.push_str(config_dir_path);
        full_image_path.data.push_str(image_path);

        if let Some(cached_image) = self.cached_images.get(&full_image_path.data) {
            *cached_image
        } else {
            self.cached_images
                .insert(std::mem::take(&mut full_image_path.data), None);
            None
        }
    }

    /// Hides a set of widgets by their identifiers from all windows.
    pub fn hide_widgets(&self, identifiers: Vec<String>) {
        let Some(mut hidden_widgets) = self.hidden_widgets.try_lock() else {
            return;
        };

        for identifier in &*identifiers {
            if !hidden_widgets.contains(identifier) {
                hidden_widgets.push(identifier.to_owned());
            }
        }
    }

    /// If the defined widgets are hidden, they'll then be shown again.
    pub fn show_widgets(&self, identifiers: Vec<String>) {
        let Some(mut hidden_widgets) = self.hidden_widgets.try_lock() else {
            return;
        };

        for identifier in &*identifiers {
            hidden_widgets.retain(|hidden_identifier| *hidden_identifier != *identifier);
        }
    }

    /// Makes all upcoming widgets be centered into the defined centered widget parent/holder.
    /// If `None`, they are no longer centered automatically.
    pub fn set_widget_auto_centered_into(&self, center_into: Option<String>) {
        *self.add_into_centered.borrow_mut() = center_into;
    }

    /// Calls `on_ui_update` on all injected DLLs which use Arctic, and passes in the UI pointer
    /// alongside with the window name so the user can identify each window.
    fn call_ui_event_on_arctic(
        &self,
        ui: &imgui::Ui,
        window_name: &str,
        base_core: Arc<RwLock<BaseCore>>,
        is_pre: bool,
    ) {
        let reader = base_core.try_read();
        let Some(arctic) = reader
            .as_ref()
            .and_then(|reader| reader.get_arctic_core().get())
        else {
            return;
        };

        let injected_dlls = arctic.get_injected_dlls();
        for module_name in &*injected_dlls {
            let Some(func) = WinUtils::get_module_symbol_address(&*module_name, c"on_ui_update")
            else {
                continue;
            };

            let func: extern "Rust" fn(*const i64, &str, bool) =
                unsafe { std::mem::transmute(func) };
            func(std::ptr::addr_of!(ui) as _, window_name, is_pre);
        }
    }

    /// Retains all widgets that have their identifier present in `identifiers`.
    pub fn retain_widgets_by_identifiers(&self, identifiers: Vec<String>) {
        let Some(mut window_widgets) = self
            .window_widgets
            .get_mut(&self.current_window_index.load(Ordering::Relaxed))
        else {
            return;
        };

        window_widgets.retain(|identifier, _| identifiers.contains(identifier));
    }

    /// Gets the pending scripts to be executed.
    pub const fn get_pending_scripts(&self) -> &RefCell<Option<Vec<String>>> {
        &self.pending_scripts
    }

    /// Gets the value of `self.cached_images`.
    pub const fn get_cached_images(&self) -> &DashMap<String, Option<TextureId>> {
        &self.cached_images
    }
}
