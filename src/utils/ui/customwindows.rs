use crate::mod_cores::base_core::BaseCore;
use crate::utils::dynwidget::SubWidgetType;
use crate::utils::{dynwidget::WidgetType, eguiutils::ImGuiUtils, stringutils::StringUtils};
use atomic_refcell::AtomicRefCell;
use dashmap::DashMap;
use hudhook::imgui::{self, Condition, TextureId, TreeNodeFlags};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use rune::{alloc::clone::TryClone, runtime::SyncFunction, Value};
use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};
use windows::Win32::Foundation::POINT;
use zstring::ZString;

/// # Safety
/// `WidgetsMap` is wrapped inside of `DashMap<T>` and uses `AtomicRefCell` with checks for
/// whether or not the value can be borrowed as mutable, or as a reference.
/// If it can't and assuming there is a check, the code returns and doesn't panic.
type WidgetsMap = IndexMap<String, Arc<AtomicRefCell<WidgetType>>>;

/// Type of callback that's meant for the function.
#[derive(Default)]
enum CallbackType {
    #[default]
    None,

    Button(String, Rc<Option<Value>>),
    I32Slider(String, i32, Rc<Option<Value>>),
    F32Slider(String, f32, Rc<Option<Value>>),
    Checkbox(String, bool, Rc<Option<Value>>),
    InputTextMultiLine(String, String, Rc<Option<Value>>),
}

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
    hidden_widgets: AtomicRefCell<Vec<String>>,

    /// If `Some()`, then adding a new widget will result in it getting added to the defined
    /// sub-widget if present.
    /// If `None`, then it's added onto the UI as-is.
    add_into_sub_widget: AtomicRefCell<Option<String>>,

    /// Pending scripts to be executed on the next UI draw call.
    pending_scripts: AtomicRefCell<Option<Vec<String>>>,

    /// Pending callbacks for buttons and sliders, as executing them at the same frame in the UI
    /// can cause deadlocks.
    /// No `*Map` because it requires traits which aren't implemented for `*Function` in order to
    /// add entries.
    pending_callbacks: Mutex<Vec<(Rc<SyncFunction>, CallbackType)>>,
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
                .and_then(|(found_index, _)| {
                    self.window_widgets.try_get(&found_index).try_unwrap()
                })
            else {
                continue;
            };

            let Some(size_constraints) = window_size_constraints.get(index) else {
                continue;
            };

            ui.window(&**custom_window)
                .size(DEFAULT_SIZE, Condition::FirstUseEver)
                .size_constraints(
                    [size_constraints[0], size_constraints[1]],
                    [size_constraints[2], size_constraints[3]],
                )
                .collapsed(true, Condition::Once)
                .build(|| {
                    self.draw_custom_window(Arc::clone(&base_core), &widgets, ui);
                    ImGuiUtils::render_software_cursor(ui, &mut self.point.get());
                });
        }

        self.call_pending_callbacks();
    }

    /// Calls all pending callbacks.
    fn call_pending_callbacks(&self) {
        let Some(mut pending_callbacks) = self.pending_callbacks.try_lock() else {
            log!("[ERROR] Pending callbacks is locked and cannot be accessed!");
            return;
        };

        for (callback, callback_type) in &*pending_callbacks {
            match callback_type {
                CallbackType::Button(identifier, opt_param) => {
                    if let Err(error) = callback
                        .call::<(Option<&Value>,), ()>((opt_param.as_ref().as_ref(),))
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
                CallbackType::I32Slider(identifier, current_value, opt_param) => {
                    if let Err(error) = callback
                        .call::<(i32, Option<&Value>), ()>((
                            *current_value,
                            opt_param.as_ref().as_ref(),
                        ))
                        .into_result()
                    {
                        log!(
                            "[ERROR] Failed calling i32 slider function on \"",
                            identifier,
                            "\", error: ",
                            error
                        );
                    }
                }
                CallbackType::F32Slider(identifier, current_value, opt_param) => {
                    if let Err(error) = callback
                        .call::<(f32, Option<&Value>), ()>((
                            *current_value,
                            opt_param.as_ref().as_ref(),
                        ))
                        .into_result()
                    {
                        log!(
                            "[ERROR] Failed calling i32 slider function on \"",
                            identifier,
                            "\", error: ",
                            error
                        );
                    }
                }
                CallbackType::Checkbox(identifier, checked, opt_param) => {
                    if let Err(error) = callback
                        .call::<(bool, Option<&Value>), ()>((*checked, opt_param.as_ref().as_ref()))
                        .into_result()
                    {
                        log!(
                            "[ERROR] Failed calling checkbox function on \"",
                            identifier,
                            "\", error: ",
                            error
                        );
                    }
                }
                CallbackType::InputTextMultiLine(identifier, current_value, opt_param) => {
                    if let Err(error) = callback
                        .call::<(&str, Option<&Value>), ()>((
                            current_value,
                            opt_param.as_ref().as_ref(),
                        ))
                        .into_result()
                    {
                        log!(
                            "[ERROR] Failed calling input text multiline function on \"",
                            identifier,
                            "\", error: ",
                            error
                        );
                    }
                }
                _ => crash!("[ERROR] Invalid callback type!"),
            }
        }

        pending_callbacks.clear();
    }

    /// Draws a custom window.
    fn draw_custom_window(
        &self,
        base_core: Arc<RwLock<BaseCore>>,
        widgets: &WidgetsMap,
        ui: &imgui::Ui,
    ) {
        for (identifier, widget) in widgets {
            let Ok(hidden_widgets) = self.hidden_widgets.try_borrow() else {
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
            WidgetType::Button(text, callback, opt_param) => {
                if button!(ui, text) {
                    self.add_callback(
                        identifier,
                        callback,
                        CallbackType::Button(identifier.to_owned(), Rc::clone(opt_param)),
                    );
                };
            }
            WidgetType::Spacing(x, y) => ui.dummy([*x, *y]),
            WidgetType::Separator => ui.separator(),
            WidgetType::F32Slider(text, min, max, current_value, callback, opt_param) => {
                if slider!(ui, text, *min, *max, *current_value) {
                    self.add_callback(
                        identifier,
                        callback,
                        CallbackType::F32Slider(
                            identifier.to_owned(),
                            *current_value,
                            Rc::clone(opt_param),
                        ),
                    );
                }
            }
            WidgetType::I32Slider(text, min, max, current_value, callback, opt_param) => {
                if slider!(ui, text, *min, *max, *current_value) {
                    self.add_callback(
                        identifier,
                        callback,
                        CallbackType::I32Slider(
                            identifier.to_owned(),
                            *current_value,
                            Rc::clone(opt_param),
                        ),
                    );
                }
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
            WidgetType::InputTextMultiLine(
                label,
                text_input,
                width,
                height,
                callback,
                opt_param,
            ) => {
                if ui
                    .input_text_multiline(label, text_input, [*width, *height])
                    .build()
                {
                    self.add_callback(
                        identifier,
                        callback,
                        CallbackType::InputTextMultiLine(
                            identifier.to_owned(),
                            text_input.to_owned(),
                            Rc::clone(opt_param),
                        ),
                    );
                }
            }
            WidgetType::SubWidget(sub_widget, widgets, ..) => {
                self.handle_sub_widget(ui, Arc::clone(&base_core), sub_widget, widgets);
            }
            WidgetType::Checkbox(text, checked, callback, opt_param) => {
                if ui.checkbox(text, checked) {
                    self.add_callback(
                        identifier,
                        callback,
                        CallbackType::Checkbox(
                            identifier.to_owned(),
                            *checked,
                            Rc::clone(opt_param),
                        ),
                    );
                }
            }
        }
    }

    /// Handles the logic of a sub-widget.
    fn handle_sub_widget(
        &self,
        ui: &imgui::Ui,
        base_core: Arc<RwLock<BaseCore>>,
        sub_widget: &mut SubWidgetType,
        widgets: &WidgetsMap,
    ) {
        let Ok(hidden_widgets) = self.hidden_widgets.try_borrow() else {
            log!("[ERROR] Hidden widgets is busy, cannot render sub-widget!");
            return;
        };

        let widgets = widgets
            .iter()
            .filter(|(identifier, _)| !hidden_widgets.contains(identifier));
        match sub_widget {
            SubWidgetType::CollapsingHeader(text) => {
                if !ui.collapsing_header(text, TreeNodeFlags::OPEN_ON_ARROW) {
                    return;
                }

                for (identifier, widget) in widgets {
                    self.handle_widget(Arc::clone(&base_core), ui, identifier, widget);
                }
            }
        }
    }

    /// Adds a callback to `self.pending_callbacks`.
    fn add_callback(
        &self,
        identifier: &str,
        callback: &Rc<SyncFunction>,
        callback_type: CallbackType,
    ) {
        let Some(mut pending_callbacks) = self.pending_callbacks.try_lock() else {
            log!("[ERROR] Pending callbacks is locked and cannot be accessed!");
            return;
        };

        pending_callbacks.push((
            callback.try_clone().unwrap_or_else(|error| {
                crash!(
                    "[ERROR] Failed cloning function named \"",
                    identifier,
                    "\", error: ",
                    error
                )
            }),
            callback_type,
        ));
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
                "[ERROR] The specified window \"",
                from,
                "\" does not exist!"
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
    pub fn add_widget(&self, mut identifier: String, widget_type: WidgetType) {
        if identifier.is_empty() {
            // Empty identifier, use a "random" string.
            // Identifier should really be an Option<String>, but that's reserved for the future if
            // anything due to compatibility reasons.
            identifier = StringUtils::get_random();
        }

        let window_widgets = self
            .window_widgets
            .try_get_mut(&self.current_window_index.load(Ordering::Relaxed));
        if window_widgets.is_locked() {
            log!("[ERROR] Window widgets is locked, no new widgets can be added to the active window!");
            return;
        }

        let Some(mut window_widgets) = window_widgets.try_unwrap() else {
            return;
        };

        if let Some(parent_identifier) = self.add_into_sub_widget.borrow().as_ref() {
            self.add_into_sub_widget(
                &mut window_widgets,
                parent_identifier,
                identifier,
                widget_type,
            );
            return;
        }

        let sub_widget_data =
            if let WidgetType::SubWidget(_, _, ref call_once, ref opt_param) = widget_type {
                Some((Rc::clone(call_once), Rc::clone(opt_param)))
            } else {
                None
            };

        #[allow(clippy::arc_with_non_send_sync)]
        let widget = Arc::new(AtomicRefCell::new(widget_type));
        window_widgets.insert(identifier.to_owned(), Arc::clone(&widget));
        drop(window_widgets);

        // If the added widget was a sub-widget, call the function attached.
        let Some((sub_widget_call_once, opt_param)) = sub_widget_data else {
            return;
        };

        self.set_sub_widget_identifier(Some(identifier.to_owned()));
        let Err(error) = sub_widget_call_once
            .call::<(Option<&Value>,), ()>((opt_param.as_ref().as_ref(),))
            .into_result()
        else {
            self.set_sub_widget_identifier(None);
            return;
        };

        log!(
            "[ERROR] Failed calling function on sub-widget \"",
            identifier,
            "\", error: ",
            error
        );
        self.set_sub_widget_identifier(None);
    }

    /// Adds `widget_type` into the sub-widget found by `sub_widget_identifier` if found.
    fn add_into_sub_widget(
        &self,
        widgets: &mut WidgetsMap,
        sub_widget_identifier: &str,
        mut identifier: String,
        widget_type: WidgetType,
    ) {
        if identifier.is_empty() {
            // Empty identifier, use a "random" string.
            // Identifier should really be an Option<String>, but that's reserved for the future if
            // anything due to compatibility reasons.
            identifier = StringUtils::get_random();
        }

        let Some(sub_widget) = widgets
            .iter()
            .find(|(identifier, _)| *identifier == sub_widget_identifier)
            .map(|(_, widget)| widget)
        else {
            log!("[ERROR] No widgets named \"", sub_widget_identifier, "\"!");
            return;
        };

        let Ok(mut sub_widget) = sub_widget.try_borrow_mut() else {
            log!(
                "[ERROR] Failed borrowing \"",
                sub_widget_identifier,
                "\" as mutable as it's currently busy, cannot add into sub-widget!"
            );
            return;
        };

        let WidgetType::SubWidget(_, widgets, ..) = &mut *sub_widget else {
            log!(
                "[ERROR] Widget \"",
                sub_widget_identifier,
                "\" is not a sub-widget, cannot add widget!"
            );
            return;
        };

        #[allow(clippy::arc_with_non_send_sync)]
        widgets.insert(identifier, Arc::new(AtomicRefCell::new(widget_type)));
    }

    /// Removes a widget from the currently selected custom window.
    pub fn remove_widget(&self, identifier: String) {
        let window_widgets = self
            .window_widgets
            .try_get_mut(&self.current_window_index.load(Ordering::Relaxed));
        if window_widgets.is_locked() {
            log!("[ERROR] Window widgets is locked, no widgets can be removed!");
            return;
        }

        let Some(mut window_widgets) = window_widgets.try_unwrap() else {
            return;
        };

        window_widgets.shift_remove(&identifier);

        // Iterate over sub-widgets and remove any potential matches.
        for (_, widget) in &*window_widgets {
            if let WidgetType::SubWidget(_, widgets, ..) = &mut *widget.borrow_mut() {
                widgets.shift_remove(&identifier);
            }
        }
    }

    /// Removes all widgets from the currently selected window.
    pub fn remove_all_widgets(&self) {
        let window_widgets = self
            .window_widgets
            .try_get_mut(&self.current_window_index.load(Ordering::Relaxed));
        if window_widgets.is_locked() {
            log!("[ERROR] Window widgets is locked, no widgets can be removed from the active window!");
            return;
        }

        let Some(mut window_widgets) = window_widgets.try_unwrap() else {
            return;
        };

        window_widgets.clear();
    }

    /// Gets a widget from a specific window.
    pub fn get_widget(&self, identifier: &str) -> Option<Arc<AtomicRefCell<WidgetType>>> {
        let window_widgets = self
            .window_widgets
            .try_get(&self.current_window_index.load(Ordering::Relaxed));
        if window_widgets.is_locked() {
            log!("[ERROR] Window widgets is locked, no widgets can be pulled from the active window!");
            return None;
        }

        let window_widgets = window_widgets.try_unwrap()?;
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
            let WidgetType::SubWidget(_, widgets, ..) = &*widget else {
                continue;
            };

            let Some(widget) = widgets.get(identifier) else {
                continue;
            };

            return Some(Arc::clone(widget));
        }

        None
    }

    /// Tries to update the text of an existing label.
    /// TODO: Make work on any widget with a label.
    pub fn update_label(&self, identifier: String, new_text: String) {
        let Some(widget) = self.get_widget(&identifier) else {
            log!("[ERROR] There are no widgets named \"", identifier, "\"!");
            return;
        };

        let widget = widget.try_borrow_mut();
        if let Err(error) = widget {
            log!(
                "[ERROR] Failed mutably borrowing widget \"",
                identifier,
                "\", error: ",
                error
            );
            return;
        }

        let mut widget = widget.unwrap();
        if let WidgetType::Label(text, _) = &mut *widget {
            *text = new_text;
            return;
        }

        if let WidgetType::LabelCustomFont(text, _) = &mut *widget {
            *text = new_text;
        }
    }

    /// Attempts to get the value of a f32-slider in the currently-active window.
    pub fn get_f32_slider_value(&self, identifier: String) -> Option<f32> {
        let WidgetType::F32Slider(_, _, _, current_value, _, _) =
            *self.get_widget(&identifier)?.borrow()
        else {
            return None;
        };

        Some(current_value)
    }

    /// Attempts to get the value of a i32-slider in the currently-active window.
    pub fn get_i32_slider_value(&self, identifier: String) -> Option<i32> {
        let WidgetType::I32Slider(_, _, _, current_value, _, _) =
            *self.get_widget(&identifier)?.borrow()
        else {
            return None;
        };

        Some(current_value)
    }

    /// Attempts to get the value of a i32-slider in the currently-active window.
    pub fn get_input_text_multiline_value(&self, identifier: String) -> Option<String> {
        let widget = self.get_widget(&identifier)?;
        let WidgetType::InputTextMultiLine(_, input, _, _, _, _) = &*widget.borrow() else {
            return None;
        };

        Some(input.to_owned())
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
        let Ok(mut hidden_widgets) = self.hidden_widgets.try_borrow_mut() else {
            log!("[ERROR] Hidden widgets is already being borrowed, cannot insert new ones at this time!");
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
        let Ok(mut hidden_widgets) = self.hidden_widgets.try_borrow_mut() else {
            log!("[ERROR] Hidden widgets is already being borrowed, cannot remove existing ones at this time!");
            return;
        };

        for identifier in &*identifiers {
            hidden_widgets.retain(|hidden_identifier| *hidden_identifier != *identifier);
        }
    }

    /// Sets the name of the sub-widget to be used for adding all upcoming widgets, until set to
    /// `None` again.
    pub fn set_sub_widget_identifier(&self, focus: Option<String>) {
        *self.add_into_sub_widget.borrow_mut() = focus;
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
    pub const fn get_pending_scripts(&self) -> &AtomicRefCell<Option<Vec<String>>> {
        &self.pending_scripts
    }

    /// Gets the value of `self.cached_images`.
    pub const fn get_cached_images(&self) -> &DashMap<String, Option<TextureId>> {
        &self.cached_images
    }
}
