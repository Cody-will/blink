//! Native, read-only detection. Paths/environment are injectable for fixture tests.
pub mod appearance;
pub mod packages;

use super::system::{Adapter, Host, Measurement};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    path::{Path, PathBuf},
};

pub struct Sources {
    pub root: PathBuf,
    pub home: PathBuf,
    pub config: PathBuf,
    pub env: BTreeMap<String, String>,
}

impl Sources {
    pub fn live() -> Self {
        let keys = [
            "LC_ALL",
            "LC_MESSAGES",
            "LANG",
            "TERM_PROGRAM",
            "TERM",
            "TERMINAL",
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_DESKTOP",
            "XDG_SESSION_WM",
            "DESKTOP_SESSION",
            "XDG_CONFIG_HOME",
            "KITTY_CONFIG_DIRECTORY",
            "GHOSTTY_RESOURCES_DIR",
            "KONSOLE_PROFILE_NAME",
            "XDG_DATA_HOME",
        ];

        let env: BTreeMap<_, _> = keys
            .into_iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k.into(), v)))
            .collect();
        let home = dirs::home_dir().unwrap_or_default();
        let config = env
            .get("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        Self {
            root: PathBuf::from("/"),
            home,
            config,
            env,
        }
    }

    pub fn path(&self, path: &str) -> PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }

    pub fn read(&self, path: &Path) -> Option<String> {
        let metadata = std::fs::metadata(path).ok()?;
        if metadata.len() > 1024 * 1024 {
            return None;
        }
        std::fs::read_to_string(path).ok()
    }

    pub fn env(&self, key: &str) -> Option<&str> {
        self.env
            .get(key)
            .map(String::as_str)
            .filter(|s| !s.trim().is_empty())
    }

    pub fn terminal(&self) -> Option<String> {
        if let Some(name) = self.env("TERM_PROGRAM").or_else(|| self.env("TERMINAL")) {
            return Some(name.to_lowercase());
        }

        #[cfg(target_os = "linux")]
        {
            let mut current = "self".to_owned();
            for _ in 0..12 {
                let Some(status) = self.read(&self.path(&format!("proc/{current}/status"))) else {
                    break;
                };

                let name = status
                    .lines()
                    .find_map(|l| l.strip_prefix("Name:").map(str::trim))
                    .unwrap_or("");
                if [
                    "kitty",
                    "alacritty",
                    "ghostty",
                    "foot",
                    "footclient",
                    "konsole",
                    "gnome-terminal",
                    "gnome-terminal-server",
                    "gnome-terminal-", // Linux comm names are limited to 15 bytes.
                ]
                .contains(&name)
                {
                    return Some(name.into());
                }
                current = status
                    .lines()
                    .find_map(|l| l.strip_prefix("PPid:").map(str::trim))?
                    .to_owned();
                if current == "0" {
                    break;
                }
            }
        }
        self.env("TERM").map(str::to_owned)
    }
}

pub fn collect(keys: &BTreeSet<String>, hostname: Option<&str>) -> BTreeMap<String, Measurement> {
    let source = Sources::live();
    let mut out = BTreeMap::new();
    if keys.contains("title")
        && let (Ok(user), Some(host)) = (whoami::username(), hostname)
    {
        out.insert("title".into(), Measurement::Text(format!("{user}@{host}")));
    }

    if keys.contains("host")
        && let Some(host) = host(&source)
    {
        out.insert("host".into(), Measurement::Host(host));
    }

    if keys.contains("packages")
        && let Some(counts) = packages::collect(&source)
    {
        out.insert("packages".into(), Measurement::Packages(counts));
    }

    if keys.contains("locale")
        && let Some(locale) = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|k| source.env(k))
    {
        out.insert("locale".into(), Measurement::Text(locale.into()));
    }

    if keys.contains("power_adapter")
        && let Some(adapters) = adapters(&source)
    {
        out.insert("power_adapter".into(), Measurement::Adapters(adapters));
    }

    if keys.contains("local_ip")
        && let Some(addresses) = addresses()
    {
        out.insert("local_ip".into(), Measurement::Addresses(addresses));
    }

    if keys.iter().any(|k| appearance::KEYS.contains(&k.as_str())) {
        let settings = appearance::collect(&source, keys);
        for (key, value) in settings {
            if keys.contains(&key) {
                out.insert(key, Measurement::Text(value));
            }
        }
    }
    out
}

