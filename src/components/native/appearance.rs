//! Read appearance settings without starting desktop/terminal executables.
use super::Sources;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const KEYS: &[&str] = &[
    "wm_theme",
    "desktop_theme",
    "icon_theme",
    "font",
    "cursor",
    "terminal_font",
];

pub type Settings = BTreeMap<String, String>;
pub fn ini(text: &str) -> BTreeMap<(String, String), String> {
    let mut group = String::new();
    let mut result = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            group = line[1..line.len() - 1].to_owned();
        } else if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                result.insert((group.clone(), key.trim().to_owned()), value.into());
            }
        }
    }
    result
}

fn value(map: &BTreeMap<(String, String), String>, group: &str, key: &str) -> Option<String> {
    map.get(&(group.into(), key.into())).cloned()
}

fn put(out: &mut Settings, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|s| !s.trim().is_empty()) {
        out.insert(key.into(), value);
    }
}

pub fn collect(s: &Sources, keys: &std::collections::BTreeSet<String>) -> Settings {
    let desktop = keys
        .iter()
        .any(|k| KEYS[..KEYS.len() - 1].contains(&k.as_str()));
    let mut out = if desktop { files(s) } else { Settings::new() };

    #[cfg(target_os = "linux")]
    if desktop
        && s.env("XDG_CURRENT_DESKTOP")
            .is_some_and(|d| d.to_uppercase().contains("GNOME"))
    {
        for (key, value) in gnome() {
            out.insert(key, value);
        }
    }

    if keys.contains("terminal_font") {
        put(&mut out, "terminal_font", terminal_font(s));
    }
    out
}

pub fn files(s: &Sources) -> Settings {
    let mut out = Settings::new();
    let session = s
        .env("XDG_CURRENT_DESKTOP")
        .or_else(|| s.env("XDG_SESSION_DESKTOP"))
        .unwrap_or("")
        .to_lowercase();
    let is_kde = session.contains("kde") || session.contains("plasma");
    let kde = is_kde
        .then(|| s.read(&s.config.join("kdeglobals")))
        .flatten()
        .map(|v| ini(&v))
        .unwrap_or_default();
    let mut themes = vec![];
    if let Some(theme) = value(&kde, "General", "ColorScheme") {
        themes.push(format!("{theme} [Qt]"));
    }
    put(&mut out, "icon_theme", value(&kde, "Icons", "Theme"));
    put(
        &mut out,
        "font",
        value(&kde, "General", "font").map(|font| qt_font(&font)),
    );
    for path in [
        s.config.join("gtk-4.0/settings.ini"),
        s.config.join("gtk-3.0/settings.ini"),
        s.home.join(".gtkrc-2.0"),
    ] {
        let Some(text) = s.read(&path) else {
            continue;
        };

        let data = ini(&text);
        let get = |key: &str| value(&data, "Settings", key).or_else(|| value(&data, "", key));
        if let Some(theme) = get("gtk-theme-name")
            && !themes.iter().any(|t| t == &format!("{theme} [GTK]"))
        {
            themes.push(format!("{theme} [GTK]"));
        }

        for (key, setting) in [
            ("icon_theme", "gtk-icon-theme-name"),
            ("font", "gtk-font-name"),
        ] {
            if !out.contains_key(key) {
                put(&mut out, key, get(setting));
            }
        }

        if !out.contains_key("cursor") {
            put(
                &mut out,
                "cursor",
                get("gtk-cursor-theme-name")
                    .map(|name| with_size(name, get("gtk-cursor-theme-size"))),
            );
        }
    }

    if !themes.is_empty() {
        out.insert("desktop_theme".into(), themes.join(", "));
    }

    if is_kde && let Some(text) = s.read(&s.config.join("kcminputrc")) {
        let data = ini(&text);
        put(
            &mut out,
            "cursor",
            value(&data, "Mouse", "cursorTheme")
                .map(|name| with_size(name, value(&data, "Mouse", "cursorSize"))),
        );
    }

    if is_kde && let Some(text) = s.read(&s.config.join("kwinrc")) {
        let data = ini(&text);
        put(
            &mut out,
            "wm_theme",
            value(&data, "org.kde.kdecoration2", "theme"),
        );
    }

    for (path, mapping) in [
        (
            "xfce4/xfconf/xfce-perchannel-xml/xsettings.xml",
            &[
                ("ThemeName", "desktop_theme"),
                ("IconThemeName", "icon_theme"),
                ("FontName", "font"),
                ("CursorThemeName", "cursor"),
            ][..],
        ),
        (
            "xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml",
            &[("theme", "wm_theme")][..],
        ),
    ] {
        if session.contains("xfce")
            && let Some(text) = s.read(&s.config.join(path))
            && let Ok(xml) = roxmltree::Document::parse(&text)
        {
            for (name, key) in mapping {
                put(
                    &mut out,
                    key,
                    xml.descendants()
                        .find(|n| n.has_tag_name("property") && n.attribute("name") == Some(*name))
                        .and_then(|n| n.attribute("value"))
                        .map(str::to_owned),
                );
            }
        }
    }

    if (session.contains("openbox")
        || s.env("XDG_SESSION_WM")
            .is_some_and(|s| s.eq_ignore_ascii_case("openbox")))
        && let Some(text) = s.read(&s.config.join("openbox/rc.xml"))
        && let Ok(xml) = roxmltree::Document::parse(&text)
    {
        put(
            &mut out,
            "wm_theme",
            xml.descendants()
                .find(|n| n.tag_name().name() == "theme")
                .and_then(|n| n.children().find(|n| n.tag_name().name() == "name"))
                .and_then(|n| n.text())
                .map(str::to_owned),
        );
    }
    out
}

