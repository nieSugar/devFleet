use crate::config;
use crate::detector;
use crate::models::{
    CustomEditor, EditorCandidate, EditorCandidateDiscovery, EditorCandidateSource, EditorLaunch,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_CANDIDATES: usize = 1000;
const MAX_WARNINGS: usize = 50;
const SNAPSHOT_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug)]
pub(crate) struct CandidateRecord {
    pub candidate: EditorCandidate,
    pub launch: EditorLaunch,
    pub icon_source: Option<String>,
}

#[derive(Clone, Debug)]
struct RawCandidate {
    name: String,
    location: PathBuf,
    source_identity: String,
    source: EditorCandidateSource,
    recommended: bool,
    launch: EditorLaunch,
    icon_source: Option<String>,
}

struct CandidateSnapshot {
    created_at: Instant,
    records: HashMap<String, CandidateRecord>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportEditorCandidateRequest {
    pub candidate_id: String,
    #[serde(default)]
    pub name: Option<String>,
}

fn snapshot() -> &'static Mutex<Option<CandidateSnapshot>> {
    static SNAPSHOT: OnceLock<Mutex<Option<CandidateSnapshot>>> = OnceLock::new();
    SNAPSHOT.get_or_init(|| Mutex::new(None))
}

fn normalized_path(raw: &str) -> String {
    let path = PathBuf::from(raw);
    let path = path.canonicalize().unwrap_or(path);
    let value = path.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    let value = value.replace('/', "\\").to_lowercase();
    value
}

fn launch_key(launch: &EditorLaunch) -> String {
    match launch {
        EditorLaunch::Executable {
            path,
            args,
            working_directory,
        } => format!(
            "exe|{}|{}|{}",
            normalized_path(path),
            args.join("\u{1f}"),
            working_directory
                .as_deref()
                .map(normalized_path)
                .unwrap_or_default()
        ),
        EditorLaunch::MacApp { path } => format!("app|{}", normalized_path(path)),
        EditorLaunch::DesktopEntry { path } => format!("desktop|{}", normalized_path(path)),
        EditorLaunch::KnownWindowsBatch { adapter_id, path } => {
            format!("batch|{}|{}", adapter_id, normalized_path(path))
        }
    }
}

fn stable_candidate_id(identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("candidate-{hash:016x}")
}

