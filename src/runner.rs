//! Side-effecting executor: check `uv` presence, echo, spawn, propagate code,
//! and apply `requirements.txt` edits.

use crate::cli::{USAGE, UV_INSTALL_URL};
use crate::config;
use crate::plan::{Plan, Step, UvCmd, VenvGate};
use crate::reqs;
use std::path::{Path, PathBuf};
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
    let needs_uv = steps.iter().any(step_needs_uv);
    if needs_uv && !uv_available() {
        eprintln!("uva: 未找到 uv。请先安装 uv：{}", UV_INSTALL_URL);
        return 1;
    }
    for step in steps {
        let code = match step {
            Step::Uv(cmd) => run_uv(&cmd),
            Step::AppendRequirements(pkgs) => edit_requirements(&pkgs, true),
            Step::RemoveRequirements(pkgs) => edit_requirements(&pkgs, false),
            Step::SetGlobalIndex => set_global_index(),
            Step::ClearGlobalIndex => clear_global_index(),
            Step::GlobalAdd(pkgs) => global_pip(&pkgs, true),
            Step::GlobalRemove(pkgs) => global_pip(&pkgs, false),
            Step::Repl => repl(),
        };
        if code != 0 {
            return code;
        }
    }
    0
}

/// Whether a step shells out to `uv` (and therefore needs it on PATH).
/// `Repl` is excluded: it only needs `uv` when it has to create the global
/// venv, which it checks for itself.
fn step_needs_uv(step: &Step) -> bool {
    matches!(
        step,
        Step::Uv(_) | Step::GlobalAdd(_) | Step::GlobalRemove(_)
    )
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

/// Write the Tsinghua index into the global `uv.toml`.
fn set_global_index() -> i32 {
    let path = match global_uv_toml() {
        Some(p) => p,
        None => {
            eprintln!("uva: 无法确定全局 uv.toml 的位置（缺少 APPDATA / HOME / XDG_CONFIG_HOME）");
            return 1;
        }
    };
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = config::set_tsinghua_index(&current);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("uva: 无法创建目录 {}: {}", parent.display(), e);
            return 1;
        }
    }
    if let Err(e) = std::fs::write(&path, &updated) {
        eprintln!("uva: 无法写入 {}: {}", path.display(), e);
        return 1;
    }
    eprintln!("# 已将全局 uv 源切换为清华镜像 → {}", path.display());
    eprintln!("  {}", config::TSINGHUA_URL);
    0
}

/// Remove `[[index]]` sections from the global `uv.toml`.
fn clear_global_index() -> i32 {
    let path = match global_uv_toml() {
        Some(p) => p,
        None => {
            eprintln!("uva: 无法确定全局 uv.toml 的位置（缺少 APPDATA / HOME / XDG_CONFIG_HOME）");
            return 1;
        }
    };
    let current = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("# 全局 uv.toml 不存在，无需清除 → {}", path.display());
            return 0;
        }
    };
    let updated = config::strip_index_sections(&current);
    if updated == current {
        eprintln!("# 全局 uv.toml 中没有 [[index]] 设置，无需清除");
        return 0;
    }
    if let Err(e) = std::fs::write(&path, &updated) {
        eprintln!("uva: 无法写入 {}: {}", path.display(), e);
        return 1;
    }
    eprintln!(
        "# 已清除全局 uv.toml 的 [[index]] 设置 → {}",
        path.display()
    );
    0
}

/// Locate uv's user-level config file, matching uv's own discovery:
///
/// 1. `UV_CONFIG_FILE`, if set — uv reads it in preference to the user config,
///    so that is where our edit must land to take effect.
/// 2. `%APPDATA%\uv\uv.toml` on Windows.
/// 3. `$XDG_CONFIG_HOME/uv/uv.toml`, else `$HOME/.config/uv/uv.toml`, on macOS
///    and Linux (uv uses the same XDG path on both — no macOS special-casing).
fn global_uv_toml() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("UV_CONFIG_FILE") {
        if !explicit.is_empty() {
            return Some(PathBuf::from(explicit));
        }
    }
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("uv").join("uv.toml"))
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => Some(PathBuf::from(x).join("uv").join("uv.toml")),
            _ => std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config").join("uv").join("uv.toml")),
        }
    }
}

