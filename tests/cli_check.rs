use std::{fs, process::Command};

#[test]
fn check_returns_nonzero_and_reports_invalid_skill_yaml() {
    let root = tempfile::tempdir().expect("temp repo");
    let skill = root.path().join("agents/01-Test/broken/SKILL.md");
    fs::create_dir_all(skill.parent().expect("skill parent")).expect("create skill parent");
    fs::write(&skill, "---\n: invalid yaml\n---\nbody\n").expect("write invalid skill");

    let output = Command::new(env!("CARGO_BIN_EXE_agent-manager"))
        .arg("--check")
        .current_dir(root.path())
        .output()
        .expect("run --check");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("LOAD ERROR"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("load_errors=1"), "{stdout}");
    assert!(stdout.contains("Policy:"), "{stdout}");
}
