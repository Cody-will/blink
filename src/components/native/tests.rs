use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    pub s: Sources,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "blink-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let home = base.join("home");
        let config = home.join(".config");
        let root = base.join("root");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        Self {
            base,
            s: Sources {
                root,
                home,
                config,
                env: BTreeMap::new(),
            },
        }
    }
    fn write(&self, path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    fn root(&self, path: &str, text: &str) {
        self.write(&self.s.path(path), text);
    }
    fn config(&self, path: &str, text: &str) {
        self.write(&self.s.config.join(path), text);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}
#[test]
fn host_reads_product_and_rejects_firmware_placeholders() {
    let f = Fixture::new();
    f.root("sys/class/dmi/id/product_name", "ThinkTest\n");
    f.root("sys/class/dmi/id/sys_vendor", "Example\n");
    f.root("sys/class/dmi/id/product_version", "Default string");
    let host = host(&f.s).unwrap();
    assert_eq!(host.model, "ThinkTest");
    assert_eq!(host.vendor.as_deref(), Some("Example"));
    assert!(host.version.is_none());
    f.root("sys/class/dmi/id/product_name", "To Be Filled By O.E.M.");
    assert!(host_or_none(&f.s));
    f.root("sys/firmware/devicetree/base/model", "Test Board\0");
    assert_eq!(super::host(&f.s).unwrap().model, "Test Board");
}
fn host_or_none(s: &Sources) -> bool {
    super::host(s).is_none()
}
#[test]
fn adapters_report_connection_and_measured_power_without_guessing() {
    let f = Fixture::new();
    for (name, kind, online) in [
        ("AC", "Mains", "1"),
        ("USB", "USB_C", "0"),
        ("BAT0", "Battery", "1"),
    ] {
        f.root(&format!("sys/class/power_supply/{name}/type"), kind);
        f.root(&format!("sys/class/power_supply/{name}/online"), online);
    }
    f.root("sys/class/power_supply/AC/voltage_now", "20000000");
    f.root("sys/class/power_supply/AC/current_now", "2000000");
    let a = adapters(&f.s).unwrap();
    assert_eq!(a.len(), 2);
    assert!(a[0].online);
    assert_eq!(a[0].watts, Some(40.0));
    assert!(!a[1].online);
    assert_eq!(a[1].watts, None);
}
#[test]
fn package_databases_count_installed_entries_and_exclude_removed_packages() {
    let f = Fixture::new();
    for name in ["one-1", "two-2"] {
        f.root(
            &format!("var/lib/pacman/local/{name}/desc"),
            "%NAME%\nexample\n",
        );
    }
    f.root("var/lib/pacman/local/ALPM_DB_VERSION", "9");
    f.root("var/lib/dpkg/status","Package: one\nStatus: install ok installed\n\nPackage: removed\nStatus: deinstall ok config-files\n");
    f.root("var/lib/flatpak/app/app.example/current", "active");
    f.root("var/lib/flatpak/app/deleted.example/metadata", "removed");
    std::fs::create_dir_all(f.s.path("var/lib/flatpak/runtime/org.example.Platform/x86_64/1"))
        .unwrap();
    std::fs::create_dir_all(f.s.path("var/lib/flatpak/runtime/org.example.Platform/x86_64/2"))
        .unwrap();
    std::fs::create_dir_all(
        f.s.path("var/lib/flatpak/runtime/org.example.Platform.Locale/x86_64/1"),
    )
    .unwrap();
    f.root(
        "var/lib/snapd/sequence/app.json",
        r#"{"current":"1","sequence":[{"revision":"1"}]}"#,
    );
    let counts = packages::collect(&f.s)
        .unwrap()
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    assert_eq!(counts["pacman"], 2);
    assert_eq!(counts["dpkg"], 1);
    assert_eq!(counts["flatpak"], 3);
    assert_eq!(counts["snap"], 1);
}
#[test]
fn rpm_sqlite_is_read_only_and_prefers_real_package_signatures() {
    let f = Fixture::new();
    let path = f.s.path("usr/lib/sysimage/rpm/rpmdb.sqlite");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE Packages(hnum INTEGER); INSERT INTO Packages VALUES(1),(2),(3); CREATE TABLE Sigmd5(hnum INTEGER); INSERT INTO Sigmd5 VALUES(1),(2);").unwrap();
    drop(db);
    let before = std::fs::read(&path).unwrap();
    let counts = packages::collect(&f.s).unwrap();
    assert_eq!(counts, vec![("rpm".into(), 2)]);
    assert_eq!(before, std::fs::read(path).unwrap());
}
#[test]
fn malformed_missing_and_unreadable_databases_are_not_zero_counts() {
    let f = Fixture::new();
    assert!(packages::collect(&f.s).is_none());
    f.root("var/lib/dpkg/status", "not a status database");
    f.root("var/lib/rpm/rpmdb.sqlite", "broken");
    assert!(packages::collect(&f.s).is_none());
    assert_eq!(packages::dpkg(std::io::Cursor::new("")), Some(0));
    struct Unreadable;
    impl std::io::Read for Unreadable {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::PermissionDenied.into())
        }
    }
    impl std::io::BufRead for Unreadable {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Err(std::io::ErrorKind::PermissionDenied.into())
        }
        fn consume(&mut self, _: usize) {}
    }
    assert_eq!(packages::dpkg(Unreadable), None);
}
#[test]
fn custom_pacman_database_path_is_respected() {
    let f = Fixture::new();
    f.root("etc/pacman.conf", "[options]\nDBPath = /custom/db\n");
    f.root("custom/db/local/example/desc", "%NAME%");
    assert_eq!(packages::collect(&f.s).unwrap(), vec![("pacman".into(), 1)]);
}
#[test]
fn gtk_settings_read_colors_icons_fonts_and_cursor_size() {
    let f = Fixture::new();
    f.config("gtk-3.0/settings.ini","[Settings]\ngtk-theme-name=Test Theme\ngtk-icon-theme-name=Test Icons\ngtk-font-name=Sans 11\ngtk-cursor-theme-name=Test Cursor\ngtk-cursor-theme-size=24\n");
    let data = appearance::files(&f.s);
    assert_eq!(data["desktop_theme"], "Test Theme [GTK]");
    assert_eq!(data["font"], "Sans 11");
    assert_eq!(data["icon_theme"], "Test Icons");
    assert_eq!(data["cursor"], "Test Cursor (24)");
}
#[test]
fn kde_settings_require_a_kde_session_and_never_replace_blink_theme() {
    let mut f = Fixture::new();
    f.config(
        "kdeglobals",
        "[General]\nColorScheme=BreezeDark\nfont=Noto Sans,10,-1,5\n[Icons]\nTheme=Breeze\n",
    );
    f.config("kwinrc", "[org.kde.kdecoration2]\ntheme=Decoration\n");
    assert!(!appearance::files(&f.s).contains_key("desktop_theme"));
    f.s.env.insert("XDG_CURRENT_DESKTOP".into(), "KDE".into());
    let data = appearance::files(&f.s);
    assert_eq!(data["desktop_theme"], "BreezeDark [Qt]");
    assert_eq!(data["wm_theme"], "Decoration");
    assert_eq!(data["font"], "Noto Sans (10)");
}
#[test]
fn xfce_and_openbox_use_native_xml_only_for_matching_sessions() {
    let mut f = Fixture::new();
    f.config("xfce4/xfconf/xfce-perchannel-xml/xsettings.xml",r#"<channel><property name="Net"><property name="ThemeName" value="Xfce Theme"/></property></channel>"#);
    f.config(
        "xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml",
        r#"<channel><property name="theme" value="Xfwm Theme"/></channel>"#,
    );
    f.s.env.insert("XDG_CURRENT_DESKTOP".into(), "XFCE".into());
    let data = appearance::files(&f.s);
    assert_eq!(data["desktop_theme"], "Xfce Theme");
    assert_eq!(data["wm_theme"], "Xfwm Theme");
    f.config(
        "openbox/rc.xml",
        "<openbox_config><theme><name>Openbox Theme</name></theme></openbox_config>",
    );
    f.s.env
        .insert("XDG_CURRENT_DESKTOP".into(), "Openbox".into());
    assert_eq!(appearance::files(&f.s)["wm_theme"], "Openbox Theme");
}
#[test]
fn terminal_fonts_detect_supported_native_configs_and_do_not_guess_others() {
    let mut f = Fixture::new();
    #[cfg(target_os = "linux")]
    {
        f.root("proc/self/status", "Name:\tblink\nPPid:\t42\n");
        f.root("proc/42/status", "Name:\tgnome-terminal-\nPPid:\t1\n");
        assert_eq!(f.s.terminal().as_deref(), Some("gnome-terminal-"));
    }
    for (terminal, path, text, expected) in [
        (
            "kitty",
            "kitty/kitty.conf",
            "include fonts.conf\n",
            "Test Mono (12)",
        ),
        (
            "alacritty",
            "alacritty/alacritty.toml",
            "[general]\nimport=['fonts.toml']\n",
            "Test Mono (13)",
        ),
        (
            "ghostty",
            "ghostty/config",
            "font-family = Test Mono\nfont-size = 14\n",
            "Test Mono (14)",
        ),
        (
            "foot",
            "foot/foot.ini",
            "[main]\nfont=Test Mono:size=15\n",
            "Test Mono:size=15",
        ),
    ] {
        f.config("kitty/fonts.conf", "font_family Test Mono\nfont_size 12\n");
        f.config(
            "alacritty/fonts.toml",
            "[font]\nsize=13\n[font.normal]\nfamily='Test Mono'\n",
        );
        f.config(path, text);
        f.s.env.insert("TERM_PROGRAM".into(), terminal.into());
        assert_eq!(appearance::terminal_font(&f.s).as_deref(), Some(expected));
    }
    f.s.env
        .insert("TERM_PROGRAM".into(), "dynamic-terminal".into());
    assert!(appearance::terminal_font(&f.s).is_none());
    f.s.env.insert("TERM_PROGRAM".into(), "kitty".into());
    f.config("kitty/kitty.conf", "include kitty.conf\n");
    assert!(appearance::terminal_font(&f.s).is_none());
}
#[test]
fn konsole_font_profile_uses_qt_font_fields() {
    let mut f = Fixture::new();
    f.s.env.insert("TERM_PROGRAM".into(), "konsole".into());
    f.config(
        "konsolerc",
        "[Desktop Entry]\nDefaultProfile=Test.profile\n",
    );
    f.write(
        &f.s.home.join(".local/share/konsole/Test.profile"),
        "[Appearance]\nFont=Test Mono,11,-1,5\n",
    );
    assert_eq!(
        appearance::terminal_font(&f.s).as_deref(),
        Some("Test Mono (11)")
    );
}
#[test]
fn local_interface_filter_excludes_loopback_and_link_local() {
    for ip in [
        "127.0.0.1",
        "169.254.1.1",
        "::1",
        "fe80::1",
        "0.0.0.0",
        "::",
    ] {
        assert!(!usable_address(ip.parse().unwrap()));
    }
    for ip in ["192.168.1.2", "10.0.0.2", "2001:db8::1"] {
        assert!(usable_address(ip.parse().unwrap()));
    }
}
#[test]
fn locale_precedence_ignores_empty_overrides() {
    let mut f = Fixture::new();
    f.s.env.insert("LANG".into(), "en_US.UTF-8".into());
    f.s.env.insert("LC_MESSAGES".into(), "fr_FR.UTF-8".into());
    f.s.env.insert("LC_ALL".into(), String::new());
    assert_eq!(
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|key| f.s.env(key)),
        Some("fr_FR.UTF-8")
    );
}
