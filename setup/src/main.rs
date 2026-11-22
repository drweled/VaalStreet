// Vaal Street Setup: the pixel window people see when they install, update or
// uninstall Vaal Street. The real installer (NSIS, made by `cargo tauri build`
// in src-tauri) is carried inside this file; it runs it out of sight while the
// window shows its own animation.
//
// The same program, built with nothing carried inside, is the uninstaller: it
// is put in the app's folder and Windows (Settings > Apps > Uninstall) starts
// it, so uninstalling looks the same as installing.
//
// Vaal Street's updater starts the setup with /UPDATE (and its own flags):
// then it goes straight to "UPDATING" and starts Vaal Street again.
// `--auto` installs at once (with the animation) and closes when done.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

// ------------------------------------------------------------ this app
const APP_NAME: &str = "Vaal Street";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Vaal Street";
const APP_EXE: &str = "VaalStreet.exe";
const FILE_STEM: &str = "VaalStreet";
/// Its folders in %APPDATA% and %LOCALAPPDATA% (wiped only when asked).
const ROAMING_DIR: &str = "VaalStreet";
const LOCAL_DIR: &str = "local.vaalstreet.app";
const UNINSTALLER_NAME: &str = "Uninstall Vaal Street.exe";
// ------------------------------------------------------------

/// The installer made from src-tauri (empty in the uninstaller).
static PAYLOAD: &[u8] = include_bytes!(env!("VS_PAYLOAD"));
/// The uninstaller, put in the app's folder after installing (empty in itself).
static UNINSTALLER: &[u8] = include_bytes!(env!("VS_UNINSTALLER"));
const VERSION: &str = env!("CARGO_PKG_VERSION");
const NO_WINDOW: u32 = 0x0800_0000;

/// Set when the uninstaller removed the app: it deletes itself on the way out.
static REMOVE_SELF: AtomicBool = AtomicBool::new(false);

fn uninstall_mode() -> bool {
    PAYLOAD.is_empty()
}

fn has_arg(flag: &str) -> bool {
    std::env::args().skip(1).any(|a| a.eq_ignore_ascii_case(flag))
}

fn update_mode() -> bool {
    has_arg("/UPDATE")
}

/// Where the app is installed, and which version, if it is.
fn installed() -> Option<(PathBuf, String)> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(UNINSTALL_KEY, KEY_READ).ok()?;
    let location: String = key.get_value("InstallLocation").ok()?;
    let version: String = key.get_value("DisplayVersion").unwrap_or_default();
    let dir = PathBuf::from(location.trim_matches('"'));
    dir.join(APP_EXE).exists().then_some((dir, version))
}

/// WebView2 (what draws this window) is there: always on Windows 11, nearly
/// always on Windows 10.
fn webview2_present() -> bool {
    const CLIENT: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let paths = [
        (HKEY_LOCAL_MACHINE, format!(r"SOFTWARE\WOW6432Node\{CLIENT}")),
        (HKEY_LOCAL_MACHINE, format!(r"SOFTWARE\{CLIENT}")),
        (HKEY_CURRENT_USER, format!(r"Software\{CLIENT}")),
    ];
    paths.iter().any(|(root, path)| {
        RegKey::predef(*root)
            .open_subkey_with_flags(path, KEY_READ)
            .and_then(|k| k.get_value::<String, _>("pv"))
            .is_ok_and(|v| !v.is_empty() && v != "0.0.0.0")
    })
}

/// The installer, written next to the other temporary files.
fn write_payload() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("{FILE_STEM}_{VERSION}_install.exe"));
    std::fs::write(&path, PAYLOAD).map_err(|e| format!("could not unpack the installer: {e}"))?;
    Ok(path)
}

/// The app must not be running while its files are replaced. Stops it and
/// waits until it, and the browser processes it started (they hold files in
/// its folder), are really gone.
fn stop_app() {
    // only the app's own browser processes (they use its data folder in
    // %LOCALAPPDATA%), never this window's own (in the temporary folder)
    let script = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object {{ $_.CommandLine -like '*{LOCAL_DIR}*' }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}"
    );
    for _ in 0..20 {
        let _ = Command::new("taskkill").args(["/IM", APP_EXE, "/F", "/T"]).creation_flags(NO_WINDOW).status();
        let _ = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]).creation_flags(NO_WINDOW).status();
        let running = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {APP_EXE}"), "/NH"])
            .creation_flags(NO_WINDOW)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&APP_EXE.to_lowercase()))
            .unwrap_or(false);
        if !running {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    // let Windows let go of the files
    std::thread::sleep(std::time::Duration::from_millis(1500));
}

/// Runs the carried installer out of sight; one more try when it stops early
/// (code 2: a file was still in use).
fn run_payload(path: &std::path::Path, args: &[String]) -> std::io::Result<std::process::ExitStatus> {
    let status = Command::new(path).args(args).status()?;
    if status.code() == Some(2) {
        stop_app();
        return Command::new(path).args(args).status();
    }
    Ok(status)
}

/// After installing: the pixel uninstaller goes into the app's folder and
/// Windows is told to start it to uninstall.
fn register_uninstaller(dir: &std::path::Path) {
    if UNINSTALLER.is_empty() {
        return;
    }
    let target = dir.join(UNINSTALLER_NAME);
    if std::fs::write(&target, UNINSTALLER).is_err() {
        return;
    }
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | KEY_WRITE) {
        let _ = key.set_value("UninstallString", &format!("\"{}\"", target.display()));
    }
}

