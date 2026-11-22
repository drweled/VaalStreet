//! "Startup on/off": Vaal Street starts with Windows when the player wants it,
//! through the user's own Run list (HKCU\Software\Microsoft\Windows\
//! CurrentVersion\Run, value "Vaal Street"). No admin rights needed.

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "Vaal Street";

fn command_line() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // started with Windows: it waits next to the clock instead of opening
    Ok(format!("\"{}\" {}", exe.display(), crate::overlay::BACKGROUND_FLAG))
}

/// Starts with Windows right now (and points at this copy of Vaal Street).
#[tauri::command]
pub fn vs_startup_get() -> bool {
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ) else { return false };
    let Ok(value) = key.get_value::<String, _>(VALUE) else { return false };
    command_line().map(|me| value.eq_ignore_ascii_case(&me)).unwrap_or(false)
}

/// Turns starting with Windows on or off; answers the new state.
#[tauri::command]
pub fn vs_startup_set(on: bool) -> Result<bool, String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if on {
        key.set_value(VALUE, &command_line()?).map_err(|e| e.to_string())?;
    } else {
        match key.delete_value(VALUE) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(vs_startup_get())
}
