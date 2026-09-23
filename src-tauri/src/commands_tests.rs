use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

struct TestProjectDir {
    path: PathBuf,
}

impl TestProjectDir {
    fn new(package_json: &str) -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "devfleet-commands-test-{}-{}-{}",
            std::process::id(),
            id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("package.json"), package_json).unwrap();
        Self { path }
    }
}

impl Drop for TestProjectDir {
    fn drop(&mut self) {
        if self.path.parent() == Some(std::env::temp_dir().as_path())
            && self
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("devfleet-commands-test-")
        {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[test]
fn exact_version_accepts_full_versions_and_rejects_ambiguous_input() {
    assert_eq!(exact_node_version("22.1.0"), Some("22.1.0"));
    assert_eq!(exact_node_version("v22.1.0"), Some("22.1.0"));
    for value in ["22", ">=22.0.0", "lts/*", "22.0.0;whoami"] {
        assert_eq!(exact_node_version(value), None, "accepted {value}");
    }
}

#[test]
fn node_output_verification_distinguishes_mismatch_and_garbage() {
    assert!(verify_node_output("22.1.0", "v22.1.0\n").is_ok());
    let mismatch = verify_node_output("22.1.0", "v21.9.0\n").unwrap_err();
    assert_eq!(mismatch.0, "NODE_VERSION_MISMATCH");
    let garbage = verify_node_output("22.1.0", "not node output").unwrap_err();
    assert_eq!(garbage.0, "NODE_VERSION_CHECK_FAILED");
}

#[test]
fn version_guard_contains_exact_runtime_check() {
    let guard = version_guard("22.1.0");
    assert!(guard.contains("process.versions.node !== '22.1.0'"));
}

#[test]
fn checked_prefix_reports_missing_version_without_node_manager() {
    let error = checked_node_prefix("999.0.0", &NodeVersionManager::None, ".").unwrap_err();
    assert_eq!(error.0, "NODE_VERSION_MISSING");
}

#[test]
fn run_script_rejects_unresolved_version_before_terminal_start() {
    let project = TestProjectDir::new(r#"{"scripts":{"dev":"node index.js"}}"#);
    let response = run_script_checked(
        project.path.to_str().unwrap(),
        "dev",
        None,
        Some("lts/*".to_string()),
    );
    assert!(!response.success);
    assert_eq!(response.code.as_deref(), Some("NODE_VERSION_UNRESOLVED"));
}

#[test]
fn run_script_rejects_unavailable_project() {
    let path = std::env::temp_dir().join(format!(
        "devfleet-missing-project-{}",
        NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    let response = run_script_checked(path.to_str().unwrap(), "dev", None, None);
    assert!(!response.success);
    assert_eq!(response.code.as_deref(), Some("PROJECT_UNAVAILABLE"));
}

#[test]
fn node_engine_range_is_preserved_by_project_detection() {
    let project =
        TestProjectDir::new(r#"{"engines":{"node":">=22.0.0"},"scripts":{"dev":"node index.js"}}"#);
    assert_eq!(
        crate::project::get_node_version(project.path.to_str().unwrap()).as_deref(),
        Some(">=22.0.0")
    );
}
