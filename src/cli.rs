//! Map CLI args to a `Plan`. Pure: takes the args and the working dir.

use crate::detect;
use crate::plan::Plan;
use std::path::Path;

pub const UV_INSTALL_URL: &str = "https://docs.astral.sh/uv/getting-started/installation/";

pub const USAGE: &str = "\
uva — uv automations（让 Python 拥有 yarn 般的体验）

用法:
  uva                        安装依赖（等同于 uva install）
  uva install                安装当前项目的依赖
  uva run [文件] [参数...]      运行脚本（不传文件时自动寻找入口）
  uva start [文件] [参数...]    等同于 uva run
  uva repl                     启动 Python REPL（使用 uva 全局环境，可 import 全局包）
  uva add <包>... [-g|--save]   安装包；-g 装到全局环境，--save 写入 pyproject.toml/requirements.txt
  uva remove <包>... [-g|--save] 卸载包；-g 从全局环境卸载，--save 同步依赖文件
  uva <文件> [参数...]          若文件存在，等同于 uva run <文件>
  uva cn                       一键全局切清华源（写入全局 uv.toml）
  uva unset-base-url           清除全局 uv.toml 的 [[index]] 设置
  uva how-to-install-uv        输出 uv 的安装地址
  uva --help                   显示本帮助
  uva --version                显示版本

包名可用空格或逗号分隔，例如: uva add requests flask  /  uva add requests,flask
-g 安装的包会进入 uva 全局环境，可被 uva repl 直接 import。
提示: uva 始终使用 uv 当前激活的 Python 版本。要切换版本，请直接使用 uv。";

/// Map args (excluding program name) to a `Plan`, inspecting `dir`.
pub fn dispatch(args: &[String], dir: &Path) -> Plan {
    match args.split_first() {
        None => detect::install_plan(dir),
        Some((first, rest)) => match first.as_str() {
            "install" => detect::install_plan(dir),
            "run" | "start" => {
                let (filename, extra) = split_filename(rest);
                detect::run_plan(dir, filename, extra)
            }
            "add" => {
                let (packages, save, global) = parse_packages(rest);
                detect::add_plan(dir, &packages, save, global)
            }
            "remove" => {
                let (packages, save, global) = parse_packages(rest);
                detect::remove_plan(dir, &packages, save, global)
            }
            "repl" => detect::repl_plan(dir),
            "cn" => Plan::Steps(vec![crate::plan::Step::SetGlobalIndex]),
            "unset-base-url" => Plan::Steps(vec![crate::plan::Step::ClearGlobalIndex]),
            "how-to-install-uv" => Plan::PrintUrl,
            "--help" | "-h" => Plan::Help,
            "--version" | "-V" => Plan::Version,
            _ => {
                if dir.join(first).is_file() {
                    detect::run_plan(dir, Some(first), rest)
                } else {
                    Plan::Usage
                }
            }
        },
    }
}

fn split_filename(rest: &[String]) -> (Option<&str>, &[String]) {
    match rest.split_first() {
        Some((f, extra)) => (Some(f.as_str()), extra),
        None => (None, &[]),
    }
}

