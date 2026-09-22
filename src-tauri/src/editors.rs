use crate::config;
use crate::detector;
use crate::icons;
use crate::models::{CustomEditor, EditorCache, EditorLaunch, EditorView};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq)]
enum LaunchOrigin {
    Known(String),
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedLaunch {
    launch: EditorLaunch,
    origin: LaunchOrigin,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpsertCustomEditorRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct LaunchIdentity {
    kind: u8,
    target: String,
    args: Vec<String>,
    working_directory: Option<String>,
    adapter_id: Option<String>,
}

fn select_saved_launch(
    editor_id: &str,
    cache: &EditorCache,
    custom_editors: &[CustomEditor],
) -> Result<Option<ResolvedLaunch>, String> {
    if detector::is_known_editor_id(editor_id) {
        return Ok(cache
            .get(editor_id)
            .filter(|editor| editor.installed)
            .and_then(|editor| {
                editor.launch.clone().map(|launch| ResolvedLaunch {
                    launch,
                    origin: LaunchOrigin::Known(editor_id.to_string()),
                })
            }));
    }

    custom_editors
        .iter()
        .find(|editor| editor.id == editor_id)
        .map(|editor| {
            Some(ResolvedLaunch {
                launch: editor.launch.clone(),
                origin: LaunchOrigin::Custom,
            })
        })
        .ok_or_else(|| format!("未知编辑器 ID: {}", editor_id))
}

fn fresh_known_launch(editor_id: &str) -> Result<ResolvedLaunch, String> {
    detector::resolve_known_editor(editor_id)
        .filter(|editor| editor.installed)
        .and_then(|editor| editor.launch)
        .map(|launch| ResolvedLaunch {
            launch,
            origin: LaunchOrigin::Known(editor_id.to_string()),
        })
        .ok_or_else(|| format!("未检测到已知编辑器: {}", editor_id))
}

fn absolute_path(raw: &str, label: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.contains('\0') || raw.contains("://") {
        return Err(format!("{}路径无效", label));
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(format!("{}必须是绝对路径: {}", label, raw));
    }
    Ok(path)
}

fn validate_project_path(raw: &str) -> Result<PathBuf, String> {
    let path = absolute_path(raw, "项目")?;
    if !path.is_dir() {
        return Err(format!("项目路径不存在或不是目录: {}", raw));
    }
    Ok(path)
}

fn validate_executable(raw: &str) -> Result<PathBuf, String> {
    let path = absolute_path(raw, "编辑器")?;
    if !path.is_file() {
        return Err(format!("编辑器可执行文件不存在: {}", raw));
    }

    #[cfg(target_os = "windows")]
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return Err(format!("Windows 直接启动目标必须是 .exe: {}", raw));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
        if !executable {
            return Err(format!("编辑器文件不可执行: {}", raw));
        }
    }

    Ok(path)
}

fn batch_argument_is_safe(value: &str) -> bool {
    !value.chars().any(|character| {
        matches!(
            character,
            '%' | '!' | '&' | '|' | '<' | '>' | '^' | '"' | '\r' | '\n'
        )
    })
}

fn validate_launch_target(resolved: &ResolvedLaunch) -> Result<(), String> {
    match &resolved.launch {
        EditorLaunch::Executable {
            path,
            args,
            working_directory,
        } => {
            validate_executable(path)?;
            if args.iter().any(|argument| argument.contains('\0')) {
                return Err("编辑器参数不能包含 NUL 字符".to_string());
            }
            if let Some(directory) = working_directory {
                let directory = absolute_path(directory, "工作目录")?;
                if !directory.is_dir() {
                    return Err("编辑器工作目录不存在或不是目录".to_string());
                }
            }
            Ok(())
        }
        EditorLaunch::MacApp { path } => {
            #[cfg(target_os = "macos")]
            {
                let bundle = absolute_path(path, "macOS 应用")?;
                if !bundle.is_dir()
                    || !bundle
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
                    || !bundle.join("Contents/Info.plist").is_file()
                {
                    return Err(format!("macOS 应用包不存在或格式无效: {}", path));
                }
                Ok(())
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = path;
                Err("当前平台不支持 macOS 应用包".to_string())
            }
        }
        EditorLaunch::DesktopEntry { path } => {
            #[cfg(target_os = "linux")]
            {
                let desktop = absolute_path(path, "desktop entry")?;
                if !desktop.is_file()
                    || !desktop
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
                    || gio::DesktopAppInfo::from_filename(&desktop).is_none()
                {
                    return Err(format!("desktop entry 不存在或无效: {}", path));
                }
                Ok(())
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = path;
                Err("当前平台不支持 desktop entry".to_string())
            }
        }
        EditorLaunch::KnownWindowsBatch { adapter_id, path } => {
            #[cfg(target_os = "windows")]
            {
                let LaunchOrigin::Known(editor_id) = &resolved.origin else {
                    return Err("批处理启动仅允许内建编辑器适配器使用".to_string());
                };
                let batch = absolute_path(path, "内建批处理适配器")?;
                let valid_extension = batch
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        extension.eq_ignore_ascii_case("cmd")
                            || extension.eq_ignore_ascii_case("bat")
                    });
                if !batch.is_file()
                    || !valid_extension
                    || !detector::is_allowed_known_windows_batch(editor_id, adapter_id, &batch)
                {
                    return Err("内建批处理适配器已失效，请重新扫描或选择真实 .exe".to_string());
                }
                Ok(())
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (adapter_id, path);
                Err("当前平台不支持 Windows 批处理适配器".to_string())
            }
        }
    }
}

