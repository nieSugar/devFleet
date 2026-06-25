use crate::models::{NodeProcessInfo, NodeProcessPort, Project};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Clone, Debug)]
struct RawNodeProcess {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    executable: Option<String>,
    command_line: Option<String>,
    launch_command: Option<String>,
    started_at: Option<String>,
    ports: Vec<NodeProcessPort>,
}

#[derive(Clone, Debug)]
struct RawProcessSnapshot {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    executable: Option<String>,
    command_line: Option<String>,
}

pub fn list_node_processes(projects: &[Project]) -> Result<Vec<NodeProcessInfo>, String> {
    let mut processes = platform_list_node_processes()?;
    let process_snapshots = platform_list_process_snapshots().unwrap_or_default();
    let mut ports_by_pid = platform_list_process_ports().unwrap_or_default();

    for process in &mut processes {
        process.launch_command = infer_launch_command(process, &process_snapshots);
        process.ports = ports_by_pid.remove(&process.pid).unwrap_or_default();
        process.ports.sort_by_key(|port| {
            (
                port.local_port,
                port.protocol.clone(),
                port.local_address.clone(),
            )
        });
    }

    processes.sort_by_key(|p| {
        (
            p.ports.is_empty(),
            p.ports
                .first()
                .map(|port| port.local_port)
                .unwrap_or(u16::MAX),
            p.pid,
        )
    });

    Ok(processes
        .into_iter()
        .map(|process| enrich_process(process, projects))
        .collect())
}

pub fn kill_node_process(pid: u32) -> Result<(), String> {
    if pid == std::process::id() {
        return Err("不能结束 DevFleet 自身进程".to_string());
    }

    if !platform_is_node_process(pid)? {
        return Err(format!("未找到 PID {} 的 Node 进程", pid));
    }

    platform_kill_process(pid)
}

fn enrich_process(process: RawNodeProcess, projects: &[Project]) -> NodeProcessInfo {
    let matched_project = match_project(&process, projects);
    let launch_command = process.launch_command.clone().or_else(|| {
        matched_project.and_then(|project| infer_project_script_command(&process, project))
    });

    NodeProcessInfo {
        pid: process.pid,
        parent_pid: process.parent_pid,
        name: process.name,
        executable: process.executable,
        command_line: process.command_line,
        launch_command,
        started_at: process.started_at,
        ports: process.ports,
        matched_project_id: matched_project.map(|p| p.id.clone()),
        matched_project_name: matched_project.map(|p| p.name.clone()),
        matched_project_path: matched_project.map(|p| p.path.clone()),
    }
}

fn push_port(
    ports_by_pid: &mut HashMap<u32, Vec<NodeProcessPort>>,
    pid: u32,
    port: NodeProcessPort,
) {
    ports_by_pid.entry(pid).or_default().push(port);
}

#[cfg(target_os = "windows")]
fn parse_port_json(stdout: &str) -> Result<HashMap<u32, Vec<NodeProcessPort>>, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(HashMap::new());
    }

    let value: Value =
        serde_json::from_str(trimmed).map_err(|e| format!("解析端口占用失败: {}", e))?;

    let values: Vec<Value> = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        item => vec![item],
    };

    let mut ports_by_pid = HashMap::new();
    for item in values {
        if let Some((pid, port)) = parse_port_value(&item) {
            push_port(&mut ports_by_pid, pid, port);
        }
    }

    Ok(ports_by_pid)
}

#[cfg(target_os = "windows")]
fn parse_port_value(value: &Value) -> Option<(u32, NodeProcessPort)> {
    let pid = value.get("pid")?.as_u64()? as u32;
    let protocol = optional_string(value.get("protocol"))?.to_lowercase();
    let local_address =
        optional_string(value.get("localAddress")).unwrap_or_else(|| "*".to_string());
    let local_port = value
        .get("localPort")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())?;
    let state = optional_string(value.get("state"));

    Some((
        pid,
        NodeProcessPort {
            protocol,
            local_address,
            local_port,
            state,
        },
    ))
}

