use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "blink-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn config(&self, text: &str) -> PathBuf {
        let path = self.0.join("config.toml");
        std::fs::write(&path, text).unwrap();
        path
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_blink"))
            .env("XDG_CONFIG_HOME", &self.0)
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn redirected_output_is_plain_even_with_explicit_graphics() {
    let f = Fixture::new();
    let path =
        f.config("[layout.first]\ntype='text'\ncontent='Hello'\n[layout.second]\nchildren=[]");
    for backend in ["kitty", "iterm2", "sixel", "ansi", "text"] {
        let result = f.run(&["--config", path.to_str().unwrap(), "--graphics", backend]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!result.stdout.contains(&0x1b));
        assert!(String::from_utf8_lossy(&result.stdout).contains("Hello"));
    }
}
#[test]
fn first_launch_creates_compact_config_and_preserves_it() {
    let f = Fixture::new();
    let path = f.0.join("new.toml");
    assert!(
        f.run(&["--config", path.to_str().unwrap(), "--plain"])
            .status
            .success()
    );
    let original = std::fs::read(&path).unwrap();
    let generated = String::from_utf8_lossy(&original);
    assert!(generated.contains("gruvbox-material"));
    assert!(generated.lines().count() < 35);
    assert!(generated.contains(r#"background = "transparent""#));
    assert!(
        f.run(&[
            "--config",
            path.to_str().unwrap(),
            "--theme",
            "nord",
            "--layout",
            "vertical",
            "--no-image",
            "--no-icons",
            "--no-color"
        ])
        .status
        .success()
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
}
#[test]
fn inspection_flags_do_not_create_config() {
    let f = Fixture::new();
    for args in [
        ["--list-themes"],
        ["--list-modules"],
        ["--print-config"],
        ["--check-config"],
    ] {
        let result = f.run(&args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(!f.0.join("blink/config.toml").exists());
}
#[test]
fn print_config_applies_overrides_without_writing() {
    let f = Fixture::new();
    let path = f.config("");
    let result = f.run(&[
        "--config",
        path.to_str().unwrap(),
        "--print-config",
        "--theme",
        "nord",
        "--layout",
        "vertical",
        "--no-icons",
        "--no-image",
    ]);
    assert!(result.status.success());
    let cfg: toml::Value = toml::from_slice(&result.stdout).unwrap();
    assert_eq!(cfg["theme"].as_str(), Some("nord"));
    assert_eq!(cfg["layout"]["direction"].as_str(), Some("vertical"));
    assert_eq!(cfg["icons"]["enabled"].as_bool(), Some(false));
    assert_eq!(cfg["layout"]["first"]["visible"].as_bool(), Some(false));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "");
}
#[test]
fn check_config_reports_bad_colors_and_assets_without_overwriting() {
    let f = Fixture::new();
    for text in [
        "[styles.label]\nforeground='#oops'",
        "[layout.first]\npath='missing.png'",
        "version=99",
    ] {
        let path = f.config(text);
        let result = f.run(&["--config", path.to_str().unwrap(), "--check-config"]);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }
}
#[test]
fn optional_missing_images_do_not_break_normal_rendering() {
    let f = Fixture::new();
    let path = f.config("[layout.first]\npath='missing.png'");
    let result = f.run(&["--config", path.to_str().unwrap(), "--plain"]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
}
#[test]
fn documented_examples_validate() {
    let f = Fixture::new();
    for name in ["default", "vertical", "minimal", "compact", "dashboard"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("examples/{name}.toml"));
        let result = f.run(&["--config", path.to_str().unwrap(), "--check-config"]);
        assert!(
            result.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
#[test]
fn help_version_and_invalid_flags_have_expected_status() {
    let f = Fixture::new();
    assert!(f.run(&["--help"]).status.success());
    assert!(f.run(&["--version"]).status.success());
    assert!(!f.run(&["--nonexistent-option"]).status.success());
    assert!(
        !f.run(&[
            "--check-config",
            "--config",
            "/definitely/missing/blink.toml"
        ])
        .status
        .success()
    );
}
#[cfg(unix)]
#[test]
fn closed_output_pipe_exits_cleanly() {
    use std::process::Stdio;
    let f = Fixture::new();
    let path = f.config("");
    let mut child = Command::new(env!("CARGO_BIN_EXE_blink"))
        .args(["--config", path.to_str().unwrap(), "--print-config"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    assert!(child.wait_with_output().unwrap().status.success());
}
