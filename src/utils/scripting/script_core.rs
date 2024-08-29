use super::script_modules::SystemModules;
use crate::{
    mod_cores::base_core::BaseCore,
    utils::{
        crosscom::CrossCom,
        extensions::{OptionExt, StringExtensions},
        scripting::{arctic::Arctic, script_modules::UIModules},
    },
};
use ahash::AHashMap;
use parking_lot::{Mutex, RwLock};
use rune::{
    termcolor::{ColorChoice, StandardStream},
    *,
};
use std::{error::Error, ffi::CString, sync::Arc};
use zstring::ZString;

/// Wrapper around `Value` to force it to be "thread-safe".
pub struct ValueWrapper(pub Value);
thread_safe_structs!(ValueWrapper);

/// Structure that implements Send and Sync so that the `Vm` inside of it can be sent between
/// threads.
/// It also embeds a hash of the script source so that it can be compared against the key hash,
/// ensuring that it becomes more difficult to tamper with.
struct VMWrapper {
    /// Virtual Machine instance.
    pub vm: Vm,

    /// Hash of the source inside the Virtual Machine.
    pub hash: String,
}

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
    /// This is a Mutex so we don't lock the entire script engine all at once whenever we try and
    /// access individual parts of it.
    compiled_scripts: Arc<Mutex<AHashMap<String, VMWrapper>>>,

    /// Modules installed outside of dynamic.
    cross_modules: Mutex<Vec<Module>>,

    /// Special comments that upon found, toggle special compilation behavior.
    compiler_special_settings: [&'static str; 2],

    /// Global Script Variables.
    global_script_variables: Arc<RwLock<AHashMap<String, ValueWrapper>>>,
}

thread_safe_structs!(ScriptCore);

impl ScriptCore {
    /// Initializes the cached compilations `AHashMap`, boosting overall performance.
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
            Arc::clone(&self.global_script_variables),
        )? {
            context.install(module)?;
        }

        for module in &*self
            .cross_modules
            .try_lock()
            .unwrap_or_crash(zencstr!("[ERROR] Cross Modules is locked!"))
        {
            context.install(module)?;
        }

        context.install(UIModules::build(
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

        if let Err(error) = &result {
            log!("[ERROR] Compile Error in unit: ", error);
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

        let hash = source.get_hash();
        let Some(reader) = base_core.try_read() else {
            log!("[ERROR] Compilation failed because BaseCore is locked!");
            return;
        };

        let use_new_thread = reader.get_config().get_use_new_rune_thread()
            || force_new_thread
            || source.contains(self.compiler_special_settings[0]);

        let Some(mut compiled_scripts) = self.compiled_scripts.try_lock() else {
            log!("[ERROR] Compilation failed because Compiled Scripts is locked!");
            log!(
                "[INFO] This is common if you are already running a script, as it has to complete before you can continue."
            );
            return;
        };

        self.perform_integrity_check(&compiled_scripts);
        let source = self.add_imports(&source, reader.get_config().get_path());
        if compiled_scripts.get(&hash).is_some() {
            let crosscom = reader.get_crosscom();
            drop(compiled_scripts);
            drop(reader);

            // Cached, run the main function without compiling.
            self.exec_main(source, crosscom, send_src_to_network, use_new_thread);
            return;
        }

        let start = std::time::Instant::now();
        let compile = self.compile(&source, Arc::clone(&base_core));
        if let Ok(vm) = compile {
            // Uncached, compile, store and run the main function.
            let hash_clone = hash.to_owned();
            compiled_scripts.insert(
                hash,
                VMWrapper {
                    vm,
                    hash: hash_clone,
                },
            );
            let crosscom = reader.get_crosscom();
            drop(compiled_scripts);
            drop(reader);

            // Run main and print the elapsed time.
            self.exec_main(source, crosscom, send_src_to_network, use_new_thread);
            log!(
                "[Script Engine] Script compiled in ",
                start.elapsed().as_millis(),
                "ms!"
            );
            return;
        }

        log!("[Script Engine] Compile error: ", compile.unwrap_err());
    }

    /// Performs a basic integrity check on all scripts, ensuring that the stored hash inside of
    /// the value structure, is the same as the key.
    fn perform_integrity_check(&self, compiled_scripts: &AHashMap<String, VMWrapper>) {
        for (script_hash, vm_info_struct) in compiled_scripts {
            if vm_info_struct.hash != *script_hash {
                crash!("[ERROR] Script Tampering detected!");
            }
        }
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
        let compiled_scripts = Arc::clone(&self.compiled_scripts);
        let code = move || {
            let compiled_scripts = compiled_scripts.try_lock();
            let Some(vm) = compiled_scripts
                .as_ref()
                .and_then(|compiled_scripts| compiled_scripts.get(&source.get_hash()))
            else {
                return;
            };

            let main = vm.vm.lookup_function(["main"]);
            if let Err(error) = main {
                log!("[ERROR] Compile error when looking up main, error: ", error);
                return;
            };

            drop(compiled_scripts);
            log!("[Script Engine] Script executing...");
            let execution = main.unwrap().call::<(), ()>(()).into_result();
            if let Err(error) = execution {
                log!("[ERROR] Compile error when executing main, error: ", error);
                return;
            };

            // Notify that the script is executing.
            log!("[Script Engine] Script finished executing!");

            // Send to party.
            if send_src_to_network {
                if let Some(reader) = crosscom.try_read() {
                    reader.send_script(&source);
                }
            }
        };

        if use_new_thread {
            log!("[Script Engine] Running script on a new thread...");
            std::thread::spawn(move || {
                code();
            });
            return;
        }

        code();
    }

    /// Adds referenced imports to the initial script, then returns the result.
    fn add_imports(&self, source: &str, config_directory: &str) -> String {
        let pub_fn_main = self.vm_string_settings[0];
        let r#macro = self.vm_string_settings[1];
        let mut new_source = source.to_owned();

        // Only process if the "macro" has a chance of existing.
        if !new_source.contains(r#macro) {
            return new_source;
        }

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
                return String::default();
            };

            // Valid source usage, process.
            let mut path = ZString::new(config_directory.to_owned());
            path.data += sourced_file.trim();
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

        new_source
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
        if let Ok(data_i64) = data.to_owned().into_integer().into_result() {
            return Some(data_i64 as *const i64);
        }

        if let Ok(data_usize) = data.to_owned().into_usize().into_result() {
            return Some(data_usize as *const usize as *const i64);
        }

        if let Ok(data_f64) = data.to_owned().into_float().into_result() {
            return Some(unsafe {
                std::mem::transmute::<*const f64, *const i64>(&data_f64 as *const f64)
            });
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
}