#[cfg(unix)]
fn parse_lsof_ports(stdout: &str) -> HashMap<u32, Vec<NodeProcessPort>> {
    let mut ports_by_pid = HashMap::new();
    let mut current_pid: Option<u32> = None;
    let mut current_protocol = "tcp".to_string();

    for line in stdout.lines() {
        if line.len() < 2 {
            continue;
        }

        let (kind, value) = line.split_at(1);
        match kind {
            "p" => current_pid = value.parse::<u32>().ok(),
            "P" => current_protocol = value.to_lowercase(),
            "n" => {
                let Some(pid) = current_pid else {
                    continue;
                };
                let Some((address, port)) = parse_lsof_address(value) else {
                    continue;
                };
                push_port(
                    &mut ports_by_pid,
                    pid,
                    NodeProcessPort {
                        protocol: current_protocol.clone(),
                        local_address: address,
                        local_port: port,
                        state: Some("Listen".to_string()),
                    },
                );
            }
            _ => {}
        }
    }

    ports_by_pid
}

#[cfg(unix)]
fn parse_lsof_address(value: &str) -> Option<(String, u16)> {
    let without_state = value
        .split_once(" (")
        .map_or(value, |(address, _state)| address);
    let (address, port_text) = without_state.rsplit_once(':')?;
    let port = port_text.parse::<u16>().ok()?;
    Some((address.trim_matches(['[', ']']).to_string(), port))
}

fn infer_launch_command(
    process: &RawNodeProcess,
    snapshots: &HashMap<u32, RawProcessSnapshot>,
) -> Option<String> {
    if let Some(command) = process
        .command_line
        .as_deref()
        .and_then(parse_package_manager_command)
    {
        return Some(command);
    }

    let mut current_pid = process.parent_pid;
    let mut visited = HashSet::new();

    for _ in 0..10 {
        let pid = current_pid?;
        if !visited.insert(pid) {
            break;
        }

        let snapshot = snapshots.get(&pid)?;
        let command_line = snapshot
            .command_line
            .as_deref()
            .or(snapshot.executable.as_deref())
            .unwrap_or(snapshot.name.as_str());

        if let Some(command) = parse_package_manager_command(command_line) {
            return Some(command);
        }

        current_pid = snapshot.parent_pid;
    }

    None
}

fn infer_project_script_command(process: &RawNodeProcess, project: &Project) -> Option<String> {
    let command_line = process.command_line.as_deref()?.to_lowercase();

    project
        .scripts
        .iter()
        .find(|script| script_matches_command_line(&script.command, &command_line))
        .map(|script| package_manager_run_command(project.package_manager.as_deref(), &script.name))
}

fn script_matches_command_line(script_command: &str, command_line: &str) -> bool {
    let script_tokens = shell_words(script_command);
    let candidates = script_tokens
        .iter()
        .filter_map(|token| script_command_candidate(token));

    candidates.into_iter().any(|candidate| {
        let lower = candidate.to_lowercase();
        command_line.contains(&lower)
            || command_line.contains(&format!("{}.js", lower))
            || command_line.contains(&format!("{}.cmd", lower))
    })
}

fn script_command_candidate(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('-')
        || trimmed.contains('=')
        || matches!(
            trimmed,
            "&&" | "||" | "|" | "cd" | "set" | "export" | "cross-env" | "env-cmd"
        )
    {
        return None;
    }

    let base = command_basename(trimmed);
    (!base.is_empty()).then_some(base)
}

fn package_manager_run_command(package_manager: Option<&str>, script_name: &str) -> String {
    match package_manager.unwrap_or("npm").to_lowercase().as_str() {
        "pnpm" => format!("pnpm {}", script_name),
        "yarn" => format!("yarn {}", script_name),
        "bun" => format!("bun run {}", script_name),
        _ => format!("npm run {}", script_name),
    }
}

fn parse_package_manager_command(command_line: &str) -> Option<String> {
    let tokens = shell_words(command_line);
    for (index, token) in tokens.iter().enumerate() {
        let Some(manager) = package_manager_from_token(token) else {
            continue;
        };

        let args = significant_args(&tokens[index + 1..]);
        if let Some(command) = format_package_manager_command(manager, &args) {
            return Some(command);
        }
    }

    None
}