fn looks_like_editor(name: &str, target: &str) -> bool {
    let value = format!("{} {}", name, target).to_lowercase();
    [
        "visual studio code",
        "vscode",
        "cursor",
        "windsurf",
        "trae",
        "webstorm",
        "intellij",
        "pycharm",
        "clion",
        "goland",
        "rider",
        "sublime text",
        "notepad++",
        "zed",
        "kiro",
        "antigravity",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn excluded_application(name: &str, target: &Path) -> bool {
    let value = format!(
        "{} {}",
        name,
        target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    )
    .to_lowercase();
    ["uninstall", "unins", "updater", "installer", "setup"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn configured_launches() -> Result<HashSet<String>, String> {
    let mut keys: HashSet<String> = config::load_custom_editors()?
        .iter()
        .map(|editor| launch_key(&editor.launch))
        .collect();
    let automatic = match config::load_editor_cache()? {
        Some(cache) => cache,
        None => detector::detect_editors(),
    };
    keys.extend(
        automatic
            .values()
            .filter_map(|editor| editor.launch.as_ref())
            .map(launch_key),
    );
    Ok(keys)
}

fn finalize_candidates(
    raw: Vec<RawCandidate>,
    configured: &HashSet<String>,
    mut warnings: Vec<String>,
) -> (EditorCandidateDiscovery, HashMap<String, CandidateRecord>) {
    let mut by_launch: HashMap<String, RawCandidate> = HashMap::new();
    for candidate in raw {
        let key = launch_key(&candidate.launch);
        by_launch
            .entry(key)
            .and_modify(|existing| existing.recommended |= candidate.recommended)
            .or_insert(candidate);
    }

    let mut raw: Vec<RawCandidate> = by_launch.into_values().collect();
    raw.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.source_identity.cmp(&right.source_identity))
    });
    let truncated = raw.len() > MAX_CANDIDATES;
    if truncated {
        raw.truncate(MAX_CANDIDATES);
        warnings.push(format!("候选数量超过 {}，结果已截断", MAX_CANDIDATES));
    }
    warnings.truncate(MAX_WARNINGS);

    let mut records = HashMap::new();
    let mut candidates = Vec::with_capacity(raw.len());
    for raw in raw {
        let id = stable_candidate_id(&raw.source_identity);
        let candidate = EditorCandidate {
            id: id.clone(),
            name: raw.name,
            path: raw.location.to_string_lossy().into_owned(),
            source: raw.source,
            added: configured.contains(&launch_key(&raw.launch)),
            recommended: raw.recommended,
        };
        records.insert(
            id,
            CandidateRecord {
                candidate: candidate.clone(),
                launch: raw.launch,
                icon_source: raw.icon_source,
            },
        );
        candidates.push(candidate);
    }

    (
        EditorCandidateDiscovery {
            candidates,
            warnings,
            truncated,
        },
        records,
    )
}

pub fn discover_editor_candidates() -> Result<EditorCandidateDiscovery, String> {
    let configured = configured_launches()?;
    let (raw, warnings) = platform_candidates();
    let (response, records) = finalize_candidates(raw, &configured, warnings);
    *snapshot().lock().unwrap_or_else(|error| error.into_inner()) = Some(CandidateSnapshot {
        created_at: Instant::now(),
        records,
    });
    Ok(response)
}

fn candidate_from(
    snapshot: &CandidateSnapshot,
    candidate_id: &str,
    now: Instant,
) -> Result<CandidateRecord, String> {
    if now.duration_since(snapshot.created_at) > SNAPSHOT_TTL {
        return Err("候选快照已失效，请重新扫描".to_string());
    }
    snapshot
        .records
        .get(candidate_id)
        .cloned()
        .ok_or_else(|| "候选不存在或快照已更新，请重新扫描".to_string())
}

pub(crate) fn candidate_from_snapshot(candidate_id: &str) -> Result<CandidateRecord, String> {
    let guard = snapshot().lock().unwrap_or_else(|error| error.into_inner());
    let snapshot = guard
        .as_ref()
        .ok_or_else(|| "候选快照已失效，请重新扫描".to_string())?;
    candidate_from(snapshot, candidate_id, Instant::now())
}

pub fn import_editor_candidate(
    request: ImportEditorCandidateRequest,
) -> Result<CustomEditor, String> {
    let record = candidate_from_snapshot(request.candidate_id.trim())?;
    let name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&record.candidate.name)
        .to_string();
    crate::editors::import_trusted_editor(name, record.launch, record.icon_source)
}

#[cfg(target_os = "windows")]
fn platform_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    windows_candidates()
}

#[cfg(target_os = "macos")]
fn platform_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    macos_candidates()
}

#[cfg(target_os = "linux")]
fn platform_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    linux_candidates()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn platform_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    (Vec::new(), vec!["当前平台不支持系统应用扫描".to_string()])
}

#[cfg(target_os = "windows")]
fn windows_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();
    scan_windows_app_paths(&mut candidates, &mut warnings);
    scan_windows_start_menu(&mut candidates, &mut warnings);
    (candidates, warnings)
}