fn with_size(name: String, size: Option<String>) -> String {
    match size {
        Some(size) => format!("{name} ({size})"),
        None => name,
    }
}

fn qt_font(font: &str) -> String {
    let mut fields = font.split(',');
    let family = fields.next().unwrap_or(font);
    with_size(
        family.into(),
        fields
            .next()
            .filter(|v| v.parse::<f64>().is_ok_and(|v| v > 0.0))
            .map(str::to_owned),
    )
}

fn font_file(s: &Sources, path: &Path, depth: usize) -> Option<Settings> {
    if depth > 8 {
        return None;
    }

    let text = s.read(path)?;
    let mut result = Settings::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (key, value) = line.split_once(char::is_whitespace)?;
        if key == "include" {
            let path = expand(s, value.trim(), path.parent()?);
            result.extend(font_file(s, &path, depth + 1)?);
        } else if key == "globinclude" || key == "envinclude" {
            return None;
        } else {
            result.insert(key.into(), value.trim().trim_matches('"').into());
        }
    }
    Some(result)
}

fn expand(s: &Sources, path: &str, base: &Path) -> PathBuf {
    if let Some(path) = path.strip_prefix("~/") {
        s.home.join(path)
    } else {
        base.join(path)
    }
}

fn alacritty(s: &Sources, path: &Path, depth: usize) -> Option<toml::Value> {
    if depth > 8 {
        return None;
    }

    let text = s.read(path)?;
    let main: toml::Value = toml::from_str(&text).ok()?;
    let mut merged = toml::Value::Table(toml::Table::new());
    if let Some(imports) = main
        .get("general")
        .and_then(|v| v.get("import"))
        .or_else(|| main.get("import"))
        .and_then(toml::Value::as_array)
    {
        for import in imports {
            let import = expand(s, import.as_str()?, path.parent()?);
            if let Some(value) = alacritty(s, &import, depth + 1) {
                crate::config::merge(&mut merged, value);
            }
        }
    }
    crate::config::merge(&mut merged, main);
    Some(merged)
}

