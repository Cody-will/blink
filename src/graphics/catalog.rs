//! Stable bundled artwork identifiers. The original filename remains a selectable alias.
pub struct Artwork {
    pub id: &'static str,
    pub original: &'static str,
    pub description: &'static str,
    pub os: Option<&'static str>,
    pub bytes: &'static [u8],
}
macro_rules! artwork {
    ($(($id:literal, $original:literal, $description:literal, $os:expr)),* $(,)?) => {
        pub static ARTWORK: &[Artwork] = &[$(Artwork {
            id: $id, original: $original, description: $description, os: $os,
            bytes: include_bytes!(concat!("../../assets/", $id, ".png")),
        }),*];
    };
}
artwork! {
    ("alpine-1", "Retro Alpine Mountain Emblem-6.png", "Alpine mountain emblem", Some("alpine")),
    ("arch-1", "Pixel-Engraved Arch Observatory Shrine.png", "Arch observatory shrine", Some("arch")),
    ("arch-2", "Pixel Astronaut Among Planets-3.png", "Astronaut with Arch shoulder logo", Some("arch")),
    ("blink-1", "Blink Eye Over Mountain Landscape.png", "Blink eye over mountains", None),
    ("blink-2", "Steampunk Blink Airship at Sunset-1.png", "Blink airship", None),
    ("blink-3", "Pixel-Art Observatory Terminal-2.png", "Blink observatory terminal", None),
    ("blink-4", "Retro Deep-Sea Blink Submersible-3.png", "Blink submarine", None),
    ("debian-1", "Debian Mountain Terminal Pixel Art.png", "Debian mountain terminal", Some("debian")),
    ("debian-2", "Pixel Art Library Desk Vignette.png", "Library with Debian swirl", Some("debian")),
    ("endeavouros-1", "Pixel-Art Spaceship Over a Ringed Planet.png", "EndeavourOS spaceship", Some("endeavouros")),
    ("fedora-1", "Retro Desert Observatory Engraving-2.png", "Fedora desert observatory", Some("fedora")),
    ("firefly-1", "Glowing Steampunk Firefly and Wildflowers.png", "Decorative steampunk firefly", None),
    ("freebsd-1", "Retro BSD Imp at Vintage Terminal.png", "FreeBSD imp at terminal", Some("freebsd")),
    ("kali-1", "Crimson Moon Dragon Pagoda-5.png", "Kali dragon and crimson moon", Some("kali")),
    ("linux-1", "Pixel-Art Tux at Sunset.png", "Generic Linux Tux at sunset", None),
    ("linuxmint-1", "Linux Mint Garden Terrace Etching.png", "Linux Mint garden terrace", Some("linuxmint")),
    ("linuxmint-2", "Pixel Art Linux Mint Sunset Cabin.png", "Linux Mint sunset cabin", Some("linuxmint")),
    ("macos-1", "Retro Macintosh Sunset Garden.png", "Macintosh sunset garden", Some("macos")),
    ("manjaro-1", "Mossy Manjaro Temple Under a Golden Moon.png", "Manjaro mossy temple", Some("manjaro")),
    ("nixos-1", "Embroidered Alchemy Forge Emblem.png", "NixOS alchemy forge", Some("nixos")),
    ("omarchy-1", "omarchy-1.png", "Omarchy cyberpunk city", Some("omarchy")),
    ("omarchy-2", "omarchy-2.png", "Omarchy observatory", Some("omarchy")),
    ("omarchy-3", "omarchy-3.png", "Omarchy desktop workshop", Some("omarchy")),
    ("openbsd-1", "Puffy’s Sunken Treasure Wreck-7.png", "OpenBSD Puffy treasure wreck", Some("openbsd")),
    ("opensuse-1", "openSUSE Steampunk Station.png", "openSUSE steampunk station", Some("opensuse")),
    ("opensuse-2", "Pixel Chameleon at Sunset Lake.png", "openSUSE chameleon at sunset lake", Some("opensuse")),
    ("popos-1", "Retro Pop!_OS Space Cockpit-4.png", "Pop!_OS space cockpit", Some("popos")),
    ("ubuntu-1", "Transparent Ubuntu Cyberpunk Cityscape.png", "Ubuntu cyberpunk cityscape", Some("ubuntu")),
    ("windows-1", "Transparent Windows Desk Cutout.png", "Windows desk cutout", Some("windows")),
    ("windows-2", "Cozy Retro Desktop in the Pines.png", "Windows desktop in the pines", Some("windows")),
}
/// Directory-bearing selectors are always custom paths, even if their basename matches.
pub fn find(selector: &str) -> Option<&'static Artwork> {
    if selector.contains(['/', '\\']) {
        return None;
    }
    let id = selector.strip_suffix(".png").unwrap_or(selector);
    ARTWORK
        .iter()
        .find(|a| a.id == id || a.original == selector)
}
/// Exact OS matches only; unknown distributions deliberately use Blink branding.
pub fn for_os(os: &str) -> &'static Artwork {
    let os = os.trim().to_ascii_lowercase();
    let os = match os.as_str() {
        "pop" | "pop_os" | "pop!_os" => "popos",
        "opensuse-leap" | "opensuse-tumbleweed" => "opensuse",
        "darwin" | "mac" => "macos",
        "archlinux" => "arch",
        os => os,
    };
    ARTWORK
        .iter()
        .find(|a| a.os == Some(os) && a.id.ends_with("-1"))
        .unwrap_or_else(|| {
            ARTWORK
                .iter()
                .find(|a| a.id == "blink-1")
                .unwrap_or(&ARTWORK[0])
        })
}

/// Older Omarchy installations retain Arch's OS ID. Recognize runtime files,
/// rather than treating every Arch derivative or an Omarchy theme as Omarchy.
pub(super) fn for_installation(os: &str, installations: &[std::path::PathBuf]) -> &'static Artwork {
    if matches!(
        os.trim().to_ascii_lowercase().as_str(),
        "arch" | "archlinux"
    ) && installations.iter().any(|dir| {
        dir.join("bin/omarchy-version").is_file()
            && std::fs::metadata(dir.join("version"))
                .is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= 128)
    }) {
        for_os("omarchy")
    } else {
        for_os(os)
    }
}
pub fn automatic() -> &'static Artwork {
    #[cfg(target_os = "linux")]
    let installations = {
        let mut paths = vec![std::path::PathBuf::from("/usr/share/omarchy")];
        if let Some(path) = std::env::var_os("OMARCHY_PATH").filter(|p| !p.is_empty()) {
            paths.push(path.into());
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".local/share/omarchy"));
        }
        paths
    };
    #[cfg(not(target_os = "linux"))]
    let installations = vec![];
    for_installation(&sysinfo::System::distribution_id(), &installations)
}