#[cfg(target_os = "windows")]
fn scan_windows_app_paths(candidates: &mut Vec<RawCandidate>, warnings: &mut Vec<String>) {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let Ok(key) = RegKey::predef(root).open_subkey_with_flags(APP_PATHS, KEY_READ | view)
            else {
                continue;
            };
            for subkey in key.enum_keys().flatten() {
                let Ok(app) = key.open_subkey_with_flags(&subkey, KEY_READ | view) else {
                    continue;
                };
                let Ok(raw_path) = app.get_value::<String, _>("") else {
                    continue;
                };
                let path = PathBuf::from(raw_path.trim().trim_matches('"'));
                if !windows_executable(&path) {
                    continue;
                }
                let name = Path::new(&subkey)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(&subkey)
                    .to_string();
                if excluded_application(&name, &path) {
                    continue;
                }
                candidates.push(RawCandidate {
                    recommended: looks_like_editor(&name, &path.to_string_lossy()),
                    source_identity: format!(
                        "app-paths|{}",
                        normalized_path(&path.to_string_lossy())
                    ),
                    location: path.clone(),
                    source: EditorCandidateSource::AppPaths,
                    name,
                    launch: EditorLaunch::Executable {
                        path: path.to_string_lossy().into_owned(),
                        args: Vec::new(),
                        working_directory: None,
                    },
                    icon_source: Some(path.to_string_lossy().into_owned()),
                });
            }
        }
    }
    let _ = warnings;
}

#[cfg(target_os = "windows")]
fn windows_executable(path: &Path) -> bool {
    path.is_absolute()
        && path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("exe"))
}

#[cfg(target_os = "windows")]
fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(path) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(path) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(path).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    roots
}

#[cfg(target_os = "windows")]
fn collect_shortcuts(root: &Path, output: &mut Vec<PathBuf>, depth: usize, limit: usize) -> bool {
    if depth == 0 || output.len() >= limit {
        return false;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return root.exists(),
    };
    let mut partial_failure = false;
    for entry in entries {
        if output.len() >= limit {
            break;
        }
        let Ok(entry) = entry else {
            partial_failure = true;
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            partial_failure = true;
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            partial_failure |= collect_shortcuts(&path, output, depth - 1, limit);
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("lnk"))
        {
            output.push(path);
        }
    }
    partial_failure
}

#[cfg(target_os = "windows")]
fn scan_windows_start_menu(candidates: &mut Vec<RawCandidate>, warnings: &mut Vec<String>) {
    let mut shortcuts = Vec::new();
    let mut partial_failure = false;
    for root in start_menu_roots() {
        partial_failure |= collect_shortcuts(&root, &mut shortcuts, 8, 5000);
    }
    if partial_failure {
        warnings.push("部分开始菜单目录无法读取，其余候选仍已返回".to_string());
    }
    if shortcuts.len() == 5000 {
        warnings.push("开始菜单快捷方式超过 5000 个，扫描已提前停止".to_string());
    }

    let initialized = match WindowsCom::initialize() {
        Ok(initialized) => initialized,
        Err(error) => {
            warnings.push(error);
            return;
        }
    };
    for shortcut in shortcuts {
        match parse_windows_shortcut(&shortcut) {
            Ok(Some(candidate)) => candidates.push(candidate),
            Ok(None) => {}
            Err(error) if warnings.len() < MAX_WARNINGS => warnings.push(format!(
                "无法读取快捷方式 {}：{}",
                shortcut.display(),
                error
            )),
            Err(_) => {}
        }
    }
    drop(initialized);
}

#[cfg(target_os = "windows")]
struct WindowsCom(bool);

