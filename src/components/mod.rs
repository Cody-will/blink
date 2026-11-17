pub mod native;
pub mod system;

use crate::config::IconMode;

pub struct Definition {
    pub name: &'static str,
    pub label: &'static str,
    pub unicode: &'static str,
    pub nerd: &'static str,
    pub ascii: &'static str,
}

macro_rules! metrics {
    ($(($name:literal, $label:literal, $u:literal, $n:literal, $a:literal)),* $(,)?) => {
        pub const DEFINITIONS: &[Definition] = &[
            $(Definition {
                name: $name,
                label: $label,
                unicode: $u,
                nerd: $n,
                ascii: $a,
            }),*
        ];
    }
}

metrics! {
    ("os", "OS", "◈", "󰣇", "O"),
    ("distribution", "Distribution", "◈", "󰌽", "D"),
    ("hostname", "Hostname", "⌂", "󰟀", "H"),
    ("kernel", "Kernel", "◇", "󰌽", "K"),
    ("architecture", "Architecture", "▦", "󰘚", "A"),
    ("uptime", "Uptime", "◷", "󰥔", "U"),
    ("shell", "Shell", "❯", "󰆍", ">"),
    ("terminal", "Terminal", "▣", "󰆍", "T"),
    ("cpu", "CPU", "◆", "󰍛", "C"),
    ("cpu_usage", "CPU usage", "◆", "󰍛", "%"),
    ("cpu_cores", "CPU cores", "▦", "󰍛", "#"),
    ("memory", "Memory", "▤", "󰘚", "M"),
    ("total_ram", "Total RAM", "▤", "󰘚", "M"),
    ("used_ram", "Used RAM", "▤", "󰘚", "M"),
    ("memory_percentage", "Memory usage", "▤", "󰘚", "%"),
    ("gpu", "GPU", "▧", "󰢮", "G"),
    ("disk", "Disk", "▰", "󰋊", "D"),
    ("battery", "Battery", "▱", "󰁹", "B"),
    ("desktop", "Desktop", "▣", "󰇄", "E"),
    ("window_manager", "Window manager", "▣", "󰖯", "W"),
    ("display", "Display", "▭", "󰍹", "R"),
    ("datetime", "Date & time", "◷", "󰥔", "@"),
    ("host", "Host", "⌂", "󰟀", "H"),
    ("packages", "Packages", "▣", "󰏖", "P"),
    ("swap", "Swap", "▤", "󰘚", "S"),
    ("local_ip", "Local IP", "↔", "󰈀", "N"),
    ("power_adapter", "Power adapter", "ϟ", "󰚥", "P"),
    ("locale", "Locale", "◎", "󰗊", "L"),
    ("wm_theme", "WM theme", "◇", "󰖯", "W"),
    ("desktop_theme", "Theme", "◈", "󰏘", "T"),
    ("icon_theme", "Icon theme", "◈", "󰀻", "I"),
    ("font", "Font", "Aa", "󰛖", "F"),
    ("cursor", "Cursor", "↖", "󰇀", "C"),
    ("terminal_font", "Terminal font", "Aa", "󰛖", "F"),
    ("title", "Title", "", "", ""),
    ("separator", "Separator", "", "", ""),
    ("break", "Break", "", "", ""),
    ("colors", "Colors", "", "", ""),
}

pub fn definition(name: &str) -> Option<&'static Definition> {
    DEFINITIONS.iter().find(|d| d.name == name)
}

pub fn icon(name: &str, mode: IconMode) -> &'static str {
    if name == "network" {
        return match mode {
            IconMode::Unicode => "↔",
            IconMode::Nerd => "󰈀",
            IconMode::Ascii => "N",
        };
    }

    let name = if name == "clock" { "datetime" } else { name };
    definition(name)
        .map(|d| match mode {
            IconMode::Unicode => d.unicode,
            IconMode::Nerd => d.nerd,
            IconMode::Ascii => d.ascii,
        })
        .unwrap_or("")
}

pub const SYSTEM: &[&str] = &[
    "os", "host", "hostname", "kernel", "uptime", "packages", "shell", "locale",
];

pub const SESSION: &[&str] = &[
    "display",
    "desktop",
    "window_manager",
    "wm_theme",
    "desktop_theme",
    "icon_theme",
    "font",
    "cursor",
    "terminal",
    "terminal_font",
];

pub const HARDWARE: &[&str] = &["cpu", "cpu_cores", "gpu", "memory", "swap"];
pub const STORAGE: &[&str] = &["disk", "local_ip", "battery", "power_adapter"];
pub const PRESETS: &[&str] = &["system", "hardware", "storage", "session"];
pub fn preset(name: &str) -> Option<(&'static str, &'static [&'static str])> {
    match name {
        "system" => Some(("System", SYSTEM)),
        "hardware" => Some(("Hardware", HARDWARE)),
        "storage" => Some(("Storage & Power", STORAGE)),
        "session" => Some(("Session", SESSION)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    Data,
    Title,
    Separator,
    Break,
    Colors,
}

impl Definition {
    pub fn presentation(&self) -> Presentation {
        match self.name {
            "title" => Presentation::Title,
            "separator" => Presentation::Separator,
            "break" => Presentation::Break,
            "colors" => Presentation::Colors,
            _ => Presentation::Data,
        }
    }

    pub fn priority(&self) -> u8 {
        match self.name {
            "os" | "cpu" | "memory" => 100,
            "host" | "kernel" | "uptime" | "gpu" | "disk" => 80,
            "packages" | "shell" | "display" | "desktop" | "window_manager" | "terminal"
            | "swap" | "local_ip" | "battery" | "power_adapter" => 60,
            "wm_theme" | "desktop_theme" | "icon_theme" | "font" | "cursor" | "terminal_font" => 20,
            "break" | "colors" => 10,
            _ => 40,
        }
    }
}

pub fn canonical(name: &str) -> Option<&'static str> {
    if let Some(d) = DEFINITIONS
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(name))
    {
        return Some(d.name);
    }
    [
        ("de", "desktop"),
        ("wm", "window_manager"),
        ("wmtheme", "wm_theme"),
        ("theme", "desktop_theme"),
        ("icons", "icon_theme"),
        ("terminalfont", "terminal_font"),
        ("localip", "local_ip"),
        ("poweradapter", "power_adapter"),
    ]
    .into_iter()
    .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
    .map(|(_, name)| name)
}
