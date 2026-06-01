use std::process::Command;

fn uva() -> Command {
    Command::new(env!("CARGO_BIN_EXE_uva"))
}

#[test]
fn how_to_install_uv_prints_url() {
    let out = uva().arg("how-to-install-uv").output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "https://docs.astral.sh/uv/getting-started/installation/"
    );
}

#[test]
fn install_outside_python_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    let out = uva()
        .arg("install")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Python"));
}

#[test]
fn bare_unknown_arg_shows_usage_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let out = uva()
        .arg("definitely-not-a-file")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("用法"));
}

#[test]
fn add_without_packages_fails() {
    let dir = tempfile::tempdir().unwrap();
    let out = uva().arg("add").current_dir(dir.path()).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("包"));
}

#[test]
fn remove_without_packages_fails() {
    let dir = tempfile::tempdir().unwrap();
    let out = uva()
        .arg("remove")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("包"));
}

#[test]
fn cn_and_unset_edit_global_uv_toml() {
    // Redirect the config dir via env so we never touch the real ~/.config or
    // %APPDATA%. The binary resolves APPDATA (Windows) or XDG_CONFIG_HOME/HOME.
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path();
    let toml = cfg.join("uv").join("uv.toml");

    let cn = uva()
        .arg("cn")
        .env_remove("UV_CONFIG_FILE")
        .env("APPDATA", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .env("HOME", cfg)
        .output()
        .unwrap();
    assert!(cn.status.success());
    let written = std::fs::read_to_string(&toml).unwrap();
    assert!(written.contains("https://pypi.tuna.tsinghua.edu.cn/simple"));
    assert!(written.contains("default = true"));

    let unset = uva()
        .arg("unset-base-url")
        .env_remove("UV_CONFIG_FILE")
        .env("APPDATA", cfg)
        .env("XDG_CONFIG_HOME", cfg)
        .env("HOME", cfg)
        .output()
        .unwrap();
    assert!(unset.status.success());
    let after = std::fs::read_to_string(&toml).unwrap();
    assert!(!after.contains("[[index]]"));
    assert!(!after.contains("tsinghua"));
}

/// Real end-to-end run through uv. Ignored by default because it may make uv
/// download a managed Python the first time. Run with `cargo test -- --ignored`.
#[test]
#[ignore]
fn run_executes_script_via_uv() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.py"), "print('hello from uva')").unwrap();
    let out = uva()
        .args(["run", "hello.py"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("hello from uva"));
}

/// Real `add --save` then `remove --save` round-trip through uv against a
/// requirements.txt project. Ignored by default (network/uv). Run with
/// `cargo test -- --ignored`.
#[test]
#[ignore]
fn add_then_remove_save_roundtrip_requirements() {
    let dir = tempfile::tempdir().unwrap();
    let req = dir.path().join("requirements.txt");
    std::fs::write(&req, "").unwrap();

    let add = uva()
        .args(["add", "six", "--save"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(add.status.success());
    assert!(std::fs::read_to_string(&req).unwrap().contains("six"));

    let remove = uva()
        .args(["remove", "six", "--save"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(remove.status.success());
    assert!(!std::fs::read_to_string(&req).unwrap().contains("six"));
}
