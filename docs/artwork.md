# Bundled artwork

All images are embedded in installed builds. Use `image = "auto"` for the exact
OS/distribution match, always variant 1. If no exact match exists, Blink uses
`blink-1`. No random choice or parent-distribution fallback occurs.

Use `image = "arch-2"` or `image = "arch-2.png"` to pick any image on any OS.
`blink --list-images` lists names, descriptions, and defaults. Generic `linux-1`
(Tux) and decorative `firefly-1` are manually selectable. The four Blink images
are the mountain eye, airship, observatory terminal, and submarine, respectively.

The image contents and transparency are unchanged. Original SHA-256 digests
are recorded in `tests/fixtures/artwork.json`. All 30 originals total about 84 MiB
and are embedded without resizing, increasing executable size accordingly.

## Renamed files

Original bare filenames are accepted as catalog aliases. Filesystem paths containing
a directory are treated as actual paths: update references to renamed repository
files using this table. User config files are never rewritten.

| Selectable name | Original filename |
| --- | --- |
| [`alpine-1`](../assets/alpine-1.png) | Retro Alpine Mountain Emblem-6.png |
| [`arch-1`](../assets/arch-1.png) | Pixel-Engraved Arch Observatory Shrine.png |
| [`arch-2`](../assets/arch-2.png) | Pixel Astronaut Among Planets-3.png |
| [`blink-1`](../assets/blink-1.png) | Blink Eye Over Mountain Landscape.png |
| [`blink-2`](../assets/blink-2.png) | Steampunk Blink Airship at Sunset-1.png |
| [`blink-3`](../assets/blink-3.png) | Pixel-Art Observatory Terminal-2.png |
| [`blink-4`](../assets/blink-4.png) | Retro Deep-Sea Blink Submersible-3.png |
| [`debian-1`](../assets/debian-1.png) | Debian Mountain Terminal Pixel Art.png |
| [`debian-2`](../assets/debian-2.png) | Pixel Art Library Desk Vignette.png |
| [`endeavouros-1`](../assets/endeavouros-1.png) | Pixel-Art Spaceship Over a Ringed Planet.png |
| [`fedora-1`](../assets/fedora-1.png) | Retro Desert Observatory Engraving-2.png |
| [`firefly-1`](../assets/firefly-1.png) | Glowing Steampunk Firefly and Wildflowers.png |
| [`freebsd-1`](../assets/freebsd-1.png) | Retro BSD Imp at Vintage Terminal.png |
| [`kali-1`](../assets/kali-1.png) | Crimson Moon Dragon Pagoda-5.png |
| [`linux-1`](../assets/linux-1.png) | Pixel-Art Tux at Sunset.png |
| [`linuxmint-1`](../assets/linuxmint-1.png) | Linux Mint Garden Terrace Etching.png |
| [`linuxmint-2`](../assets/linuxmint-2.png) | Pixel Art Linux Mint Sunset Cabin.png |
| [`macos-1`](../assets/macos-1.png) | Retro Macintosh Sunset Garden.png |
| [`manjaro-1`](../assets/manjaro-1.png) | Mossy Manjaro Temple Under a Golden Moon.png |
| [`nixos-1`](../assets/nixos-1.png) | Embroidered Alchemy Forge Emblem.png |
| [`omarchy-1`](../assets/omarchy-1.png) | omarchy-1.png |
| [`omarchy-2`](../assets/omarchy-2.png) | omarchy-2.png |
| [`omarchy-3`](../assets/omarchy-3.png) | omarchy-3.png |
| [`openbsd-1`](../assets/openbsd-1.png) | Puffy’s Sunken Treasure Wreck-7.png |
| [`opensuse-1`](../assets/opensuse-1.png) | openSUSE Steampunk Station.png |
| [`opensuse-2`](../assets/opensuse-2.png) | Pixel Chameleon at Sunset Lake.png |
| [`popos-1`](../assets/popos-1.png) | Retro Pop!_OS Space Cockpit-4.png |
| [`ubuntu-1`](../assets/ubuntu-1.png) | Transparent Ubuntu Cyberpunk Cityscape.png |
| [`windows-1`](../assets/windows-1.png) | Transparent Windows Desk Cutout.png |
| [`windows-2`](../assets/windows-2.png) | Cozy Retro Desktop in the Pines.png |

`builtin` remains an alias for automatic selection. To keep the previous astronaut,
select `arch-2` explicitly. Matching uses native OS/distribution identifiers; Pop!_OS
and openSUSE Leap/Tumbleweed normalize to their respective catalog names. Unknown
Linux distributions, generic Linux without a recognized distribution, and other
unsupported operating systems use Blink branding.

## Custom artwork

Custom raster paths retain relative-to-config and `~` expansion. A recognized bare
catalog filename selects the bundled asset; `./arch-1.png` forces a local file.
ASCII art accepts `ascii = "path/to/art.txt"` or an inline `ascii = { text = ... }`
literal string. See the [README](../README.md#images-and-terminal-support) and
[ASCII example](../examples/ascii.toml) for styling and whitespace behavior.

Omarchy defaults to `omarchy-1`; select `omarchy-2` or `omarchy-3` explicitly for
another variant. Older installations that report `ID=arch` are recognized through
a nonempty `version` file and `bin/omarchy-version` under `/usr/share/omarchy`,
`$OMARCHY_PATH`, or `~/.local/share/omarchy`. A theme/config folder alone does not
change Arch's artwork. These locations follow Omarchy's
[native runtime layout](https://github.com/omacom/omarchy/blob/quattro/docs/file-layout.md).