/// Split positional args into package names, honoring both space- and
/// comma-separation, and detect the optional `--save` and `-g`/`--global`
/// flags (both default false). Returns `(packages, save, global)`.
fn parse_packages(rest: &[String]) -> (Vec<String>, bool, bool) {
    let mut packages = Vec::new();
    let mut save = false;
    let mut global = false;
    for arg in rest {
        match arg.as_str() {
            "--save" => {
                save = true;
                continue;
            }
            "-g" | "--global" => {
                global = true;
                continue;
            }
            _ => {}
        }
        for part in arg.split(',') {
            let p = part.trim();
            if !p.is_empty() {
                packages.push(p.to_string());
            }
        }
    }
    (packages, save, global)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Step, UvCmd};
    use std::fs;
    use tempfile::tempdir;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn no_args_is_install() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            dispatch(&[], dir.path()),
            Plan::uv(vec![UvCmd::new(["sync"])])
        );
    }

    #[test]
    fn install_keyword() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["install"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["sync"])])
        );
    }

    #[test]
    fn how_to_install_uv() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["how-to-install-uv"]), dir.path()),
            Plan::PrintUrl
        );
    }

    #[test]
    fn run_explicit_file() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["run", "foo.py"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "foo.py"])])
        );
    }

    #[test]
    fn run_forwards_extra_args() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["run", "foo.py", "--port", "8000"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "foo.py", "--port", "8000"])])
        );
    }

    #[test]
    fn start_is_run() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["start", "foo.py"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "foo.py"])])
        );
    }

    #[test]
    fn bare_existing_filename_runs() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("script.py"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["script.py"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "script.py"])])
        );
    }

    #[test]
    fn bare_existing_filename_forwards_args() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("script.py"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["script.py", "arg1"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "script.py", "arg1"])])
        );
    }

    #[test]
    fn bare_missing_filename_is_usage() {
        let dir = tempdir().unwrap();
        assert_eq!(dispatch(&s(&["nope.py"]), dir.path()), Plan::Usage);
    }

    #[test]
    fn help_and_version_flags() {
        let dir = tempdir().unwrap();
        assert_eq!(dispatch(&s(&["--help"]), dir.path()), Plan::Help);
        assert_eq!(dispatch(&s(&["--version"]), dir.path()), Plan::Version);
    }

    #[test]
    fn add_parses_comma_space_and_save() {
        let dir = tempdir().unwrap(); // no pyproject → requirements path
        assert_eq!(
            dispatch(
                &s(&["add", "requests,flask", "numpy", "--save"]),
                dir.path()
            ),
            Plan::Steps(vec![
                Step::Uv(UvCmd::new(["venv"]).only_if_venv_missing()),
                Step::Uv(UvCmd::new(["pip", "install", "requests", "flask", "numpy"])),
                Step::AppendRequirements(s(&["requests", "flask", "numpy"])),
            ])
        );
    }

    #[test]
    fn add_without_save_does_not_edit_files() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["add", "requests"]), dir.path()),
            Plan::Steps(vec![
                Step::Uv(UvCmd::new(["venv"]).only_if_venv_missing()),
                Step::Uv(UvCmd::new(["pip", "install", "requests"])),
            ])
        );
    }

    #[test]
    fn add_save_with_pyproject_uses_uv_add() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["add", "requests", "--save"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["add", "requests"])])
        );
    }

    #[test]
    fn add_no_packages_fails() {
        let dir = tempdir().unwrap();
        assert!(matches!(dispatch(&s(&["add"]), dir.path()), Plan::Fail(_)));
        assert!(matches!(
            dispatch(&s(&["add", "--save"]), dir.path()),
            Plan::Fail(_)
        ));
    }

    #[test]
    fn remove_temporary_uninstalls() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["remove", "flask", "requests"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["pip", "uninstall", "flask", "requests"])])
        );
    }

    #[test]
    fn remove_save_with_pyproject_uses_uv_remove() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["remove", "flask", "--save"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["remove", "flask"])])
        );
    }

    #[test]
    fn cn_sets_global_index() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["cn"]), dir.path()),
            Plan::Steps(vec![Step::SetGlobalIndex])
        );
    }

    #[test]
    fn unset_base_url_clears_global_index() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["unset-base-url"]), dir.path()),
            Plan::Steps(vec![Step::ClearGlobalIndex])
        );
    }

    #[test]
    fn repl_outside_project_uses_global() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["repl"]), dir.path()),
            Plan::Steps(vec![Step::Repl])
        );
    }

    #[test]
    fn repl_in_project_uses_uv_run_python() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(
            dispatch(&s(&["repl"]), dir.path()),
            Plan::uv(vec![UvCmd::new(["run", "python"])])
        );
    }

    #[test]
    fn add_global_flag() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["add", "-g", "requests", "flask"]), dir.path()),
            Plan::Steps(vec![Step::GlobalAdd(s(&["requests", "flask"]))])
        );
        // long form, flag after packages
        assert_eq!(
            dispatch(&s(&["add", "requests", "--global"]), dir.path()),
            Plan::Steps(vec![Step::GlobalAdd(s(&["requests"]))])
        );
    }

    #[test]
    fn remove_global_flag() {
        let dir = tempdir().unwrap();
        assert_eq!(
            dispatch(&s(&["remove", "-g", "requests"]), dir.path()),
            Plan::Steps(vec![Step::GlobalRemove(s(&["requests"]))])
        );
    }
}
