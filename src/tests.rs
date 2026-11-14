use super::*;
use crate::{
    components::system::{Measurement, Snapshot, bytes, percentage},
    config::{IconMode, Unavailable},
    content::Resolved,
    graphics::{ImageRenderer, Iterm2, Kitty, Prepared, Sixel},
    layout::{
        Size,
        node::{Fit, ImageOptions, Kind},
    },
    styles::{Alignment, BorderType, ContainerStyleConfig, Padding},
    terminal::{Capabilities, CellPixels},
};
use image::DynamicImage;
use rusty_x_ansi::util::strip;
use std::{collections::BTreeSet, path::Path, rc::Rc};
use unicode_width::UnicodeWidthStr;
fn snapshot() -> Snapshot {
    Snapshot {
        values: std::collections::BTreeMap::from([
            ("os".into(), Measurement::Text("Test OS".into())),
            ("kernel".into(), Measurement::Text("6.0".into())),
            ("cpu".into(), Measurement::Text("Test CPU".into())),
            (
                "memory".into(),
                Measurement::Memory {
                    used: 8 << 30,
                    total: 32 << 30,
                },
            ),
        ]),
    }
}
fn resolve(text: &str) -> Resolved {
    let cfg = Config::parse(text).unwrap();
    content::resolve(
        &cfg,
        &snapshot(),
        Path::new("."),
        &mut graphics::assets::Assets::default(),
    )
    .unwrap()
}
fn leaf(size: Size) -> Resolved {
    let mut n = Resolved::empty(ContainerStyleConfig::plain());
    n.size = size;
    n.lines = vec![vec![("abc".into(), styles::TextStyleConfig::default())]];
    n
}
fn options(backend: Backend) -> render::RenderOptions {
    render::RenderOptions {
        backend,
        color: true,
        cell: CellPixels {
            width: 8,
            height: 16,
        },
        passthrough: false,
    }
}
#[test]
fn nested_defaults_preserve_styles_and_node_type() {
    let cfg=Config::parse("[styles.label]\nitalic=true\n[styles.window.padding]\nleft=3\n[layout.first]\nsize={percent=40}").unwrap();
    assert!(cfg.styles.label.bold && cfg.styles.label.italic);
    assert_eq!(cfg.styles.window.padding.right, 1);
    assert_eq!(cfg.layout.first.kind, Kind::Image);
    assert_eq!(cfg.layout.first.size, Size::Percent(40));
}
#[test]
fn changed_type_and_empty_children_replace_defaults() {
    let cfg =
        Config::parse("[layout.first]\ntype='text'\ncontent='hello'\n[layout.second]\nchildren=[]")
            .unwrap();
    assert_eq!(cfg.layout.first.size, Size::Fill);
    assert!(cfg.layout.second.children.is_empty());
}
#[test]
fn complete_default_config_roundtrips() {
    let cfg = Config::parse(&toml::to_string_pretty(&Config::default()).unwrap()).unwrap();
    assert_eq!(cfg.theme, "gruvbox-material");
    assert_eq!(cfg.layout.second.children[0].children[0].name, "os");
}
#[test]
fn config_rejects_invalid_fields_values_and_colors() {
    for text in [
        "version=2",
        "reserved_rows=-1",
        "graphics='fake'",
        "theme='unknown'",
        "unknown=1",
        "[styles.text]\nbodl=true",
        "[styles.text]\nforeground='#bg'",
        "[styles.label]\nforeground='@unknown'",
        "[palette]\naccent='invalid'",
        "[layout.first]\nsize={percent=101}",
        "[layout.first]\ntype='component'\nname='unknown'",
        "[layout.first]\ntype='text'\nchildren=[{type='text'}]",
        "[layout.first.style]\nunknown=1",
        "[display]\ncell_width=0",
    ] {
        assert!(Config::parse(text).is_err(), "accepted: {text}");
    }
}
#[test]
fn themes_and_palette_overrides_are_consistent() {
    for name in config::theme::NAMES {
        let cfg = Config::parse(&format!("theme='{name}'")).unwrap();
        assert!(cfg.colors().unwrap().fg.starts_with('#'));
    }
    let cfg = Config::parse(
        "theme='nord'\n[palette]\naccent='#123'\n[styles.text]\nforeground='@accent'",
    )
    .unwrap();
    assert_eq!(
        cfg.styles
            .text
            .colors(&cfg.colors().unwrap())
            .unwrap()
            .foreground,
        "#112233"
    );
}
#[test]
fn presets_expand_and_unknown_presets_fail() {
    let cfg = Config::parse("[layout.first]\ntype='module'\npreset='storage'").unwrap();
    assert_eq!(cfg.layout.first.children[0].name, "disk");
    assert!(Config::parse("[layout.first]\ntype='module'\npreset='fake'").is_err());
}
#[test]
fn original_config_converts_without_losing_style_or_layout() {
    let cfg = Config::parse(
        r##"
name="legacy"
theme="gruvbox"
[window.container.padding]
left=2
[first_section.container.border]
kind="none"
[second_section.container.border]
kind="none"
[module.container.border]
kind="double"
[layout.horizontal.left]
size={percent=40}
child={type="image",path="logo.png"}
[layout.horizontal.right]
size="fill"
child={type="column",gap=1,children=[]}
"##,
    )
    .unwrap();
    assert_eq!(cfg.name, "legacy");
    assert_eq!(cfg.styles.window.padding.left, 2);
    assert_eq!(cfg.layout.first.path.as_deref(), Some("logo.png"));
    assert!(matches!(cfg.styles.module.border.kind, BorderType::Double));
}
#[test]
fn unmappable_legacy_settings_fail_explicitly() {
    assert!(
        Config::parse("[module]\ngap=2")
            .unwrap_err()
            .to_string()
            .contains("gap")
    );
}
#[test]
fn missing_metrics_render_na_or_hide() {
    let text = "[layout.first]\ntype='module'\nchildren=[{type='component',name='battery',size='auto'}]\n[layout.second]\nchildren=[]";
    let mut cfg = Config::parse(text).unwrap();
    let root = content::resolve(
        &cfg,
        &Snapshot::default(),
        Path::new("."),
        &mut graphics::assets::Assets::default(),
    )
    .unwrap();
    assert!(render::plain(&root).contains("N/A"));
    cfg.display.unavailable = Unavailable::Hide;
    let root = content::resolve(
        &cfg,
        &Snapshot::default(),
        Path::new("."),
        &mut graphics::assets::Assets::default(),
    )
    .unwrap();
    assert!(!render::plain(&root).contains("Battery"));
}
#[test]
fn unit_formatting_and_zero_totals_are_safe() {
    assert_eq!(bytes(1024), "1.0 KiB");
    assert_eq!(bytes(8 << 30), "8.0 GiB");
    assert_eq!(percentage(1, 0), None);
    assert_eq!(percentage(1, 4), Some(25.0));
    assert_eq!(percentage(5, 4), Some(100.0));
}
#[test]
fn requested_metrics_ignore_hidden_and_static_components() {
    let cfg=Config::parse("[layout.first]\ntype='column'\nchildren=[{type='component',name='cpu_usage',visible=false},{type='component',name='os',value='static'},{type='component',name='memory'}]\n[layout.second]\nchildren=[]").unwrap();
    assert_eq!(
        Snapshot::requested([&cfg.layout.first, &cfg.layout.second]),
        BTreeSet::from(["memory".into()])
    );
}
#[test]
fn label_columns_align_unicode_without_manual_spaces() {
    let root = resolve(
        "[layout.first]\ntype='module'\nchildren=[{type='component',value='first',label='界',icon='X'},{type='component',value='second',label='Long label',icon=''}]\n[layout.second]\nchildren=[]",
    );
    let text = render::plain(&root);
    let a = text.lines().find(|s| s.contains("first")).unwrap();
    let b = text.lines().find(|s| s.contains("second")).unwrap();
    assert_eq!(
        a[..a.find("first").unwrap()].width(),
        b[..b.find("second").unwrap()].width()
    );
}
#[test]
fn static_text_visibility_and_multiline_values_work() {
    let root = resolve(
        "[layout.first]\ntype='module'\nchildren=[{type='component',value='one\\ntwo',label='Custom'},{type='text',content='hidden',visible=false}]\n[layout.second]\nchildren=[]",
    );
    assert!(!render::plain(&root).contains("hidden"));
}
#[test]
fn icons_have_safe_modes_and_can_be_disabled() {
    assert!(components::icon("cpu", IconMode::Ascii).is_ascii());
    assert_ne!(
        components::icon("cpu", IconMode::Unicode),
        components::icon("cpu", IconMode::Nerd)
    );
    let mut cfg = Config::parse(
        "[layout.first]\ntype='component'\nname='os'\nicon='custom'\n[layout.second]\nchildren=[]",
    )
    .unwrap();
    cfg.icons.enabled = false;
    let root = content::resolve(
        &cfg,
        &snapshot(),
        Path::new("."),
        &mut graphics::assets::Assets::default(),
    )
    .unwrap();
    assert!(!render::plain(&root).contains("custom"));
}
#[test]
fn allocator_reserves_requests_before_fill() {
    let nodes = [
        leaf(Size::Fill),
        leaf(Size::Fixed(10)),
        leaf(Size::Percent(25)),
        leaf(Size::Auto),
        leaf(Size::Fill),
    ];
    let rects =
        layout::engine::allocate(&nodes, Direction::Horizontal, 1, Rect::new(0, 0, 104, 10));
    assert_eq!(
        rects.iter().map(|r| r.width).collect::<Vec<_>>(),
        vec![31, 10, 25, 3, 31]
    );
}
#[test]
fn fill_and_overflow_stay_bounded_in_both_axes() {
    for direction in [Direction::Horizontal, Direction::Vertical] {
        for length in 0..20 {
            let rects = layout::engine::allocate(
                &[
                    leaf(Size::Fixed(100)),
                    leaf(Size::Fill),
                    leaf(Size::Percent(80)),
                ],
                direction,
                3,
                Rect::new(0, 0, length, length),
            );
            assert!(
                rects
                    .iter()
                    .all(|r| r.x + r.width <= length && r.y + r.height <= length)
            );
        }
    }
}
#[test]
fn measurement_accounts_for_border_padding_and_unicode() {
    let mut n = leaf(Size::Auto);
    n.lines[0][0].0 = "界x".into();
    n.container = ContainerStyleConfig {
        padding: Padding::all(1),
        ..Default::default()
    };
    assert_eq!(n.measure(), (7, 5));
}
#[test]
fn border_types_and_hidden_geometry_parse() {
    for kind in ["none", "normal", "rounded", "thick", "double", "hidden"] {
        let cfg = Config::parse(&format!("[styles.module.border]\nkind='{kind}'")).unwrap();
        assert_eq!(cfg.styles.module.border.inset(), u16::from(kind != "none"));
    }
}
#[test]
fn block_alignment_uses_assigned_region() {
    let mut n = leaf(Size::Auto);
    n.container.align_h = Alignment::End;
    n.container.align_v = Alignment::End;
    let mut root = Resolved::empty(ContainerStyleConfig::plain());
    root.children.push(n);
    root.container.align_v = Alignment::End;
    let placed = root.place(Rect::new(0, 0, 20, 10));
    assert_eq!(placed.children[0].area.y, 9);
}
#[test]
fn optional_default_visuals_are_removed_on_small_terminals() {
    let mut cfg = Config::default();
    content::prepare(&mut cfg, 30, 10, false, false, false);
    assert!(!cfg.layout.first.visible);
    assert!(!cfg.layout.second.children[2].visible);
}
#[test]
fn missing_images_degrade_to_placeholders() {
    let mut assets = graphics::assets::Assets::default();
    let cfg = Config::parse("[layout.first]\npath='missing-blink-image.png'").unwrap();
    let root = content::resolve(&cfg, &snapshot(), Path::new("/tmp"), &mut assets).unwrap();
    assert!(render::plain(&root).contains("[image unavailable]"));
    assert_eq!(assets.warnings.len(), 1);
}
#[test]
fn bundled_asset_is_cached_per_invocation() {
    let mut assets = graphics::assets::Assets::default();
    let a = assets.load(None, Path::new(".")).unwrap();
    let b = assets.load(None, Path::new(".")).unwrap();
    assert!(Rc::ptr_eq(&a, &b));
    assert_eq!(assets.decoded_count(), 1);
}
#[test]
fn paths_expand_relative_and_home() {
    assert_eq!(
        graphics::assets::expand("logo.png", Path::new("/tmp/config")).unwrap(),
        Path::new("/tmp/config/logo.png")
    );
    assert!(
        graphics::assets::expand("~/logo.png", Path::new("/tmp"))
            .unwrap()
            .ends_with("logo.png")
    );
    assert!(graphics::assets::expand("~other/logo", Path::new(".")).is_err());
}
#[test]
fn image_contain_cover_and_stretch_use_cell_dimensions() {
    let image = DynamicImage::new_rgba8(100, 100);
    let area = Rect::new(0, 0, 20, 20);
    let cell = CellPixels {
        width: 8,
        height: 16,
    };
    let p = graphics::prepare(&image, area, &ImageOptions::default(), cell).unwrap();
    assert_eq!(p.area, Rect::new(0, 5, 20, 10));
    for fit in [Fit::Cover, Fit::Stretch] {
        let p = graphics::prepare(
            &image,
            area,
            &ImageOptions {
                fit,
                ..Default::default()
            },
            cell,
        )
        .unwrap();
        assert_eq!((p.image.width(), p.image.height()), (160, 320));
        assert_eq!(p.area, area);
    }
    assert!(
        graphics::prepare(
            &image,
            Rect::new(0, 0, 0, 0),
            &ImageOptions::default(),
            cell
        )
        .is_none()
    );
}
#[test]
fn alpha_blends_against_theme_background() {
    let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([255, 0, 0, 128]),
    ));
    let result = graphics::flatten(&image, [0, 0, 255]).to_rgba8();
    assert_eq!(result.get_pixel(0, 0).0, [128, 0, 127, 255]);
}
#[test]
fn kitty_chunks_and_iterm_png_are_complete() {
    let p = Prepared {
        image: DynamicImage::new_rgba8(64, 64),
        area: Rect::new(2, 3, 10, 5),
    };
    let mut output = vec![];
    Kitty.render(&p, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("c=10,r=5,C=1"));
    assert!(text.contains("m=1;"));
    assert!(text.contains("m=0;"));
    assert!(text.ends_with("\x1b\\"));
    let mut output = vec![];
    Iterm2.render(&p, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.starts_with("\x1b]1337;File=inline=1;width=10;height=5;"));
    use base64::Engine;
    let encoded = text.split_once(':').unwrap().1.trim_end_matches("\x1b\\");
    let png = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    assert_eq!(image::load_from_memory(&png).unwrap().width(), 64);
}
#[test]
fn sixel_roundtrips_a_known_image() {
    let p = Prepared {
        image: DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            8,
            6,
            image::Rgba([255, 0, 0, 255]),
        )),
        area: Rect::new(0, 0, 1, 1),
    };
    let mut output = vec![];
    Sixel.render(&p, &mut output).unwrap();
    let decoded = icy_sixel::SixelImage::decode(&output).unwrap();
    assert_eq!(decoded.dimensions(), (8, 6));
    assert_eq!(&decoded.pixels[..4], &[255, 0, 0, 255]);
}
#[test]
fn capability_replies_select_priority_and_do_not_use_names() {
    let mut caps = Capabilities::default();
    caps.features("T1Sx");
    assert_eq!(caps.select(), Backend::Sixel);
    caps.replies(b"\x1b]1337;Capabilities=F\x1b\\");
    assert_eq!(caps.select(), Backend::Iterm2);
    caps.replies(b"\x1b_Gi=314159;OK\x1b\\");
    assert_eq!(caps.select(), Backend::Kitty);
    assert_eq!(
        terminal::backend(
            Backend::Kitty,
            false,
            true,
            true,
            &config::Display::default()
        )
        .unwrap(),
        Backend::Text
    );
}
#[test]
fn component_content_strips_ansi_before_measurement() {
    assert_eq!(content::clean("\x1b[31m界\x1b[0m\t\r"), "界");
}
#[test]
fn renderer_keeps_unicode_and_cursor_within_bounds() {
    let root = resolve(
        "[layout.first]\ntype='module'\nchildren=[{type='text',content='界é👩‍🚀界'}]\n[layout.second]\nchildren=[]",
    );
    for (width, height) in [(0, 0), (1, 1), (3, 2), (20, 8), (81, 23)] {
        let output = render::compose(
            &root.place(Rect::new(0, 0, width, height)),
            &options(Backend::Text),
        )
        .unwrap();
        if width > 0 && height > 0 {
            assert!(output.starts_with(b"\x1b[2J\x1b[H"));
            assert!(output.ends_with(format!("\x1b[{};1H\x1b[J", height + 1).as_bytes()));
        }
        let mut text = String::from_utf8(output).unwrap().replace("\x1b[H", "");
        for row in 2..=height + 1 {
            text = text.replace(&format!("\x1b[{row};1H"), "\n");
        }
        let plain = strip(&text);
        assert!(
            plain.lines().all(|line| line.width() <= usize::from(width)),
            "{plain:?}"
        );
        if width == 81 {
            assert!(plain.lines().last().unwrap().ends_with('╯'));
        }
    }
}
#[test]
fn cli_parses_all_advertised_overrides() {
    let args = Args::try_parse_from([
        "blink",
        "--theme",
        "nord",
        "--layout",
        "vertical",
        "--graphics",
        "sixel",
        "--no-image",
        "--no-icons",
        "--no-color",
        "--plain",
        "--width",
        "30",
        "--height",
        "10",
    ])
    .unwrap();
    assert_eq!(args.layout, Some(Direction::Vertical));
    assert!(args.no_image && args.no_icons && args.no_color && args.plain);
    assert!(Args::try_parse_from(["blink", "--graphics", "bogus"]).is_err());
}

