//! Collection returns typed measurements. Formatting never performs system queries.
use crate::layout::node::{Kind, Node};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

#[derive(Debug, Clone)]
pub enum Measurement {
    Text(String),
    Bytes(u64),
    Percent(f64),
    Seconds(u64),
    Memory {
        used: u64,
        total: u64,
    },
    Cores {
        logical: usize,
        physical: Option<usize>,
    },
    Disks(Vec<Disk>),
    Batteries(Vec<Battery>),
    Host(Host),
    Packages(Vec<(String, u64)>),
    Addresses(Vec<(String, Vec<std::net::IpAddr>)>),
    Adapters(Vec<Adapter>),
}
#[derive(Debug, Clone)]
pub struct Host {
    pub vendor: Option<String>,
    pub model: String,
    pub version: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Adapter {
    pub name: String,
    pub online: bool,
    pub watts: Option<f64>,
}
#[derive(Debug, Clone)]
pub struct Disk {
    pub mount: String,
    pub total: u64,
    pub available: u64,
}
#[derive(Debug, Clone)]
pub struct Battery {
    pub percentage: f64,
    pub state: String,
}
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub values: BTreeMap<String, Measurement>,
}
impl Snapshot {
    pub fn requested<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> BTreeSet<String> {
        fn visit(n: &Node, keys: &mut BTreeSet<String>) {
            if !n.visible {
                return;
            }
            if n.kind == Kind::Component
                && n.value.is_none()
                && let Some(d) = crate::components::definition(&n.name)
            {
                match d.presentation() {
                    crate::components::Presentation::Data => {
                        keys.insert(n.name.clone());
                    }
                    crate::components::Presentation::Title
                    | crate::components::Presentation::Separator => {
                        keys.insert("title".into());
                    }
                    _ => {}
                }
            }
            for c in n.header.iter().chain(&n.children).chain(&n.footer) {
                visit(c, keys);
            }
        }
        let mut keys = BTreeSet::new();
        for n in nodes {
            visit(n, &mut keys);
        }
        keys
    }
    pub fn collect(keys: &BTreeSet<String>) -> Self {
        let mut out = Self::default();
        let need_cpu = keys
            .iter()
            .any(|k| matches!(k.as_str(), "cpu" | "cpu_usage" | "cpu_cores"));
        let need_ram = keys.iter().any(|k| {
            matches!(
                k.as_str(),
                "memory" | "total_ram" | "used_ram" | "memory_percentage" | "swap"
            )
        });
        let mut refresh = RefreshKind::nothing();
        if need_cpu {
            refresh = refresh.with_cpu(if keys.contains("cpu_usage") {
                CpuRefreshKind::nothing().with_usage()
            } else {
                CpuRefreshKind::nothing()
            });
        }
        if need_ram {
            refresh = refresh.with_memory(if keys.contains("swap") {
                MemoryRefreshKind::nothing().with_ram().with_swap()
            } else {
                MemoryRefreshKind::nothing().with_ram()
            });
        }
        let mut sys = if need_cpu || need_ram {
            System::new_with_specifics(refresh).ok()
        } else {
            None
        };
        let sample_started = Instant::now();
        let hostname = if keys.contains("hostname") || keys.contains("title") {
            System::host_name().ok()
        } else {
            None
        };
        out.values
            .extend(super::native::collect(keys, hostname.as_deref()));
        for key in keys {
            let value = match key.as_str() {
                "os" => System::name().ok().map(Measurement::Text),
                "distribution" => System::long_os_version().ok().map(Measurement::Text),
                "hostname" => hostname.clone().map(Measurement::Text),
                "kernel" => System::kernel_version().ok().map(Measurement::Text),
                "architecture" => Some(Measurement::Text(System::cpu_arch())),
                "uptime" => System::uptime().ok().map(Measurement::Seconds),
                "shell" => shell().map(Measurement::Text),
                "terminal" => {
                    env_first(&["TERM_PROGRAM", "TERMINAL", "TERM"]).map(Measurement::Text)
                }
                "desktop" => env_first(&[
                    "XDG_CURRENT_DESKTOP",
                    "XDG_SESSION_DESKTOP",
                    "DESKTOP_SESSION",
                ])
                .map(Measurement::Text),
                "window_manager" => window_manager().map(Measurement::Text),
                "datetime" => Some(Measurement::Text(
                    chrono::Local::now()
                        .format("%Y-%m-%d %H:%M:%S %:z")
                        .to_string(),
                )),
                "cpu" => sys
                    .as_ref()
                    .and_then(|s| s.cpus().first())
                    .map(|c| Measurement::Text(c.brand().into())),
                "cpu_cores" => {
                    sys.as_ref()
                        .filter(|s| !s.cpus().is_empty())
                        .map(|s| Measurement::Cores {
                            logical: s.cpus().len(),
                            physical: System::physical_core_count().ok(),
                        })
                }
                "memory" => {
                    sys.as_ref()
                        .filter(|s| s.total_memory() > 0)
                        .map(|s| Measurement::Memory {
                            used: s.used_memory(),
                            total: s.total_memory(),
                        })
                }
                "swap" => sys.as_ref().map(|s| Measurement::Memory {
                    used: s.used_swap(),
                    total: s.total_swap(),
                }),
                "total_ram" => sys
                    .as_ref()
                    .filter(|s| s.total_memory() > 0)
                    .map(|s| Measurement::Bytes(s.total_memory())),
                "used_ram" => sys
                    .as_ref()
                    .filter(|s| s.total_memory() > 0)
                    .map(|s| Measurement::Bytes(s.used_memory())),
                "memory_percentage" => sys
                    .as_ref()
                    .and_then(|s| percentage(s.used_memory(), s.total_memory()))
                    .map(Measurement::Percent),
                "gpu" => sysinfo::Gpus::new_with_refreshed_list().ok().and_then(|g| {
                    let names: Vec<_> = g
                        .iter()
                        .filter_map(|g| g.model().map(str::to_owned))
                        .collect();
                    (!names.is_empty()).then(|| Measurement::Text(names.join("\n")))
                }),
                "disk" => sysinfo::Disks::new_with_refreshed_list()
                    .ok()
                    .and_then(|d| {
                        let mut disks: Vec<_> = d
                            .iter()
                            .filter(|d| d.total_space() > 0)
                            .map(|d| Disk {
                                mount: d.mount_point().display().to_string(),
                                total: d.total_space(),
                                available: d.available_space(),
                            })
                            .collect();
                        disks.sort_by(|a, b| a.mount.cmp(&b.mount));
                        disks.dedup_by(|a, b| a.mount == b.mount);
                        (!disks.is_empty()).then_some(Measurement::Disks(disks))
                    }),
                "battery" => batteries(),
                "display" => displays(),
                _ => None,
            };
            if let Some(value) =
                value.filter(|v| !matches!(v,Measurement::Text(s) if s.trim().is_empty()))
            {
                out.values.insert(key.clone(), value);
            }
        }
        if keys.contains("cpu_usage")
            && let Some(s) = sys.as_mut()
        {
            std::thread::sleep(
                sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.saturating_sub(sample_started.elapsed()),
            );
            s.refresh_cpu_usage();
            if !s.cpus().is_empty() {
                out.values.insert(
                    "cpu_usage".into(),
                    Measurement::Percent(f64::from(s.global_cpu_usage())),
                );
            }
        }
        out
    }
    pub fn formatted(&self, key: &str) -> Option<String> {
        self.values.get(key).map(Measurement::format)
    }
}
impl Measurement {
    pub fn format(&self) -> String {
        match self {
            Self::Host(host) => [
                host.vendor.as_deref(),
                Some(host.model.as_str()),
                host.version.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" "),
            Self::Packages(counts) => counts
                .iter()
                .map(|(name, n)| format!("{n} ({name})"))
                .collect::<Vec<_>>()
                .join(", "),
            Self::Addresses(addresses) => addresses
                .iter()
                .map(|(name, ips)| {
                    format!(
                        "{name}: {}",
                        ips.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Adapters(adapters) => adapters
                .iter()
                .map(|a| {
                    format!(
                        "{}: {}{}",
                        a.name,
                        if a.online {
                            "Connected"
                        } else {
                            "Disconnected"
                        },
                        a.watts
                            .map(|w| format!(" ({w:.1} W now)"))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Text(s) => s.clone(),
            Self::Bytes(n) => bytes(*n),
            Self::Percent(p) => format!("{p:.1}%"),
            Self::Seconds(n) => format!("{}d {}h {}m", n / 86400, n / 3600 % 24, n / 60 % 60),
            Self::Memory { total: 0, .. } => "Not configured (0 B)".into(),
            Self::Memory { used, total } => format!(
                "{} / {} ({:.1}%)",
                bytes(*used),
                bytes(*total),
                percentage(*used, *total).unwrap_or(0.0)
            ),
            Self::Cores { logical, physical } => match physical {
                Some(p) => format!("{logical} logical / {p} physical"),
                None => format!("{logical} logical"),
            },
            Self::Disks(disks) => disks
                .iter()
                .map(|d| {
                    format!(
                        "{}  {} / {} ({:.1}%)",
                        d.mount,
                        bytes(d.total.saturating_sub(d.available)),
                        bytes(d.total),
                        percentage(d.total.saturating_sub(d.available), d.total).unwrap_or(0.0)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Batteries(batteries) => batteries
                .iter()
                .enumerate()
                .map(|(i, b)| format!("{}: {:.0}% ({})", i + 1, b.percentage, b.state))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}
pub fn bytes(value: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut n = value as f64;
    let mut unit = 0;
    while n >= 1024.0 && unit < units.len() - 1 {
        n /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{n:.1} {}", units[unit])
    }
}
pub fn percentage(used: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| (used as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}
fn env_first(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
}
fn shell() -> Option<String> {
    #[cfg(target_os = "linux")]
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(pid) = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:").map(str::trim))
        && let Ok(name) = std::fs::read_to_string(format!("/proc/{pid}/comm"))
    {
        let name = name.trim();
        if [
            "bash", "zsh", "fish", "sh", "dash", "ksh", "nu", "pwsh", "xonsh",
        ]
        .contains(&name)
        {
            return Some(name.into());
        }
    }
    env_first(&["SHELL", "COMSPEC"]).map(|s| {
        std::path::Path::new(&s)
            .file_name()
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or(s)
    })
}
fn window_manager() -> Option<String> {
    // Only explicit session evidence; do not infer a WM from the desktop name.
    if std::env::var_os("SWAYSOCK").is_some() {
        Some("Sway".into())
    } else if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        Some("Hyprland".into())
    } else {
        env_first(&["WINDOW_MANAGER", "XDG_SESSION_WM"])
    }
}
fn batteries() -> Option<Measurement> {
    let manager = battery::Manager::new().ok()?;
    let values: Vec<_> = manager
        .batteries()
        .ok()?
        .filter_map(Result::ok)
        .map(|b| Battery {
            percentage: f64::from(b.state_of_charge().value) * 100.0,
            state: format!("{:?}", b.state()),
        })
        .collect();
    (!values.is_empty()).then_some(Measurement::Batteries(values))
}
fn displays() -> Option<Measurement> {
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return None;
    }
    let displays = display_info::DisplayInfo::all().ok()?;
    let values: Vec<_> = displays
        .iter()
        .map(|d| format!("{}×{}", d.width, d.height))
        .collect();
    (!values.is_empty()).then(|| Measurement::Text(values.join(", ")))
}
