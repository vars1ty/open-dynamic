#![feature(stmt_expr_attributes)]
#![feature(optimize_attribute)]
#![feature(c_variadic)]

#[macro_use]
extern crate obfstr;

extern crate zstring;

#[macro_use]
#[path = "utils/macros.rs"]
mod macros;

mod globals;
mod mod_cores;
mod ui;
mod utils;
mod winutils;

use crate::{
    mod_cores::base_core::BaseCore,
    ui::unknown::dx11_ui::DX11UI,
    utils::prompter::Prompter,
    winutils::{Renderer, WinUtils},
};
use hudhook::{
    hooks::{
        dx11::ImguiDx11Hooks, dx12::ImguiDx12Hooks, dx9::ImguiDx9Hooks, opengl3::ImguiOpenGl3Hooks,
    },
    windows::Win32::Foundation::HINSTANCE,
    Hudhook,
};
use parking_lot::RwLock;
use std::{
    ffi::c_void,
    io::IsTerminal,
    sync::{atomic::AtomicBool, Arc},
};
use utils::hooks::GenericHoooks;
use windows::Win32::Foundation::BOOL;
use winutils::{AllocConsole, FreeConsole};
use zstring::ZString;

/// Called when the DLL has been injected/detached.
#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    reserved: *const c_void,
) -> BOOL {
    std::env::set_var("RUST_BACKTRACE", "full");
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;

    match call_reason {
        DLL_PROCESS_ATTACH => {
            std::thread::spawn(move || {
                hook(dll_module);
            });
        }
        DLL_PROCESS_DETACH => std::process::exit(0),
        _ => (),
    }

    BOOL(1)
}

/// Begins initializing and hooking everything.
fn hook(hmodule: HINSTANCE) {
    // Allocate a console window.
    let is_terminal = std::io::stdout().is_terminal();
    let allocated = unsafe { AllocConsole() } == 1;
    if allocated || is_terminal {
        println!("{}", include_str!("../resources/ascii"));
        log!("Console Window active, close this window and the process will also close!");
        if !is_terminal {
            log!("If you wish to remove this window, either set `free_console` in the config to `true`, or write `free_console`.");
        }
    } else {
        WinUtils::display_message_box(
            &zencstr!("Warning").data,
            &zencstr!("Couldn't allocate a console window, skipping.").data,
            0x00000030,
        )
    }

    log!("Initializing Base Core...");
    let base_core = Arc::new(RwLock::new(BaseCore::init()));
    let base_core_reader = base_core.read();
    base_core_reader.try_start_deadlock_detection();
    log!("Base Core initialized, hooking...");

    // If `free_console` is `true` and there's an allocated console, free the console.
    if allocated {
        if base_core_reader.get_config().get_free_console() {
            unsafe { FreeConsole() };
        } else {
            let base_core_clone = Arc::clone(&base_core);
            std::thread::spawn(move || {
                // No freeing the console, we are free to listen for additional commands.
                let mut prompt = Prompter::new_any_response(
                    "Commands:\n» free_console\n» execute_script [relative_path]\n» exit",
                );
                loop {
                    // Can't be None here due to new_any_response, so it's safe to use unchecked.
                    let result = unsafe { prompt.prompt().unwrap_unchecked() };
                    on_console_command(
                        &result.prompt.data,
                        result.args,
                        Arc::clone(&base_core_clone),
                    );
                }
            });
        }
    }

    drop(base_core_reader);
    prepare_hooks(base_core, hmodule);
    log!("Initialized!");
}

/// Prepares the hooking process and calls `hook_based_on_renderer`.
fn prepare_hooks(base_core: Arc<RwLock<BaseCore>>, hmodule: HINSTANCE) {
    log!("Hooking into unknown process, proceed at your own risk!");
    hook_based_on_renderer(Arc::clone(&base_core), hmodule);
}

