#[cfg(all(target_os = "windows", not(test)))]
use std::path::Path;

#[cfg(target_os = "linux")]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[cfg(any(target_os = "macos", test))]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::{fs, path::PathBuf, process::Command};

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellContextMenuMode {
    Managed,
    Packaged,
    Unsupported,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellContextMenuState {
    pub supported: bool,
    pub enabled: bool,
    pub mode: ShellContextMenuMode,
}

#[cfg(target_os = "windows")]
const MENU_KEY_NAME: &str = "DevFleet";
#[cfg(target_os = "windows")]
const DIRECTORY_SHELL_PATH: &str = r"Software\Classes\Directory\shell";
#[cfg(target_os = "windows")]
const BACKGROUND_SHELL_PATH: &str = r"Software\Classes\Directory\Background\shell";
#[cfg(target_os = "windows")]
const MENU_TITLE: &str = "Add to DevFleet";
#[cfg(target_os = "linux")]
const MENU_TITLE: &str = "Add to DevFleet";
#[cfg(target_os = "macos")]
const MENU_TITLE: &str = "Add to DevFleet";
#[cfg(target_os = "linux")]
const DOLPHIN_SERVICE_FILE_NAME: &str = "devfleet-add-project.desktop";
#[cfg(target_os = "macos")]
const MACOS_SERVICE_DIR_NAME: &str = "Add to DevFleet.workflow";
#[cfg(target_os = "macos")]
const MACOS_WORKFLOW_CONTENTS_DIR: &str = "Contents";
#[cfg(target_os = "macos")]
const MACOS_WORKFLOW_DOCUMENT_FILE: &str = "document.wflow";
#[cfg(target_os = "macos")]
const MACOS_WORKFLOW_INFO_FILE: &str = "Info.plist";

#[cfg(target_os = "windows")]
fn command_value(exe_path: &Path, placeholder: &str) -> String {
    format!(
        "\"{}\" --add-project \"{}\"",
        exe_path.display(),
        placeholder
    )
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

#[cfg(target_os = "linux")]
fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")))
}

#[cfg(target_os = "linux")]
fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
}

#[cfg(target_os = "linux")]
fn nautilus_script_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(data_home) = data_home() {
        paths.push(data_home.join("nautilus").join("scripts").join(MENU_TITLE));
        paths.push(data_home.join("nemo").join("scripts").join(MENU_TITLE));
    }

    if let Some(config_home) = config_home() {
        paths.push(config_home.join("caja").join("scripts").join(MENU_TITLE));
    }

    paths
}

#[cfg(target_os = "linux")]
fn dolphin_service_path() -> Option<PathBuf> {
    data_home().map(|data_home| {
        data_home
            .join("kio")
            .join("servicemenus")
            .join(DOLPHIN_SERVICE_FILE_NAME)
    })
}

#[cfg(target_os = "linux")]
fn integration_paths() -> Vec<PathBuf> {
    let mut paths = nautilus_script_paths();
    if let Some(path) = dolphin_service_path() {
        paths.push(path);
    }
    paths
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

#[cfg(any(target_os = "linux", test))]
fn desktop_exec_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', r"\\").replace('"', "\\\""))
}

