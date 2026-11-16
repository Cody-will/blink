use super::{
    assets::{Assets, ascii},
    catalog,
};
use crate::{
    components::system::Snapshot,
    config::Config,
    content,
    layout::{node::Kind, rect::Rect},
    render, responsive,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "blink-art-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.0.join(name), bytes).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn resolve(text: &str, base: &Path, assets: &mut Assets) -> content::Resolved {
    content::resolve(
        &Config::parse(text).unwrap(),
        &Snapshot::default(),
        base,
        assets,
    )
    .unwrap()
}
#[test]
fn catalog_matches_manifest_and_every_original_image_decodes() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/artwork.json")).unwrap();
    let manifest = manifest.as_array().unwrap();
    assert_eq!(manifest.len(), 30);
    assert_eq!(catalog::ARTWORK.len(), manifest.len());
    let mut ids = BTreeSet::new();
    for (art, old) in catalog::ARTWORK.iter().zip(manifest) {
        assert!(ids.insert(art.id));
        assert_eq!(art.id, old["id"]);
        assert_eq!(art.original, old["original"]);
        assert_eq!(
            art.bytes,
            std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("assets/{}.png", art.id))
            )
            .unwrap()
        );
        let mut assets = Assets::default();
        let image = assets
            .load(Some(art.id), Path::new("/not/the/repository"))
            .unwrap();
        assert!(image.width() > 0 && image.height() > 0);
        for alias in [format!("{}.png", art.id), art.original.to_owned()] {
            assert!(Rc::ptr_eq(
                &image,
                &assets.load(Some(&alias), Path::new("/unrelated")).unwrap()
            ));
        }
        assert_eq!(assets.decoded_count(), 1);
        assert!(assets.warnings.is_empty());
    }
}
#[test]
fn exact_os_matches_choose_variant_one_and_unknown_systems_use_blink() {
    for art in catalog::ARTWORK
        .iter()
        .filter(|a| a.os.is_some() && a.id.ends_with("-1"))
    {
        let os = art.os.unwrap();
        assert_eq!(catalog::for_os(os).id, art.id);
        assert_eq!(catalog::for_os(&os.to_uppercase()).id, art.id);
    }
    for (os, expected) in [
        ("pop", "popos-1"),
        ("pop!_os", "popos-1"),
        ("opensuse-tumbleweed", "opensuse-1"),
        ("opensuse-leap", "opensuse-1"),
        ("darwin", "macos-1"),
        ("archlinux", "arch-1"),
    ] {
        assert_eq!(catalog::for_os(os).id, expected);
    }
    for os in [
        "",
        "linux",
        "unknown",
        "arch-derivative",
        "centos",
        "android",
    ] {
        assert_eq!(catalog::for_os(os).id, "blink-1");
    }
    let mut assets = Assets::default();
    let auto = assets.load(None, Path::new(".")).unwrap();
    for selector in ["auto", "builtin"] {
        assert!(Rc::ptr_eq(
            &auto,
            &assets.load(Some(selector), Path::new(".")).unwrap()
        ));
    }
    let selected = catalog::automatic();
    assert!(Rc::ptr_eq(
        &auto,
        &assets.load(Some(selected.id), Path::new(".")).unwrap()
    ));
    assert_eq!(assets.decoded_count(), 1);
}
#[test]
fn explicit_file_paths_override_catalog_names_and_custom_formats_still_load() {
    let f = Fixture::new();
    image::DynamicImage::new_rgba8(3, 2)
        .save(f.0.join("arch-1.png"))
        .unwrap();
    assert!(catalog::find("./arch-1.png").is_none());
    assert!(catalog::find("folder\\arch-1.png").is_none());
    let mut assets = Assets::default();
    let bundled = assets.load(Some("arch-1.png"), &f.0).unwrap();
    let file = assets.load(Some("./arch-1.png"), &f.0).unwrap();
    assert_eq!((file.width(), file.height()), (3, 2));
    assert_ne!(bundled.width(), file.width());
    assert!(Rc::ptr_eq(
        &file,
        &assets
            .load(f.0.join("arch-1.png").to_str(), Path::new("/elsewhere"))
            .unwrap()
    ));
    assert_eq!(assets.decoded_count(), 2);
    assert!(assets.load(Some("nonexistent-1"), &f.0).is_none());
    assert!(assets.load(Some("nonexistent-1"), &f.0).is_none());
    assert_eq!(assets.warnings.len(), 1);
}
#[test]
fn image_and_ascii_configuration_exports_losslessly_and_rejects_conflicts() {
    for source in [
        "image='auto'",
        "image='builtin'",
        "image='arch-2'",
        "image='arch-2.png'",
        "image='./arch-1.png'",
        "type='image'\npath='auto'",
        "type='image'\npath='builtin'",
        "ascii='~/art.txt'",
        "ascii={text='  ART  '}",
        "ascii={text=''}",
        "type='ascii'\ncontent='ABC'",
        "type='ascii'\npath='art.txt'",
    ] {
        let cfg = Config::parse(&format!("[layout.first]\n{source}")).unwrap();
        let exported = cfg.editable_toml().unwrap();
        assert_eq!(
            toml::Value::try_from(&cfg).unwrap(),
            toml::Value::try_from(Config::parse(&exported).unwrap()).unwrap(),
            "{source}"
        );
    }
    for source in [
        "ascii='art.txt'\nimage='auto'",
        "ascii='art.txt'\npath='other.txt'",
        "ascii={text='a',other='b'}",
        "ascii=3",
        "ascii={file='a'}",
        "type='ascii'\ncontent='art'\npath='art.txt'",
        "ascii='art.txt'\nchildren=[{type='text',content='child'}]",
        "type='ascii'\n[layout.first.image]\nfit='cover'",
    ] {
        assert!(
            Config::parse(&format!("[layout.first]\n{source}")).is_err(),
            "{source}"
        );
    }
    let defaults = Config::default().editable_toml().unwrap();
    assert!(defaults.contains("image = \"auto\""));
    assert!(defaults.lines().count() < 50);
}
#[test]
fn ascii_preserves_art_whitespace_and_expands_tabs_after_sanitizing_controls() {
    assert_eq!(
        ascii("\u{feff}  X  \r\n\r\n界\tZ\n"),
        "  X  \n\n界      Z\n"
    );
    assert_eq!(ascii("\x1b[31mABC\x1b[0m\tD\x1b[2J\x07\r"), "ABC     D");
    assert_eq!(ascii("👨‍👩‍👧‍👦\tX"), "👨‍👩‍👧‍👦      X");
    assert_eq!(ascii("\x1b]0;title\x07ART"), "ART");
}
#[test]
fn ascii_files_cache_and_invalid_sources_warn_once_without_panicking() {
    let f = Fixture::new();
    f.write("art.txt", b"  A  \r\n\nB\tC\n");
    let mut assets = Assets::default();
    let text = assets.load_ascii("art.txt", &f.0).unwrap();
    assert_eq!(&*text, "  A  \n\nB       C\n");
    assert!(Rc::ptr_eq(
        &text,
        &assets.load_ascii("./art.txt", &f.0).unwrap()
    ));
    f.write("invalid.txt", &[0xff, 0xfe]);
    std::fs::File::create(f.0.join("large.txt"))
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    for name in ["missing.txt", "invalid.txt", "large.txt"] {
        assert!(assets.load_ascii(name, &f.0).is_none());
        assert!(assets.load_ascii(name, &f.0).is_none());
    }
    assert_eq!(assets.warnings.len(), 3);
    let cfg = "[layout.first]\nascii='missing.txt'\n[layout.second]\nchildren=[]";
    assert!(render::plain(&resolve(cfg, &f.0, &mut assets)).contains("[art unavailable]"));
    assert_eq!(assets.warnings.len(), 3);
}
#[test]
fn ascii_layout_styles_and_plain_output_preserve_content_at_small_sizes() {
    let text = "[layout.first]\nascii={text='''  界  \n\nEND  \n'''}\n[layout.first.style]\nalign_h='center'\nalign_v='end'\n[layout.first.text_style]\nforeground='#123456'\n[layout.second]\nchildren=[]";
    let mut cfg = Config::parse(text).unwrap();
    content::prepare(&mut cfg, 1, 1, true, false, true);
    let root = content::resolve(
        &cfg,
        &Snapshot::default(),
        Path::new("."),
        &mut Assets::default(),
    )
    .unwrap();
    let art = &root.children[0].children[0];
    assert_eq!(art.kind, Kind::Ascii);
    assert_eq!(art.measure(), (6, 4));
    assert_eq!(art.lines[0][0].1.foreground, "#123456");
    assert!(render::plain(&root).ends_with("  界  \n\nEND  \n\n"));
    let options = render::RenderOptions {
        backend: crate::config::Backend::Text,
        color: false,
        cell: crate::terminal::CellPixels {
            width: 8,
            height: 16,
        },
        passthrough: false,
    };
    for (w, h) in [(20, 10), (3, 2), (1, 1), (0, 0)] {
        let out = render::compose(&root.place(Rect::new(0, 0, w, h)), &options).unwrap();
        use unicode_width::UnicodeWidthStr;
        for chunk in std::str::from_utf8(&out).unwrap().split("\x1b[") {
            if let Some((_, line)) = chunk.split_once('H') {
                assert!(line.width() <= usize::from(w));
            }
        }
    }
}
#[test]
fn optional_ascii_sections_are_removed_to_make_responsive_modules_fit() {
    let cfg =
        Config::parse("[layout.first]\nascii={text='ART'}\nsize='35%'\noptional=true").unwrap();
    let mut root = content::resolve(
        &cfg,
        &Snapshot::default(),
        Path::new("."),
        &mut Assets::default(),
    )
    .unwrap();
    assert_eq!(root.children.len(), 2);
    responsive::adapt(&mut root, Rect::new(0, 0, 80, 4), true);
    assert_eq!(root.children.len(), 1);
    assert!(!render::plain(&root).contains("ART"));
}

#[test]
fn omarchy_runtime_markers_override_arch_only_for_complete_installations() {
    let f = Fixture::new();
    let paths = vec![f.0.clone()];
    assert_eq!(catalog::for_installation("omarchy", &[]).id, "omarchy-1");
    assert_eq!(catalog::for_installation("arch", &paths).id, "arch-1");
    f.write("version", b"3.8.5\n");
    assert_eq!(catalog::for_installation("arch", &paths).id, "arch-1");
    std::fs::create_dir(f.0.join("bin")).unwrap();
    f.write("bin/omarchy-version", b"#!/bin/bash\n");
    assert_eq!(catalog::for_installation("arch", &paths).id, "omarchy-1");
    assert_eq!(
        catalog::for_installation("ARCHLINUX", &paths).id,
        "omarchy-1"
    );
    assert_eq!(catalog::for_installation("ubuntu", &paths).id, "ubuntu-1");
    assert_eq!(catalog::for_installation("unknown", &paths).id, "blink-1");
    f.write("version", b"");
    assert_eq!(catalog::for_installation("arch", &paths).id, "arch-1");
}