/// Intended for `GameTitle::Unknown` where it figures out the renderer and hooks based on the
/// result.
fn hook_based_on_renderer(base_core: Arc<RwLock<BaseCore>>, hmodule: HINSTANCE) {
    let mut builder = Hudhook::builder().with_hmodule(hmodule);
    let base_core_reader = base_core.read();
    let renderer_target = base_core_reader.get_config().get_renderer_target();
    let mut setup_ui = true;

    // Determine the renderer target from the config.
    match renderer_target {
        Renderer::DirectX9 => {
            let (_, disable_set_cursor_pos_clone) = setup_generic_hooks();
            builder = builder.with::<ImguiDx9Hooks>(DX11UI::new(
                Arc::clone(&base_core),
                disable_set_cursor_pos_clone,
            ));
        }
        Renderer::DirectX11 => {
            let (_, disable_set_cursor_pos_clone) = setup_generic_hooks();
            builder = builder.with::<ImguiDx11Hooks>(DX11UI::new(
                Arc::clone(&base_core),
                disable_set_cursor_pos_clone,
            ));
        }
        Renderer::DirectX12 => {
            let (_, disable_set_cursor_pos_clone) = setup_generic_hooks();
            builder = builder.with::<ImguiDx12Hooks>(DX11UI::new(
                Arc::clone(&base_core),
                disable_set_cursor_pos_clone,
            ));
        }
        Renderer::OpenGL => {
            let (_, disable_set_cursor_pos_clone) = setup_generic_hooks();
            builder = builder.with::<ImguiOpenGl3Hooks>(DX11UI::new(
                Arc::clone(&base_core),
                disable_set_cursor_pos_clone,
            ));
        }
        Renderer::None => {
            log!("Renderer hooks DISABLED, you are now entirely on your own!");
            setup_ui = false;
        }
    }

    if setup_ui {
        builder.build().apply().unwrap_or_else(|error| {
            crash!(
                "[ERROR] Failed applying UI hook. Render Target: ",
                format!("{renderer_target:?}"),
                ", error: ",
                format!("{error:?}")
            )
        });
    } else {
        drop(builder);
    }

    let Some(startup_rune_script) = base_core_reader.get_config().get_startup_rune_script() else {
        return;
    };

    ZString::default().use_string(|output| {
        if !base_core_reader
            .get_config()
            .get_file_content(&ZString::new(startup_rune_script.to_owned()).data, output)
        {
            crash!(
                "[ERROR] Failed reading startup Rune file, ensure the relative path is correct!"
            );
        }

        base_core_reader.get_script_core().execute(
            std::mem::take(output),
            Arc::clone(&base_core),
            false,
            false,
        );
    });
}

/// Installs generic hooks and returns the instance, including the `disable_set_cursor_pos`
/// `Arc<AtomicBool>`.
fn setup_generic_hooks() -> (GenericHoooks, Arc<AtomicBool>) {
    let generic_hooks = GenericHoooks::init();
    let disable_set_cursor_pos_clone = Arc::clone(&generic_hooks.disable_set_cursor_pos);
    (generic_hooks, disable_set_cursor_pos_clone)
}

/// Called when a console command should be looked up and executed, after everything's initialized.
fn on_console_command(prompt: &str, args: Vec<String>, base_core: Arc<RwLock<BaseCore>>) {
    match prompt {
        "free_console" => {
            log!(
                "Freeing console in 5 seconds. To get it back again, call the AllocConsole symbol via fn_call or restart the process."
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
            unsafe { FreeConsole() };
        }
        "execute_script" => {
            ZString::default().use_string(|data| {
                let Some(relative_path) = args.get(1) else {
                    log!("[ERROR] No arguments passed! Usage: execute_script [relative_path: String]");
                    return;
                };

                let Some(base_core_reader) = base_core.try_read() else {
                    log!("[ERROR] Base Core is locked, try again later!");
                    return;
                };

                if !base_core_reader.get_config().get_file_content(relative_path, data) {
                    log!("[ERROR] Script could not be executed, ensure the relative path is correct!");
                    return;
                }

                log!("[CMD] Executing script at relative path \"", relative_path, "\"...");
                base_core_reader.get_script_core().execute(std::mem::take(data), Arc::clone(&base_core), false, false);
            });
        }
        "exit" => std::process::exit(0),
        _ => (),
    }
}