#[cfg(any(target_os = "linux", test))]
fn linux_script_content(exe_path: &Path) -> String {
    let exe = shell_single_quote(&exe_path.to_string_lossy());
    format!(
        r#"#!/bin/sh
set -eu

APP={exe}

first_line() {{
  printf '%s\n' "$1" | {{
    IFS= read -r line || true
    printf '%s\n' "$line"
  }}
}}

uri_to_path() {{
  URI=$(first_line "$1")
  [ -n "$URI" ] || return 1

  if command -v python3 >/dev/null 2>&1; then
    DEVFLEET_FILE_URI="$URI" python3 -c 'import os, urllib.parse; parsed = urllib.parse.urlparse(os.environ["DEVFLEET_FILE_URI"]); print(urllib.parse.unquote(parsed.path) if parsed.scheme == "file" else "")'
    return
  fi

  case "$URI" in
    file://localhost/*)
      printf '/%s\n' "${{URI#file://localhost/}}"
      ;;
    file:///*)
      printf '/%s\n' "${{URI#file:///}}"
      ;;
    file:/*)
      printf '%s\n' "${{URI#file:}}"
      ;;
    *)
      return 1
      ;;
  esac
}}

selected_path() {{
  if [ -n "${{NAUTILUS_SCRIPT_SELECTED_FILE_PATHS:-}}" ]; then
    first_line "$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"
    return
  fi

  if [ -n "${{NEMO_SCRIPT_SELECTED_FILE_PATHS:-}}" ]; then
    first_line "$NEMO_SCRIPT_SELECTED_FILE_PATHS"
    return
  fi

  if [ -n "${{CAJA_SCRIPT_SELECTED_FILE_PATHS:-}}" ]; then
    first_line "$CAJA_SCRIPT_SELECTED_FILE_PATHS"
    return
  fi

  if [ -n "${{NAUTILUS_SCRIPT_SELECTED_URIS:-}}" ]; then
    uri_to_path "$NAUTILUS_SCRIPT_SELECTED_URIS"
    return
  fi

  if [ -n "${{NEMO_SCRIPT_SELECTED_URIS:-}}" ]; then
    uri_to_path "$NEMO_SCRIPT_SELECTED_URIS"
    return
  fi

  if [ -n "${{CAJA_SCRIPT_SELECTED_URIS:-}}" ]; then
    uri_to_path "$CAJA_SCRIPT_SELECTED_URIS"
    return
  fi

  if [ "$#" -gt 0 ]; then
    printf '%s\n' "$1"
    return
  fi

  printf '%s\n' "$PWD"
}}

TARGET=$(selected_path "$@")
[ -d "$TARGET" ] || exit 0

"$APP" --add-project "$TARGET" >/dev/null 2>&1 &
"#
    )
}

#[cfg(any(target_os = "linux", test))]
fn dolphin_service_content(exe_path: &Path) -> String {
    let exe = desktop_exec_quote(&exe_path.to_string_lossy());
    format!(
        r#"[Desktop Entry]
Type=Service
MimeType=inode/directory;
Actions=addToDevFleet;
X-KDE-ServiceTypes=KonqPopupMenu/Plugin
X-KDE-Priority=TopLevel

[Desktop Action addToDevFleet]
Name={menu_title}
Icon=devfleet
Exec={exe} --add-project %f
"#,
        menu_title = MENU_TITLE,
        exe = exe
    )
}

#[cfg(target_os = "linux")]
fn write_text_file(path: &Path, content: &str, executable: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    fs::write(path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))?;

    if executable {
        let mut permissions = fs::metadata(path)
            .map_err(|e| format!("failed to read {} permissions: {e}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|e| format!("failed to set {} executable: {e}", path.display()))?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn delete_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("failed to remove {}: {e}", path.display())),
    }
}

#[cfg(target_os = "macos")]
fn macos_services_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("Library").join("Services"))
}

#[cfg(target_os = "macos")]
fn macos_service_dir() -> Option<PathBuf> {
    macos_services_dir().map(|services| services.join(MACOS_SERVICE_DIR_NAME))
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(any(target_os = "macos", test))]
fn macos_service_script(exe_path: &Path) -> String {
    let exe = shell_single_quote(&exe_path.to_string_lossy());
    format!(
        r#"APP={exe}

for TARGET in "$@"; do
  [ -d "$TARGET" ] || continue
  "$APP" --add-project "$TARGET" >/dev/null 2>&1 &
done
"#
    )
}

#[cfg(target_os = "macos")]
fn macos_info_plist_content() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSServices</key>
  <array>
    <dict>
      <key>NSMenuItem</key>
      <dict>
        <key>default</key>
        <string>{menu_title}</string>
      </dict>
      <key>NSMessage</key>
      <string>runWorkflowAsService</string>
    </dict>
  </array>
</dict>
</plist>
"#,
        menu_title = xml_escape(MENU_TITLE)
    )
}

#[cfg(any(target_os = "macos", test))]
fn macos_document_wflow_content(exe_path: &Path) -> String {
    let script = xml_escape(&macos_service_script(exe_path));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>AMApplicationBuild</key>
  <string>523</string>
  <key>AMApplicationVersion</key>
  <string>2.10</string>
  <key>AMDocumentVersion</key>
  <string>2</string>
  <key>actions</key>
  <array>
    <dict>
      <key>action</key>
      <dict>
        <key>AMAccepts</key>
        <dict>
          <key>Container</key>
          <string>List</string>
          <key>Optional</key>
          <true/>
          <key>Types</key>
          <array>
            <string>com.apple.cocoa.path</string>
          </array>
        </dict>
        <key>AMActionVersion</key>
        <string>2.0.3</string>
        <key>AMApplication</key>
        <array>
          <string>Automator</string>
        </array>
        <key>AMBundleIdentifier</key>
        <string>com.apple.RunShellScript</string>
        <key>AMCategory</key>
        <array>
          <string>AMCategoryUtilities</string>
        </array>
        <key>AMIconName</key>
        <string>Automator</string>
        <key>AMName</key>
        <string>Run Shell Script</string>
        <key>AMProvides</key>
        <dict>
          <key>Container</key>
          <string>List</string>
          <key>Types</key>
          <array>
            <string>com.apple.cocoa.path</string>
          </array>
        </dict>
        <key>ActionBundlePath</key>
        <string>/System/Library/Automator/Run Shell Script.action</string>
        <key>ActionName</key>
        <string>Run Shell Script</string>
        <key>ActionParameters</key>
        <dict>
          <key>COMMAND_STRING</key>
          <string>{script}</string>
          <key>CheckedForUserDefaultShell</key>
          <true/>
          <key>inputMethod</key>
          <integer>1</integer>
          <key>shell</key>
          <string>/bin/sh</string>
          <key>source</key>
          <string></string>
        </dict>
        <key>BundleIdentifier</key>
        <string>com.apple.RunShellScript</string>
        <key>CFBundleVersion</key>
        <string>2.0.3</string>
        <key>CanShowSelectedItemsWhenRun</key>
        <false/>
        <key>CanShowWhenRun</key>
        <true/>
        <key>Category</key>
        <array>
          <string>AMCategoryUtilities</string>
        </array>
        <key>Class Name</key>
        <string>RunShellScriptAction</string>
        <key>Keywords</key>
        <array>
          <string>Shell</string>
          <string>Script</string>
          <string>Command</string>
          <string>Run</string>
          <string>Unix</string>
        </array>
      </dict>
    </dict>
  </array>
  <key>connectors</key>
  <dict/>
  <key>workflowMetaData</key>
  <dict>
    <key>serviceApplicationBundleIdentifier</key>
    <string>com.apple.finder</string>
    <key>serviceInputTypeIdentifier</key>
    <string>com.apple.Automator.fileSystemObject</string>
    <key>serviceOutputTypeIdentifier</key>
    <string>com.apple.Automator.nothing</string>
    <key>serviceProcessesInput</key>
    <integer>0</integer>
    <key>workflowTypeIdentifier</key>
    <string>com.apple.Automator.servicesMenu</string>
  </dict>
</dict>
</plist>
"#
    )
}

#[cfg(target_os = "macos")]
fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    fs::write(path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

#[cfg(target_os = "macos")]
fn refresh_macos_services_menu() {
    let _ = Command::new("/System/Library/CoreServices/pbs")
        .arg("-flush")
        .status();
}

#[cfg(target_os = "macos")]
fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("failed to remove {}: {e}", path.display())),
    }
}

#[cfg(target_os = "windows")]
pub fn get_state() -> ShellContextMenuState {
    ShellContextMenuState {
        supported: true,
        enabled: has_menu_key(DIRECTORY_SHELL_PATH),
        mode: ShellContextMenuMode::Managed,
    }
}

#[cfg(target_os = "linux")]
pub fn get_state() -> ShellContextMenuState {
    let paths = integration_paths();
    let supported = !paths.is_empty();
    ShellContextMenuState {
        supported,
        enabled: supported && paths.iter().all(|path| path.exists()),
        mode: if supported {
            ShellContextMenuMode::Managed
        } else {
            ShellContextMenuMode::Unsupported
        },
    }
}

#[cfg(target_os = "macos")]
pub fn get_state() -> ShellContextMenuState {
    let Some(service_dir) = macos_service_dir() else {
        return ShellContextMenuState {
            supported: false,
            enabled: false,
            mode: ShellContextMenuMode::Unsupported,
        };
    };
    let contents_dir = service_dir.join(MACOS_WORKFLOW_CONTENTS_DIR);

    ShellContextMenuState {
        supported: true,
        enabled: contents_dir.join(MACOS_WORKFLOW_DOCUMENT_FILE).exists()
            && contents_dir.join(MACOS_WORKFLOW_INFO_FILE).exists(),
        mode: ShellContextMenuMode::Managed,
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn get_state() -> ShellContextMenuState {
    ShellContextMenuState {
        supported: false,
        enabled: false,
        mode: ShellContextMenuMode::Unsupported,
    }
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<ShellContextMenuState, String> {
    if enabled {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("failed to resolve current exe: {e}"))?;
        set_menu_key(DIRECTORY_SHELL_PATH, "%1", &exe_path)?;
        delete_menu_key(BACKGROUND_SHELL_PATH)?;
    } else {
        delete_menu_key(DIRECTORY_SHELL_PATH)?;
        delete_menu_key(BACKGROUND_SHELL_PATH)?;
    }

    Ok(get_state())
}

#[cfg(target_os = "linux")]
pub fn set_enabled(enabled: bool) -> Result<ShellContextMenuState, String> {
    if enabled {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("failed to resolve current exe: {e}"))?;
        let script_content = linux_script_content(&exe_path);

        for path in nautilus_script_paths() {
            write_text_file(&path, &script_content, true)?;
        }

        if let Some(path) = dolphin_service_path() {
            write_text_file(&path, &dolphin_service_content(&exe_path), false)?;
        }
    } else {
        for path in integration_paths() {
            delete_file_if_exists(&path)?;
        }
    }

    Ok(get_state())
}

#[cfg(target_os = "windows")]
pub fn refresh_existing_registration() -> Result<(), String> {
    if has_menu_key(DIRECTORY_SHELL_PATH) || has_menu_key(BACKGROUND_SHELL_PATH) {
        set_enabled(true)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn refresh_existing_registration() -> Result<(), String> {
    if get_state().enabled {
        set_enabled(true)?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn set_enabled(enabled: bool) -> Result<ShellContextMenuState, String> {
    let service_dir = macos_service_dir()
        .ok_or_else(|| "failed to resolve macOS Services directory".to_string())?;
    let contents_dir = service_dir.join(MACOS_WORKFLOW_CONTENTS_DIR);

    if enabled {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("failed to resolve current exe: {e}"))?;
        write_text_file(
            &contents_dir.join(MACOS_WORKFLOW_INFO_FILE),
            &macos_info_plist_content(),
        )?;
        write_text_file(
            &contents_dir.join(MACOS_WORKFLOW_DOCUMENT_FILE),
            &macos_document_wflow_content(&exe_path),
        )?;
    } else {
        remove_dir_if_exists(&service_dir)?;
    }

    refresh_macos_services_menu();
    Ok(get_state())
}

#[cfg(target_os = "macos")]
pub fn refresh_existing_registration() -> Result<(), String> {
    if get_state().enabled {
        set_enabled(true)?;
    }

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn set_enabled(_enabled: bool) -> Result<ShellContextMenuState, String> {
    Err("shell context menu is not supported on this system".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn refresh_existing_registration() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_context_menu_targets_selected_folder_key() {
        assert_eq!(DIRECTORY_SHELL_PATH, r"Software\Classes\Directory\shell");
        assert_eq!(
            BACKGROUND_SHELL_PATH,
            r"Software\Classes\Directory\Background\shell"
        );
        assert!(!DIRECTORY_SHELL_PATH.contains("Background"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_context_menu_command_passes_selected_folder_path() {
        let command = command_value(Path::new(r"C:\Program Files\DevFleet\devfleet.exe"), "%1");

        assert_eq!(
            command,
            r#""C:\Program Files\DevFleet\devfleet.exe" --add-project "%1""#
        );
    }

    #[test]
    fn xml_escape_escapes_special_characters() {
        assert_eq!(
            xml_escape(r#"<DevFleet & "Projects" 'Menu'>"#),
            "&lt;DevFleet &amp; &quot;Projects&quot; &apos;Menu&apos;&gt;"
        );
    }

    #[test]
    fn macos_service_script_quotes_executable_path() {
        let script = macos_service_script(Path::new(
            "/Applications/Dev Fleet's.app/Contents/MacOS/devfleet",
        ));

        assert!(
            script.contains("APP='/Applications/Dev Fleet'\"'\"'s.app/Contents/MacOS/devfleet'")
        );
        assert!(script.contains(r#""$APP" --add-project "$TARGET""#));
        assert!(script.contains(r#"for TARGET in "$@"; do"#));
    }

    #[test]
    fn macos_workflow_contains_finder_service_metadata() {
        let workflow = macos_document_wflow_content(Path::new(
            "/Applications/Dev & Fleet.app/Contents/MacOS/devfleet",
        ));

        assert!(workflow.contains("<key>serviceApplicationBundleIdentifier</key>"));
        assert!(workflow.contains("<string>com.apple.finder</string>"));
        assert!(workflow.contains("<key>serviceInputTypeIdentifier</key>"));
        assert!(workflow.contains("<string>com.apple.Automator.fileSystemObject</string>"));
        assert!(workflow.contains("<string>com.apple.Automator.servicesMenu</string>"));
        assert!(workflow.contains("&amp;"));
        assert!(!workflow.contains("/Applications/Dev & Fleet.app"));
    }

    #[test]
    fn linux_script_supports_common_file_manager_selection_vars() {
        let script = linux_script_content(Path::new("/opt/Dev Fleet's/devfleet"));

        assert!(script.contains("APP='/opt/Dev Fleet'\"'\"'s/devfleet'"));
        assert!(script.contains("NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"));
        assert!(script.contains("NEMO_SCRIPT_SELECTED_FILE_PATHS"));
        assert!(script.contains("CAJA_SCRIPT_SELECTED_FILE_PATHS"));
        assert!(script.contains("NAUTILUS_SCRIPT_SELECTED_URIS"));
        assert!(script.contains("NEMO_SCRIPT_SELECTED_URIS"));
        assert!(script.contains("CAJA_SCRIPT_SELECTED_URIS"));
        assert!(script.contains("DEVFLEET_FILE_URI"));
        assert!(script.contains("urllib.parse.unquote"));
        assert!(script.contains("file://localhost/*"));
        assert!(script.contains(r#""$APP" --add-project "$TARGET""#));
        assert!(script.contains("[ -d \"$TARGET\" ] || exit 0"));
    }

    #[test]
    fn dolphin_service_targets_directories_and_quotes_exec_path() {
        let service = dolphin_service_content(Path::new(r#"/opt/Dev "Fleet"/devfleet"#));

        assert!(service.contains("Type=Service"));
        assert!(service.contains("MimeType=inode/directory;"));
        assert!(service.contains("Actions=addToDevFleet;"));
        assert!(service.contains("X-KDE-ServiceTypes=KonqPopupMenu/Plugin"));
        assert!(service.contains("Name=Add to DevFleet"));
        assert!(service.contains(r#"Exec="/opt/Dev \"Fleet\"/devfleet" --add-project %f"#));
    }
}
