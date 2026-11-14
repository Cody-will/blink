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
    ($(($name:literal,$label:literal,$u:literal,$n:literal,$a:literal)),* $(,)?) => {
        pub const DEFINITIONS:&[Definition]=&[$(Definition{name:$name,label:$label,unicode:$u,nerd:$n,ascii:$a}),*];
    }
}
metrics! {
    ("os","OS","◈","󰣇","O"),("distribution","Distribution","◈","󰌽","D"),
    ("hostname","Host","⌂","󰟀","H"),("kernel","Kernel","◇","󰌽","K"),
    ("architecture","Architecture","▦","󰘚","A"),("uptime","Uptime","◷","󰥔","U"),
    ("shell","Shell","❯","󰆍",">"),("terminal","Terminal","▣","󰆍","T"),
    ("cpu","CPU","◆","󰍛","C"),("cpu_usage","CPU usage","◆","󰍛","%"),
    ("cpu_cores","CPU cores","▦","󰍛","#"),("memory","Memory","▤","󰘚","M"),
    ("total_ram","Total RAM","▤","󰘚","M"),("used_ram","Used RAM","▤","󰘚","M"),
    ("memory_percentage","Memory usage","▤","󰘚","%"),("gpu","GPU","▧","󰢮","G"),
    ("disk","Disk","▰","󰋊","D"),("battery","Battery","▱","󰁹","B"),
    ("desktop","Desktop","▣","󰇄","E"),("window_manager","Window manager","▣","󰖯","W"),
    ("display","Display","▭","󰍹","R"),("datetime","Date & time","◷","󰥔","@"),
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
pub fn preset(name: &str) -> Option<(&'static str, &'static [&'static str])> {
    match name {
        "system" => Some(("System", &["os", "hostname", "kernel", "uptime", "shell"])),
        "hardware" => Some(("Hardware", &["cpu", "cpu_cores", "memory", "gpu"])),
        "storage" => Some(("Storage", &["disk", "battery"])),
        "session" => Some((
            "Session",
            &[
                "desktop",
                "window_manager",
                "terminal",
                "display",
                "datetime",
            ],
        )),
        _ => None,
    }
}
