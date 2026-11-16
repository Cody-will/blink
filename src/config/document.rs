//! Export concise, editable TOML rather than serializing runtime implementation fields.
use super::Config;
use crate::layout::{
    Size,
    node::{Kind, Node},
};
use anyhow::Context;
use toml::{Table, Value};

fn difference(value: &Value, default: &Value) -> Option<Value> {
    if value == default {
        return None;
    }
    match (value, default) {
        (Value::Table(table), Value::Table(default)) => {
            let result = table
                .iter()
                .filter_map(|(key, value)| {
                    let value = match default.get(key) {
                        Some(default) => difference(value, default),
                        None => Some(value.clone()),
                    }?;
                    Some((key.clone(), value))
                })
                .collect::<Table>();
            (!result.is_empty()).then_some(Value::Table(result))
        }
        _ => Some(value.clone()),
    }
}
fn component_default(name: &str) -> Node {
    let mut node = super::module("", &[name]).children.remove(0);
    node.kind = Kind::Component;
    node
}
fn size(size: Size) -> Value {
    match size {
        Size::Auto => Value::String("auto".into()),
        Size::Fill => Value::String("fill".into()),
        Size::Fixed(n) => Value::Integer(i64::from(n)),
        Size::Percent(n) => Value::String(format!("{n}%")),
    }
}
fn node(n: &Node, base: &Node, component: bool, module: bool) -> anyhow::Result<Value> {
    let fallback = Node::default();
    let base = if n.kind == base.kind { base } else { &fallback };
    let full = Value::try_from(n)?;
    let mut table = difference(&full, &Value::try_from(base)?)
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    // Content lists always replace, including deliberately empty lists.
    for field in ["children", "image", "preset", "header", "footer"] {
        table.remove(field);
    }
    table.insert("size".into(), size(n.size));
    if component {
        table.remove("type");
        table.insert("name".into(), Value::String(n.name.clone()));
        if n.size == Size::Auto {
            table.remove("size");
        }
    } else if module {
        table.remove("type");
        if n.size == Size::Auto {
            table.remove("size");
        }
    } else {
        table.insert("type".into(), Value::try_from(n.kind)?);
    }
    match n.kind {
        Kind::Image => {
            if !matches!(n.path.as_deref(), Some("builtin" | "auto"))
                && Value::try_from(&n.image)?
                    == Value::try_from(crate::layout::node::ImageOptions::default())?
            {
                table.remove("type");
                table.remove("path");
                table.insert(
                    "image".into(),
                    Value::String(n.path.clone().unwrap_or_else(|| "auto".into())),
                );
            } else {
                table.insert("image".into(), Value::try_from(&n.image)?);
            }
        }
        Kind::Ascii => {
            // Use the concise source form for both file-backed and inline text art.
            table.remove("type");
            table.remove("path");
            table.remove("content");
            let source = if let Some(path) = &n.path {
                Value::String(path.clone())
            } else {
                Value::Table(Table::from_iter([(
                    "text".into(),
                    Value::String(n.content.clone()),
                )]))
            };
            table.insert("ascii".into(), source);
        }
        Kind::Module => {
            if n.children.iter().all(|c| c.kind == Kind::Component) {
                table.insert("components".into(), component_values(&n.children)?);
            } else {
                table.insert("children".into(), children(&n.children)?);
            }
        }
        Kind::Column | Kind::Row => {
            let swatch = n.children.last().filter(|c| c.kind == Kind::Swatches);
            let modules = if swatch.is_some() {
                &n.children[..n.children.len() - 1]
            } else {
                &n.children
            };
            let default_swatch = Node {
                kind: Kind::Swatches,
                size: Size::Auto,
                optional: true,
                ..Default::default()
            };
            let simple_swatch = swatch.map(Value::try_from).transpose()?
                == swatch
                    .map(|_| Value::try_from(&default_swatch))
                    .transpose()?;
            if modules.iter().all(|c| c.kind == Kind::Module)
                && simple_swatch
                && ((n.kind == Kind::Row && n.direction == crate::layout::Direction::Horizontal)
                    || (n.kind == Kind::Column
                        && n.direction == crate::layout::Direction::Vertical))
            {
                table.remove("type");
                if n.kind == Kind::Row {
                    table.insert("direction".into(), Value::String("horizontal".into()));
                } else {
                    table.insert("direction".into(), Value::String("vertical".into()));
                }
                table.insert(
                    "modules".into(),
                    Value::Array(
                        modules
                            .iter()
                            .map(|c| {
                                node(
                                    c,
                                    &Node {
                                        kind: Kind::Module,
                                        size: Size::Auto,
                                        ..Default::default()
                                    },
                                    false,
                                    true,
                                )
                            })
                            .collect::<anyhow::Result<_>>()?,
                    ),
                );
                table.insert("swatches".into(), Value::Boolean(swatch.is_some()));
            } else {
                table.insert("children".into(), children(&n.children)?);
            }
        }
        _ => {}
    }
    for (key, nodes) in [("header", &n.header), ("footer", &n.footer)] {
        if !nodes.is_empty() {
            table.insert(key.into(), component_values(nodes)?);
        }
    }
    if n.responsive {
        table.insert("responsive".into(), Value::Boolean(true));
    }
    // Node-specific image options only belong on image nodes, unless explicitly customized.
    if n.kind != Kind::Image && n.image != crate::layout::node::ImageOptions::default() {
        table.insert("image".into(), Value::try_from(&n.image)?);
    }
    if let Some(align) = n.align_h.filter(|h| Some(*h) == n.align_v) {
        table.remove("align_h");
        table.remove("align_v");
        table.insert("align".into(), Value::try_from(align)?);
    }
    Ok(Value::Table(table))
}
fn component_values(nodes: &[Node]) -> anyhow::Result<Value> {
    Ok(Value::Array(
        nodes
            .iter()
            .map(|c| {
                let default = component_default(&c.name);
                if Value::try_from(c)? == Value::try_from(&default)? {
                    Ok(Value::String(c.name.clone()))
                } else {
                    node(c, &default, true, false)
                }
            })
            .collect::<anyhow::Result<_>>()?,
    ))
}
fn children(nodes: &[Node]) -> anyhow::Result<Value> {
    Ok(Value::Array(
        nodes
            .iter()
            .map(|n| node(n, &Node::default(), false, false))
            .collect::<anyhow::Result<_>>()?,
    ))
}
pub fn export(cfg: &Config) -> anyhow::Result<String> {
    let defaults = Config::default();
    let mut root = difference(&Value::try_from(cfg)?, &Value::try_from(&defaults)?)
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    root.insert("theme".into(), Value::String(cfg.theme.clone()));
    root.insert(
        "background".into(),
        Value::String(
            cfg.styles
                .window
                .background
                .clone()
                .unwrap_or_else(|| "transparent".into()),
        ),
    );
    if let Some(styles) = root.get_mut("styles").and_then(Value::as_table_mut) {
        if let Some(window) = styles.get_mut("window").and_then(Value::as_table_mut) {
            window.remove("background");
            if window.is_empty() {
                styles.remove("window");
            }
        }
        if styles.is_empty() {
            root.remove("styles");
        }
    }
    let layout = Table::from_iter([
        ("direction".into(), Value::try_from(cfg.layout.direction)?),
        ("gap".into(), Value::Integer(i64::from(cfg.layout.gap))),
        (
            "first".into(),
            node(&cfg.layout.first, &defaults.layout.first, false, false)?,
        ),
        (
            "second".into(),
            node(&cfg.layout.second, &defaults.layout.second, false, false)?,
        ),
    ]);
    root.insert("layout".into(), Value::Table(layout));
    let text = toml::to_string(&root)?;
    // Every export must be readable and preserve all configurable behavior.
    let loaded = Config::parse(&text).context("Exported configuration failed validation")?;
    if Value::try_from(&loaded)? != Value::try_from(cfg)? {
        anyhow::bail!("Compact configuration export would lose settings");
    }
    Ok(text)
}