fn package_manager_from_token(token: &str) -> Option<&'static str> {
    let base = command_basename(token);
    let lower = token.replace('\\', "/").to_lowercase();

    match base.as_str() {
        "npm" | "npm.cmd" | "npm.exe" | "npm-cli.js" => Some("npm"),
        "pnpm" | "pnpm.cmd" | "pnpm.exe" | "pnpm.cjs" => Some("pnpm"),
        "yarn" | "yarn.cmd" | "yarn.exe" | "yarn.js" | "yarnpkg" | "yarnpkg.cmd" => Some("yarn"),
        "bun" | "bun.exe" => Some("bun"),
        _ if lower.contains("/npm/bin/npm-cli.js") => Some("npm"),
        _ if lower.contains("/pnpm/bin/pnpm.cjs") || lower.contains("/pnpm.cjs") => Some("pnpm"),
        _ if lower.contains("/yarn/bin/yarn.js") || lower.contains("/yarn.js") => Some("yarn"),
        _ => None,
    }
}

fn format_package_manager_command(manager: &str, args: &[String]) -> Option<String> {
    if args.is_empty() {
        return None;
    }

    match manager {
        "npm" => format_npm_command(args),
        "pnpm" => format_script_manager_command("pnpm", args),
        "yarn" => format_script_manager_command("yarn", args),
        "bun" => format_bun_command(args),
        _ => None,
    }
}

fn format_npm_command(args: &[String]) -> Option<String> {
    if let Some(index) = find_arg(args, &["run", "run-script"]) {
        let script = args.get(index + 1)?;
        return Some(format!("npm run {}", script));
    }

    if let Some(index) = find_arg(args, &["start", "test", "restart", "stop"]) {
        return Some(format!("npm {}", args[index]));
    }

    if let Some(index) = find_arg(args, &["exec", "x"]) {
        let command = args.get(index + 1)?;
        return Some(format!("npm exec {}", command));
    }

    None
}

fn format_script_manager_command(manager: &str, args: &[String]) -> Option<String> {
    if let Some(index) = find_arg(args, &["run"]) {
        let script = args.get(index + 1)?;
        return Some(format!("{} run {}", manager, script));
    }

    let command = args.first()?;
    if matches!(
        command.as_str(),
        "install" | "add" | "remove" | "update" | "config" | "exec" | "dlx"
    ) {
        return None;
    }

    Some(format!("{} {}", manager, command))
}

fn format_bun_command(args: &[String]) -> Option<String> {
    if let Some(index) = find_arg(args, &["run"]) {
        let script = args.get(index + 1)?;
        return Some(format!("bun run {}", script));
    }

    None
}

fn find_arg(args: &[String], names: &[&str]) -> Option<usize> {
    args.iter()
        .position(|arg| names.iter().any(|name| arg.eq_ignore_ascii_case(name)))
}

fn significant_args(tokens: &[String]) -> Vec<String> {
    let mut args = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = clean_token(&tokens[index]);
        let lower = token.to_lowercase();
        if token.is_empty() {
            index += 1;
            continue;
        }

        if option_takes_value(&lower) {
            index += 2;
            continue;
        }

        if lower.starts_with('-') {
            index += 1;
            continue;
        }

        args.push(token);
        index += 1;
    }

    args
}

fn option_takes_value(option: &str) -> bool {
    matches!(
        option,
        "--prefix" | "--workspace" | "--filter" | "--dir" | "--cwd" | "-c" | "-f"
    )
}

fn command_basename(token: &str) -> String {
    clean_token(token)
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_lowercase()
}

