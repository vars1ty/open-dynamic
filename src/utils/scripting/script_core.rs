use super::script_modules::SystemModules;
use crate::{
    mod_cores::base_core::BaseCore,
    utils::{
        crosscom::CrossCom,
        extensions::{OptionExt, StringExtensions},
        scripting::{arctic::Arctic, script_modules::UIModules},
    },
};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use rune::{
    termcolor::{ColorChoice, StandardStream},
    *,
};
use runtime::SyncFunction;
use std::{error::Error, ffi::CString, sync::Arc};
use zstring::ZString;

/// Wrapper around `Value` to force it to be "thread-safe".
pub struct ValueWrapper(pub Value);
thread_safe_structs!(ValueWrapper);

/// Experimental Mutexes for Rune.
#[derive(Any)]
pub struct MutexValue {
    /// Inner `Value` of the Mutex.
    inner: Mutex<Value>,
}

impl MutexValue {
    /// Creates a new instance of `Self`.
    pub fn new(value: Value) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }

    /// Tries to clone and return the inner value of the Mutex.
    /// Returns `None` if locked.
    pub fn try_get(&self) -> Option<Value> {
        self.inner.try_lock().as_deref().cloned()
    }

    /// Tries to change the inner value of the Mutex.
    /// Returns `false` if locked.
    pub fn try_set(&self, new_value: Value) -> bool {
        let Some(mut inner) = self.inner.try_lock() else {
            return false;
        };

        *inner = new_value;
        true
    }

    /// Checks if the inner value is locked or not.
    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }
}

/// Information about a frame update callback.
pub struct FrameUpdateCallback {
    /// Callback function.
    callback: SyncFunction,

    /// Optional parameter to pass into `callback`.
    opt_param: Option<Value>,
}

impl FrameUpdateCallback {
    /// Builds a new instance of `FrameUpdateCallback`.
    pub fn new(callback: SyncFunction, opt_param: Option<Value>) -> Self {
        Self {
            callback,
            opt_param,
        }
    }
}

/// Structure that implements Send and Sync so that the `Vm` inside of it can be used for
/// `compiled_scripts`.
/// It is **not** recommended to send the VM instance across threads, instead use `SyncFunction` if
/// you need to call functions.
struct VMWrapper(pub Vm);
thread_safe_structs!(VMWrapper);

