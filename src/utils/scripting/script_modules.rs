use super::{
    fncaller::FNCaller,
    script_core::{MutexValue, ScriptCore, ValueWrapper},
};
use crate::{
    globals::*,
    mod_cores::base_core::BaseCore,
    utils::{
        crosscom::CrossCom,
        dynwidget::{SubWidgetType, WidgetType},
        extensions::{F32Ext, OptionExt, ResultExtensions},
        runedetour::RDetour,
        scripting::rune_ext_structs::RuneDoubleResultPrimitive,
        stringutils::StringUtils,
        ui::customwindows::CustomWindowsUtils,
    },
    winutils::WinUtils,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use rune::{
    alloc::clone::TryClone,
    runtime::{Function, SyncFunction},
    ContextError, Module, Value,
};
use std::{
    ffi::CString,
    fmt::{Debug, Display},
    rc::Rc,
    str::FromStr,
    sync::{atomic::Ordering, Arc},
};
use windows::Win32::System::Threading::GetCurrentProcess;
use wmem::Memory;
use zstring::ZString;

/// System modules, like Memory operations and such.
pub struct SystemModules;

impl SystemModules {
    /// Builds this module.
    #[optimize(size)]
    pub fn build(
        base_core: Arc<RwLock<BaseCore>>,
        crosscom: Arc<RwLock<CrossCom>>,
        serials: Arc<Vec<String>>,
    ) -> Result<Vec<Module>, ContextError> {
        let base_core_reader = base_core.read();
        let script_core = base_core_reader.get_script_core();
        let config = base_core_reader.get_config();
        drop(base_core_reader);

        let mut module = Module::new();
        let mut dynamic_module = Module::with_crate(&zencstr!("dynamic").data)?;
        let mut compiler_module = Module::with_crate(&zencstr!("Compiler").data)?;
        let mut task_module = Module::with_crate(&zencstr!("Task").data)?;
        let mut parse_module = Module::with_crate(&zencstr!("Parse").data)?;
        let mut math_module = Module::with_crate(&zencstr!("Math").data)?;
        let mut windows_module = Module::with_crate(&zencstr!("Windows").data)?;
        let mut memory_module = Module::with_crate(&zencstr!("Memory").data)?;
        let mut sellix_module = Module::with_crate(&zencstr!("Sellix").data)?;
        let mut config_module = Module::with_crate(&zencstr!("Config").data)?;
        let mut arctic_module = Module::with_crate(&zencstr!("Arctic").data)?;
        let mut std_module = Module::with_crate(&zencstr!("std").data)?;
        let mut mutex_module = Module::with_crate(&zencstr!("Mutex").data)?;

        module.ty::<RuneDoubleResultPrimitive>()?;
        module.ty::<MutexValue>()?;

        mutex_module.function("new", MutexValue::new).build()?;
        module
            .function("try_get", MutexValue::try_get)
            .build_associated::<MutexValue>()?;
        module
            .function("try_set", MutexValue::try_set)
            .build_associated::<MutexValue>()?;
        module
            .function("is_locked", MutexValue::is_locked)
            .build_associated::<MutexValue>()?;

        module
            .function("read", |ptr: i64| Self::read_primitive(ptr))
            .build_associated::<i64>()?;
        module
            .function("read_offset", |ptr: i64, offset: i64| {
                Self::read_primitive(ptr + offset)
            })
            .build_associated::<i64>()?;

        module
            .function("write", |ptr: i64, value: Value| Self::write(ptr, value))
            .build_associated::<i64>()?;
        module
            .function("write_offset", |ptr: i64, offset: i64, value: Value| {
                Self::write(ptr + offset, value)
            })
            .build_associated::<i64>()?;

        module
            .function("sqrt", |value: f32| value.sqrt())
            .build_associated::<f32>()?;
        module
            .function("round", |value: f64| value.round())
            .build_associated::<f64>()?;
        module
            .function("sin_cos", |value: f32| value.sin_cos())
            .build_associated::<f32>()?;
        module
            .function("lerp", |value: f32, to: f32, time: f32| {
                value + time * (to - value)
            })
            .build_associated::<f32>()?;
        module
            .function("lerp", |value: f64, to: f64, time: f64| {
                value + time * (to - value)
            })
            .build_associated::<f64>()?;

        dynamic_module
            .function("log", |data: &str| {
                log!(data);
            })
            .build()?;

        dynamic_module
            .function("is_key_down", WinUtils::is_key_down)
            .build()?;
        dynamic_module
            .function("get_delta_time", || DELTA_TIME.load(Ordering::Relaxed))
            .build()?;
        compiler_module
            .function("run_multi_threaded", Self::run_multi_threaded)
            .build()?;
        task_module
            .function("sleep_secs", Self::sleep_secs)
            .build()?;
        task_module.function("sleep_ms", Self::sleep_ms).build()?;
        parse_module.function("i8", Self::r#as::<i8>).build()?;
        parse_module.function("u8", Self::r#as::<u8>).build()?;
        parse_module.function("i16", Self::r#as::<i16>).build()?;
        parse_module.function("u16", Self::r#as::<u16>).build()?;
        parse_module.function("i32", Self::r#as::<i32>).build()?;
        parse_module.function("u32", Self::r#as::<u32>).build()?;
        parse_module.function("i64", Self::r#as::<i64>).build()?;
        parse_module.function("u64", Self::r#as::<u64>).build()?;
        parse_module.function("f32", Self::r#as::<f32>).build()?;
        parse_module.function("f64", Self::r#as::<f64>).build()?;
        parse_module.function("bool", Self::r#as::<bool>).build()?;
        parse_module
            .function("hex_to_primitive", WinUtils::hex_to_primitive)
            .build()?;

        // Deprecated: To be moved into f32/f64.
        math_module
            .function("pi", || std::f32::consts::PI)
            .build()?;

        windows_module
            .function("get_cursor_xy", Self::get_cursor_xy)
            .build()?;
        windows_module
            .function("show_alert", |caption: &str, text: &str| {
                WinUtils::display_message_box(caption, text, 0x00000010)
            })
            .build()?;
        windows_module
            .function("get_base_of_module", |module_name: &str| {
                WinUtils::get_base_of(module_name) as i64
            })
            .build()?;
        windows_module
            .function(
                "get_address_of_symbol",
                |module_name: &str, symbol: &str| {
                    WinUtils::get_module_symbol_address(
                        module_name,
                        &CString::new(symbol)
                            .dynamic_expect(zencstr!("Failed converting symbol to a C-String")),
                    )
                    .map(|value| value as i64)
                },
            )
            .build()?;
        memory_module.function("write", Self::write).build()?;
        memory_module
            .function("read", Self::read_primitive)
            .build()?;
        memory_module.function("scan", Self::pattern_scan).build()?;
        memory_module
            .function("read_string", Self::read_string)
            .build()?;
        memory_module
            .function("fn_call", FNCaller::call_auto)
            .build()?;
        memory_module
            .function("fn_call_raw", FNCaller::call_auto_raw)
            .build()?;
        memory_module
            .function("hook_function", RDetour::install_detour_auto)
            .build()?;
        memory_module
            .function("drop_hook", RDetour::drop_rdetour_at)
            .build()?;
        memory_module
            .function("free_cstring", |ptr: i64| {
                if ptr == 0 {
                    log!("[ERROR] free_cstring called with nullptr, cancelling.");
                    return;
                }

                drop(unsafe { CString::from_raw(ptr as _) });
            })
            .build()?;

        let serials_clone = Arc::clone(&serials);
        let crosscom_clone = Arc::clone(&crosscom);
        sellix_module
            .function(
                "is_paying_for_product",
                move |product_id: String, bearer_token: String| {
                    crosscom_clone.read().check_is_ex_serial_ok(
                        product_id,
                        bearer_token,
                        Arc::clone(&serials_clone),
                    )
                },
            )
            .build()?;
        config_module
            .function("has_serial", move |serial: String| {
                serials.contains(&serial)
            })
            .build()?;

        let base_core_clone = Arc::clone(&base_core);
        arctic_module
            .function("inject_plugin", move |dll_name| {
                base_core_clone
                    .read()
                    .get_arctic_core()
                    .get()
                    .unwrap_or_crash(zencstr!(
                        "[ERROR] Unitialized Arctic instance inside of Script Engine!"
                    ))
                    .inject_plugin(dll_name)
            })
            .build()?;

        let base_core_clone = Arc::clone(&base_core);
        arctic_module
            .function("is_plugin_active", move |identifier: &str| {
                base_core_clone
                    .read()
                    .get_arctic_core()
                    .get()
                    .unwrap_or_crash(zencstr!(
                        "[ERROR] Unitialized Arctic instance inside of Script Engine!"
                    ))
                    .is_gateway_active(identifier)
            })
            .build()?;

        std_module
            .function("get_lines_from_string", |input: &str| {
                input
                    .lines()
                    .map(|line| line.to_owned())
                    .collect::<Vec<_>>()
            })
            .build()?;

        std_module
            .function("write_file", std::fs::write::<String, String>)
            .build()?;

        std_module
            .function("read_file", std::fs::read_to_string::<String>)
            .build()?;

        std_module
            .function("file_exists", |path: &str| {
                std::path::Path::new(path).is_file()
            })
            .build()?;

        std_module
            .function("dir_exists", |path: &str| {
                std::path::Path::new(path).is_dir()
            })
            .build()?;

        std_module
            .function("get_current_directory", || config.get_path())
            .build()?;

        std_module
            .function("value_as_ptr", |value: Value| {
                ScriptCore::value_as_ptr(&value).map(|value| value as i64)
            })
            .build()?;

        std_module
            .function("get_random_string", StringUtils::get_random)
            .build()?;

        std_module.function("ftoi", |f: f32| f as i32).build()?;
        std_module.function("dtoi", |f: f64| f as i32).build()?;

        std_module.function("itos", |i: i32| i as i8).build()?;
        std_module.function("itol", |i: i32| i as i64).build()?;
        std_module.function("itof", |i: i32| i as f32).build()?;
        std_module.function("itod", |i: i32| i as f64).build()?;

        std_module.function("ltos", |l: i64| l as i8).build()?;
        std_module.function("ltoi", |l: i64| l as i32).build()?;
        std_module.function("ltof", |l: i64| l as f32).build()?;
        std_module.function("ltod", |l: i64| l as f64).build()?;

        std_module
            .function("define_global", move |variable_name, value| {
                Self::define_global(
                    variable_name,
                    value,
                    script_core.get_global_script_variables(),
                )
            })
            .build()?;

        std_module
            .function("get_global", move |variable_name| {
                Self::get_global(variable_name, script_core.get_global_script_variables())
            })
            .build()?;

        std_module
            .function("f32_approx_eq", |value: f32, compare: f32| value == compare)
            .build()?;
        std_module
            .function("f64_approx_eq", |value: f64, compare: f64| value == compare)
            .build()?;

        let crosscom_clone = Arc::clone(&crosscom);
        std_module
            .function("send_script_to_group", move |source: &str| {
                crosscom_clone
                    .try_read()
                    .unwrap_or_crash(zencstr!(
                        "[ERROR] CrossCom is locked, cannot call std::send_script_to_group!"
                    ))
                    .send_script(source);
            })
            .build()?;
        std_module
            .function("malloc", |size| unsafe { libc::malloc(size) } as i64)
            .build()?;
        std_module
            .function("free", |ptr: i64| unsafe {
                if ptr == 0 {
                    log!("[ERROR] std::free called with a nullptr!");
                    return;
                }

                if ptr < 0 {
                    log!("[ERROR] std::free called with a negative pointer!");
                    return;
                }

                libc::free(ptr as *mut _);
            })
            .build()?;

        Ok(vec![
            module,
            dynamic_module,
            compiler_module,
            task_module,
            parse_module,
            math_module,
            windows_module,
            memory_module,
            sellix_module,
            config_module,
            arctic_module,
            std_module,
            mutex_module,
        ])
    }

    /// Defines a new global variable if not present, otherwise updates the existing variable.
    fn define_global(
        variable_name: String,
        value: Value,
        global_script_variables: Arc<DashMap<String, ValueWrapper>>,
    ) {
        global_script_variables.insert(variable_name, ValueWrapper(value));
    }

    /// Gets a clone of the value from the identified global variable.
    fn get_global(
        variable_name: String,
        global_script_variables: Arc<DashMap<String, ValueWrapper>>,
    ) -> Option<Value> {
        global_script_variables.get(&variable_name).map(|value| {
            value
                .0
                .try_clone() // Stupid, but either that or &'static due to lifetime issues.
                .dynamic_expect(zencstr!("Failed cloning value"))
        })
    }

    /// Runs a defined function on a new thread. This is especially useful when the user doesn't
    /// want to block the main thread, or the already newly-created thread from the special
    /// compiler option.
    fn run_multi_threaded(function: Function, opt_param: Option<Value>) {
        let opt_param = opt_param.map(ValueWrapper);
        let function = function
            .into_sync()
            .into_result()
            .dynamic_expect(zencstr!("Failed turning Function into SyncFunction"));

        std::thread::spawn(move || {
            let Err(error) = function
                .call::<_, ()>((opt_param.map(|value| value.0),))
                .into_result()
            else {
                return;
            };

            log!(
                "[ERROR] Failed calling function on a new thread, error: ",
                error
            );
        });
    }

    /// Attempts to read a string at `address`.
    pub fn read_string(address: i64) -> &'static str {
        WinUtils::ptr_to_string(address as _).unwrap_or_default()
    }

    /// Scans for a pattern in memory.
    fn pattern_scan(module: &str, hex_string: String) -> Vec<i64> {
        let ptr = hex_string.as_ptr();
        WinUtils::find_from_signature(
            module,
            &StringUtils::hex_string_to_bytes(hex_string)
                .unwrap_or_crash(zencstr!("[ERROR] Failed converting hex string into bytes!")),
            true,
        )
        .iter()
        .map(|address| *address as i64)
        .filter(|address| *address != 0 && *address != ptr as i64)
        .collect()
    }

    /// Writes to the specified memory address.
    /// Supports these types:
    /// - Integers
    /// - Unsigned Integers
    /// - Decimals, only f32 for now
    /// - Strings with automatic termination
    /// - Byte-arrays via a special string: `"b[00 00 00 00]"` - The byte array being embedded
    ///   within `b[...]`.
    fn write(address: i64, data: Value) {
        if address == 0 {
            log!("[ERROR] Address passed into Memory::write was null!");
            return;
        }

        let current_process_handle = unsafe { GetCurrentProcess() };

        // Read one byte from the address just to see if it errors and potentially hinder crashes.
        if let Err(error) =
            Memory::read::<u8>(&current_process_handle, address as *const i64, Some(1))
        {
            log!(
                "[ERROR] Memory at address ",
                format!("{:?}", address as *const i64),
                " is not valid, error: ",
                error
            );
            return;
        }

        let on_error = |error: windows::core::Error| {
            log!(
                "[ERROR] Failed writing to memory address at ",
                format!("{:?}", address as *const i64),
                ", error: ",
                format!("{error:?}")
            );
        };

        if let Ok(data_i64) = data.as_integer().into_result() {
            if let Err(error) = Memory::write(
                &current_process_handle,
                address as _,
                &(data_i64 as i32),
                None,
            ) {
                on_error(error);
            }

            return;
        }

        if let Ok(data_usize) = data.as_usize().into_result() {
            if let Err(error) =
                Memory::write(&current_process_handle, address as _, &data_usize, None)
            {
                on_error(error);
            }

            return;
        }

        if let Ok(data_f64) = data.as_float().into_result() {
            if let Err(error) = Memory::write(
                &current_process_handle,
                address as _,
                &(data_f64 as f32),
                None,
            ) {
                on_error(error);
            }

            return;
        }

        let Ok(data_string) = data.to_owned().into_string().into_result() else {
            return;
        };

        let Ok(data_string) = data_string.borrow_ref() else {
            log!("[ERROR] Invalid type to be written!");
            log!("[INFO] You may only use primitive values, strings and byte-strings!");
            return;
        };

        let mut bytes = data_string.as_bytes().to_vec();
        if !bytes.ends_with(b"\0") {
            bytes.push(b'\0');
        }

        // If the string starts with 'b[' and ends with ']', it's classified as a
        // byte-string. Content goes inside 'b[]'.
        zencstr!("b[").use_string(|start_pfx| {
            if data_string.starts_with(&*start_pfx) && data_string.ends_with(']') {
                if let Some(data_bytes) = StringUtils::hex_string_to_bytes(
                    data_string.replace(&*start_pfx, "").replace([']', ' '], ""),
                ) {
                    bytes = data_bytes;
                }
            }
        });

        if let Err(error) = Memory::write(
            &current_process_handle,
            address as _,
            &bytes,
            Some(bytes.len()),
        ) {
            on_error(error);
        }
    }

    /// Reads a primitive from `address`.
    fn read_primitive(address: i64) -> RuneDoubleResultPrimitive {
        if address == 0 {
            log!("[ERROR] read_primitive called with a nullptr, returning RuneDoubleResultPrimitive::default()!");
            return RuneDoubleResultPrimitive::default();
        }

        unsafe {
            let read_i64: i64 = std::ptr::read(address as _);
            let read_f32: f32 = std::ptr::read(address as _);
            let read_f64: f64 = std::ptr::read(address as _);
            RuneDoubleResultPrimitive::new(
                read_i64 as i8,
                read_i64 as i32,
                read_i64,
                read_f32,
                read_f64,
            )
        }
    }

    /// Gets the X and Y-Coordinate of the cursor.
    fn get_cursor_xy() -> Vec<RuneDoubleResultPrimitive> {
        let mut vec = Vec::with_capacity(2);
        let cursor_pos = WinUtils::get_cursor_pos();
        let (x, y) = (cursor_pos.x, cursor_pos.y);
        vec.push(RuneDoubleResultPrimitive::new(
            x as i8, x, x as i64, x as f32, x as f64,
        ));
        vec.push(RuneDoubleResultPrimitive::new(
            y as i8, y, y as i64, y as f32, y as f64,
        ));
        vec
    }

    /// Attempts to parse the given data as a number.
    fn r#as<T: FromStr + Debug + Default>(data: &str) -> T
    where
        <T as FromStr>::Err: Display,
    {
        data.parse().unwrap_or_else(|error| {
            log!(
                "[ERROR] Failed parsing \"",
                data,
                "\", returning ",
                std::any::type_name::<T>(),
                "::default(). Error: ",
                error
            );
            T::default()
        })
    }

    /// Puts the calling task to sleep for a few seconds.
    fn sleep_secs(seconds: u64) {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }

    /// Puts the calling thread to sleep for a few milliseconds.
    fn sleep_ms(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

/// ImGui Modules.
pub struct UIModules;

impl UIModules {
    /// Builds this module.
    #[optimize(size)]
    pub fn build(
        base_core: Arc<RwLock<BaseCore>>,
        custom_window_utils: &'static CustomWindowsUtils,
    ) -> Result<Module, ContextError> {
        let base_core_reader = base_core.read();
        let script_core = base_core_reader.get_script_core();
        drop(base_core_reader);

        let mut module = Module::with_crate(&zencstr!("ui").data)?; // <-- TODO: Rename to `UI`.

        module
            .function("add_window", |name| custom_window_utils.add_window(name))
            .build()?;

        module
            .function("remove_window", |name| {
                custom_window_utils.remove_window(name)
            })
            .build()?;

        module
            .function(
                "add_label",
                |(window_name, identifier): (String, String), content: String| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier,
                        WidgetType::Label(ZString::new(content), 0),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_bold_label",
                |(window_name, identifier): (String, String), content: String| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier,
                        WidgetType::Label(ZString::new(content), 2),
                    );
                },
            )
            .build()?;

        module
            .function(
                "add_custom_font_label",
                |(window_name, identifier): (String, String), content, relative_font_path| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier,
                        WidgetType::LabelCustomFont(content, Arc::new(relative_font_path)),
                    )
                },
            )
            .build()?;

        module
            .function(
                "update_label",
                |(window_name, identifier): (String, String), new_text| {
                    custom_window_utils.update_label(&window_name, identifier, new_text)
                },
            )
            .build()?;

        module
            .function(
                "add_button",
                |(window_name, identifier): (String, String), text: String, function, opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::Button(
                            ZString::new(text),
                            Self::function_into_rc_sync(function, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function("add_separator", |window_name: &str, identifier| {
                custom_window_utils.add_widget(window_name, identifier, WidgetType::Separator)
            })
            .build()?;

        module
            .function(
                "add_spacing",
                |(window_name, identifier): (String, String), x, y| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier,
                        WidgetType::Spacing(x, y),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_f32_slider",
                |(window_name, identifier): (String, String),
                 text: String,
                 (min, max, default_value),
                 function,
                 opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::F32Slider(
                            ZString::new(text),
                            min,
                            max,
                            default_value,
                            Self::function_into_rc_sync(function, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_i32_slider",
                |(window_name, identifier): (String, String),
                 text: String,
                 (min, max, default_value),
                 function,
                 opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::I32Slider(
                            ZString::new(text),
                            min,
                            max,
                            default_value,
                            Self::function_into_rc_sync(function, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function("get_f32_slider_value", |window_name: &str, identifier| {
                custom_window_utils.get_f32_slider_value(window_name, identifier)
            })
            .build()?;

        module
            .function("get_i32_slider_value", |window_name: &str, identifier| {
                custom_window_utils.get_i32_slider_value(window_name, identifier)
            })
            .build()?;

        module
            .function("remove_widget", |window_name: &str, identifier| {
                custom_window_utils.remove_widget(window_name, identifier)
            })
            .build()?;

        module
            .function("remove_all_widgets", |window_name: &str| {
                custom_window_utils.remove_all_widgets(window_name)
            })
            .build()?;

        module
            .function(
                "set_next_item_width",
                |(window_name, identifier): (String, String), width| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier,
                        WidgetType::NextWidgetWidth(width),
                    )
                },
            )
            .build()?;

        module
            .function(
                "set_next_item_same_line",
                |window_name: &str, identifier| {
                    custom_window_utils.add_widget(window_name, identifier, WidgetType::SameLine)
                },
            )
            .build()?;

        module
            .function(
                "add_image",
                |(window_name, identifier): (String, String),
                 image_path,
                 (width, height),
                 callback,
                 opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::Image(
                            image_path,
                            width,
                            height,
                            false,
                            false,
                            Self::function_into_rc_sync(callback, identifier),
                            Rc::new(opt_param),
                            false,
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_image_overlay",
                |(window_name, identifier): (String, String),
                 image_path,
                 (width, height),
                 callback,
                 opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::Image(
                            image_path,
                            width,
                            height,
                            true,
                            false,
                            Self::function_into_rc_sync(callback, identifier),
                            Rc::new(opt_param),
                            false,
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_image_background",
                |(window_name, identifier): (String, String), image_path, (width, height)| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::Image(
                            image_path,
                            width,
                            height,
                            false,
                            true,
                            Self::function_into_rc_sync(Function::new(|| {}), identifier),
                            Rc::new(None),
                            false,
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "replace_image",
                |identifier, new_image_path, width, height| {
                    custom_window_utils.replace_image(identifier, new_image_path, [width, height])
                },
            )
            .build()?;

        module
            .function(
                "set_size_constraints",
                |window_name: &str, min_x, min_y, max_x, max_y| {
                    custom_window_utils
                        .set_window_size_constraints(window_name, [min_x, min_y, max_x, max_y])
                },
            )
            .build()?;

        module
            .function("clear_cached_images", || {
                custom_window_utils.clear_cached_images()
            })
            .build()?;

        module
            .function("hide_widgets", |identifiers| {
                custom_window_utils.hide_widgets(identifiers)
            })
            .build()?;

        module
            .function("show_widgets", |identifiers| {
                custom_window_utils.show_widgets(identifiers)
            })
            .build()?;

        module
            .function(
                "add_input_text_multiline",
                |(window_name, identifier): (String, String),
                 label: String,
                 (width, height),
                 callback,
                 opt_param| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::InputTextMultiLine(
                            ZString::new(label),
                            String::default(),
                            width,
                            height,
                            Self::function_into_rc_sync(callback, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "get_input_text_multiline_value",
                |window_name: &str, identifier| {
                    custom_window_utils.get_input_text_multiline_value(window_name, identifier)
                },
            )
            .build()?;

        module
            .function(
                "retain_widgets_by_identifiers",
                |window_name: &str, identifiers| {
                    custom_window_utils.retain_widgets_by_identifiers(window_name, identifiers)
                },
            )
            .build()?;

        module
            .function("is_cursor_in_ui", || {
                IS_CURSOR_IN_UI.load(Ordering::Relaxed)
            })
            .build()?;

        module
            .function("has_widget", |window_name: &str, identifier: &str| {
                custom_window_utils
                    .get_widget(window_name, identifier)
                    .is_some()
            })
            .build()?;

        module
            .function(
                "add_collapsing_section",
                move |(window_name, section_identifier): (String, String),
                      text: String,
                      call_once: Function,
                      opt_param: Option<Value>| {
                    Self::add_sub_widget(
                        &window_name,
                        section_identifier,
                        SubWidgetType::CollapsingHeader(ZString::new(text)),
                        call_once,
                        opt_param,
                        custom_window_utils,
                    );
                },
            )
            .build()?;

        module
            .function(
                "add_checkbox",
                |(window_name, identifier): (String, String),
                 text: String,
                 checked,
                 on_value_changed: Function,
                 opt_param: Option<Value>| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::Checkbox(
                            ZString::new(text),
                            checked,
                            Self::function_into_rc_sync(on_value_changed, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function(
                "add_combobox",
                |(window_name, identifier): (String, String),
                 text: String,
                 (items, selected_index),
                 on_value_changed: Function,
                 opt_param: Option<Value>| {
                    custom_window_utils.add_widget(
                        &window_name,
                        identifier.to_owned(),
                        WidgetType::ComboBox(
                            ZString::new(text),
                            selected_index,
                            items,
                            Self::function_into_rc_sync(on_value_changed, identifier),
                            Rc::new(opt_param),
                        ),
                    )
                },
            )
            .build()?;

        module
            .function("set_color_preset_for", |window_name: String, preset| {
                custom_window_utils.set_color_preset_for(window_name, preset)
            })
            .build()?;
        module
            .function(
                "register_frame_update_callback",
                |identifier: String, callback, opt_param| {
                    script_core.register_frame_update_callback(
                        identifier.to_owned(),
                        Self::function_into_sync(callback, identifier),
                        opt_param,
                    );
                },
            )
            .build()?;
        module
            .function("remove_frame_update_callback", |identifier: &str| {
                script_core.remove_frame_update_callback(identifier);
            })
            .build()?;

        Ok(module)
    }

    /// Turns `Function` into a `SyncFunction`, crashing if it fails.
    fn function_into_sync(function: Function, identifier: String) -> SyncFunction {
        function.into_sync().into_result().dynamic_expect(zencstr!(
            "Failed turning Function into SyncFunction at \"",
            identifier,
            "\""
        ))
    }

    /// Turns `Function` into a `Rc<SyncFunction>`, crashing if it fails.
    fn function_into_rc_sync(function: Function, identifier: String) -> Rc<SyncFunction> {
        Rc::new(Self::function_into_sync(function, identifier))
    }

    /// Helper function for making it easier to add sub-widgets.
    fn add_sub_widget(
        window_name: &str,
        section_identifier: String,
        sub_widget_type: SubWidgetType,
        call_once: Function,
        opt_param: Option<Value>,
        custom_window_utils: &'static CustomWindowsUtils,
    ) {
        custom_window_utils.add_widget(
            window_name,
            section_identifier.to_owned(),
            WidgetType::SubWidget(
                sub_widget_type,
                Default::default(),
                Self::function_into_rc_sync(call_once, section_identifier),
                Rc::new(opt_param),
            ),
        )
    }
}