fn clean_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn shell_words(command_line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in command_line.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_npm_cli_run_command() {
        let command = parse_package_manager_command(
            r#""C:\node\node.exe" "C:\node\node_modules\npm\bin\npm-cli.js" run dev"#,
        );

        assert_eq!(command.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn parses_pnpm_filtered_script_command() {
        let command = parse_package_manager_command("pnpm --filter web dev");

        assert_eq!(command.as_deref(), Some("pnpm dev"));
    }

    #[test]
    fn matches_vite_project_script_command() {
        assert!(script_matches_command_line(
            "vite --host 127.0.0.1",
            r#"node e:\repo\node_modules\vite\bin\vite.js --host 127.0.0.1"#,
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_tasklist_csv_image_name() {
        let row = r#""node.exe","1240","Console","1","32,112 K""#;

        assert_eq!(first_csv_field(row).as_deref(), Some("node.exe"));
    }
}

fn match_project<'a>(process: &RawNodeProcess, projects: &'a [Project]) -> Option<&'a Project> {
    let haystack = [
        process.name.as_str(),
        process.executable.as_deref().unwrap_or_default(),
        process.command_line.as_deref().unwrap_or_default(),
    ]
    .join(" ")
    .to_lowercase();
    let slash_haystack = haystack.replace('\\', "/");

    projects.iter().find(|project| {
        let path = project.path.trim();
        if path.is_empty() {
            return false;
        }

        let normalized = path.to_lowercase();
        let slash_normalized = normalized.replace('\\', "/");
        haystack.contains(&normalized) || slash_haystack.contains(&slash_normalized)
    })
}

fn is_node_process_name(name: &str) -> bool {
    let lowered = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim()
        .to_lowercase();

    matches!(
        lowered.as_str(),
        "node" | "node.exe" | "nodejs" | "nodejs.exe"
    )
}

#[cfg(target_os = "windows")]
fn optional_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::Bool(b)) => Some(b.to_string()),
        Some(Value::Object(map)) => map
            .get("DateTime")
            .or_else(|| map.get("value"))
            .and_then(|v| optional_string(Some(v))),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn platform_list_node_processes() -> Result<Vec<RawNodeProcess>, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = r#"
$ErrorActionPreference = 'Stop'
$items = Get-CimInstance Win32_Process -Filter "Name = 'node.exe'" |
  Select-Object `
    @{Name='pid'; Expression={[uint32]$_.ProcessId}},
    @{Name='parentPid'; Expression={[uint32]$_.ParentProcessId}},
    @{Name='name'; Expression={$_.Name}},
    @{Name='executable'; Expression={$_.ExecutablePath}},
    @{Name='commandLine'; Expression={$_.CommandLine}},
    @{Name='startedAt'; Expression={if ($_.CreationDate) { ([datetime]$_.CreationDate).ToString('yyyy-MM-dd HH:mm:ss') } else { $null }}}
if ($null -eq $items) {
  '[]'
} else {
  $items | ConvertTo-Json -Compress -Depth 3
}
"#;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("读取 Node 进程失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取 Node 进程失败", &output));
    }

    parse_windows_process_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "windows")]
fn parse_windows_process_json(stdout: &str) -> Result<Vec<RawNodeProcess>, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value: Value =
        serde_json::from_str(trimmed).map_err(|e| format!("解析 Node 进程列表失败: {}", e))?;

    let values: Vec<Value> = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        item => vec![item],
    };

    Ok(values
        .iter()
        .filter_map(parse_windows_process_value)
        .collect())
}

#[cfg(target_os = "windows")]
fn parse_windows_process_value(value: &Value) -> Option<RawNodeProcess> {
    let pid = value.get("pid")?.as_u64()? as u32;
    let parent_pid = value
        .get("parentPid")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let name = optional_string(value.get("name")).unwrap_or_else(|| "node.exe".to_string());
    let executable = optional_string(value.get("executable"));
    let command_line = optional_string(value.get("commandLine"));
    let started_at = optional_string(value.get("startedAt"));

    (is_node_process_name(&name) || executable.as_deref().is_some_and(is_node_process_name))
        .then_some(RawNodeProcess {
            pid,
            parent_pid,
            name,
            executable,
            command_line,
            launch_command: None,
            started_at,
            ports: Vec::new(),
        })
}

#[cfg(target_os = "windows")]
fn platform_is_node_process(pid: u32) -> Result<bool, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let filter = format!("PID eq {}", pid);
    let output = Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("读取进程信息失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取进程信息失败", &output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| {
        first_csv_field(line)
            .as_deref()
            .is_some_and(is_node_process_name)
    }))
}

#[cfg(target_os = "windows")]
fn first_csv_field(row: &str) -> Option<String> {
    let row = row.trim();
    if row.is_empty() {
        return None;
    }

    let Some(rest) = row.strip_prefix('"') else {
        return row
            .split(',')
            .next()
            .map(|field| field.trim().to_string())
            .filter(|field| !field.is_empty());
    };

    let mut field = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if chars.peek().copied() == Some('"') {
                field.push('"');
                chars.next();
            } else {
                return Some(field);
            }
        } else {
            field.push(ch);
        }
    }

    (!field.is_empty()).then_some(field)
}

#[cfg(target_os = "windows")]
fn platform_list_process_snapshots() -> Result<HashMap<u32, RawProcessSnapshot>, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = r#"
$ErrorActionPreference = 'Stop'
$items = Get-CimInstance Win32_Process |
  Select-Object `
    @{Name='pid'; Expression={[uint32]$_.ProcessId}},
    @{Name='parentPid'; Expression={[uint32]$_.ParentProcessId}},
    @{Name='name'; Expression={$_.Name}},
    @{Name='executable'; Expression={$_.ExecutablePath}},
    @{Name='commandLine'; Expression={$_.CommandLine}}
if ($null -eq $items) {
  '[]'
} else {
  $items | ConvertTo-Json -Compress -Depth 3
}
"#;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("读取进程父链失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取进程父链失败", &output));
    }

    parse_windows_snapshot_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "windows")]