#[test]
fn lip_gloss_container_geometry_matches_allocated_outer_dimensions() {
    let style = ContainerStyleConfig::default()
        .colors(&config::theme::Palette::theme("nord").unwrap())
        .unwrap();
    for (width, height) in [(80, 22), (45, 7), (2, 2), (1, 1)] {
        let decoration = strip(&style.decoration(width, height));
        assert_eq!(decoration.lines().count(), usize::from(height));
        assert!(
            decoration
                .lines()
                .all(|line| line.width() == usize::from(width)),
            "{decoration:?}"
        );
        if width > 2 && height > 2 {
            assert!(decoration.lines().last().unwrap().starts_with('╰'));
        }
    }
}

#[test]
fn native_images_keep_separate_placements() {
    let mut root = Resolved::empty(ContainerStyleConfig::plain());
    root.direction = Direction::Horizontal;
    for _ in 0..2 {
        let mut child = Resolved::empty(ContainerStyleConfig::plain());
        child.image = Some(Rc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255])),
        )));
        root.children.push(child);
    }
    for backend in [Backend::Kitty, Backend::Iterm2, Backend::Sixel] {
        let output =
            render::compose(&root.place(Rect::new(0, 0, 8, 4)), &options(backend)).unwrap();
        let text = String::from_utf8(output).unwrap();
        let marker = match backend {
            Backend::Kitty => "\x1b_Ga=T",
            Backend::Iterm2 => "\x1b]1337;File",
            _ => "\x1bP",
        };
        assert_eq!(text.matches(marker).count(), 2);
        assert!(text.contains("\x1b[2;1H"));
        assert!(text.contains("\x1b[2;5H"));
    }
}
#[test]
fn image_backends_propagate_output_failure() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let p = Prepared {
        image: DynamicImage::new_rgba8(2, 2),
        area: Rect::new(0, 0, 1, 1),
    };
    assert!(Kitty.render(&p, &mut Broken).is_err());
    assert!(Iterm2.render(&p, &mut Broken).is_err());
}
#[test]
fn static_multiline_values_align_continuations() {
    let root = resolve(
        r#"
[layout.first]
type="module"
children=[{type="component",value="one\ntwo",label="Custom",icon=""}]
[layout.second]
children=[]
"#,
    );
    let text = render::plain(&root);
    let rows = text.lines().collect::<Vec<_>>();
    let a = rows.iter().find(|s| s.contains("one")).unwrap();
    let b = rows.iter().find(|s| s.contains("two")).unwrap();
    assert_eq!(a.find("one"), b.find("two"));
}

