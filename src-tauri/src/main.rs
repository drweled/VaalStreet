// Vaal Street: your Path of Exile stash's worth (poe.ninja prices), its graph
// over time, and how much each strategy run made. Lives next to the clock and
// opens over the game with Ctrl+W.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod overlay;
mod poe;
mod startup;
mod updater;
mod wealth;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

fn main() {
    let background = std::env::args().any(|arg| arg == overlay::BACKGROUND_FLAG);

    tauri::Builder::default()
        // starting it again (Start menu, desktop) opens the running one
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|arg| arg == overlay::BACKGROUND_FLAG) {
                overlay::show(app);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && shortcut.id() == overlay::ctrl_w().id() {
                        overlay::toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(poe::Poe::new())
        .invoke_handler(tauri::generate_handler![
            poe::vs_open_url,
            poe::vs_fetch,
            poe::vs_cached,
            poe::vs_login,
            poe::vs_logout,
            poe::vs_session,
            overlay::vs_hide,
            overlay::vs_drag,
            overlay::vs_resize,
            overlay::vs_reset_size,
            wealth::wealth_account,
            wealth::wealth_tabs,
            wealth::wealth_tab,
            wealth::wealth_wait,
            wealth::wealth_load,
            wealth::wealth_save,
            wealth::wealth_sign_out,
            startup::vs_startup_get,
            startup::vs_startup_set,
            updater::vs_update_check,
            updater::vs_update_install,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            overlay::setup(&handle)?;
            poe::restore_session(&handle);
            // started with Windows: wait next to the clock; started by hand: open
            if !background {
                overlay::show(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != poe::MAIN_LABEL {
                // the sign-in window closed without signing in: back to Vaal Street
                if window.label() == poe::LOGIN_LABEL {
                    if let WindowEvent::Destroyed = event {
                        overlay::show(window.app_handle());
                    }
                }
                return;
            }
            match event {
                // Alt+F4 puts it away; "Quit" in the tray menu ends it
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    overlay::hide(window.app_handle());
                }
                WindowEvent::Focused(false) => overlay::on_blur(window.app_handle()),
                WindowEvent::Resized(_) => overlay::on_resized(window.app_handle()),
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("Vaal Street could not start")
        .run(|_app, event| {
            // a tray program keeps running with its window put away
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