pub fn terminal_font(s: &Sources) -> Option<String> {
    let terminal = s.terminal()?;
    if terminal.contains("kitty") {
        let path = s
            .env("KITTY_CONFIG_DIRECTORY")
            .map(PathBuf::from)
            .unwrap_or_else(|| s.config.join("kitty"))
            .join("kitty.conf");
        let font = font_file(s, &path, 0)?;
        return font
            .get("font_family")
            .map(|name| with_size(name.clone(), font.get("font_size").cloned()));
    }

    if terminal.contains("alacritty") {
        let value = alacritty(s, &s.config.join("alacritty/alacritty.toml"), 0)?;
        let font = value.get("font")?;
        let family = font.get("normal")?.get("family")?.as_str()?;
        return Some(with_size(
            family.into(),
            font.get("size").map(ToString::to_string),
        ));
    }

    if terminal.contains("ghostty") {
        let text = s.read(&s.config.join("ghostty/config"))?;
        if text
            .lines()
            .any(|line| line.trim_start().starts_with("config-file"))
        {
            return None;
        }

        let settings = ini(&text);
        return value(&settings, "", "font-family")
            .map(|name| with_size(name, value(&settings, "", "font-size")));
    }

    if terminal == "foot" || terminal == "footclient" {
        let text = s.read(&s.config.join("foot/foot.ini"))?;
        let data = ini(&text);
        if data.keys().any(|(_, key)| key == "include") {
            return None;
        }

        return value(&data, "main", "font");
    }

    if terminal.contains("konsole") {
        let profile = s
            .env("KONSOLE_PROFILE_NAME")
            .map(|n| format!("{n}.profile"))
            .or_else(|| {
                let text = s.read(&s.config.join("konsolerc"))?;
                value(&ini(&text), "Desktop Entry", "DefaultProfile")
            })?;
        let paths = [
            s.home.join(".local/share/konsole").join(&profile),
            s.path("usr/share/konsole").join(profile),
        ];

        for path in paths {
            if let Some(text) = s.read(&path)
                && let Some(font) = value(&ini(&text), "Appearance", "Font")
            {
                return Some(qt_font(&font));
            }
        }
    }

    #[cfg(target_os = "linux")]
    if terminal.contains("gnome-terminal") {
        return gnome_terminal_font();
    }
    None
}

#[cfg(target_os = "linux")]
fn setting(schema: &str, path: Option<&str>) -> Option<(gio::Settings, gio::SettingsSchema)> {
    let schema = gio::SettingsSchemaSource::default()?.lookup(schema, true)?;
    if schema.path().is_none() && path.is_none() {
        return None;
    }
    Some((
        gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, path),
        schema,
    ))
}

#[cfg(target_os = "linux")]
fn get(schema: &str, key: &str, path: Option<&str>) -> Option<String> {
    use gio::prelude::SettingsExt;
    let (settings, schema) = setting(schema, path)?;
    if !schema.has_key(key) {
        return None;
    }

    let value = settings.value(key);
    let value = value.get::<String>()?;
    (!value.is_empty()).then_some(value)
}

#[cfg(target_os = "linux")]
fn gnome() -> Settings {
    use gio::prelude::SettingsExt;
    let mut out = Settings::new();
    if let Some((settings, schema)) = setting("org.gnome.desktop.interface", None) {
        for (key, name) in [
            ("desktop_theme", "gtk-theme"),
            ("icon_theme", "icon-theme"),
            ("font", "font-name"),
            ("cursor", "cursor-theme"),
        ] {
            if schema.has_key(name) {
                put(&mut out, key, settings.value(name).get::<String>());
            }
        }

        if schema.has_key("cursor-size")
            && settings.int("cursor-size") > 0
            && let Some(name) = out.get("cursor").cloned()
        {
            out.insert(
                "cursor".into(),
                with_size(name, Some(settings.int("cursor-size").to_string())),
            );
        }
    }
    put(
        &mut out,
        "wm_theme",
        get("org.gnome.shell.extensions.user-theme", "name", None)
            .or_else(|| get("org.gnome.desktop.wm.preferences", "theme", None)),
    );
    out
}

#[cfg(target_os = "linux")]
fn gnome_terminal_font() -> Option<String> {
    use gio::prelude::SettingsExt;
    let id = get("org.gnome.Terminal.ProfilesList", "default", None)?;
    // Only a UUID can form a relocatable profile path.
    if !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return None;
    }

    let path = format!("/org/gnome/terminal/legacy/profiles:/:{id}/");
    let (settings, schema) = setting("org.gnome.Terminal.Legacy.Profile", Some(&path))?;
    if schema.has_key("use-system-font") && settings.boolean("use-system-font") {
        get("org.gnome.desktop.interface", "monospace-font-name", None)
    } else {
        get("org.gnome.Terminal.Legacy.Profile", "font", Some(&path))
    }
}