#[test]
fn common_raster_formats_and_static_gif_frame_load() {
    let directory = std::env::temp_dir().join(format!("blink-formats-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([255, 0, 0, 255]),
    ));
    let mut assets = graphics::assets::Assets::default();
    for (name, format) in [
        ("test.png", image::ImageFormat::Png),
        ("test.jpg", image::ImageFormat::Jpeg),
        ("test.webp", image::ImageFormat::WebP),
    ] {
        let path = directory.join(name);
        if format == image::ImageFormat::Jpeg {
            DynamicImage::ImageRgb8(image.to_rgb8())
                .save_with_format(&path, format)
                .unwrap();
        } else {
            image.save_with_format(&path, format).unwrap();
        }
        assert_eq!(assets.load(Some(name), &directory).unwrap().width(), 2);
    }
    let path = directory.join("test.gif");
    let mut gif = image::codecs::gif::GifEncoder::new(std::fs::File::create(path).unwrap());
    gif.encode_frames([
        image::Frame::new(image.to_rgba8()),
        image::Frame::new(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([0, 0, 255, 255]),
        )),
    ])
    .unwrap();
    drop(gif);
    assert_eq!(
        assets
            .load(Some("test.gif"), &directory)
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)
            .0,
        [255, 0, 0, 255]
    );
    assert_eq!(assets.decoded_count(), 4);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn plain_output_keeps_optional_information_on_small_terminals() {
    let mut cfg = Config::default();
    content::prepare(&mut cfg, 0, 0, false, false, true);
    assert!(
        cfg.layout.second.children[0]
            .children
            .iter()
            .all(|c| c.visible)
    );
}

