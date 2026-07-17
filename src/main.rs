#![forbid(unsafe_code)]

use std::{env, path::PathBuf, process::ExitCode};

use agent_manager::{
    AppPaths,
    validator::{self, Severity},
};

fn find_root() -> anyhow::Result<PathBuf> {
    let current = env::current_dir()?;
    if current.join("agents").is_dir() {
        return Ok(current);
    }
    let executable = env::current_exe()?;
    for ancestor in executable.ancestors().skip(1) {
        if ancestor.join("agents").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }
    anyhow::bail!("找不到 agents/；請從 Agent Manager 專案根目錄啟動")
}

fn main() -> ExitCode {
    let paths = match find_root().map(AppPaths::from_root) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("{error:#}");
            return ExitCode::FAILURE;
        }
    };
    if env::args().any(|argument| argument == "--check") {
        let loaded = match agent_manager::storage::list_skills_checked(&paths, None) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("Agent Manager 2.0 check failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        let issues: Vec<_> = loaded.skills.iter().flat_map(validator::validate).collect();
        let count = |severity| {
            issues
                .iter()
                .filter(|issue| issue.severity == severity)
                .count()
        };
        let critical = count(Severity::Critical);
        let high = count(Severity::High);
        let medium = count(Severity::Medium);
        let low = count(Severity::Low);
        for diagnostic in &loaded.diagnostics {
            eprintln!(
                "LOAD ERROR {}: {}",
                diagnostic.path.display(),
                diagnostic.message
            );
        }
        println!(
            "Agent Manager 2.0 check: {} skills; load_errors={}; CRITICAL={critical}; HIGH={high}; MEDIUM={medium}; LOW={low}",
            loaded.skills.len(),
            loaded.diagnostics.len(),
        );
        println!("Policy: load errors or any CRITICAL/HIGH validation issue => exit 1");
        return if loaded.diagnostics.is_empty() && critical == 0 && high == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    match agent_manager::app::run(paths) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("GUI 啟動失敗：{error}");
            ExitCode::FAILURE
        }
    }
}