fn parse_windows_snapshot_json(stdout: &str) -> Result<HashMap<u32, RawProcessSnapshot>, String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(HashMap::new());
    }

    let value: Value =
        serde_json::from_str(trimmed).map_err(|e| format!("解析进程父链失败: {}", e))?;

    let values: Vec<Value> = match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        item => vec![item],
    };

    Ok(values
        .iter()
        .filter_map(parse_windows_snapshot_value)
        .map(|snapshot| (snapshot.pid, snapshot))
        .collect())
}

#[cfg(target_os = "windows")]
fn parse_windows_snapshot_value(value: &Value) -> Option<RawProcessSnapshot> {
    let pid = value.get("pid")?.as_u64()? as u32;
    let parent_pid = value
        .get("parentPid")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let name = optional_string(value.get("name")).unwrap_or_default();
    let executable = optional_string(value.get("executable"));
    let command_line = optional_string(value.get("commandLine"));

    Some(RawProcessSnapshot {
        pid,
        parent_pid,
        name,
        executable,
        command_line,
    })
}

#[cfg(target_os = "windows")]
fn platform_list_process_ports() -> Result<HashMap<u32, Vec<NodeProcessPort>>, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = r#"
$ErrorActionPreference = 'Stop'
$tcp = @()
$udp = @()
try {
  $tcp = Get-NetTCPConnection -State Listen -ErrorAction Stop |
    Select-Object `
      @{Name='pid'; Expression={[uint32]$_.OwningProcess}},
      @{Name='protocol'; Expression={'tcp'}},
      @{Name='localAddress'; Expression={$_.LocalAddress}},
      @{Name='localPort'; Expression={[uint16]$_.LocalPort}},
      @{Name='state'; Expression={$_.State.ToString()}}
} catch {}
try {
  $udp = Get-NetUDPEndpoint -ErrorAction Stop |
    Select-Object `
      @{Name='pid'; Expression={[uint32]$_.OwningProcess}},
      @{Name='protocol'; Expression={'udp'}},
      @{Name='localAddress'; Expression={$_.LocalAddress}},
      @{Name='localPort'; Expression={[uint16]$_.LocalPort}},
      @{Name='state'; Expression={'Listen'}}
} catch {}
$items = @($tcp) + @($udp)
if ($items.Count -eq 0) {
  '[]'
} else {
  $items | ConvertTo-Json -Compress -Depth 3
}
"#;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("读取端口占用失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取端口占用失败", &output));
    }

    parse_port_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "windows")]
fn platform_kill_process(pid: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let pid_text = pid.to_string();
    let output = Command::new("taskkill")
        .args(["/PID", &pid_text, "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("结束 Node 进程失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("结束 Node 进程失败", &output))
    }
}

