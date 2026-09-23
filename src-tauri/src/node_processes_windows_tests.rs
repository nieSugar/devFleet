use super::*;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW: u32 = 0x08000000;

struct TestTree {
    temp_root: PathBuf,
    directory: PathBuf,
    root: Option<Child>,
}

impl Drop for TestTree {
    fn drop(&mut self) {
        if let Some(root) = &mut self.root {
            // Only target the root owned by this test, never a discovered process.
            if matches!(root.try_wait(), Ok(None)) {
                let _ = Command::new("taskkill")
                    .args(["/PID", &root.id().to_string(), "/T", "/F"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
                let _ = root.wait();
            }
        }
        if self.directory.parent() == Some(self.temp_root.as_path())
            && self.directory.canonicalize().ok().as_ref() == Some(&self.directory)
        {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}

#[test]
#[ignore = "requires Node.js; starts only isolated test processes"]
fn kills_package_manager_tree_and_releases_ports_with_unicode_path() {
    let started = Instant::now();
    let temp_root = std::env::temp_dir().canonicalize().unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = temp_root.join(format!(
        "devfleet 中文 smoke {} {nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let mut tree = TestTree {
        temp_root,
        directory,
        root: None,
    };
    let script = tree.directory.join("pnpm.mjs");
    std::fs::write(&script, r#"
import { spawn } from 'node:child_process';
import net from 'node:net';
setTimeout(() => process.exit(1), 45000);
if (process.argv.includes('--server')) {
  const server = net.createServer(socket => socket.end());
  server.listen(0, '127.0.0.1', () => console.log(JSON.stringify([process.pid, server.address().port])));
} else {
  const children = [0, 1].map(() => spawn(process.execPath, [process.argv[1], '--server'], {
    windowsHide: true, stdio: ['ignore', 'inherit', 'inherit']
  }));
  process.on('exit', () => children.forEach(child => child.kill()));
}
"#.as_bytes()).unwrap();
    tree.root = Some(
        Command::new("node")
            .arg(script.to_string_lossy().trim_start_matches(r"\\?\"))
            .args(["run", "dev"])
            .env_remove("NODE_OPTIONS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("Node.js must be on PATH"),
    );
    let root = tree.root.as_mut().unwrap();
    let root_pid = root.id();
    // Both parent and children self-destruct, so a failed startup cannot hold this pipe forever.
    let servers: Vec<(u32, u16)> = BufReader::new(root.stdout.take().unwrap())
        .lines()
        .take(2)
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(servers.len(), 2, "both isolated servers must start");
    let pids = [root_pid, servers[0].0, servers[1].0];
    let processes = platform_list_node_processes().unwrap();
    for pid in pids {
        let process = processes.iter().find(|process| process.pid == pid).unwrap();
        assert!(process
            .command_line
            .as_deref()
            .unwrap()
            .contains("devfleet 中文 smoke "));
        if pid != root_pid {
            assert_eq!(process.parent_pid, Some(root_pid));
        }
    }
    for &(_, port) in &servers {
        assert!(TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(200)
        )
        .is_ok());
    }
    let snapshots = platform_list_process_snapshots().unwrap();
    assert_eq!(
        resolve_node_kill_root_pid(servers[0].0, &snapshots),
        root_pid
    );
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "startup exceeded the fixture's safety budget"
    );
    let target = processes
        .iter()
        .find(|process| process.pid == servers[0].0)
        .unwrap();
    assert!(kill_node_process(
        target.pid,
        Some("stale-process-identity"),
        target.command_line.as_deref(),
        target.executable.as_deref()
    )
    .is_err());
    kill_node_process(
        target.pid,
        target.started_at.as_deref(),
        target.command_line.as_deref(),
        target.executable.as_deref(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = platform_list_node_processes().unwrap();
        if !remaining.iter().any(|process| pids.contains(&process.pid)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the package-manager process tree survived"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(tree.root.as_mut().unwrap().try_wait().unwrap().is_some());
    for &(_, port) in &servers {
        assert!(TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(200)
        )
        .is_err());
    }
    assert!(
        started.elapsed() < Duration::from_secs(40),
        "must finish before self-destruction can hide a regression"
    );
}