/// Rune Scripting core.
pub struct ScriptCore {
    /// VM String settings.
    /// 0 -> Executing function, typically pub fn main().
    /// 1 -> Inline code keyword, typically import.
    vm_string_settings: [&'static str; 2],

    /// All compiled scripts with their own VM instance.
    /// Key being the script content hashed, value being the VM inside of `VMWrapper`.
    /// Key is hashed to prevent finding it in memory, and it doesn't need to be plain-text.
    compiled_scripts: Arc<DashMap<String, VMWrapper>>,

    /// Modules installed outside of dynamic.
    cross_modules: Mutex<Vec<Module>>,

    /// Special comments that upon found, toggle special compilation behavior.
    compiler_special_settings: [&'static str; 2],

    /// Global Script Variables.
    global_script_variables: Arc<DashMap<String, ValueWrapper>>,

    /// Frame update callbacks. Each function is called every new frame, once for each window.
    on_frame_update_callbacks: Arc<DashMap<String, FrameUpdateCallback>>,
}

thread_safe_structs!(ScriptCore);

impl ScriptCore {
    /// Initializes everything needed for the Rune implementation to work.
    pub fn init() -> Self {
        Self {
            vm_string_settings: ["pub fn main()", "import"],
            compiled_scripts: Default::default(),
            cross_modules: Default::default(),
            compiler_special_settings: [
                "//# EnableCompilerOption: NewThreadMain",
                "//# DisableCompilerOption: CLIDiagnostics",
            ],
            global_script_variables: Default::default(),
            on_frame_update_callbacks: Default::default(),
        }
    }

    /// Initializes the Rune runtime and compiles some code.
    pub fn compile(
        &self,
        source: &str,
        base_core: Arc<RwLock<BaseCore>>,
    ) -> Result<Vm, Box<dyn Error>> {
        let Some(base_core_reader) = base_core.try_read() else {
            return Err("Failed reading BaseCore!".into());
        };

        // Init Arctic if not already done.
        base_core_reader
            .get_arctic_core()
            .get_or_init(|| Arctic::init(Arc::clone(&base_core)));

        let mut context = Context::with_default_modules()?;
        for module in SystemModules::build(
            Arc::clone(&base_core),
            base_core_reader.get_crosscom(),
            base_core_reader.get_config().get_product_serials(),
        )? {
            context.install(module)?;
        }

        for module in &*self
            .cross_modules
            .try_lock()
            .ok_or("Cross Modules is locked!")?
        {
            context.install(module)?;
        }

        context.install(UIModules::build(
            Arc::clone(&base_core),
            base_core_reader.get_custom_window_utils(),
        )?)?;
        drop(base_core_reader);

        let runtime = Arc::new(context.runtime()?);
        let mut sources = Sources::new();
        sources.insert(Source::new("main", source)?)?;

        let mut diagnostics = Diagnostics::new();
        let result = rune::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !source.contains(self.compiler_special_settings[1]) && !diagnostics.is_empty() {
            // Never show colors as not all terminals handle it properly.
            // Auto doesn't realize this.
            let mut writer = StandardStream::stderr(ColorChoice::Never);
            diagnostics.emit(&mut writer, &sources)?;
        }

        let unit = result?;

        // VM isn't stored here directly, it's only stored in `execute`, assuming compilation
        // **and** execution was successful.
        Ok(Vm::new(runtime, Arc::new(unit)))
    }

    /// Takes the source of a script, then calls the `main` function.
    /// If the source of the script hasn't been compiled before, its compiled, cached and then the
    /// `main` function is called.
    /// Optionally it also sends the `source` to the active party.
    pub fn execute(
        &self,
        source: String,
        base_core: Arc<RwLock<BaseCore>>,
        send_src_to_network: bool,
        force_new_thread: bool,
    ) {
        if source.is_empty() {
            log!("[WARN] Attempted to execute empty source, cancelling.");
            return;
        }

        let Some(reader) = base_core.try_read() else {
            log!("[ERROR] Compilation failed because BaseCore is locked!");
            return;
        };

        let use_new_thread = reader.get_config().get_use_new_rune_thread()
            || force_new_thread
            || source.contains(self.compiler_special_settings[0]);

        let source = self
            .add_imports(&source, reader.get_config().get_path())
            .unwrap_or(source);

        let hash = source.get_hash();
        if self.compiled_scripts.get(&hash).is_some() {
            let crosscom = reader.get_crosscom();
            drop(reader);

            // Cached, run the main function without compiling.
            self.exec_main(source, crosscom, send_src_to_network, use_new_thread);
            return;
        }

        let start = std::time::Instant::now();
        let compile = self.compile(&source, Arc::clone(&base_core));
        if let Ok(vm) = compile {
            // Uncached source. Compile, store and run the main function.
            self.compiled_scripts.insert(hash, VMWrapper(vm));
            let crosscom = reader.get_crosscom();
            drop(reader);

            // Run main and print the elapsed time.
            self.exec_main(source, crosscom, send_src_to_network, use_new_thread);
            log!(
                "[Script Engine] Script compiled in ",
                format!("{:.2?}!", start.elapsed())
            );
            return;
        }

        log!("[Script Engine] Compile error: ", compile.unwrap_err());
    }

    /// Executes the main function found in `source`.
    /// This is **not** asynchronous due to the hard barriers put in place thanks to unsafe code
    /// and its poor stability with runtimes.
    /// It works by forcing a wrapper of `Vm` to be thread-safe, then sends it into a new standard
    /// thread for execution.
    /// This is _not_ intended to work, and the downsides are that it may crash without error
    /// messages.
    fn exec_main(
        &self,
        source: String,
        crosscom: Arc<RwLock<CrossCom>>,
        send_src_to_network: bool,
        use_new_thread: bool,
    ) {
        let Some(vm) = self.compiled_scripts.get(&source.get_hash()) else {
            return;
        };

        let main = vm.0.lookup_function(["main"]);
        if let Err(error) = main {
            log!("[ERROR] Compile error when looking up main, error: ", error);
            return;
        };

        let main_sync = main.unwrap().into_sync().into_result();
        if let Err(error) = main_sync {
            log!(
                "[ERROR] Failed turning main into a SyncFunction, error: ",
                error
            );
            return;
        }

        let code = move || {
            log!("[Script Engine] Script executing...");
            let execution = main_sync.unwrap().call::<(), ()>(()).into_result();
            if let Err(error) = execution {
                log!("[ERROR] Compile error when executing main, error: ", error);
                return;
            };

            log!("[Script Engine] Script finished executing!");
            if send_src_to_network && let Some(reader) = crosscom.try_read() {
                reader.send_script(&source);
            }
        };

        if use_new_thread {
            log!("[Script Engine] Running script on a new thread...");
            std::thread::spawn(move || {
                code();
                log!("[Script Engine] Script finished executing!");
            });
            return;
        }

        code();
    }

    /// Adds referenced imports to the initial script, then returns the result.
    fn add_imports(&self, source: &str, config_directory: &str) -> Option<String> {
        let pub_fn_main = self.vm_string_settings[0];
        let r#macro = self.vm_string_settings[1];

        // Only process if the "macro" has a chance of existing.
        if !source.contains(r#macro) {
            return None;
        }

        let mut new_source = source.to_owned();

        // Loop over all lines until we hit `pub fn main()`.
        for (i, line) in source.lines().enumerate() {
            // If the line starts with `pub fn main()`, exit loop.
            if line.starts_with(pub_fn_main) {
                break;
            }

            // If the line doesn't start with `import ` and end with `.rn`, skip.
            if !line.starts_with(r#macro) && !line.ends_with(&zencstr!(".rn").data) {
                continue;
            }

            // Collect information.
            let Some(sourced_file) = line.split(r#macro).nth(1) else {
                log!(
                    "[ERROR] Invalid use of the `",
                    self.vm_string_settings[1],
                    "` macro, error at line #",
                    i,
                    " when processing script!"
                );
                return None;
            };

            // Valid source usage, process.
            let mut path = ZString::new(config_directory.to_owned());
            path.push_zstring(ZString::new(sourced_file.trim()));
            let read = std::fs::read_to_string(&path.data);
            drop(path);

            if let Ok(read_content) = read {
                // Source read, append.
                new_source.push('\n');
                new_source.push_str(&read_content);
                new_source.push('\n');
                continue;
            }

            // Couldn't read import file, print error information.
            log!(
                "[ERROR] Failed reading import file, error: ",
                read.unwrap_err()
            );
        }

        // Remove all lines that start with "import ".
        new_source = new_source
            .lines()
            .filter(|line| !line.starts_with(r#macro))
            .collect::<Vec<_>>()
            .join("\n");

        Some(new_source)
    }

    /// Adds a module to `cross_modules` which is a set of modules that have been added from
    /// outside of dynamic.
    /// TODO: Implement a string identifier tied to the module which can be used to remove the
    /// module.
    /// Not removing a module after a plugin has been ejected results in crashes.
    pub fn add_rune_module(&self, module: Module) {
        self.cross_modules
            .try_lock()
            .unwrap_or_crash(zencstr!(
                "[ERROR] Cross Modules is locked, modules cannot be inserted!"
            ))
            .push(module);
    }

    /// Casts `data` as a `*const i64` pointer, note that this is **not** recommended for
    /// floating-point numbers.
    /// If a string, the C-String is "forgotten about" and should be dropped manually!
    pub fn value_as_ptr(data: &Value) -> Option<*const i64> {
        if let Ok(data_i64) = data.as_integer().into_result() {
            return Some(data_i64 as *const i64);
        }

        if let Ok(data_usize) = data.as_usize().into_result() {
            return Some(data_usize as *const usize as *const i64);
        }

        if let Ok(data_f64) = data.as_float().into_result() {
            return Some(unsafe {
                std::mem::transmute::<*const f64, *const i64>(&data_f64 as *const f64)
            });
        }

        if let Ok(data_bool) = data.as_bool().into_result() {
            return Some(data_bool as u8 as *const i64);
        }

        let Ok(data_string) = data.to_owned().into_string().into_result() else {
            return None;
        };

        let Ok(data_string) = data_string.borrow_ref() else {
            return None;
        };

        let cstr = CString::new(data_string.to_owned()).ok()?;
        Some(cstr.into_raw() as *const i64)
    }

    /// Adds a new `on_frame_update` callback to `self.on_frame_update`. If there is already a
    /// callback defined as `identifier`, then it's replaced.
    pub fn register_frame_update_callback(
        &self,
        identifier: String,
        callback: SyncFunction,
        opt_param: Option<Value>,
    ) {
        self.on_frame_update_callbacks
            .insert(identifier, FrameUpdateCallback::new(callback, opt_param));
    }

    /// Removes the defined callback if present.
    pub fn remove_frame_update_callback(&self, identifier: &str) {
        self.on_frame_update_callbacks.remove(identifier);
    }

    /// Calls all callbacks and passes in `window` and `ui`.
    /// If `window` and/or `ui` are `None`, then the callback was issued outside of a window.
    pub fn call_frame_update_callbacks(
        &self,
        window: Option<&str>,
        ui: Option<&hudhook::imgui::Ui>,
    ) {
        if self.on_frame_update_callbacks.is_empty() {
            return;
        }

        let ui_ptr = ui.map(|ui| std::ptr::addr_of!(ui) as i64);
        for entry in &*self.on_frame_update_callbacks {
            let frame_update_callback_data = entry.value();
            if let Err(error) = frame_update_callback_data
                .callback
                .call::<(Option<&Value>, Option<&str>, Option<i64>), ()>((
                    frame_update_callback_data.opt_param.as_ref(),
                    window,
                    ui_ptr,
                ))
                .into_result()
            {
                log!(
                    "[ERROR] Failed calling frame update callback on \"",
                    entry.key(),
                    "\", error: ",
                    error
                );
            }
        }
    }

    /// Returns `self.global_script_variables`.
    pub fn get_global_script_variables(&self) -> Arc<DashMap<String, ValueWrapper>> {
        Arc::clone(&self.global_script_variables)
    }
}
