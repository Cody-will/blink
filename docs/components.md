# Components and native detection

Blink covers the 29 entries in [Fastfetch's default structure](https://github.com/fastfetch-cli/fastfetch/blob/dev/CMakeLists.txt),
recorded from Fastfetch 2.71 in `tests/fixtures/fastfetch-2.71-defaults.txt`.
This is default-list coverage, not Fastfetch's entire optional module catalog or
identical detection coverage on every platform. Blink does not require Fastfetch.

All entries below are enabled in the generated configuration. Hostname and CPU
core count are additional default rows. Unavailable entries are hidden; use
`[display] unavailable = "show"` to show `N/A`. Responsive terminal output may
omit lower-priority rows when space runs out. Plain output retains all available
requested information. Existing configuration files are never rewritten to add rows.

| Fastfetch default | Blink name | Native source / behavior |
| --- | --- | --- |
| Title | `title` | Native username and hostname, rendered as `user@host` |
| Separator | `separator` | Separator matching the title's terminal display width |
| OS | `os` | sysinfo operating-system name/version |
| Host | `host` | Linux DMI vendor/product/version; device-tree model fallback; no serial numbers |
| Kernel | `kernel` | sysinfo kernel version |
| Uptime | `uptime` | sysinfo uptime |
| Packages | `packages` | Native package databases listed below |
| Shell | `shell` | Recognized parent shell on Linux, then session environment |
| Display | `display` | display-info native APIs, where exposed |
| DE | `desktop` | Explicit desktop-session environment |
| WM | `window_manager` | Explicit session evidence |
| WMTheme | `wm_theme` | Active KDE, Xfce, Openbox, or GNOME settings |
| Theme | `desktop_theme` | GTK settings; active KDE or GNOME settings |
| Icons | `icon_theme` | GTK settings; active KDE, Xfce, or GNOME settings |
| Font | `font` | GTK settings; active KDE, Xfce, or GNOME settings |
| Cursor | `cursor` | GTK settings; active KDE, Xfce, or GNOME settings; size when known |
| Terminal | `terminal` | Session environment |
| TerminalFont | `terminal_font` | Supported terminal configurations/profiles listed below |
| CPU | `cpu` | sysinfo model |
| GPU | `gpu` | sysinfo native GPU APIs, driver/platform dependent |
| Memory | `memory` | sysinfo used/total RAM and valid percentage, binary units |
| Swap | `swap` | sysinfo used/total swap; zero total means not configured |
| Disk | `disk` | sysinfo mounted filesystems, used/total capacity |
| LocalIp | `local_ip` | Native operational interfaces; IPv4/IPv6 excluding loopback/link-local; no remote request |
| Battery | `battery` | Native battery library, percentage and charging state |
| PowerAdapter | `power_adapter` | Linux power-supply connection state; measured watts only when exposed |
| Locale | `locale` | First nonempty `LC_ALL`, `LC_MESSAGES`, then `LANG` |
| Break | `break` | Blank presentation row |
| Colors | `colors` | Two rows of eight ANSI indexed colors; omitted in plain/no-color output |

Names and aliases are case-insensitive; exporting config writes canonical names.
`host` means hardware model; `hostname` means network/system hostname.
`desktop_theme` describes the desktop, independently of Blink's own `theme` setting.
`colors` uses ANSI colors; the existing `swatches` node uses Blink's theme palette.

## Package databases

Supported databases are pacman local `desc` entries (including configured
`DBPath`/`RootDir`), dpkg installed status records, RPM SQLite (read-only), system
and user Flatpak apps/runtimes, and Snap sequence JSON. Flatpak excludes `.Locale`
and `.Debug` extension runtimes. A missing/unreadable/unsupported database is
unavailable, rather than a fabricated zero; a readable empty database is zero.
Counts are labeled by manager and may differ from other tools' extension/runtime
policies. RPM Berkeley DB, Nix, APK, Homebrew, Windows package managers, and custom
Flatpak installations are not implemented.

## Appearance and terminal fonts

Linux GNOME uses GIO/GSettings directly, checking schemas/keys first. Other
appearance sources are GTK settings files, KDE `kdeglobals`/`kcminputrc`/`kwinrc`,
Xfce xfconf XML, and Openbox XML. Desktop-specific files are used only for matching
sessions. GTK files may describe toolkit settings in another desktop. Compositor
appearance without a reliable source remains unavailable; Blink never invents a
default theme or font.

Terminal font sources: Kitty explicit family/size with static includes;
Alacritty TOML with imports; Ghostty explicit family/size; Foot explicit font;
Konsole named/default profile; GNOME Terminal default profile or system monospace
font. Include depth is bounded. Kitty dynamic includes, Ghostty `config-file`,
Foot includes, other terminals, and absent explicit font settings remain unavailable.
These describe configuration, not a live font query: command-line overrides,
on-the-fly changes, and nondefault running profiles may differ. No subprocesses
are launched to read desktop or terminal settings.

Linux is the primary tested platform. Portable existing metrics, username,
interfaces, and swap use native libraries on supported systems. Linux-specific
hardware/appearance/package detection does not claim macOS/Windows parity.
Power readings are current draw, not the adapter's advertised maximum wattage.
