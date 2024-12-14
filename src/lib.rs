#![feature(stmt_expr_attributes)]
#![feature(optimize_attribute)]
#![feature(c_variadic)]
#![feature(let_chains)]

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
    windows::Win32::{Foundation::HINSTANCE, System::Console::FreeConsole},
    Hudhook,
};
use parking_lot::RwLock;
use std::{ffi::c_void, io::IsTerminal, sync::Arc};
use windows::Win32::System::Console::{AllocConsole, GetConsoleWindow};
use zstring::ZString;

/// Called when the DLL has been injected/detached.
#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: isize, call_reason: u32, reserved: *const c_void) -> i32 {
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

    1
}

/// Begins initializing and hooking everything.
fn hook(hmodule: isize) {
    // Allocate a console window.
    let is_terminal = std::io::stdout().is_terminal();
    let allocated = unsafe { AllocConsole() }.is_ok();
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
    log!("Base Core initialized, hooking...");

    // If `free_console` is `true` and there's an allocated console, free the console.
    if allocated {
        if base_core_reader.get_config().get_free_console() {
            unsafe {
                let _ = FreeConsole();
            }
        } else {
            let base_core_clone = Arc::clone(&base_core);
            std::thread::spawn(move || unsafe {
                // No freeing the console, we are free to listen for additional commands.
                let mut prompt = Prompter::new_any_response(
                    "Commands:\n» free_console\n» execute_script [relative_path (String)]\n» exit",
                );
                while GetConsoleWindow().0 != 0 {
                    // Can't be None here due to new_any_response, so it's safe to use unchecked.
                    let result = prompt.prompt().unwrap_unchecked();
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
fn prepare_hooks(base_core: Arc<RwLock<BaseCore>>, hmodule: isize) {
    log!("Hooking into unknown process, proceed at your own risk!");
    hook_based_on_renderer(Arc::clone(&base_core), hmodule);
}

/// Intended for `GameTitle::Unknown` where it figures out the renderer and hooks based on the
/// result.
fn hook_based_on_renderer(base_core: Arc<RwLock<BaseCore>>, hmodule: isize) {
    let mut builder = Hudhook::builder().with_hmodule(HINSTANCE(hmodule));
    let base_core_reader = base_core.read();
    let renderer_target = base_core_reader.get_config().get_renderer_target();
    let mut setup_ui = true;

    // Determine the renderer target from the config.
    match renderer_target {
        Renderer::DirectX9 => {
            builder = builder.with::<ImguiDx9Hooks>(DX11UI::new(Arc::clone(&base_core)));
        }
        Renderer::DirectX11 => {
            builder = builder.with::<ImguiDx11Hooks>(DX11UI::new(Arc::clone(&base_core)));
        }
        Renderer::DirectX12 => {
            builder = builder.with::<ImguiDx12Hooks>(DX11UI::new(Arc::clone(&base_core)));
        }
        Renderer::OpenGL => {
            builder = builder.with::<ImguiOpenGl3Hooks>(DX11UI::new(Arc::clone(&base_core)));
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

    let Some(startup_rune_scripts) = base_core_reader.get_config().get_startup_rune_scripts()
    else {
        return;
    };

    for startup_rune_script in startup_rune_scripts {
        ZString::default().use_string(|output| {
            if !base_core_reader
                .get_config()
                .get_file_content(&ZString::new(startup_rune_script.to_owned()).data, output)
            {
                log!(
                    "[WARN] Failed reading startup Rune file \"",
                    startup_rune_script,
                    "\", ensure the relative path is correct!"
                );
                return;
            }

            base_core_reader.get_script_core().execute(
                std::mem::take(output),
                Arc::clone(&base_core),
                false,
                false,
            );
        });
    }
}

/// Called when a console command should be looked up and executed, after everything's initialized.
fn on_console_command(prompt: &str, args: Vec<String>, base_core: Arc<RwLock<BaseCore>>) {
    match prompt {
        "free_console" => {
            log!(
                "Freeing console in 5 seconds. To get it back again, call the AllocConsole symbol via fn_call or restart the process."
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = unsafe { FreeConsole() };
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