#[test]
fn transparent_backgrounds_validate_and_roundtrip_without_affecting_foregrounds() {
    let cfg=Config::parse("[styles.window]\nbackground='transparent'\n[styles.text]\nbackground='transparent'\n[layout.first.style]\nbackground='transparent'").unwrap();
    let cfg = Config::parse(&toml::to_string_pretty(&cfg).unwrap()).unwrap();
    assert_eq!(cfg.styles.window.background.as_deref(), Some("transparent"));
    assert_eq!(
        cfg.styles
            .text
            .colors(&cfg.colors().unwrap())
            .unwrap()
            .background
            .as_deref(),
        Some("transparent")
    );
    assert!(Config::parse("[styles.text]\nforeground='transparent'").is_err());
}
#[test]
fn transparent_text_clears_inheritance_while_omitted_background_inherits() {
    let style = styles::TextStyleConfig {
        foreground: "#ffffff".into(),
        ..Default::default()
    };
    assert!(
        style
            .resolve_with_background(Some("#123456"))
            .render("text")
            .contains("48;2;18;52;86")
    );
    let style = styles::TextStyleConfig {
        background: Some("transparent".into()),
        ..style
    };
    assert!(
        !style
            .resolve_with_background(Some("#123456"))
            .render("text")
            .contains("48;")
    );
}
#[test]
fn transparent_window_emits_no_solid_background_colors() {
    let root = resolve(
        "[styles.window]\nbackground='transparent'\n[layout.first]\ntype='module'\ntitle='Title'\nchildren=[{type='component',value='VALUE',label='Label',icon='X'}]\n[layout.second]\nchildren=[]",
    );
    let output = String::from_utf8(
        render::compose(
            &root.place(Rect::new(0, 0, 40, 12)),
            &options(Backend::Text),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(strip(&output).contains("VALUE"));
    assert!(!output.contains("48;"), "{output:?}");
}
/// Track SGR backgrounds up to a text marker in the composed output.
fn background_at(output: &str, marker: &str) -> Option<Vec<u16>> {
    let prefix = &output[..output.find(marker).unwrap()];
    let mut bg = None;
    let bytes = prefix.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] != 0x1b || bytes[i + 1] != b'[' {
            i += 1;
            continue;
        }
        let start = i + 2;
        i = start;
        while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'm' {
            let params = std::str::from_utf8(&bytes[start..i]).unwrap();
            let params: Vec<u16> = if params.is_empty() {
                vec![0]
            } else {
                params.split(';').map(|n| n.parse().unwrap()).collect()
            };
            let mut p = 0;
            while p < params.len() {
                match params[p] {
                    0 | 49 => bg = None,
                    48 if params.get(p + 1) == Some(&2) => {
                        bg = Some(params[p + 2..p + 5].to_vec());
                        p += 4;
                    }
                    48 if params.get(p + 1) == Some(&5) => {
                        bg = Some(vec![params[p + 2]]);
                        p += 2;
                    }
                    38 | 58 if params.get(p + 1) == Some(&2) => p += 4,
                    38 | 58 if params.get(p + 1) == Some(&5) => p += 2,
                    _ => {}
                }
                p += 1;
            }
        }
        i += 1;
    }
    bg
}
#[test]
fn transparent_module_and_text_override_a_solid_parent() {
    for overrides in [
        "[layout.first.style]\nbackground='transparent'",
        "[styles.text]\nbackground='transparent'",
    ] {
        let root = resolve(&format!(
            "[styles.window]\nbackground='#123456'\n[layout.first]\ntype='module'\nchildren=[{{type='component',value='VALUE',label='Label'}}]\n[layout.second]\nchildren=[]\n{overrides}"
        ));
        let output = String::from_utf8(
            render::compose(
                &root.place(Rect::new(0, 0, 40, 12)),
                &options(Backend::Text),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(output.contains("48;2;18;52;86"));
        assert_eq!(background_at(&output, "VALUE"), None, "{output:?}");
    }
}

#[test]
fn compact_default_is_short_editable_and_transparent() {
    let cfg = Config::default();
    let text = cfg.editable_toml().unwrap();
    assert!(
        text.lines().count() < 35,
        "{} lines: {text}",
        text.lines().count()
    );
    assert!(text.contains("background = \"transparent\""));
    assert!(text.contains("components = [\"os\""));
    assert!(
        !text.contains(".children") && !text.contains("text_style") && !text.contains("[palette]")
    );
    let loaded = Config::parse(&text).unwrap();
    assert_eq!(
        toml::Value::try_from(loaded).unwrap(),
        toml::Value::try_from(cfg).unwrap()
    );
}
#[test]
fn shorthand_handles_images_modules_component_options_and_sizes() {
    let cfg = Config::parse(
        r##"
background="transparent"
[layout.first]
image="~/Pictures/logo.png"
size="40%"
[layout.second]
direction="horizontal"
modules=[
    {title="Software",components=["os",{name="kernel",label="Version",icon="K"}]},
    {title="Custom",components=[{label="Profile",value="Dev",size=12}]},
]
"##,
    )
    .unwrap();
    assert_eq!(
        cfg.layout.first.path.as_deref(),
        Some("~/Pictures/logo.png")
    );
    assert_eq!(cfg.layout.first.size, Size::Percent(40));
    assert_eq!(cfg.layout.second.kind, Kind::Row);
    let c = &cfg.layout.second.children[0].children[1];
    assert_eq!(c.label.as_deref(), Some("Version"));
    assert_eq!(c.icon.as_deref(), Some("K"));
    assert_eq!(
        cfg.layout.second.children[1].children[0].size,
        Size::Fixed(12)
    );
    cfg.editable_toml().unwrap();
}
#[test]
fn compact_export_preserves_old_verbose_configs_and_advanced_examples() {
    let mut cfg = Config::default();
    cfg.styles.module.align_v = Alignment::Center;
    cfg.layout.first.optional = false;
    cfg.layout.second.children[0].children[1].optional = false;
    let verbose = toml::to_string_pretty(&cfg).unwrap();
    let old = Config::parse(&verbose).unwrap();
    let compact = old.editable_toml().unwrap();
    assert!(compact.lines().count() < 60);
    assert_eq!(
        toml::Value::try_from(Config::parse(&compact).unwrap()).unwrap(),
        toml::Value::try_from(old).unwrap()
    );
    for text in [
        include_str!("../examples/default.toml"),
        include_str!("../examples/vertical.toml"),
        include_str!("../examples/minimal.toml"),
        include_str!("../examples/compact.toml"),
        include_str!("../examples/dashboard.toml"),
    ] {
        Config::parse(text).unwrap().editable_toml().unwrap();
    }
}
#[test]
fn compact_export_preserves_nested_images_styles_and_empty_containers() {
    for text in [
        "[layout.first]\ntype='image'\npath='builtin'\n[layout.first.image]\nfit='cover'",
        "[layout.first]\ntype='image'\npath='builtin'",
        "[layout.second]\ntype='column'\nchildren=[]",
        "[layout.first]\ntype='module'\nchildren=[{type='component',name='cpu',size='fill',text_style={foreground='#123'}}]",
        "[layout.first]\ntype='row'\ndirection='vertical'\nchildren=[{type='module',children=[]}]",
        "[layout.first]\ntype='column'\ndirection='horizontal'\nchildren=[]",
    ] {
        Config::parse(text).unwrap().editable_toml().unwrap();
    }
}
#[test]
fn shorthand_empty_arrays_clear_defaults_and_presets_expand() {
    let cfg = Config::parse("[layout.second]\nmodules=[]").unwrap();
    assert!(cfg.layout.second.children.is_empty());
    let cfg = Config::parse("[layout.second]\nmodules=['system','storage']").unwrap();
    assert_eq!(cfg.layout.second.children[1].children[0].name, "disk");
    let cfg = Config::parse("[layout.second]\nmodules=[{title='Empty',components=[]}]").unwrap();
    assert!(cfg.layout.second.children[0].children.is_empty());
}
#[test]
fn partial_section_shortcuts_keep_default_modules() {
    let cfg = Config::parse("[layout.second]\nswatches=false").unwrap();
    assert_eq!(cfg.layout.second.children.len(), 2);
    let cfg = Config::parse("[layout.second]\ndirection='horizontal'").unwrap();
    assert_eq!(cfg.layout.second.kind, Kind::Row);
    assert_eq!(cfg.layout.second.children.len(), 3);
}
#[test]
fn ambiguous_or_invalid_shorthand_has_errors() {
    for text in [
        "background='transparent'\n[styles.window]\nbackground='#123'",
        "[layout.first]\nimage='builtin'\npath='other.png'",
        "[layout.second]\nmodules=[]\nchildren=[]",
        "[layout.second]\ncomponents=[]\nchildren=[]",
        "[layout.second]\nmodules=42",
        "[layout.second]\nmodules=[{components=[42]}]",
        "[layout.first]\nsize='101%'",
        "[layout.first]\nsize=-1",
        "[layout.second]\nmodules=[{components=['unknown-metric']}]",
    ] {
        assert!(Config::parse(text).is_err(), "Accepted {text}");
    }
}

#[test]
fn section_alignment_shortcut_resolves_and_exports_compactly() {
    let cfg = Config::parse("[layout.second]\nalign='center'\nalign_v='end'").unwrap();
    assert_eq!(cfg.layout.second.align_h, Some(Alignment::Center));
    assert_eq!(cfg.layout.second.align_v, Some(Alignment::End));
    let root = content::resolve(
        &cfg,
        &snapshot(),
        Path::new("."),
        &mut graphics::assets::Assets::default(),
    )
    .unwrap();
    let children = &root.children[1].children[0];
    assert_eq!(children.children_align_h, Some(Alignment::Center));
    assert_eq!(children.children_align_v, Some(Alignment::End));
    cfg.editable_toml().unwrap();
    let cfg = Config::parse("[layout.second]\nalign='center'").unwrap();
    let text = cfg.editable_toml().unwrap();
    assert!(text.contains("align = \"center\""));
    assert!(Config::parse("[layout.second]\nalign='sideways'").is_err());
}
#[test]
fn children_align_on_both_axes_in_rows_and_columns() {
    for direction in [Direction::Vertical, Direction::Horizontal] {
        for (align, main_delta, cross_a, cross_b) in if direction == Direction::Vertical {
            [
                (Alignment::Start, 0, 0, 0),
                (Alignment::Center, 4, 8, 7),
                (Alignment::End, 8, 17, 15),
            ]
        } else {
            [
                (Alignment::Start, 0, 0, 0),
                (Alignment::Center, 5, 5, 5),
                (Alignment::End, 11, 11, 10),
            ]
        } {
            let mut parent = Resolved::empty(ContainerStyleConfig::plain());
            parent.direction = direction;
            parent.gap = 1;
            parent.children_align_h = Some(align);
            parent.children_align_v = Some(align);
            let a = leaf(Size::Auto);
            let mut b = leaf(Size::Auto);
            b.lines = vec![
                vec![("12345".into(), styles::TextStyleConfig::default())],
                vec![("x".into(), styles::TextStyleConfig::default())],
            ];
            parent.children = vec![a, b];
            let placed = parent.place(Rect::new(10, 20, 20, 12));
            let expected = if direction == Direction::Vertical {
                [
                    Rect::new(10 + cross_a, 20 + main_delta, 3, 1),
                    Rect::new(10 + cross_b, 22 + main_delta, 5, 2),
                ]
            } else {
                [
                    Rect::new(10 + main_delta, 20 + cross_a, 3, 1),
                    Rect::new(14 + main_delta, 20 + cross_b, 5, 2),
                ]
            };
            assert_eq!(placed.children[0].area, expected[0]);
            assert_eq!(placed.children[1].area, expected[1]);
        }
    }
}
#[test]
fn child_alignment_respects_padding_clipping_and_default_stretch() {
    let mut parent = Resolved::empty(ContainerStyleConfig {
        padding: Padding::all(1),
        ..Default::default()
    });
    parent.children_align_h = Some(Alignment::End);
    parent.children_align_v = Some(Alignment::End);
    parent.children.push(leaf(Size::Auto));
    assert_eq!(
        parent.place(Rect::new(0, 0, 10, 8)).children[0].area,
        Rect::new(5, 5, 3, 1)
    );
    for (width, height) in [(0, 0), (1, 1), (3, 2), (8, 4)] {
        let placed = parent.place(Rect::new(0, 0, width, height));
        for child in &placed.children {
            assert!(child.area.x + child.area.width <= width);
            assert!(child.area.y + child.area.height <= height);
        }
    }
    parent.children_align_h = None;
    parent.children_align_v = None;
    assert_eq!(
        parent.place(Rect::new(0, 0, 10, 8)).children[0].area,
        Rect::new(2, 2, 6, 1)
    );
}
#[test]
fn fill_children_use_main_axis_space_without_exceeding_alignment_bounds() {
    let mut parent = Resolved::empty(ContainerStyleConfig::plain());
    parent.children_align_h = Some(Alignment::Center);
    parent.children_align_v = Some(Alignment::End);
    parent.children.push(leaf(Size::Fill));
    assert_eq!(
        parent.place(Rect::new(0, 0, 20, 12)).children[0].area,
        Rect::new(8, 0, 3, 12)
    );
}
