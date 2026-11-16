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
    assert!(generated.contains("responsive = true"));
    for name in [
        "host",
        "packages",
        "swap",
        "local_ip",
        "power_adapter",
        "locale",
        "wm_theme",
        "desktop_theme",
        "icon_theme",
        "font",
        "cursor",
        "terminal_font",
        "title",
        "separator",
        "break",
        "colors",
    ] {
        assert!(
            generated.contains(&format!("\"{name}\"")),
            "missing default: {name}"
        );
    }
    assert!(generated.lines().count() < 50);
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
        ["--list-images"],
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
    for name in [
        "default",
        "vertical",
        "minimal",
        "compact",
        "dashboard",
        "ascii",
    ] {
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

#[test]
fn aliases_and_section_edges_export_without_changing_the_source() {
    let f = Fixture::new();
    let text = r#"[layout.second]
header = ["Title", "Separator"]
footer = ["Break", "Colors"]
modules = [{title="Session", components=["DE", "WM", "WMTheme", "Theme", "Icons", "TerminalFont", "LocalIp", "PowerAdapter"]}]
"#;
    let path = f.config(text);
    let result = f.run(&["--config", path.to_str().unwrap(), "--print-config"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let exported = String::from_utf8(result.stdout).unwrap();
    for name in [
        "desktop",
        "window_manager",
        "wm_theme",
        "desktop_theme",
        "icon_theme",
        "terminal_font",
        "local_ip",
        "power_adapter",
    ] {
        assert!(exported.contains(&format!("\"{name}\"")));
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    let normalized = f.0.join("normalized.toml");
    std::fs::write(&normalized, exported).unwrap();
    assert!(
        f.run(&["--config", normalized.to_str().unwrap(), "--check-config"])
            .status
            .success()
    );
}

#[test]
fn plain_output_keeps_rows_and_omits_palette_at_tiny_dimensions() {
    let f = Fixture::new();
    let path = f.config(
        r#"
[layout.first]
visible = false
[layout.second]
header = [{name="title", value="fixture@host"}, {name="separator", value="----"}]
footer = ["break", "colors"]
components = [{name="os", value="Fixture OS"}, {name="swap", value="Not configured (0 B)"}]
"#,
    );
    let result = f.run(&[
        "--config",
        path.to_str().unwrap(),
        "--plain",
        "--width",
        "1",
        "--height",
        "1",
    ]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("Fixture OS"));
    assert!(text.contains("Not configured (0 B)"));
    assert!(!text.contains('\u{1b}'));
    assert!(!text.contains('█'));
}

#[test]
fn bundled_catalog_lists_without_loading_config_and_validates_selected_assets() {
    let f = Fixture::new();
    f.config("invalid TOML!!!");
    let result = f.run(&["--list-images"]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let text = String::from_utf8(result.stdout).unwrap();
    assert_eq!(text.lines().count(), 30);
    assert!(text.contains("arch-1\t") && text.contains("arch-2\t"));
    assert!(text.contains("[default: arch]") && text.contains("[fallback]"));
    assert!(text.contains("[default: omarchy]"));
    assert!(!text.contains('\u{1b}'));
    for source in [
        "auto",
        "builtin",
        "arch-2",
        "omarchy-1",
        "omarchy-2",
        "omarchy-3.png",
        "linuxmint-2.png",
        "Glowing Steampunk Firefly and Wildflowers.png",
    ] {
        let path = f.config(&format!("[layout.first]\nimage='{source}'"));
        let result = f.run(&["--config", path.to_str().unwrap(), "--check-config"]);
        assert!(
            result.status.success(),
            "{source}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(!f.run(&["--list-images", "--check-config"]).status.success());
    assert!(!f.run(&["--list-images", "--print-config"]).status.success());
}

#[test]
fn ascii_file_output_preserves_art_and_validation_reports_missing_or_invalid_files() {
    let f = Fixture::new();
    std::fs::write(f.0.join("art.txt"), "  /界\\  \r\n\r\n\tART\n").unwrap();
    let original = "[layout.first]\nascii='art.txt'\n[layout.second]\nchildren=[]";
    let path = f.config(original);
    assert!(
        f.run(&["--config", path.to_str().unwrap(), "--check-config"])
            .status
            .success()
    );
    let result = f.run(&[
        "--config",
        path.to_str().unwrap(),
        "--plain",
        "--no-image",
        "--width",
        "1",
        "--height",
        "1",
    ]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("  /界\\  \n\n        ART\n"), "{text:?}");
    assert!(!text.contains('\u{1b}'));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::write(f.0.join("art.txt"), [0xff]).unwrap();
    assert!(
        !f.run(&["--config", path.to_str().unwrap(), "--check-config"])
            .status
            .success()
    );
    let result = f.run(&["--config", path.to_str().unwrap(), "--plain"]);
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("[art unavailable]")
    );
    assert!(String::from_utf8(result.stderr).unwrap().contains("UTF-8"));
    let path = f.config("[layout.first]\nascii='missing.txt'");
    assert!(
        !f.run(&["--config", path.to_str().unwrap(), "--check-config"])
            .status
            .success()
    );
}