fn validate_launch(resolved: &ResolvedLaunch, project_path: &Path) -> Result<(), String> {
    validate_launch_target(resolved)?;
    if let EditorLaunch::KnownWindowsBatch { path, .. } = &resolved.launch {
        if !batch_argument_is_safe(path) || !batch_argument_is_safe(&project_path.to_string_lossy())
        {
            return Err(
                "该内建批处理适配器无法安全处理项目路径中的 Shell 特殊字符，请选择真实 .exe"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn normalize_args(args: Vec<String>) -> Result<Vec<String>, String> {
    let args: Vec<String> = args
        .into_iter()
        .filter(|argument| !argument.is_empty())
        .collect();
    if args.iter().any(|argument| argument.contains('\0')) {
        return Err("编辑器参数不能包含 NUL 字符".to_string());
    }
    Ok(args)
}

fn launch_path(launch: &EditorLaunch) -> &str {
    match launch {
        EditorLaunch::Executable { path, .. }
        | EditorLaunch::MacApp { path }
        | EditorLaunch::DesktopEntry { path }
        | EditorLaunch::KnownWindowsBatch { path, .. } => path,
    }
}

fn launch_args(launch: &EditorLaunch) -> Vec<String> {
    match launch {
        EditorLaunch::Executable { args, .. } => args.clone(),
        _ => Vec::new(),
    }
}

fn with_launch_args(mut launch: EditorLaunch, args: Vec<String>) -> Result<EditorLaunch, String> {
    match &mut launch {
        EditorLaunch::Executable {
            args: launch_args, ..
        } => *launch_args = args,
        _ if !args.is_empty() => {
            return Err("该编辑器类型不支持固定启动参数".to_string());
        }
        _ => {}
    }
    Ok(launch)
}

fn normalized_target(raw: &str) -> String {
    let path = PathBuf::from(raw);
    let path = path.canonicalize().unwrap_or(path);
    let value = path.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    let value = value.replace('/', "\\").to_lowercase();
    value
}

fn launch_identity(launch: &EditorLaunch) -> LaunchIdentity {
    match launch {
        EditorLaunch::Executable {
            path,
            args,
            working_directory,
        } => LaunchIdentity {
            kind: 0,
            target: normalized_target(path),
            args: args.clone(),
            working_directory: working_directory.as_deref().map(normalized_target),
            adapter_id: None,
        },
        EditorLaunch::MacApp { path } => LaunchIdentity {
            kind: 1,
            target: normalized_target(path),
            args: Vec::new(),
            working_directory: None,
            adapter_id: None,
        },
        EditorLaunch::DesktopEntry { path } => LaunchIdentity {
            kind: 2,
            target: normalized_target(path),
            args: Vec::new(),
            working_directory: None,
            adapter_id: None,
        },
        EditorLaunch::KnownWindowsBatch { adapter_id, path } => LaunchIdentity {
            kind: 3,
            target: normalized_target(path),
            args: Vec::new(),
            working_directory: None,
            adapter_id: Some(adapter_id.clone()),
        },
    }
}

fn infer_custom_launch(raw: &str) -> Result<EditorLaunch, String> {
    let path = absolute_path(raw, "编辑器")?;
    let path_string = path.to_string_lossy().into_owned();

    #[cfg(target_os = "windows")]
    {
        validate_executable(&path_string)?;
        return Ok(EditorLaunch::Executable {
            path: path_string,
            args: Vec::new(),
            working_directory: None,
        });
    }

    #[cfg(target_os = "macos")]
    {
        if path.is_dir()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        {
            return Ok(EditorLaunch::MacApp { path: path_string });
        }
        validate_executable(&path_string)?;
        return Ok(EditorLaunch::Executable {
            path: path_string,
            args: Vec::new(),
            working_directory: None,
        });
    }

    #[cfg(target_os = "linux")]
    {
        if path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
        {
            let launch = EditorLaunch::DesktopEntry { path: path_string };
            validate_launch_target(&ResolvedLaunch {
                launch: launch.clone(),
                origin: LaunchOrigin::Custom,
            })?;
            return Ok(launch);
        }
        validate_executable(&path_string)?;
        return Ok(EditorLaunch::Executable {
            path: path_string,
            args: Vec::new(),
            working_directory: None,
        });
    }

    #[allow(unreachable_code)]
    Err("当前平台不支持自定义编辑器".to_string())
}

fn default_editor_name(launch: &EditorLaunch) -> String {
    Path::new(launch_path(launch))
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Custom Editor")
        .to_string()
}

fn editor_name(
    requested: Option<&str>,
    existing: Option<&str>,
    launch: &EditorLaunch,
) -> Result<String, String> {
    let name = match requested {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        Some(_) => default_editor_name(launch),
        None => existing
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| default_editor_name(launch)),
    };
    if name.contains('\0') {
        return Err("编辑器名称不能包含 NUL 字符".to_string());
    }
    Ok(name)
}

fn ensure_unique_launch(
    editors: &[CustomEditor],
    current_id: Option<&str>,
    launch: &EditorLaunch,
) -> Result<(), String> {
    let identity = launch_identity(launch);
    if let Some(existing) = editors.iter().find(|editor| {
        Some(editor.id.as_str()) != current_id && launch_identity(&editor.launch) == identity
    }) {
        return Err(format!("该编辑器已添加（ID: {}）", existing.id));
    }
    Ok(())
}

fn generate_custom_id(editors: &[CustomEditor]) -> String {
    loop {
        let id = format!("custom-{:016x}", rand::random::<u64>());
        if !editors.iter().any(|editor| editor.id == id) {
            return id;
        }
    }
}

fn upsert_custom_editor_in(
    editors: &mut Vec<CustomEditor>,
    requested_id: Option<String>,
    requested_name: Option<String>,
    requested_launch: Option<EditorLaunch>,
    requested_args: Option<Vec<String>>,
) -> Result<CustomEditor, String> {
    if let Some(id) = requested_id {
        let position = editors
            .iter()
            .position(|editor| editor.id == id)
            .ok_or_else(|| format!("未找到自定义编辑器: {}", id))?;
        let existing = editors[position].clone();
        let path_changed = requested_launch.is_some();
        let launch = requested_launch.unwrap_or_else(|| existing.launch.clone());
        let args = requested_args.unwrap_or_else(|| launch_args(&existing.launch));
        let launch = with_launch_args(launch, args)?;
        ensure_unique_launch(editors, Some(&id), &launch)?;
        let updated = CustomEditor {
            id,
            name: editor_name(requested_name.as_deref(), Some(&existing.name), &launch)?,
            icon_source: if path_changed {
                Some(launch_path(&launch).to_string())
            } else {
                existing
                    .icon_source
                    .or_else(|| Some(launch_path(&launch).to_string()))
            },
            launch,
        };
        editors[position] = updated.clone();
        Ok(updated)
    } else {
        let launch = requested_launch.ok_or_else(|| "请选择编辑器程序".to_string())?;
        let launch = with_launch_args(launch, requested_args.unwrap_or_default())?;
        ensure_unique_launch(editors, None, &launch)?;
        let created = CustomEditor {
            id: generate_custom_id(editors),
            name: editor_name(requested_name.as_deref(), None, &launch)?,
            icon_source: Some(launch_path(&launch).to_string()),
            launch,
        };
        editors.push(created.clone());
        Ok(created)
    }
}

pub fn upsert_custom_editor(request: UpsertCustomEditorRequest) -> Result<CustomEditor, String> {
    let requested_id = request.id.as_deref().map(str::trim).map(str::to_string);
    if requested_id
        .as_deref()
        .is_some_and(detector::is_known_editor_id)
    {
        return Err("内建编辑器不能通过自定义接口修改".to_string());
    }
    let requested_args = request.args.map(normalize_args).transpose()?;
    let requested_launch = request
        .path
        .as_deref()
        .map(infer_custom_launch)
        .transpose()?;
    let requested_name = request.name;

    config::update_custom_editors(move |editors| {
        upsert_custom_editor_in(
            editors,
            requested_id,
            requested_name,
            requested_launch,
            requested_args,
        )
    })
}

fn remove_custom_editor_in(
    editors: &mut Vec<CustomEditor>,
    editor_id: &str,
) -> Result<CustomEditor, String> {
    let position = editors
        .iter()
        .position(|editor| editor.id == editor_id)
        .ok_or_else(|| format!("未找到自定义编辑器: {}", editor_id))?;
    Ok(editors.remove(position))
}

fn import_trusted_editor_in(
    editors: &mut Vec<CustomEditor>,
    name: String,
    launch: EditorLaunch,
    icon_source: Option<String>,
) -> Result<CustomEditor, String> {
    validate_launch_target(&ResolvedLaunch {
        launch: launch.clone(),
        origin: LaunchOrigin::Custom,
    })?;
    let identity = launch_identity(&launch);
    if let Some(existing) = editors
        .iter()
        .find(|editor| launch_identity(&editor.launch) == identity)
    {
        return Ok(existing.clone());
    }
    let created = CustomEditor {
        id: generate_custom_id(editors),
        name: editor_name(Some(&name), None, &launch)?,
        icon_source: icon_source.or_else(|| Some(launch_path(&launch).to_string())),
        launch,
    };
    editors.push(created.clone());
    Ok(created)
}

pub(crate) fn import_trusted_editor(
    name: String,
    launch: EditorLaunch,
    icon_source: Option<String>,
) -> Result<CustomEditor, String> {
    config::update_custom_editors(move |editors| {
        import_trusted_editor_in(editors, name, launch, icon_source)
    })
}

pub fn remove_custom_editor(editor_id: &str) -> Result<CustomEditor, String> {
    let editor_id = editor_id.trim();
    if detector::is_known_editor_id(editor_id) {
        return Err("内建编辑器不能删除".to_string());
    }
    config::update_custom_editors(|editors| remove_custom_editor_in(editors, editor_id))
}

fn custom_editor_is_available(editor: &CustomEditor) -> bool {
    validate_launch_target(&ResolvedLaunch {
        launch: editor.launch.clone(),
        origin: LaunchOrigin::Custom,
    })
    .is_ok()
}

pub fn project_editor_views(
    auto_editors: &EditorCache,
    custom_editors: &[CustomEditor],
) -> HashMap<String, EditorView> {
    let mut views = icons::project_auto_editors(auto_editors);
    for custom in custom_editors {
        if detector::is_known_editor_id(&custom.id) {
            continue;
        }
        let identity = launch_identity(&custom.launch);
        for duplicate in auto_editors.iter().filter_map(|(id, editor)| {
            editor
                .launch
                .as_ref()
                .filter(|launch| launch_identity(launch) == identity)
                .map(|_| id.clone())
        }) {
            views.remove(&duplicate);
        }
        views.insert(
            custom.id.clone(),
            icons::project_custom_editor(custom, custom_editor_is_available(custom)),
        );
    }
    views
}

fn reaper_tx() -> &'static mpsc::Sender<std::process::Child> {
    static TX: OnceLock<mpsc::Sender<std::process::Child>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<std::process::Child>();
        std::thread::spawn(move || {
            for mut child in rx {
                let _ = child.wait();
            }
        });
        tx
    })
}

