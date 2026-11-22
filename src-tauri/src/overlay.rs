//! Vaal Street lives next to the clock (the tray) and opens over the game:
//!  - Ctrl+W opens or closes it while Path of Exile (or Vaal Street) is in
//!    front. The key is only taken then, so Ctrl+W keeps closing tabs in your
//!    browser and everywhere else.
//!  - a left click on the tray icon opens or closes it; a right click shows
//!    Open / Quit.
//!  - Esc, Ctrl+W or a click anywhere outside puts it away again.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, GA_ROOTOWNER};

use crate::poe::{LOGIN_LABEL, MAIN_LABEL};

pub const BACKGROUND_FLAG: &str = "--background";
/// The panel's size in text heights (WeledFTW's Wealthy Weled: 1240 px wide
/// and about 686 px tall with 14 px text).
const WIDTH_EM: f64 = 1240.0 / 14.0;
const HEIGHT_EM: f64 = 686.0 / 14.0;

/// When the window was last put away because the player clicked elsewhere.
static HIDDEN_BY_BLUR_AT: AtomicU64 = AtomicU64::new(0);
/// Placed once (centered on the screen it opened on); then it stays where the
/// player dragged it.
static PLACED: AtomicBool = AtomicBool::new(false);
static KEY_TAKEN: AtomicBool = AtomicBool::new(false);
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn ctrl_w() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL), Code::KeyW)
}

/// Shows Vaal Street in front, with the keyboard.
pub fn show(app: &AppHandle) {
    let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
    if !PLACED.swap(true, Ordering::SeqCst) {
        place(app, &win);
    }
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_always_on_top(true);
    let _ = win.set_focus();
}

/// Centers the window on the screen under the mouse: the size the player
/// gave it last time, else the size of WeledFTW's Wealthy Weled panel on
/// this screen (1240 x 686 at 1080p, bigger on bigger screens).
fn place(app: &AppHandle, win: &tauri::WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| win.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (aw, ah) = (area.size.width as f64 / scale, area.size.height as f64 / scale);
    let full = monitor.size();
    let font = font_px(full.width as f64 / scale, full.height as f64 / scale);
    // never smaller than what the whole panel needs at this text size
    let (min_w, min_h) = ((MIN_W_EM * font + MARGIN).min(aw), (MIN_H_EM * font + MARGIN).min(ah));
    let _ = win.set_min_size(Some(LogicalSize::new(min_w, min_h)));
    let (w, h) = saved_size(app).unwrap_or_else(|| default_size(font, aw, ah));
    let (w, h) = (w.clamp(min_w, aw), h.clamp(min_h, ah));
    let _ = win.set_size(LogicalSize::new(w, h));
    let x = area.position.x + ((aw - w) * scale / 2.0) as i32;
    let y = area.position.y + ((ah - h) * scale / 2.0) as i32;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// The smallest panel, in text heights: the title bar (on two lines if it
/// must), the wealth and breakdown cards and a few rows of tabs and items.
const MIN_W_EM: f64 = 62.0;
const MIN_H_EM: f64 = 38.0;
/// The see-through margin around the panel (the page's padding, both sides).
const MARGIN: f64 = 28.0;

/// The page's text size on a screen of this size (the page does the same with
/// its screen.width / screen.height): 14 px at 1080p, more on bigger screens.
fn font_px(sw: f64, sh: f64) -> f64 {
    (14.0 * (sh / 1080.0).min(sw / 1920.0)).round().max(11.0)
}

fn default_size(font: f64, aw: f64, ah: f64) -> (f64, f64) {
    let w = (WIDTH_EM * font + MARGIN).min(aw * 0.96);
    let h = (HEIGHT_EM * font + MARGIN).min(ah * 0.94);
    (w, h)
}

fn size_file(app: &AppHandle) -> std::path::PathBuf {
    crate::poe::data_dir(app).join("window.json")
}

fn saved_size(app: &AppHandle) -> Option<(f64, f64)> {
    let text = std::fs::read_to_string(size_file(app)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some((v.get("width")?.as_f64()?, v.get("height")?.as_f64()?))
}

/// The window was made bigger or smaller: remembered for next time.
pub fn on_resized(app: &AppHandle) {
    if !RESIZING.load(Ordering::SeqCst) {
        return;
    }
    let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
    let (Ok(size), Ok(scale)) = (win.inner_size(), win.scale_factor()) else { return };
    let size = size.to_logical::<f64>(scale);
    let _ = std::fs::create_dir_all(crate::poe::data_dir(app));
    let _ = std::fs::write(size_file(app), serde_json::json!({ "width": size.width, "height": size.height }).to_string());
}

/// Only sizes the player makes by dragging are kept (not the first placing).
static RESIZING: AtomicBool = AtomicBool::new(false);

/// The panel's resize corner was grabbed (the page starts the drag itself).
#[tauri::command]
pub fn vs_resize() {
    RESIZING.store(true, Ordering::SeqCst);
}

/// Double-click on the resize corner: back to the normal size.
#[tauri::command]
pub fn vs_reset_size(app: AppHandle) {
    RESIZING.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_file(size_file(&app));
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        place(&app, &win);
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.hide();
    }
}

/// Ctrl+W and the tray icon. A click on the tray first takes the focus away
/// (which already put the window away): that click must not open it again.
pub fn toggle(app: &AppHandle) {
    let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
    if win.is_visible().unwrap_or(false) {
        hide(app);
    } else if now_ms().saturating_sub(HIDDEN_BY_BLUR_AT.load(Ordering::SeqCst)) > 600 {
        show(app);
    }
}

/// The window lost the keyboard: put it away, unless it went to one of
/// Vaal Street's own windows (the sign-in window, a dropdown list).
pub fn on_blur(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        // the new foreground window takes a moment to be known
        thread::sleep(Duration::from_millis(120));
        let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
        if win.is_focused().unwrap_or(false) || app.get_webview_window(LOGIN_LABEL).is_some() || still_ours() {
            return;
        }
        if win.is_visible().unwrap_or(false) {
            HIDDEN_BY_BLUR_AT.store(now_ms(), Ordering::SeqCst);
            let _ = win.hide();
        }
    });
}