#[cfg(target_os = "windows")]
impl WindowsCom {
    fn initialize() -> Result<Self, String> {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        let status = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if status == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
            return Ok(Self(false));
        }
        status
            .ok()
            .map(|_| Self(true))
            .map_err(|error| format!("初始化 Windows Shell COM 失败：{}", error))
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsCom {
    fn drop(&mut self) {
        if self.0 {
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

#[cfg(target_os = "windows")]
fn wide_string(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
fn utf16_buffer(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

#[cfg(target_os = "windows")]
fn split_windows_arguments(raw: &str) -> Result<Vec<String>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::UI::Shell::CommandLineToArgvW;

    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let command = format!("devfleet-placeholder {}", raw);
    let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
    let mut count = 0;
    let argv = unsafe { CommandLineToArgvW(PCWSTR(wide.as_ptr()), &mut count) };
    if argv.is_null() || count < 1 {
        return Err("快捷方式参数格式无效".to_string());
    }
    let values = unsafe { std::slice::from_raw_parts(argv, count as usize) };
    let result = values
        .iter()
        .skip(1)
        .map(|value| unsafe { value.to_string() }.map_err(|error| error.to_string()))
        .collect();
    unsafe {
        let _ = LocalFree(Some(HLOCAL(argv.cast())));
    }
    result
}

#[cfg(target_os = "windows")]
fn parse_windows_shortcut(shortcut: &Path) -> Result<Option<RawCandidate>, String> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .map_err(|error| error.to_string())?;
    let persist: IPersistFile = link.cast().map_err(|error| error.to_string())?;
    let shortcut_wide = wide_string(shortcut);
    unsafe { persist.Load(PCWSTR(shortcut_wide.as_ptr()), STGM_READ) }
        .map_err(|error| error.to_string())?;

    let mut target = vec![0u16; 32768];
    let mut find_data = WIN32_FIND_DATAW::default();
    unsafe { link.GetPath(&mut target, &mut find_data, 0) }.map_err(|error| error.to_string())?;
    let target = PathBuf::from(utf16_buffer(&target));
    if !windows_executable(&target) {
        return Ok(None);
    }

    let name = shortcut
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Application")
        .to_string();
    if excluded_application(&name, &target) {
        return Ok(None);
    }

    let mut arguments = vec![0u16; 32768];
    unsafe { link.GetArguments(&mut arguments) }.map_err(|error| error.to_string())?;
    let args = split_windows_arguments(&utf16_buffer(&arguments))?;

    let mut directory = vec![0u16; 32768];
    unsafe { link.GetWorkingDirectory(&mut directory) }.map_err(|error| error.to_string())?;
    let working_directory = utf16_buffer(&directory);
    let working_directory = if working_directory.is_empty() {
        None
    } else {
        let directory = PathBuf::from(working_directory);
        if !directory.is_absolute() || !directory.is_dir() {
            return Err("快捷方式工作目录无效".to_string());
        }
        Some(directory.to_string_lossy().into_owned())
    };

    let mut icon_path = vec![0u16; 32768];
    let mut icon_index = 0;
    unsafe { link.GetIconLocation(&mut icon_path, &mut icon_index) }
        .map_err(|error| error.to_string())?;
    let icon_path = PathBuf::from(utf16_buffer(&icon_path));
    let icon_source = if icon_path.is_absolute() && icon_path.is_file() {
        icon_path
    } else {
        shortcut.to_path_buf()
    };

    Ok(Some(RawCandidate {
        recommended: looks_like_editor(&name, &target.to_string_lossy()),
        source_identity: format!(
            "start-menu|{}",
            normalized_path(&shortcut.to_string_lossy())
        ),
        location: shortcut.to_path_buf(),
        source: EditorCandidateSource::StartMenu,
        name,
        launch: EditorLaunch::Executable {
            path: target.to_string_lossy().into_owned(),
            args,
            working_directory,
        },
        icon_source: Some(icon_source.to_string_lossy().into_owned()),
    }))
}

#[cfg(target_os = "macos")]
fn macos_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                add_macos_app(
                    &mut candidates,
                    entry.path(),
                    EditorCandidateSource::Applications,
                );
            }
        }
    }
    match spotlight_apps() {
        Ok(paths) => {
            for path in paths {
                add_macos_app(&mut candidates, path, EditorCandidateSource::Spotlight);
            }
        }
        Err(error) => warnings.push(error),
    }
    (candidates, warnings)
}

#[cfg(target_os = "macos")]
fn add_macos_app(candidates: &mut Vec<RawCandidate>, path: PathBuf, source: EditorCandidateSource) {
    if !path.is_absolute()
        || !path.is_dir()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
    {
        return;
    }
    let (display_name, bundle_id) = macos_bundle_metadata(&path);
    let name = display_name.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("Application")
            .to_string()
    });
    if excluded_application(&name, &path) {
        return;
    }
    let path_string = path.to_string_lossy().into_owned();
    candidates.push(RawCandidate {
        recommended: looks_like_editor(&name, &path_string),
        source_identity: format!(
            "mac-app|{}|{}",
            normalized_path(&path_string),
            bundle_id.unwrap_or_default()
        ),
        location: path.clone(),
        source,
        name,
        launch: EditorLaunch::MacApp {
            path: path_string.clone(),
        },
        icon_source: Some(path_string),
    });
}