#[cfg(target_os = "macos")]
fn platform_list_node_processes() -> Result<Vec<RawNodeProcess>, String> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,comm=,lstart=,command="])
        .output()
        .map_err(|e| format!("读取 Node 进程失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取 Node 进程失败", &output));
    }

    Ok(parse_ps_output(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_list_node_processes() -> Result<Vec<RawNodeProcess>, String> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,ppid=,comm=,lstart=,args="])
        .output()
        .map_err(|e| format!("读取 Node 进程失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取 Node 进程失败", &output));
    }

    Ok(parse_ps_output(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(unix)]
fn parse_ps_output(stdout: &str) -> Vec<RawNodeProcess> {
    stdout.lines().filter_map(parse_ps_line).collect()
}

#[cfg(unix)]
fn parse_ps_line(line: &str) -> Option<RawNodeProcess> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 8 {
        return None;
    }

    let pid = parts[0].parse::<u32>().ok()?;
    let parent_pid = parts[1].parse::<u32>().ok();
    let name = parts[2].to_string();
    let started_at = Some(parts[3..8].join(" "));
    let command_line = (parts.len() > 8).then(|| parts[8..].join(" "));

    if !is_node_process_name(&name) {
        return None;
    }

    Some(RawNodeProcess {
        pid,
        parent_pid,
        executable: Some(name.clone()),
        name,
        command_line,
        launch_command: None,
        started_at,
        ports: Vec::new(),
    })
}

#[cfg(unix)]
fn platform_is_node_process(pid: u32) -> Result<bool, String> {
    let pid_text = pid.to_string();
    let output = Command::new("ps")
        .args(["-p", &pid_text, "-o", "comm="])
        .output()
        .map_err(|e| format!("读取进程信息失败: {}", e))?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(is_node_process_name))
}

#[cfg(target_os = "macos")]
fn platform_list_process_snapshots() -> Result<HashMap<u32, RawProcessSnapshot>, String> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,comm=,args="])
        .output()
        .map_err(|e| format!("读取进程父链失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取进程父链失败", &output));
    }

    Ok(parse_ps_snapshot_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_list_process_snapshots() -> Result<HashMap<u32, RawProcessSnapshot>, String> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,ppid=,comm=,args="])
        .output()
        .map_err(|e| format!("读取进程父链失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取进程父链失败", &output));
    }

    Ok(parse_ps_snapshot_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(unix)]
fn parse_ps_snapshot_output(stdout: &str) -> HashMap<u32, RawProcessSnapshot> {
    stdout
        .lines()
        .filter_map(parse_ps_snapshot_line)
        .map(|snapshot| (snapshot.pid, snapshot))
        .collect()
}

#[cfg(unix)]
fn parse_ps_snapshot_line(line: &str) -> Option<RawProcessSnapshot> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let pid = parts[0].parse::<u32>().ok()?;
    let parent_pid = parts[1].parse::<u32>().ok();
    let name = parts[2].to_string();
    let command_line = (parts.len() > 3).then(|| parts[3..].join(" "));

    Some(RawProcessSnapshot {
        pid,
        parent_pid,
        executable: Some(name.clone()),
        name,
        command_line,
    })
}

#[cfg(unix)]
fn platform_list_process_ports() -> Result<HashMap<u32, Vec<NodeProcessPort>>, String> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "Pn"])
        .output()
        .map_err(|e| format!("读取端口占用失败: {}", e))?;

    if !output.status.success() {
        return Err(command_error("读取端口占用失败", &output));
    }

    Ok(parse_lsof_ports(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(unix)]
fn platform_kill_process(pid: u32) -> Result<(), String> {
    let pid_text = pid.to_string();
    let output = Command::new("kill")
        .args(["-TERM", &pid_text])
        .output()
        .map_err(|e| format!("结束 Node 进程失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("结束 Node 进程失败", &output))
    }
}

fn command_error(prefix: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };

    if detail.is_empty() {
        prefix.to_string()
    } else {
        format!("{}: {}", prefix, detail)
    }
}