fn process_name(pid: u32) -> String {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return String::new();
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length);
        CloseHandle(process);
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        path.rsplit('\\').next().unwrap_or("").to_lowercase()
    }
}

/// The window in front belongs to Vaal Street: its own, one it owns, or a
/// dropdown list of its page (those come from the WebView2 process).
fn still_ours() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() {
            return false;
        }
        let main = MAIN_HWND.load(Ordering::SeqCst);
        if main != 0 && GetAncestor(fg, GA_ROOTOWNER) as isize == main {
            return true;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(fg, &mut pid);
        pid == GetCurrentProcessId() || process_name(pid) == "msedgewebview2.exe"
    }
}

/// Path of Exile (1, not 2) is the window in front.
fn game_in_front(fg: HWND) -> bool {
    unsafe {
        let mut title = [0u16; 32];
        let length = GetWindowTextW(fg, title.as_mut_ptr(), title.len() as i32);
        if length <= 0 {
            return false;
        }
        let title = String::from_utf16_lossy(&title[..length as usize]);
        if !title.starts_with("Path of Exile") || title.starts_with("Path of Exile 2") {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(fg, &mut pid);
        // the standalone, Steam, Epic and Kakao clients
        process_name(pid).starts_with("pathofexile")
    }
}

/// Takes Ctrl+W while the game or Vaal Street is in front, gives it back
/// otherwise. Checks a few times a second.
fn watch_keys(app: AppHandle) {
    let mut last_fg: isize = 0;
    let mut wanted = false;
    loop {
        let fg = unsafe { GetForegroundWindow() };
        let main = MAIN_HWND.load(Ordering::SeqCst);
        if fg as isize != last_fg {
            last_fg = fg as isize;
            wanted = !fg.is_null() && ((main != 0 && fg as isize == main) || game_in_front(fg));
        }
        let taken = KEY_TAKEN.load(Ordering::SeqCst);
        let shortcuts = app.global_shortcut();
        if wanted && !taken {
            if shortcuts.register(ctrl_w()).is_ok() || shortcuts.is_registered(ctrl_w()) {
                KEY_TAKEN.store(true, Ordering::SeqCst);
            }
        } else if !wanted && taken {
            let _ = shortcuts.unregister(ctrl_w());
            KEY_TAKEN.store(false, Ordering::SeqCst);
        }
        thread::sleep(Duration::from_millis(150));
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        if let Ok(hwnd) = win.hwnd() {
            MAIN_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
        }
    }
    build_tray(app)?;
    let watcher = app.clone();
    thread::spawn(move || watch_keys(watcher));
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Vaal Street (Ctrl+W)", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Vaal Street", true, None::<&str>)?;
    let line = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &line, &quit])?;
    let mut tray = TrayIconBuilder::with_id("vaalstreet")
        .tooltip("Vaal Street")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The page's close button, Esc and Ctrl+W.
#[tauri::command]
pub fn vs_hide(app: AppHandle) {
    hide(&app);
}

/// Dragging the window by its title bar.
#[tauri::command]
pub fn vs_drag(app: AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.start_dragging();
    }
}