fn meaningful(value: &str) -> Option<String> {
    let value = value.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    if value.is_empty()
        || [
            "none",
            "not specified",
            "default string",
            "system product name",
            "to be filled by o.e.m.",
            "unknown",
        ]
        .contains(&value.to_lowercase().as_str())
    {
        None
    } else {
        Some(value.into())
    }
}

pub fn host(s: &Sources) -> Option<Host> {
    let read = |field: &str| {
        ["sys/devices/virtual/dmi/id", "sys/class/dmi/id"]
            .into_iter()
            .find_map(|dir| {
                s.read(&s.path(&format!("{dir}/{field}")))
                    .and_then(|v| meaningful(&v))
            })
    };

    let model = read("product_name").or_else(|| {
        s.read(&s.path("sys/firmware/devicetree/base/model"))
            .and_then(|v| meaningful(&v))
    })?;
    Some(Host {
        vendor: read("sys_vendor"),
        model,
        version: read("product_version"),
    })
}

pub fn adapters(s: &Sources) -> Option<Vec<Adapter>> {
    let mut values = vec![];
    for entry in std::fs::read_dir(s.path("sys/class/power_supply")).ok()? {
        let entry = entry.ok()?;
        let dir = entry.path();
        let read = |field: &str| s.read(&dir.join(field)).map(|s| s.trim().to_owned());
        let Some(kind) = read("type") else {
            continue;
        };

        if !matches!(
            kind.as_str(),
            "Mains" | "USB" | "USB_C" | "USB_PD" | "USB_PD_DRP" | "Wireless"
        ) {
            continue;
        }

        let Some(online) = read("online").and_then(|v| match v.as_str() {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        }) else {
            continue;
        };

        let microwatts = read("power_now")
            .and_then(|n| n.parse::<f64>().ok())
            .or_else(|| {
                let volts = read("voltage_now")?.parse::<f64>().ok()?;
                let amps = read("current_now")?.parse::<f64>().ok()?;
                Some(volts * amps / 1_000_000.0)
            });
        let watts = microwatts
            .map(|n| n / 1_000_000.0)
            .filter(|n| n.is_finite() && *n >= 0.0);
        values.push(Adapter {
            name: read("model_name")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| entry.file_name().to_string_lossy().into()),
            online,
            watts,
        });
    }
    values.sort_by(|a, b| a.name.cmp(&b.name));
    (!values.is_empty()).then_some(values)
}

pub fn usable_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !v.is_loopback() && !v.is_link_local() && !v.is_unspecified() && !v.is_multicast()
        }
        IpAddr::V6(v) => {
            !v.is_loopback()
                && !v.is_unicast_link_local()
                && !v.is_unspecified()
                && !v.is_multicast()
        }
    }
}

fn addresses() -> Option<Vec<(String, Vec<IpAddr>)>> {
    let mut addresses: BTreeMap<String, BTreeSet<IpAddr>> = BTreeMap::new();
    for interface in if_addrs::get_if_addrs().ok()? {
        if interface.is_oper_up() && usable_address(interface.ip()) {
            addresses
                .entry(interface.name.clone())
                .or_default()
                .insert(interface.ip());
        }
    }
    (!addresses.is_empty()).then(|| {
        addresses
            .into_iter()
            .map(|(name, ips)| (name, ips.into_iter().collect()))
            .collect()
    })
}

#[cfg(test)]
mod tests;
