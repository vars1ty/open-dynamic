use crate::{
    globals::DELTA_TIME,
    mod_cores::base_core::BaseCore,
    utils::{eguiutils::ImGuiUtils, extensions::OptionExt, scripting::script_modules::*},
    winutils::WinUtils,
};
use dll_syringe::{
    process::{BorrowedProcess, OwnedProcess, ProcessModule},
    Syringe,
};
use hudhook::imgui::{FontStackToken, Ui};
use parking_lot::{Mutex, RwLock};
use rune::Module;
use std::{
    collections::HashMap,
    os::windows::io::FromRawHandle,
    sync::{Arc, OnceLock},
};
use windows::Win32::System::Threading::GetCurrentProcess;

/// A structure that contains a set of functions from dynamic.
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub struct DNXFunctions {
    /// `dynamic::log(message)` function. Logs both to the side-messages, and to `stdout`.
    dynamic_log: extern "Rust" fn(&str),

    /// `Memory::read_string(address) function. Attempts to read a string at `address`.
    memory_read_string: extern "Rust" fn(i64) -> &'static str,

    /// `dynamic::get_delta_time()` function. Gets the current delta-time of the process.
    dynamic_get_delta_time: extern "Rust" fn() -> f32,

    /// Special function for making dynamic eject the DLL, rather than the other way around.
    /// This is needed because otherwise the process crashes.
    dynamic_eject_payload:
        Box<dyn Fn(OwnedProcess, ProcessModule<BorrowedProcess<'static>>) + Send + Sync>,

    /// Rune VM which you may use to execute Rune code.
    rune_vm_execute: Box<dyn Fn(String) + Send + Sync>,

    /// `dynamic::create_thread_key(name)` function. Creates a globally-accessible thread-key.
    dyamic_add_thread_key: extern "Rust" fn(String),

    /// `dynamic::set_thread_key_value(name, value)` function. Sets the value of a thread-key.
    dynamic_set_thread_key_value: extern "Rust" fn(String, bool),

    /// `dynamic::get_thread_key(name)` function. Returns the value of the thread-key.
    dynamic_get_thread_key: extern "Rust" fn(String) -> bool,

    /// `ui::add_window(name)` function. Allocates and displays a new custom window.
    ui_add_window: Box<dyn Fn(String) + Send + Sync>,

    /// `ui::focus_window` function. Focuses the defined window if present.
    ui_focus_window: Box<dyn Fn(String) + Send + Sync>,

    /// `Sellix::is_paying_for_product(product_id, bearer_tolen)` function. Checks if the user is
    /// paying for the specified Sellix product.
    sellix_is_paying_for_product: Box<dyn Fn(String, String) -> bool + Send + Sync>,

    /// `Config::has_serial(serial)` function. Checks if the defined serial is present in the
    /// config.
    config_has_serial: Box<dyn Fn(String) -> bool + Send + Sync>,

    /// The underlying implementation of this function tries to check if the variable you defined
    /// has an address cached, if it does then it's returned.
    /// If it hasn't been cached, then the `sig_scan_address` function is called and you are
    /// expected to return the address as an `usize`, which then CrossCom stores in memory for you
    /// to later retrieve from the cache, using this exact same function.
    ///
    /// The server caches the offset to the static address you defined, via the `base_address` and
    /// then returns (or caches it if not present) it for you.
    server_aob_scan:
        Box<dyn Fn(&str, usize, Box<dyn FnOnce() -> usize>) -> *const i64 + Send + Sync>,

    /// Tries to find the underlying string-value of the data you requested.
    /// If no data was found, `None` is returned.
    server_get_variable: Box<dyn Fn(&str) -> Option<String> + Send + Sync>,

    /// Displays an error message and closes dynamic alongside with the parent process.
    crash: extern "Rust" fn(&str) -> !,

    /// Installs a module into dynamic's Rune implementation.
    install_rune_module: Box<dyn Fn(Module) + Send + Sync>,

    /// Returns the channel which the user is currently in.
    server_get_current_channel: Box<dyn Fn() -> String + Send + Sync>,

    /// Attempts to join the specified channel.
    server_join_channel: Box<dyn Fn(&str) + Send + Sync>,

    /// Sends the specified content from your client, to the rest of the clients in the active
    /// party.
    /// How they treat the input is up to their implementations.
    server_send_script: Box<dyn Fn(&str) + Send + Sync>,

    /// Attempts to get the content from the defined relative path, returning `true` if successful.
    config_get_file_content: Box<dyn Fn(&str, &mut String) -> bool + Send + Sync>,

    /// Attempts to save the content into the defined relative path, returning `true` if successful.
    config_save_to_file: Box<dyn Fn(&str, &str) -> bool + Send + Sync>,

    /// Gets the absolute path to the directory where dynamic is located at.
    config_get_path: Box<dyn Fn() -> &'static str + Send + Sync>,

    /// Gets all of the serials from the config.
    config_get_serials: Box<dyn Fn() -> &'static Vec<String> + Send + Sync>,

    // ImGui functions, no explanation needed as they are self-explanatory and documented online.
    imgui_text: extern "Rust" fn(&Ui, &str),
    imgui_button: extern "Rust" fn(&Ui, &str) -> bool,
    imgui_button_with_size: extern "Rust" fn(&Ui, &str, [f32; 2]) -> bool,
    imgui_slider_i32: extern "Rust" fn(&Ui, &str, i32, i32, &mut i32) -> bool,
    imgui_slider_u32: extern "Rust" fn(&Ui, &str, u32, u32, &mut u32) -> bool,
    imgui_slider_f32: extern "Rust" fn(&Ui, &str, f32, f32, &mut f32) -> bool,
    imgui_cursor_pos: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_cursor_screen_pos: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_set_cursor_pos: extern "Rust" fn(&Ui, [f32; 2]),
    imgui_set_cursor_screen_pos: extern "Rust" fn(&Ui, [f32; 2]),
    imgui_checkbox: extern "Rust" fn(&Ui, &str, &mut bool) -> bool,
    imgui_item_rect_size: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_item_rect_min: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_item_rect_max: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_calc_text_size: extern "Rust" fn(&Ui, &str) -> [f32; 2],
    imgui_group: extern "Rust" fn(&Ui, Box<dyn FnOnce()>),
    imgui_input_text_multiline: extern "Rust" fn(&Ui, &str, &mut String, [f32; 2]) -> bool,
    imgui_input_text: extern "Rust" fn(&Ui, &str, &mut String) -> bool,
    imgui_activate_font_by_rpath:
        Box<dyn Fn(&Ui, Arc<String>) -> Option<FontStackToken<'_>> + Send + Sync>,
    imgui_pop_font: extern "Rust" fn(&Ui, FontStackToken<'_>),
    imgui_dummy: extern "Rust" fn(&Ui, [f32; 2]),
    imgui_same_line: extern "Rust" fn(&Ui),
    imgui_window_size: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_separator: extern "Rust" fn(&Ui),
    imgui_background_add_rect: extern "Rust" fn(&Ui, [f32; 2], [f32; 2], f32, bool, [f32; 4]),
    imgui_window_add_rect: extern "Rust" fn(&Ui, [f32; 2], [f32; 2], f32, bool, [f32; 4]),
    imgui_window_pos: extern "Rust" fn(&Ui) -> [f32; 2],
    imgui_set_next_item_width: extern "Rust" fn(&Ui, f32),
    imgui_columns: extern "Rust" fn(&Ui, i32, &str, bool),
    imgui_next_column: extern "Rust" fn(&Ui),
    imgui_set_column_offset: extern "Rust" fn(&Ui, i32, f32),
    imgui_set_column_width: extern "Rust" fn(&Ui, i32, f32),
    imgui_current_column_index: extern "Rust" fn(&Ui) -> i32,
    imgui_current_column_offset: extern "Rust" fn(&Ui) -> f32,
    imgui_column_width: extern "Rust" fn(&Ui, i32) -> f32,
    imgui_begin_combo: extern "Rust" fn(&Ui, &str, &str, Box<dyn FnOnce()>),
    imgui_selectable: extern "Rust" fn(&Ui, &str) -> bool,
    imgui_set_item_default_focus: extern "Rust" fn(&Ui),
}

/// Arctic Gateways are a plugin system for dynamic which is capable of loading user-created DLLs
/// and inject a pre-defined function for passing certain functions.
pub struct Arctic {
    /// Cached DNXFunctions structure.
    cached_functions: OnceLock<Arc<DNXFunctions>>,

    /// BaseCore instance.
    base_core: Arc<RwLock<BaseCore>>,

    /// All injected DLLs.
    injected_dlls: Arc<Mutex<HashMap<ProcessModule<BorrowedProcess<'static>>, String>>>,
}

impl Arctic {
    /// Initializes Arctic and returns an instance of `self`.
    pub fn init(base_core: Arc<RwLock<BaseCore>>) -> Self {
        let injected_dlls = Arc::new(Mutex::new(HashMap::new()));
        let instance = Self {
            cached_functions: {
                let base_core_reader = base_core
                    .try_read()
                    .unwrap_or_crash(zencstr!("[ERROR] Failed reading BaseCore!"));

                let crosscom_check_is_ex_serial_ok = base_core_reader.get_crosscom();
                let crosscom_server_aob_scan = base_core_reader.get_crosscom();
                let crosscom_seerver_get_variable = base_core_reader.get_crosscom();
                let crosscom_server_get_current_channel = base_core_reader.get_crosscom();
                let crosscom_server_join_channel = base_core_reader.get_crosscom();
                let crosscom_server_send_script = base_core_reader.get_crosscom();

                let imgui_utils = base_core_reader.get_imgui_utils();

                let config = base_core_reader.get_config();
                let serials = config.get_product_serials();
                let base_core_rune = Arc::clone(&base_core);
                let base_core_install_rune_module = Arc::clone(&base_core);
                let injected_dlls = Arc::clone(&injected_dlls);

                let custom_window_utils = base_core_reader.get_custom_window_utils();

                drop(base_core_reader);

                let funcs = DNXFunctions {
                    dynamic_log: |data| {
                        log!(data);
                    },
                    memory_read_string: SystemModules::read_string,
                    dynamic_get_delta_time: || unsafe { DELTA_TIME },
                    dynamic_eject_payload: Box::new(move |owned_process, payload| {
                        Self::eject_payload(owned_process, payload, Arc::clone(&injected_dlls))
                    }),
                    rune_vm_execute: Box::new(move |source| {
                        let reader = base_core_rune.read();
                        reader.get_script_core().execute(
                            source,
                            Arc::clone(&base_core_rune),
                            false,
                            false,
                        );

                        drop(reader);
                    }),
                    dyamic_add_thread_key: SystemModules::create_thread_key,
                    dynamic_set_thread_key_value: SystemModules::set_thread_key_value,
                    dynamic_get_thread_key: SystemModules::get_thread_key,
                    ui_add_window: Box::new(move |name| {
                        custom_window_utils.add_window(name);
                    }),
                    ui_focus_window: Box::new(move |name| {
                        custom_window_utils.set_current_window_to(name);
                    }),
                    sellix_is_paying_for_product: Box::new(move |product_id, bearer_token| {
                        crosscom_check_is_ex_serial_ok.read().check_is_ex_serial_ok(
                            product_id,
                            bearer_token,
                            serials,
                        )
                    }),
                    config_has_serial: Box::new(move |serial| serials.contains(&serial)),
                    server_aob_scan: Box::new(
                        move |variable_name, base_address, sig_scan_address| {
                            let crosscom = Arc::clone(&crosscom_server_aob_scan);
                            let variables = Arc::new(RwLock::new(crosscom.read().get_variables()));
                            WinUtils::server_aob_scan(
                                variable_name,
                                base_address,
                                variables,
                                sig_scan_address,
                                crosscom,
                            )
                        },
                    ),
                    server_get_variable: Box::new(move |variable_name| {
                        let crosscom = Arc::clone(&crosscom_seerver_get_variable);
                        let reader = crosscom.read();
                        reader.get_variables().get(variable_name).cloned()
                    }),
                    crash: |message| crash!(message),
                    install_rune_module: Box::new(move |module| {
                        base_core_install_rune_module
                            .read()
                            .get_script_core()
                            .add_rune_module(module);
                    }),
                    server_get_current_channel: Box::new(move || {
                        crosscom_server_get_current_channel
                            .read()
                            .get_current_channel()
                            .borrow()
                            .to_owned()
                    }),
                    server_join_channel: Box::new(move |channel| {
                        crosscom_server_join_channel.read().join_channel(channel);
                    }),
                    server_send_script: Box::new(move |source| {
                        crosscom_server_send_script.read().send_script(source);
                    }),
                    config_get_file_content: Box::new(move |relative_path, output_string| {
                        config.get_file_content(relative_path, output_string)
                    }),
                    config_save_to_file: Box::new(move |relative_path, content| {
                        config.save_to_file(relative_path, content)
                    }),
                    config_get_path: Box::new(|| config.get_path()),
                    imgui_text: |ui, text| ui.text(text),
                    imgui_button: |ui, text| ui.button(text),
                    imgui_button_with_size: |ui, text, size| ui.button_with_size(text, size),
                    imgui_slider_i32: |ui, text, min, max, output| {
                        slider!(ui, text, min, max, *output)
                    },
                    imgui_slider_u32: |ui, text, min, max, output| {
                        slider!(ui, text, min, max, *output)
                    },
                    imgui_slider_f32: |ui, text, min, max, output| {
                        slider!(ui, text, min, max, *output)
                    },
                    imgui_cursor_pos: |ui| ui.cursor_pos(),
                    imgui_cursor_screen_pos: |ui| ui.cursor_screen_pos(),
                    imgui_set_cursor_pos: |ui, pos| ui.set_cursor_pos(pos),
                    imgui_set_cursor_screen_pos: |ui, pos| ui.set_cursor_screen_pos(pos),
                    imgui_checkbox: |ui, text, output| ui.checkbox(text, output),
                    imgui_item_rect_size: |ui| ui.item_rect_size(),
                    imgui_item_rect_min: |ui| ui.item_rect_min(),
                    imgui_item_rect_max: |ui| ui.item_rect_max(),
                    imgui_calc_text_size: |ui, text| ui.calc_text_size(text),
                    imgui_group: |ui, closure| ui.group(closure),
                    imgui_dummy: |ui, size| ui.dummy(size),
                    imgui_input_text_multiline: |ui, text, output, size| {
                        ui.input_text_multiline(text, output, size).build()
                    },
                    imgui_input_text: |ui, text, output| ui.input_text(text, output).build(),
                    imgui_columns: |ui, count, id, border| ui.columns(count, id, border),
                    imgui_pop_font: |_, font_stack_token| font_stack_token.pop(),
                    imgui_same_line: |ui| ui.same_line(),
                    imgui_separator: |ui| ui.separator(),
                    imgui_window_pos: |ui| ui.window_pos(),
                    imgui_selectable: |ui, text| ui.selectable(text),
                    imgui_window_size: |ui| ui.window_size(),
                    imgui_next_column: |ui| ui.next_column(),
                    imgui_begin_combo: |ui, text, preview_value, closure| {
                        let Some(combo) = ui.begin_combo(text, preview_value) else {
                            return;
                        };

                        closure();
                        combo.end();
                    },
                    imgui_column_width: |ui, id| ui.column_width(id),
                    imgui_window_add_rect: |ui, start, end, rounding, filled, color| {
                        ui.get_window_draw_list()
                            .add_rect(start, end, color)
                            .filled(filled)
                            .rounding(rounding)
                            .build()
                    },
                    imgui_background_add_rect: |ui, start, end, rounding, filled, color| {
                        ui.get_background_draw_list()
                            .add_rect(start, end, color)
                            .filled(filled)
                            .rounding(rounding)
                            .build()
                    },
                    imgui_set_column_width: |ui, id, width| ui.set_column_width(id, width),
                    imgui_set_column_offset: |ui, id, offset| ui.set_column_offset(id, offset),
                    imgui_set_next_item_width: |ui, width| ui.set_next_item_width(width),
                    imgui_current_column_index: |ui| ui.current_column_index(),
                    imgui_current_column_offset: |ui| ui.current_column_offset(),
                    imgui_activate_font_by_rpath: Box::new(move |ui, relative_font_path| {
                        ImGuiUtils::activate_font(
                            ui,
                            imgui_utils
                                .try_read()?
                                .get_cfont_from_rpath(Arc::clone(&relative_font_path)),
                        )
                    }),
                    imgui_set_item_default_focus: |ui| ui.set_item_default_focus(),
                    config_get_serials: Box::new(|| serials),
                };

                // Create the OnceLock instance and assign it before returning.
                let lock = OnceLock::new();
                lock.get_or_init(|| Arc::new(funcs));
                lock
            },
            base_core,
            injected_dlls,
        };

        instance.link_script_received();
        instance
    }

    /// Injects an Arctic DLL and calls its `arctic_gateway` function.
    pub fn arctic_inject_gateway(&self, dll_name: String) -> bool {
        let config_path = self.base_core.read().get_config().get_path();
        let mut dll_path = String::with_capacity(config_path.len() + dll_name.len());
        dll_path.push_str(config_path);
        dll_path.push_str(&dll_name);

        if !std::path::Path::new(&dll_path).is_file() {
            log!(
                "[ERROR] The DLL \"",
                dll_name,
                "\" does not exist, ensure the relative path is correct!"
            );
            return false;
        }

        let target_process = unsafe { OwnedProcess::from_raw_handle(GetCurrentProcess().0 as _) };

        let payload = Box::leak(Box::new(Syringe::for_process(target_process))).inject(dll_path);
        if let Err(error) = payload {
            log!("[ERROR] DLL Injection failed with error code: ", error);
            log!("[INFO] Run dynamic's DLL injector as an Administrator and disable Windows Defender.");
            log!("[INFO] If the issue persists, make a new post in #assistance.");
            return false;
        }

        let address = WinUtils::get_module_symbol_address(&dll_name, c"arctic_gateway")
            .unwrap_or_crash(zencstr!(
                "[ERROR] Can't get the address to arctic_gateway, badly-written plugin!"
            )) as *const ();

        let func: extern "Rust" fn(
            OwnedProcess,
            ProcessModule<BorrowedProcess<'static>>,
            Arc<DNXFunctions>,
        ) = unsafe { std::mem::transmute(address) };

        // Save the DLL and its payload so we remember it.
        let payload = payload
            .unwrap_or_else(|error| crash!("[ERROR] Failed getting payload, error: ", error));
        self.injected_dlls.lock().insert(payload, dll_name);

        // Handle function calls from the library.
        func(
            unsafe { OwnedProcess::from_raw_handle(GetCurrentProcess().0 as _) },
            payload,
            Arc::clone(
                self.cached_functions
                    .get()
                    .unwrap_or_crash(zencstr!("[ERROR] No cached functions!")),
            ),
        );

        true
    }

    /// Attempts to safely eject the payload.
    pub fn eject_payload(
        process: OwnedProcess,
        payload: ProcessModule<BorrowedProcess<'static>>,
        injected_dlls: Arc<Mutex<HashMap<ProcessModule<BorrowedProcess<'static>>, String>>>,
    ) {
        std::thread::spawn(move || {
            // Remove the payload from the saved DLLs list.
            if let Some(removed) = injected_dlls.lock().remove(&payload).take() {
                drop(removed);
            }

            // Wait for 500ms to prevent crashing, and to give the DLL some time to really finish
            // what it's doing.
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Attempt eject.
            Syringe::for_process(process)
                .eject(payload)
                .unwrap_or_else(|error| crash!("[ERROR] Ejection error: ", error));
        });
    }

    /// Gets the injected DLLs.
    pub fn get_injected_dlls(
        &self,
    ) -> Arc<Mutex<HashMap<ProcessModule<BorrowedProcess<'static>>, String>>> {
        Arc::clone(&self.injected_dlls)
    }

    /// Hooks script/source received events from CrossCom with a function that calls `on_script_received` on all injected DLLs once received.
    fn link_script_received(&self) {
        let Some(reader) = self.base_core.try_read() else {
            log!("[ERROR] Can't access BaseCore for Arctic::link_script_received!");
            return;
        };

        let crosscom_clone_reader = reader.get_crosscom();
        let crosscom = reader.get_crosscom();
        let Some(reader) = crosscom_clone_reader.try_read() else {
            log!("[ERROR] Can't access CrossCom for Arctic::link_script_received!");
            return;
        };

        let injected_dlls = self.get_injected_dlls();
        reader
            .get_network_listener()
            .hook_on_script_received(crosscom, move |mut source| {
                let Some(injected_dlls) = injected_dlls.try_lock() else {
                    log!("[ERROR] Failed locking injected DLLs HashMap, can't call functions!");
                    return;
                };

                for module_name in injected_dlls.values() {
                    if let Some(func) =
                        WinUtils::get_module_symbol_address(module_name, c"on_script_received")
                    {
                        let func: extern "Rust" fn(String) = unsafe { std::mem::transmute(func) };
                        func(std::mem::take(&mut source));
                    }
                }
            });
    }

    /// Checks if a gateway/plugin is currently active.
    pub fn is_gateway_active(&self, identifier: String) -> bool {
        self.get_injected_dlls()
            .try_lock()
            .map_or(false, |injected_dlls| {
                injected_dlls
                    .values()
                    .any(|module_name| *module_name == identifier)
            })
    }
}