fn spawn_detached(mut command: Command) -> Result<(), String> {
    let child = command
        .spawn()
        .map_err(|error| format!("启动编辑器失败: {}", error))?;
    reaper_tx()
        .send(child)
        .map_err(|_| "启动编辑器失败: 子进程回收器不可用".to_string())
}

fn executable_command(
    path: &Path,
    args: &[String],
    working_directory: Option<&str>,
    project_path: &Path,
) -> Command {
    let mut command = Command::new(path);
    command.args(args).arg(project_path);
    if let Some(directory) = working_directory {
        command.current_dir(directory);
    }
    command
}

#[cfg(target_os = "windows")]
fn windows_cmd_path() -> Result<PathBuf, String> {
    let system_cmd = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32/cmd.exe"));
    let comspec = std::env::var_os("ComSpec").map(PathBuf::from);
    system_cmd
        .into_iter()
        .chain(comspec)
        .find(|candidate| {
            candidate.is_absolute()
                && candidate.is_file()
                && candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("cmd.exe"))
        })
        .ok_or_else(|| "未找到可信的 Windows cmd.exe".to_string())
}

#[cfg(target_os = "windows")]
fn windows_batch_command(path: &Path, project_path: &Path) -> Result<Command, String> {
    let path = path
        .to_str()
        .ok_or_else(|| "批处理路径包含无效 Unicode".to_string())?;
    let project_path = project_path
        .to_str()
        .ok_or_else(|| "项目路径包含无效 Unicode".to_string())?;
    if !batch_argument_is_safe(path) || !batch_argument_is_safe(project_path) {
        return Err("批处理适配器无法安全处理包含 Shell 特殊字符的路径".to_string());
    }

    let cmd_path = windows_cmd_path()?;
    let cmd_dir = cmd_path
        .parent()
        .ok_or_else(|| "Windows cmd.exe 路径无父目录".to_string())?;
    let mut command = Command::new("cmd.exe");
    command.env("PATH", cmd_dir);
    use std::os::windows::process::CommandExt;
    command
        .args(["/D", "/V:OFF", "/S", "/C"])
        .raw_arg(format!(r#"""{}" "{}"""#, path, project_path));
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
    Ok(command)
}

fn launch(resolved: &ResolvedLaunch, project_path: &Path) -> Result<(), String> {
    match &resolved.launch {
        EditorLaunch::Executable {
            path,
            args,
            working_directory,
        } => spawn_detached(executable_command(
            Path::new(path),
            args,
            working_directory.as_deref(),
            project_path,
        )),
        EditorLaunch::MacApp { path } => {
            #[cfg(target_os = "macos")]
            {
                let status = Command::new("/usr/bin/open")
                    .arg("-a")
                    .arg(path)
                    .arg("--")
                    .arg(project_path)
                    .status()
                    .map_err(|error| format!("启动 macOS 应用失败: {}", error))?;
                status
                    .success()
                    .then_some(())
                    .ok_or_else(|| format!("macOS open 启动失败: {}", status))
            }
            #[cfg(not(target_os = "macos"))]
            Err(format!("当前平台不支持 macOS 应用包: {}", path))
        }
        EditorLaunch::DesktopEntry { path } => {
            #[cfg(target_os = "linux")]
            {
                use gio::prelude::*;
                let app = gio::DesktopAppInfo::from_filename(path)
                    .ok_or_else(|| format!("desktop entry 无效: {}", path))?;
                let project = gio::File::for_path(project_path);
                app.launch(&[project], None::<&gio::AppLaunchContext>)
                    .map_err(|error| format!("启动 desktop entry 失败: {}", error))
            }
            #[cfg(not(target_os = "linux"))]
            Err(format!("当前平台不支持 desktop entry: {}", path))
        }
        EditorLaunch::KnownWindowsBatch { path, .. } => {
            #[cfg(target_os = "windows")]
            {
                spawn_detached(windows_batch_command(Path::new(path), project_path)?)
            }
            #[cfg(not(target_os = "windows"))]
            Err(format!("当前平台不支持 Windows 批处理适配器: {}", path))
        }
    }
}

pub fn open_editor(editor_id: &str, project_path: &str) -> Result<(), String> {
    let project_path = validate_project_path(project_path)?;
    let cache = config::load_editor_cache()?.unwrap_or_default();
    let custom_editors = config::load_custom_editors()?;
    let mut resolved = match select_saved_launch(editor_id, &cache, &custom_editors)? {
        Some(resolved) => resolved,
        None => fresh_known_launch(editor_id)?,
    };

    if let Err(cached_error) = validate_launch(&resolved, &project_path) {
        if matches!(resolved.origin, LaunchOrigin::Known(_)) {
            resolved = fresh_known_launch(editor_id)?;
            validate_launch(&resolved, &project_path).map_err(|fresh_error| {
                format!("{}; 重新解析后仍不可用: {}", cached_error, fresh_error)
            })?;
        } else {
            return Err(cached_error);
        }
    }

    launch(&resolved, &project_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "devfleet-editors-{}-{}-{}",
            std::process::id(),
            rand::random::<u64>(),
            name
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_launch(path: impl Into<String>, args: &[&str]) -> EditorLaunch {
        EditorLaunch::Executable {
            path: path.into(),
            args: args.iter().map(|value| (*value).to_string()).collect(),
            working_directory: None,
        }
    }

    #[test]
    fn custom_editor_crud_preserves_id_and_missing_args() {
        let mut editors = Vec::new();
        let created = upsert_custom_editor_in(
            &mut editors,
            None,
            Some("My Editor".to_string()),
            Some(test_launch("/tools/editor", &[])),
            Some(vec!["--wait".to_string()]),
        )
        .unwrap();
        assert!(created.id.starts_with("custom-"));

        let renamed = upsert_custom_editor_in(
            &mut editors,
            Some(created.id.clone()),
            Some("Renamed".to_string()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.name, "Renamed");
        assert_eq!(launch_args(&renamed.launch), vec!["--wait"]);
        assert_eq!(renamed.icon_source, created.icon_source);

        let cleared = upsert_custom_editor_in(
            &mut editors,
            Some(created.id.clone()),
            None,
            None,
            Some(Vec::new()),
        )
        .unwrap();
        assert!(launch_args(&cleared.launch).is_empty());
        assert_eq!(
            remove_custom_editor_in(&mut editors, &created.id)
                .unwrap()
                .id,
            created.id
        );
        assert!(editors.is_empty());
    }

    #[test]
    fn custom_request_rejects_launch_injection() {
        let result = serde_json::from_value::<UpsertCustomEditorRequest>(serde_json::json!({
            "name": "Injected",
            "path": "/safe/editor",
            "launch": { "kind": "knownWindowsBatch", "path": "bad.cmd" }
        }));
        assert!(result.is_err());
    }

    #[test]
    fn built_in_editor_cannot_be_removed() {
        assert!(remove_custom_editor("vscode")
            .unwrap_err()
            .contains("内建编辑器"));
    }

    #[test]
    fn duplicate_launch_is_rejected_but_same_name_different_target_is_allowed() {
        let mut editors = Vec::new();
        upsert_custom_editor_in(
            &mut editors,
            None,
            Some("Same Name".to_string()),
            Some(test_launch("/tools/editor-a", &[])),
            None,
        )
        .unwrap();
        assert!(upsert_custom_editor_in(
            &mut editors,
            None,
            Some("Other Name".to_string()),
            Some(test_launch("/tools/editor-a", &[])),
            None,
        )
        .is_err());
        assert!(upsert_custom_editor_in(
            &mut editors,
            None,
            Some("Same Name".to_string()),
            Some(test_launch("/tools/editor-b", &[])),
            None,
        )
        .is_ok());
    }

    #[test]
    fn non_executable_launch_rejects_fixed_args() {
        let mut editors = Vec::new();
        let result = upsert_custom_editor_in(
            &mut editors,
            None,
            None,
            Some(EditorLaunch::MacApp {
                path: "/Applications/Editor.app".to_string(),
            }),
            Some(vec!["--wait".to_string()]),
        );
        assert!(result.unwrap_err().contains("不支持固定启动参数"));
    }

    #[test]
    fn trusted_import_preserves_args_working_directory_and_icon_source() {
        let directory = std::env::temp_dir();
        let executable = std::env::current_exe().unwrap();
        let launch = EditorLaunch::Executable {
            path: executable.to_string_lossy().into_owned(),
            args: vec!["--wait".to_string()],
            working_directory: Some(directory.to_string_lossy().into_owned()),
        };
        let mut editors = Vec::new();
        let first = import_trusted_editor_in(
            &mut editors,
            "Imported".to_string(),
            launch.clone(),
            Some("shortcut.lnk".to_string()),
        )
        .unwrap();
        let second =
            import_trusted_editor_in(&mut editors, "Duplicate".to_string(), launch, None).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(editors.len(), 1);
        assert_eq!(first.icon_source.as_deref(), Some("shortcut.lnk"));
        let EditorLaunch::Executable {
            args,
            working_directory,
            ..
        } = first.launch
        else {
            panic!("expected executable")
        };
        assert_eq!(args, vec!["--wait"]);
        assert_eq!(working_directory.as_deref(), directory.to_str());
    }

    #[test]
    fn custom_view_replaces_matching_auto_and_keeps_missing_custom() {
        use crate::models::EditorInfo;

        let executable = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let launch = test_launch(executable, &[]);
        let auto = EditorCache::from([(
            "vscode".to_string(),
            EditorInfo {
                name: "Auto".to_string(),
                installed: true,
                launch: Some(launch.clone()),
                icon_source: None,
            },
        )]);
        let custom = CustomEditor {
            id: "custom-view".to_string(),
            name: "Custom".to_string(),
            launch,
            icon_source: None,
        };
        let missing = CustomEditor {
            id: "custom-missing".to_string(),
            name: "Missing".to_string(),
            launch: test_launch("/definitely/missing/editor.exe", &[]),
            icon_source: None,
        };

        let views = project_editor_views(&auto, &[custom, missing]);
        assert!(!views.contains_key("vscode"));
        assert!(views["custom-view"].installed);
        assert!(!views["custom-missing"].installed);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn custom_windows_scripts_are_rejected() {
        let directory = temp_dir("custom-script");
        let script = directory.join("editor.cmd");
        fs::write(&script, "@echo off\r\n").unwrap();
        assert!(infer_custom_launch(script.to_str().unwrap()).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unknown_editor_id_is_rejected_without_config_io() {
        let error = select_saved_launch("unknown-editor", &EditorCache::new(), &[]).unwrap_err();
        assert!(error.contains("未知编辑器 ID"));
    }

    #[test]
    fn invalid_project_path_is_rejected() {
        let error = validate_project_path("relative/project").unwrap_err();
        assert!(error.contains("绝对路径"));
    }

    #[test]
    fn missing_executable_is_rejected() {
        let missing = temp_dir("missing-executable").join(if cfg!(windows) {
            "missing.exe"
        } else {
            "missing"
        });
        let error = validate_executable(missing.to_str().unwrap()).unwrap_err();
        assert!(error.contains("不存在"));
        fs::remove_dir_all(missing.parent().unwrap()).unwrap();
    }

    #[test]
    fn executable_command_preserves_controlled_argv_boundaries() {
        let project = temp_dir("项目 with spaces & ^ % !");
        let executable = std::env::current_exe().unwrap();
        validate_executable(executable.to_str().unwrap()).unwrap();
        let fixed = vec!["--fixed".to_string(), "值 with spaces & calc".to_string()];
        let command = executable_command(&executable, &fixed, None, &project);
        let actual: Vec<OsString> = command.get_args().map(OsString::from).collect();

        assert_eq!(
            actual,
            vec![
                OsString::from("--fixed"),
                OsString::from("值 with spaces & calc"),
                project.as_os_str().to_os_string(),
            ]
        );
        fs::remove_dir_all(project).unwrap();
    }

    #[test]
    fn batch_safety_rejects_shell_metacharacters() {
        for character in ['%', '!', '&', '|', '<', '>', '^', '"', '\r', '\n'] {
            assert!(!batch_argument_is_safe(&format!(
                "C:\\work\\bad{}path",
                character
            )));
        }
        assert!(batch_argument_is_safe(r"C:\work\safe project"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_batch_command_uses_one_quoted_command_string() {
        let batch = Path::new(r"C:\Program Files\Editor\editor.cmd");
        let project = Path::new(r"D:\work\project with spaces");
        let command = windows_batch_command(batch, project).unwrap();
        let actual: Vec<OsString> = command.get_args().map(OsString::from).collect();

        assert_eq!(
            actual,
            vec![
                OsString::from("/D"),
                OsString::from("/V:OFF"),
                OsString::from("/S"),
                OsString::from("/C"),
                OsString::from(
                    r#"""C:\Program Files\Editor\editor.cmd" "D:\work\project with spaces"""#,
                ),
            ]
        );
    }

    #[cfg(target_os = "windows")]
    fn assert_windows_batch_receives_project(project: &Path) {
        let fixture = temp_dir("batch helper with spaces");
        let batch = fixture.join("capture.cmd");
        let marker = fixture.join("captured.txt");
        fs::write(
            &batch,
            format!(
                "@echo off\r\nif not \"%~1\"==\"{}\" exit /b 7\r\ntype nul > \"{}\"\r\n",
                project.display(),
                marker.display()
            ),
        )
        .unwrap();

        let status = windows_batch_command(&batch, project)
            .unwrap()
            .status()
            .unwrap();
        assert!(status.success());
        assert!(marker.is_file());
        fs::remove_dir_all(fixture).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_batch_command_passes_project_path_as_one_argument() {
        let project = temp_dir("batch project with spaces");
        assert_windows_batch_receives_project(&project);
        fs::remove_dir_all(project).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_batch_command_handles_drive_root() {
        assert_windows_batch_receives_project(Path::new(r"C:\"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_batch_command_handles_unc_root() {
        assert_windows_batch_receives_project(Path::new(r"\\server\share\"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn custom_records_cannot_request_known_batch_privileges() {
        let project = temp_dir("batch-origin");
        let resolved = ResolvedLaunch {
            launch: EditorLaunch::KnownWindowsBatch {
                adapter_id: "idea".to_string(),
                path: r"C:\missing\idea.cmd".to_string(),
            },
            origin: LaunchOrigin::Custom,
        };

        let error = validate_launch(&resolved, &project).unwrap_err();
        assert!(error.contains("仅允许内建编辑器"));
        fs::remove_dir_all(project).unwrap();
    }
}
