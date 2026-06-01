//! Pure filesystem inspection: decide the install/add/remove plans and resolve
//! the default `run` script. Every function takes the directory explicitly so
//! it can be unit-tested against a temp dir.

use crate::plan::{Plan, Step, UvCmd};
use std::path::Path;

/// Build the install plan by inspecting `dir` for dependency files.
pub fn install_plan(dir: &Path) -> Plan {
    if dir.join("uv.lock").is_file() || dir.join("pyproject.toml").is_file() {
        Plan::uv(vec![UvCmd::new(["sync"])])
    } else if dir.join("requirements.txt").is_file() {
        Plan::uv(vec![
            UvCmd::new(["venv"]).only_if_venv_missing(),
            UvCmd::new(["pip", "install", "-r", "requirements.txt"]),
        ])
    } else {
        Plan::Fail("当前目录不是一个 Python 项目".to_string())
    }
}

/// Build the `run` plan: `uv run <file> [extra...]`. When `filename` is None,
/// resolve the default script; on failure, return a Fail plan listing attempts.
pub fn run_plan(dir: &Path, filename: Option<&str>, extra: &[String]) -> Plan {
    let file = match filename {
        Some(f) => f.to_string(),
        None => match resolve_default_script(dir) {
            Ok(f) => f,
            Err(attempts) => {
                return Plan::Fail(format!("未指定源文件。已尝试过：{}", attempts.join("、")));
            }
        },
    };
    let mut args = vec!["run".to_string(), file];
    args.extend(extra.iter().cloned());
    Plan::uv(vec![UvCmd::new(args)])
}

/// Build the `add` plan: install packages into the environment, optionally
/// persisting them. With `--save` and a `pyproject.toml`, defer to `uv add`;
/// otherwise `uv pip install` into `.venv` (and append to `requirements.txt`
/// when saving).
pub fn add_plan(dir: &Path, packages: &[String], save: bool) -> Plan {
    if packages.is_empty() {
        return Plan::Fail("请指定要添加的包，例如：uva add requests".to_string());
    }

    if save && dir.join("pyproject.toml").is_file() {
        return Plan::uv(vec![UvCmd::new(prepend("add", packages))]);
    }

    let mut steps = vec![
        Step::Uv(UvCmd::new(["venv"]).only_if_venv_missing()),
        Step::Uv(UvCmd::new(prepend2("pip", "install", packages))),
    ];
    if save {
        steps.push(Step::AppendRequirements(packages.to_vec()));
    }
    Plan::Steps(steps)
}

/// Build the `remove` plan: uninstall packages, optionally persisting the
/// removal. With `--save` and a `pyproject.toml`, defer to `uv remove`;
/// otherwise edit `requirements.txt` (when saving) and `uv pip uninstall`.
pub fn remove_plan(dir: &Path, packages: &[String], save: bool) -> Plan {
    if packages.is_empty() {
        return Plan::Fail("请指定要移除的包，例如：uva remove requests".to_string());
    }

    if save && dir.join("pyproject.toml").is_file() {
        return Plan::uv(vec![UvCmd::new(prepend("remove", packages))]);
    }

    let uninstall = UvCmd::new(prepend2("pip", "uninstall", packages));
    if save {
        // Persist the removal first, then uninstall best-effort (only if a
        // `.venv` exists — there is nothing to uninstall otherwise).
        Plan::Steps(vec![
            Step::RemoveRequirements(packages.to_vec()),
            Step::Uv(uninstall.only_if_venv_present()),
        ])
    } else {
        Plan::uv(vec![uninstall])
    }
}

/// `[lead, packages...]`
fn prepend(lead: &str, packages: &[String]) -> Vec<String> {
    let mut args = vec![lead.to_string()];
    args.extend(packages.iter().cloned());
    args
}

/// `[a, b, packages...]`
fn prepend2(a: &str, b: &str, packages: &[String]) -> Vec<String> {
    let mut args = vec![a.to_string(), b.to_string()];
    args.extend(packages.iter().cloned());
    args
}

/// Resolve the default script when no filename is given.
/// Ok(relative path string) or Err(list of attempted locations).
pub fn resolve_default_script(dir: &Path) -> Result<String, Vec<String>> {
    let mut attempts = Vec::new();

    attempts.push("当前目录下唯一的 .py 文件".to_string());
    if let Some(name) = single_py_file(dir) {
        return Ok(name);
    }

    attempts.push("src/ 下唯一的 .py 文件".to_string());
    if let Some(name) = single_py_file(&dir.join("src")) {
        return Ok(format!("src/{}", name));
    }

    attempts.push("main.py".to_string());
    if dir.join("main.py").is_file() {
        return Ok("main.py".to_string());
    }

    attempts.push("src/main.py".to_string());
    if dir.join("src").join("main.py").is_file() {
        return Ok("src/main.py".to_string());
    }

    Err(attempts)
}

