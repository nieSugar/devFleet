#[cfg(target_os = "windows")]
use std::path::Path;

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellContextMenuState {
    pub supported: bool,
    pub enabled: bool,
}

#[cfg(target_os = "windows")]
const MENU_KEY_NAME: &str = "DevFleet";
#[cfg(target_os = "windows")]
const DIRECTORY_SHELL_PATH: &str = r"Software\Classes\Directory\shell";
#[cfg(target_os = "windows")]
const BACKGROUND_SHELL_PATH: &str = r"Software\Classes\Directory\Background\shell";
#[cfg(target_os = "windows")]
const MENU_TITLE: &str = "Open with DevFleet";

#[cfg(target_os = "windows")]
fn command_value(exe_path: &Path, placeholder: &str) -> String {
    format!("\"{}\" \"{}\"", exe_path.display(), placeholder)
}

#[cfg(target_os = "windows")]
fn set_menu_key(parent_path: &str, command_arg: &str, exe_path: &Path) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let shell = hkcu
        .create_subkey_with_flags(parent_path, KEY_WRITE)
        .map_err(|e| format!("failed to open shell registry key: {e}"))?
        .0;
    let menu = shell
        .create_subkey_with_flags(MENU_KEY_NAME, KEY_WRITE)
        .map_err(|e| format!("failed to create context menu key: {e}"))?
        .0;
    menu.set_value("", &MENU_TITLE)
        .map_err(|e| format!("failed to write context menu title: {e}"))?;
    menu.set_value("Icon", &exe_path.display().to_string())
        .map_err(|e| format!("failed to write context menu icon: {e}"))?;

    let command = menu
        .create_subkey_with_flags("command", KEY_WRITE)
        .map_err(|e| format!("failed to create context menu command key: {e}"))?
        .0;
    command
        .set_value("", &command_value(exe_path, command_arg))
        .map_err(|e| format!("failed to write context menu command: {e}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_menu_key(parent_path: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(shell) = hkcu.open_subkey_with_flags(parent_path, KEY_WRITE) else {
        return Ok(());
    };

    match shell.delete_subkey_all(MENU_KEY_NAME) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("failed to remove context menu key: {e}")),
    }
}

#[cfg(target_os = "windows")]
fn has_menu_key(parent_path: &str) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey_with_flags(format!(r"{parent_path}\{MENU_KEY_NAME}\command"), KEY_READ)
        .is_ok()
}

#[cfg(target_os = "windows")]
pub fn get_state() -> ShellContextMenuState {
    ShellContextMenuState {
        supported: true,
        enabled: has_menu_key(DIRECTORY_SHELL_PATH) && has_menu_key(BACKGROUND_SHELL_PATH),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_state() -> ShellContextMenuState {
    ShellContextMenuState {
        supported: false,
        enabled: false,
    }
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<ShellContextMenuState, String> {
    if enabled {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("failed to resolve current exe: {e}"))?;
        set_menu_key(DIRECTORY_SHELL_PATH, "%1", &exe_path)?;
        set_menu_key(BACKGROUND_SHELL_PATH, "%V", &exe_path)?;
    } else {
        delete_menu_key(DIRECTORY_SHELL_PATH)?;
        delete_menu_key(BACKGROUND_SHELL_PATH)?;
    }

    Ok(get_state())
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool) -> Result<ShellContextMenuState, String> {
    Err("shell context menu is only supported on Windows".to_string())
}