/// While the installer works, other windows (Explorer redrawing icons) may
/// take the front: the setup window keeps coming back on top until it is done.
fn keep_in_front(app: &AppHandle) -> Arc<AtomicBool> {
    let busy = Arc::new(AtomicBool::new(true));
    let flag = busy.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.set_always_on_top(true);
            while flag.load(Ordering::SeqCst) {
                if win.is_minimized().unwrap_or(false) {
                    let _ = win.unminimize();
                }
                if !win.is_visible().unwrap_or(true) {
                    let _ = win.show();
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            let _ = win.set_always_on_top(false);
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
    });
    busy
}

#[tauri::command]
fn setup_info() -> Value {
    let installed = installed().map(|(_, v)| v);
    json!({
        "name": APP_NAME,
        "version": VERSION,
        "installed": installed,
        "updateMode": update_mode(),
        "uninstallMode": uninstall_mode(),
        "auto": has_arg("--auto"),
    })
}

#[tauri::command]
async fn setup_install(app: AppHandle, shortcut: bool) -> Result<(), String> {
    let busy = keep_in_front(&app);
    let result = tauri::async_runtime::spawn_blocking(move || {
        stop_app();
        let path = write_payload()?;
        // the window here is the progress: the installer itself shows none
        let mut args = vec!["/S".to_string()];
        if update_mode() || installed().is_some() {
            args.push("/UPDATE".into());
        }
        if !shortcut && !update_mode() {
            args.push("/NS".into());
        }
        let status = run_payload(&path, &args).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&path);
        let status = status?;
        if !status.success() {
            return Err(format!("the installer stopped (code {})", status.code().unwrap_or(-1)));
        }
        let Some((dir, _)) = installed() else {
            return Err(format!("{APP_NAME} was not found after installing"));
        };
        register_uninstaller(&dir);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string());
    busy.store(false, Ordering::SeqCst);
    result?
}

#[tauri::command]
async fn setup_uninstall(app: AppHandle, wipe: bool) -> Result<(), String> {
    let busy = keep_in_front(&app);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let Some((dir, _)) = installed() else { return Err(format!("{APP_NAME} is not installed")) };
        stop_app();
        // _?= runs the uninstaller in place, so this waits for it to finish
        let status = Command::new(dir.join("uninstall.exe"))
            .arg("/S")
            .arg(format!("_?={}", dir.display()))
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("the uninstaller stopped (code {})", status.code().unwrap_or(-1)));
        }
        // run in place, it cannot delete itself or its folder
        let _ = std::fs::remove_file(dir.join("uninstall.exe"));
        let uninstaller = dir.join(UNINSTALLER_NAME);
        let me_name = std::env::current_exe().ok().and_then(|m| m.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_default();
        if me_name.eq_ignore_ascii_case(UNINSTALLER_NAME) {
            // this program is that file: it goes when this window closes
            REMOVE_SELF.store(true, Ordering::SeqCst);
        } else {
            let _ = std::fs::remove_file(&uninstaller);
            let _ = std::fs::remove_dir(&dir);
        }
        if wipe {
            if let Some(roaming) = dirs::data_dir() {
                let _ = std::fs::remove_dir_all(roaming.join(ROAMING_DIR));
            }
            if let Some(local) = dirs::data_local_dir() {
                let _ = std::fs::remove_dir_all(local.join(LOCAL_DIR));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string());
    busy.store(false, Ordering::SeqCst);
    result?
}

/// The uninstaller deletes itself and the emptied folder once it has closed:
/// a hidden helper keeps trying for up to 30 s (the window's browser
/// processes take a moment to let go of the file).
fn remove_self_later() {
    let Ok(me) = std::env::current_exe() else { return };
    let Some(dir) = me.parent() else { return };
    let quote = |p: &std::path::Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "for ($i = 0; $i -lt 30; $i++) {{ Start-Sleep -Seconds 1; Remove-Item -LiteralPath '{me}' -Force -ErrorAction SilentlyContinue; if (-not (Test-Path -LiteralPath '{me}')) {{ break }} }}; Remove-Item -LiteralPath '{dir}' -Force -ErrorAction SilentlyContinue",
        me = quote(&me),
        dir = quote(dir)
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .current_dir(std::env::temp_dir())
        .creation_flags(NO_WINDOW)
        .spawn();
}

#[tauri::command]
fn setup_launch(app: AppHandle) {
    if let Some((dir, _)) = installed() {
        let _ = Command::new(dir.join(APP_EXE)).current_dir(&dir).spawn();
    }
    app.exit(0);
}

#[tauri::command]
fn setup_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn setup_drag(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.start_dragging();
    }
}

fn main() {
    // no WebView2 to draw the pixel window: the classic installer (or
    // uninstaller) does it all
    if !webview2_present() {
        if uninstall_mode() {
            if let Some((dir, _)) = installed() {
                let _ = Command::new(dir.join("uninstall.exe")).status();
            }
        } else if let Ok(path) = write_payload() {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let _ = Command::new(&path).args(&args).status();
            let _ = std::fs::remove_file(&path);
        }
        return;
    }
    // the window's browser data goes to a temporary folder, removed at the end
    let browser_data = std::env::temp_dir().join(format!("{FILE_STEM}Setup.webview"));
    let cleanup = browser_data.clone();
    let title = if uninstall_mode() { format!("Uninstall {APP_NAME}") } else { format!("{APP_NAME} Setup") };
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![setup_info, setup_install, setup_uninstall, setup_launch, setup_quit, setup_drag])
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title(&title)
                .inner_size(600.0, 440.0)
                .center()
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .focused(true)
                .data_directory(browser_data.clone())
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("{APP_NAME} Setup could not start: {e}"))
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                let _ = std::fs::remove_dir_all(&cleanup);
                if REMOVE_SELF.load(Ordering::SeqCst) {
                    remove_self_later();
                }
            }
        });
}