// --- Global environment (`uva add -g`, `uva remove -g`, `uva repl`) -------
//
// uva keeps one shared "global" venv. `-g` installs into it, and `uva repl`
// launches its Python, so packages added with `-g` are importable in the REPL.

/// Install or uninstall packages in the global venv via `uv pip`.
fn global_pip(packages: &[String], install: bool) -> i32 {
    let dir = match global_venv_dir() {
        Some(d) => d,
        None => {
            eprintln!("uva: 无法确定全局环境位置（缺少 LOCALAPPDATA / HOME / XDG_DATA_HOME）");
            return 1;
        }
    };
    if install {
        if let Err(code) = ensure_global_venv(&dir) {
            return code;
        }
    } else if !dir.join("pyvenv.cfg").is_file() {
        // Nothing installed globally yet — nothing to remove.
        eprintln!("# 全局环境不存在，无需卸载。");
        return 0;
    }

    let sub = if install { "install" } else { "uninstall" };
    let mut args = vec![
        "pip".to_string(),
        sub.to_string(),
        "--python".to_string(),
        dir.to_string_lossy().into_owned(),
    ];
    args.extend(packages.iter().cloned());
    eprintln!("$ uv {}", args.join(" "));
    match Command::new("uv").args(&args).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("uva: 无法执行 uv: {}", e);
            1
        }
    }
}

/// Launch an interactive Python REPL outside a project: use the local `.venv`
/// if one exists (e.g. a requirements.txt project), else uva's global venv.
/// (Inside a pyproject/uv.lock project, dispatch routes to `uv run python`
/// instead, so this is only reached outside such projects.)
fn repl() -> i32 {
    let local = Path::new(".venv");
    let dir = if local.is_dir() {
        local.to_path_buf()
    } else {
        if !uv_available() {
            eprintln!("uva: 未找到 uv。请先安装 uv：{}", UV_INSTALL_URL);
            return 1;
        }
        let g = match global_venv_dir() {
            Some(d) => d,
            None => {
                eprintln!("uva: 无法确定全局环境位置（缺少 LOCALAPPDATA / HOME / XDG_DATA_HOME）");
                return 1;
            }
        };
        if let Err(code) = ensure_global_venv(&g) {
            return code;
        }
        g
    };
    let py = match venv_python(&dir) {
        Some(p) => p,
        None => {
            eprintln!("uva: 未在 {} 找到 python 解释器", dir.display());
            return 1;
        }
    };
    eprintln!("$ {}", py.display());
    match Command::new(&py).status() {
        Ok(status) => status.code().unwrap_or(0),
        Err(e) => {
            eprintln!("uva: 无法启动 Python: {}", e);
            1
        }
    }
}

/// Create the global venv with `uv venv <dir>` if it doesn't exist yet.
fn ensure_global_venv(dir: &Path) -> Result<(), i32> {
    if dir.join("pyvenv.cfg").is_file() {
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    eprintln!("$ uv venv {}", dir.display());
    match Command::new("uv").arg("venv").arg(dir).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("uva: 无法执行 uv: {}", e);
            Err(1)
        }
    }
}

/// Path to uva's global venv: `%LOCALAPPDATA%\uva\venv` on Windows, else
/// `$XDG_DATA_HOME/uva/venv` (falling back to `$HOME/.local/share/uva/venv`).
fn global_venv_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("uva").join("venv"))
    } else {
        match std::env::var_os("XDG_DATA_HOME") {
            Some(x) if !x.is_empty() => Some(PathBuf::from(x).join("uva").join("venv")),
            _ => std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".local")
                    .join("share")
                    .join("uva")
                    .join("venv")
            }),
        }
    }
}

/// Locate a venv's Python interpreter (works for any venv directory).
fn venv_python(dir: &Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![dir.join("Scripts").join("python.exe")]
    } else {
        vec![
            dir.join("bin").join("python3"),
            dir.join("bin").join("python"),
        ]
    };
    candidates.into_iter().find(|p| p.is_file())
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
