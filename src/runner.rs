//! Side-effecting executor: check `uv` presence, echo, spawn, propagate code,
//! and apply `requirements.txt` edits.

use crate::cli::{UV_INSTALL_URL, USAGE};
use crate::plan::{Plan, Step, UvCmd, VenvGate};
use crate::reqs;
use std::path::Path;
use std::process::{Command, Stdio};

const REQUIREMENTS: &str = "requirements.txt";

/// Execute a plan; return the process exit code uva should terminate with.
pub fn execute(plan: Plan) -> i32 {
    match plan {
        Plan::PrintUrl => {
            println!("{}", UV_INSTALL_URL);
            0
        }
        Plan::Help => {
            println!("{}", USAGE);
            0
        }
        Plan::Version => {
            println!("uva {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Plan::Usage => {
            eprintln!("{}", USAGE);
            2
        }
        Plan::Fail(msg) => {
            eprintln!("uva: {}", msg);
            1
        }
        Plan::Steps(steps) => run_steps(steps),
    }
}

fn run_steps(steps: Vec<Step>) -> i32 {
    let needs_uv = steps.iter().any(|s| matches!(s, Step::Uv(_)));
    if needs_uv && !uv_available() {
        eprintln!("uva: 未找到 uv。请先安装 uv：{}", UV_INSTALL_URL);
        return 1;
    }
    for step in steps {
        let code = match step {
            Step::Uv(cmd) => run_uv(&cmd),
            Step::AppendRequirements(pkgs) => edit_requirements(&pkgs, true),
            Step::RemoveRequirements(pkgs) => edit_requirements(&pkgs, false),
        };
        if code != 0 {
            return code;
        }
    }
    0
}

fn run_uv(cmd: &UvCmd) -> i32 {
    let venv = Path::new(".venv").exists();
    let should_run = match cmd.gate {
        VenvGate::Always => true,
        VenvGate::OnlyIfMissing => !venv,
        VenvGate::OnlyIfPresent => venv,
    };
    if !should_run {
        return 0;
    }
    eprintln!("$ uv {}", cmd.args.join(" "));
    match Command::new("uv").args(&cmd.args).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("uva: 无法执行 uv: {}", e);
            1
        }
    }
}

/// Append or remove package specs in `requirements.txt`. Returns an exit code.
fn edit_requirements(packages: &[String], append: bool) -> i32 {
    let current = std::fs::read_to_string(REQUIREMENTS).unwrap_or_default();
    let updated = if append {
        reqs::append(&current, packages)
    } else {
        reqs::remove(&current, packages)
    };
    if updated == current {
        return 0; // nothing to do — don't create or rewrite the file needlessly
    }
    if let Err(e) = std::fs::write(REQUIREMENTS, &updated) {
        eprintln!("uva: 无法写入 {}: {}", REQUIREMENTS, e);
        return 1;
    }
    let verb = if append { "写入" } else { "更新" };
    eprintln!("# 已{} {}", verb, REQUIREMENTS);
    0
}

/// Whether `uv` is resolvable on PATH.
fn uv_available() -> bool {
    Command::new("uv")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