#[cfg(target_os = "macos")]
fn macos_bundle_metadata(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(data) = std::fs::read(path.join("Contents/Info.plist")) else {
        return (None, None);
    };
    if data.len() > 1024 * 1024 {
        return (None, None);
    }
    let Ok(text) = String::from_utf8(data) else {
        return (None, None);
    };
    (
        plist_string(&text, "CFBundleDisplayName").or_else(|| plist_string(&text, "CFBundleName")),
        plist_string(&text, "CFBundleIdentifier"),
    )
}

#[cfg(target_os = "macos")]
fn plist_string(text: &str, key: &str) -> Option<String> {
    let after_key = text.split_once(&format!("<key>{}</key>", key))?.1;
    let after_string = after_key.split_once("<string>")?.1;
    let value = after_string.split_once("</string>")?.0.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(target_os = "macos")]
fn spotlight_apps() -> Result<Vec<PathBuf>, String> {
    let mut command = std::process::Command::new("mdfind");
    command.arg("kMDItemContentType == 'com.apple.application-bundle'");
    let output = detector::output_with_timeout(command, 5)?;
    if !output.status.success() {
        return Err("Spotlight 扫描失败".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .take(5000)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

#[cfg(target_os = "linux")]
fn linux_candidates() -> (Vec<RawCandidate>, Vec<String>) {
    use gio::prelude::*;

    let mut candidates = Vec::new();
    for app in gio::AppInfo::all() {
        if !app.should_show() {
            continue;
        }
        let Ok(desktop) = app.downcast::<gio::DesktopAppInfo>() else {
            continue;
        };
        let Some(path) = desktop.filename() else {
            continue;
        };
        if !path.is_absolute() || !path.is_file() {
            continue;
        }
        let name = desktop.display_name().to_string();
        if excluded_application(&name, &path) {
            continue;
        }
        let categories = desktop.categories().unwrap_or_default();
        let recommended = categories.split(';').any(|value| value == "IDE")
            || looks_like_editor(&name, &path.to_string_lossy());
        let path_string = path.to_string_lossy().into_owned();
        candidates.push(RawCandidate {
            source_identity: format!("desktop|{}", normalized_path(&path_string)),
            location: path.clone(),
            source: EditorCandidateSource::DesktopEntry,
            recommended,
            name,
            launch: EditorLaunch::DesktopEntry {
                path: path_string.clone(),
            },
            icon_source: Some(path_string),
        });
    }
    (candidates, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, path: &str, identity: &str) -> RawCandidate {
        RawCandidate {
            name: name.to_string(),
            location: PathBuf::from(path),
            source_identity: identity.to_string(),
            source: EditorCandidateSource::AppPaths,
            recommended: false,
            launch: EditorLaunch::Executable {
                path: path.to_string(),
                args: Vec::new(),
                working_directory: None,
            },
            icon_source: Some(path.to_string()),
        }
    }

    #[test]
    fn candidate_ids_are_stable() {
        assert_eq!(stable_candidate_id("same"), stable_candidate_id("same"));
        assert_ne!(stable_candidate_id("same"), stable_candidate_id("other"));
    }

    #[test]
    fn same_launch_is_merged_but_same_name_different_target_is_kept() {
        let first = raw("Editor", "/apps/editor-a", "source-a");
        let duplicate = raw("Alias", "/apps/editor-a", "source-b");
        let other = raw("Editor", "/apps/editor-b", "source-c");
        let (result, records) =
            finalize_candidates(vec![first, duplicate, other], &HashSet::new(), Vec::new());
        assert_eq!(result.candidates.len(), 2);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn configured_launch_is_marked_added() {
        let candidate = raw("Editor", "/apps/editor", "source");
        let configured = HashSet::from([launch_key(&candidate.launch)]);
        let (result, _) = finalize_candidates(vec![candidate], &configured, Vec::new());
        assert!(result.candidates[0].added);
    }

    #[test]
    fn candidate_limit_is_enforced() {
        let raw = (0..=MAX_CANDIDATES)
            .map(|index| {
                raw(
                    &format!("Editor {index}"),
                    &format!("/apps/editor-{index}"),
                    &format!("source-{index}"),
                )
            })
            .collect();
        let (result, records) = finalize_candidates(raw, &HashSet::new(), Vec::new());
        assert_eq!(result.candidates.len(), MAX_CANDIDATES);
        assert_eq!(records.len(), MAX_CANDIDATES);
        assert!(result.truncated);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn expired_or_unknown_snapshot_candidate_is_rejected() {
        let candidate = raw("Editor", "/apps/editor", "source");
        let (result, records) = finalize_candidates(vec![candidate], &HashSet::new(), Vec::new());
        let snapshot = CandidateSnapshot {
            created_at: Instant::now() - SNAPSHOT_TTL - Duration::from_secs(1),
            records,
        };
        assert!(
            candidate_from(&snapshot, &result.candidates[0].id, Instant::now())
                .unwrap_err()
                .contains("失效")
        );

        let fresh = CandidateSnapshot {
            created_at: Instant::now(),
            records: HashMap::new(),
        };
        assert!(candidate_from(&fresh, "candidate-missing", Instant::now()).is_err());
    }

    #[test]
    fn import_request_rejects_client_launch_data() {
        let request = serde_json::from_value::<ImportEditorCandidateRequest>(serde_json::json!({
            "candidateId": "candidate-safe",
            "launch": { "kind": "executable", "path": "bad.exe", "args": [] }
        }));
        assert!(request.is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn controlled_shell_link_round_trip_preserves_args_and_working_directory() {
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER};
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

        let _com = WindowsCom::initialize().unwrap();
        let directory = std::env::temp_dir().join(format!(
            "devfleet-candidate-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let target = std::env::current_exe().unwrap();
        let shortcut = directory.join("Controlled Editor.lnk");
        let link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.unwrap();
        let target_wide = wide_string(&target);
        let directory_wide = wide_string(&directory);
        let arguments: Vec<u16> = "--wait \"two words\""
            .encode_utf16()
            .chain(Some(0))
            .collect();
        unsafe {
            link.SetPath(PCWSTR(target_wide.as_ptr())).unwrap();
            link.SetWorkingDirectory(PCWSTR(directory_wide.as_ptr()))
                .unwrap();
            link.SetArguments(PCWSTR(arguments.as_ptr())).unwrap();
        }
        let persist: IPersistFile = link.cast().unwrap();
        let shortcut_wide = wide_string(&shortcut);
        unsafe {
            persist.Save(PCWSTR(shortcut_wide.as_ptr()), true).unwrap();
        }

        let parsed = parse_windows_shortcut(&shortcut).unwrap().unwrap();
        let EditorLaunch::Executable {
            args,
            working_directory,
            ..
        } = parsed.launch
        else {
            panic!("expected executable launch")
        };
        assert_eq!(args, vec!["--wait", "two words"]);
        assert_eq!(
            working_directory.as_deref(),
            Some(directory.to_string_lossy().as_ref())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn corrupt_shell_link_is_rejected_without_execution() {
        let _com = WindowsCom::initialize().unwrap();
        let directory = std::env::temp_dir().join(format!(
            "devfleet-bad-candidate-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let shortcut = directory.join("Broken.lnk");
        std::fs::write(&shortcut, b"not a shell link").unwrap();
        assert!(parse_windows_shortcut(&shortcut).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_system_scan_returns_only_supported_local_targets() {
        let (candidates, _) = windows_candidates();
        for candidate in candidates {
            let EditorLaunch::Executable { path, .. } = candidate.launch else {
                panic!("Windows candidates must resolve to executables")
            };
            assert!(windows_executable(Path::new(&path)));
        }
    }
}
