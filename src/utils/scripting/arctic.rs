use crate::{
    globals::DELTA_TIME,
    mod_cores::base_core::BaseCore,
    utils::{extensions::OptionExt, scripting::script_modules::*},
    winutils::WinUtils,
};
use dashmap::DashMap;
use dll_syringe::{
    process::{BorrowedProcess, OwnedProcess, ProcessModule},
    Syringe,
};
use parking_lot::RwLock;
use rune::Module;
use std::{
    os::windows::io::FromRawHandle,
    sync::{atomic::Ordering, Arc, OnceLock},
};
use windows::Win32::System::Threading::GetCurrentProcess;

/// A structure that contains a set of functions from dynamic.
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub struct DNXFunctions {
    /// `dynamic::log(message)` function. Logs both to the side-messages, and to `stdout`.
    dynamic_log: fn(&str),

    /// `Memory::read_string(address) function. Attempts to read a string at `address`.
    memory_read_string: fn(i64) -> &'static str,

    /// `dynamic::get_delta_time()` function. Gets the current delta-time of the process.
    dynamic_get_delta_time: fn() -> f32,

    /// Special function for making dynamic eject the DLL, rather than the other way around.
    /// This is needed because otherwise the process crashes.
    dynamic_eject_payload:
        Box<dyn Fn(OwnedProcess, ProcessModule<BorrowedProcess<'static>>) + Send + Sync>,

    /// Rune VM which you may use to execute Rune code.
    rune_vm_execute: Box<dyn Fn(String) + Send + Sync>,

    /// `dynamic::create_thread_key(name)` function. Creates a globally-accessible thread-key.
    dyamic_add_thread_key: Box<dyn Fn(String) + Send + Sync>,

    /// `dynamic::set_thread_key_value(name, value)` function. Sets the value of a thread-key.
    dynamic_set_thread_key_value: Box<dyn Fn(String, bool) + Send + Sync>,

    /// `dynamic::get_thread_key(name)` function. Returns the value of the thread-key.
    dynamic_get_thread_key: Box<dyn Fn(String) -> bool + Send + Sync>,

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
    crash: fn(&str) -> !,

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
    config_get_serials: Box<dyn Fn() -> Arc<Vec<String>> + Send + Sync>,
}

/// Arctic is a plugin system for dynamic which is capable of loading user-created DLLs
/// and pass a structure with function wrappers that let you call back to dynamic.
pub struct Arctic {
    /// Cached DNXFunctions structure.
    cached_functions: OnceLock<Arc<DNXFunctions>>,

    /// BaseCore instance.
    base_core: Arc<RwLock<BaseCore>>,

    /// All injected DLLs.
    injected_dlls: Arc<DashMap<ProcessModule<BorrowedProcess<'static>>, String>>,
}

impl Arctic {
    /// Initializes Arctic and returns an instance of `self`.
    pub fn init(base_core: Arc<RwLock<BaseCore>>) -> Self {
        let injected_dlls = Arc::new(DashMap::new());
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

                let config = base_core_reader.get_config();
                let serials_sellix_is_paying_for_product = config.get_product_serials();
                let serials_config_has_serial = config.get_product_serials();
                let serials_config_get_serials = config.get_product_serials();

                let base_core_rune = Arc::clone(&base_core);
                let base_core_install_rune_module = Arc::clone(&base_core);
                let injected_dlls = Arc::clone(&injected_dlls);

                let custom_window_utils = base_core_reader.get_custom_window_utils();
                let script_core = base_core_reader.get_script_core();

                drop(base_core_reader);

                let funcs = DNXFunctions {
                    dynamic_log: |data| {
                        log!(data);
                    },
                    memory_read_string: SystemModules::read_string,
                    dynamic_get_delta_time: || DELTA_TIME.load(Ordering::SeqCst),
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
                    dyamic_add_thread_key: Box::new(|identifier| {
                        SystemModules::create_thread_key(identifier, script_core.get_thread_keys())
                    }),
                    dynamic_set_thread_key_value: Box::new(|identifier, enabled| {
                        SystemModules::set_thread_key_value(
                            identifier,
                            enabled,
                            script_core.get_thread_keys(),
                        )
                    }),
                    dynamic_get_thread_key: Box::new(|identifier| {
                        SystemModules::get_thread_key(identifier, script_core.get_thread_keys())
                    }),
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
                            Arc::clone(&serials_sellix_is_paying_for_product),
                        )
                    }),
                    config_has_serial: Box::new(move |serial| {
                        serials_config_has_serial.contains(&serial)
                    }),
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
                    config_get_serials: Box::new(move || Arc::clone(&serials_config_get_serials)),
                };

                // Create the OnceLock instance and assign it before returning.
                let lock = OnceLock::new();
                lock.get_or_init(|| Arc::new(funcs));
                lock
            },
            base_core,
            injected_dlls,
        };

        instance
    }

    /// Injects an Arctic DLL and calls its `arctic_gateway` function.
    pub fn inject_plugin(&self, dll_name: String) -> bool {
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

        let func: fn(OwnedProcess, ProcessModule<BorrowedProcess<'static>>, Arc<DNXFunctions>) =
            unsafe { std::mem::transmute(address) };

        // Save the DLL and its payload so we remember it.
        let payload = payload
            .unwrap_or_else(|error| crash!("[ERROR] Failed getting payload, error: ", error));
        self.injected_dlls.insert(payload, dll_name);

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
        injected_dlls: Arc<DashMap<ProcessModule<BorrowedProcess<'static>>, String>>,
    ) {
        std::thread::spawn(move || {
            // Remove the payload from the saved DLLs list.
            if let Some(removed) = injected_dlls.remove(&payload).take() {
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
    ) -> Arc<DashMap<ProcessModule<BorrowedProcess<'static>>, String>> {
        Arc::clone(&self.injected_dlls)
    }

    /// Checks if a gateway/plugin is currently active.
    pub fn is_gateway_active(&self, identifier: String) -> bool {
        self.get_injected_dlls()
            .iter()
            .any(|module_name| *module_name == identifier)
    }
}