/// Return the file name of the sole `*.py` file in `dir`, or None if there are
/// zero or more than one (or the dir can't be read).
fn single_py_file(dir: &Path) -> Option<String> {
    let mut found: Option<String> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("py") {
            if found.is_some() {
                return None;
            }
            found = Some(entry.file_name().to_string_lossy().into_owned());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn pkgs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn install_prefers_uv_lock() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("uv.lock"), "").unwrap();
        assert_eq!(
            install_plan(dir.path()),
            Plan::uv(vec![UvCmd::new(["sync"])])
        );
    }

    #[test]
    fn install_uses_pyproject() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            install_plan(dir.path()),
            Plan::uv(vec![UvCmd::new(["sync"])])
        );
    }

    #[test]
    fn install_requirements_txt() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        assert_eq!(
            install_plan(dir.path()),
            Plan::uv(vec![
                UvCmd::new(["venv"]).only_if_venv_missing(),
                UvCmd::new(["pip", "install", "-r", "requirements.txt"]),
            ])
        );
    }

    #[test]
    fn install_no_files_fails() {
        let dir = tempdir().unwrap();
        match install_plan(dir.path()) {
            Plan::Fail(msg) => assert!(msg.contains("Python")),
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn resolve_single_py_in_cwd() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.py"), "").unwrap();
        assert_eq!(resolve_default_script(dir.path()), Ok("app.py".to_string()));
    }

    #[test]
    fn resolve_multiple_py_falls_through_to_main() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.py"), "").unwrap();
        fs::write(dir.path().join("b.py"), "").unwrap();
        fs::write(dir.path().join("main.py"), "").unwrap();
        assert_eq!(
            resolve_default_script(dir.path()),
            Ok("main.py".to_string())
        );
    }

    #[test]
    fn resolve_single_py_in_src() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/server.py"), "").unwrap();
        assert_eq!(
            resolve_default_script(dir.path()),
            Ok("src/server.py".to_string())
        );
    }

    #[test]
    fn resolve_src_main_py() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/a.py"), "").unwrap();
        fs::write(dir.path().join("src/main.py"), "").unwrap();
        assert_eq!(
            resolve_default_script(dir.path()),
            Ok("src/main.py".to_string())
        );
    }

    #[test]
    fn resolve_nothing_errors_with_four_attempts() {
        let dir = tempdir().unwrap();
        match resolve_default_script(dir.path()) {
            Err(attempts) => assert_eq!(attempts.len(), 4),
            Ok(p) => panic!("expected Err, got {}", p),
        }
    }

    #[test]
    fn run_plan_explicit_file() {
        let dir = tempdir().unwrap();
        assert_eq!(
            run_plan(dir.path(), Some("foo.py"), &[]),
            Plan::uv(vec![UvCmd::new(["run", "foo.py"])])
        );
    }

    #[test]
    fn run_plan_forwards_extra() {
        let dir = tempdir().unwrap();
        let extra = vec!["--port".to_string(), "8000".to_string()];
        assert_eq!(
            run_plan(dir.path(), Some("foo.py"), &extra),
            Plan::uv(vec![UvCmd::new(["run", "foo.py", "--port", "8000"])])
        );
    }

    #[test]
    fn add_no_packages_fails() {
        let dir = tempdir().unwrap();
        assert!(matches!(add_plan(dir.path(), &[], false), Plan::Fail(_)));
        assert!(matches!(add_plan(dir.path(), &[], true), Plan::Fail(_)));
    }

    #[test]
    fn add_temporary_installs_into_venv() {
        let dir = tempdir().unwrap();
        assert_eq!(
            add_plan(dir.path(), &pkgs(&["flask"]), false),
            Plan::Steps(vec![
                Step::Uv(UvCmd::new(["venv"]).only_if_venv_missing()),
                Step::Uv(UvCmd::new(["pip", "install", "flask"])),
            ])
        );
    }

    #[test]
    fn add_save_with_pyproject_uses_uv_add() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            add_plan(dir.path(), &pkgs(&["flask", "requests"]), true),
            Plan::uv(vec![UvCmd::new(["add", "flask", "requests"])])
        );
    }

    #[test]
    fn add_save_without_pyproject_uses_requirements() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        assert_eq!(
            add_plan(dir.path(), &pkgs(&["flask"]), true),
            Plan::Steps(vec![
                Step::Uv(UvCmd::new(["venv"]).only_if_venv_missing()),
                Step::Uv(UvCmd::new(["pip", "install", "flask"])),
                Step::AppendRequirements(pkgs(&["flask"])),
            ])
        );
    }

    #[test]
    fn remove_no_packages_fails() {
        let dir = tempdir().unwrap();
        assert!(matches!(remove_plan(dir.path(), &[], false), Plan::Fail(_)));
    }

    #[test]
    fn remove_temporary_uninstalls() {
        let dir = tempdir().unwrap();
        assert_eq!(
            remove_plan(dir.path(), &pkgs(&["flask"]), false),
            Plan::uv(vec![UvCmd::new(["pip", "uninstall", "flask"])])
        );
    }

    #[test]
    fn remove_save_with_pyproject_uses_uv_remove() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            remove_plan(dir.path(), &pkgs(&["flask"]), true),
            Plan::uv(vec![UvCmd::new(["remove", "flask"])])
        );
    }

    #[test]
    fn remove_save_without_pyproject_edits_requirements() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "").unwrap();
        assert_eq!(
            remove_plan(dir.path(), &pkgs(&["flask"]), true),
            Plan::Steps(vec![
                Step::RemoveRequirements(pkgs(&["flask"])),
                Step::Uv(UvCmd::new(["pip", "uninstall", "flask"]).only_if_venv_present()),
            ])
        );
    }
}
