//! User-facing shortcuts normalize into the existing recursive configuration model.
use anyhow::{Context, bail};
use toml::{Table, Value};

fn kind(table: &mut Table, expected: &str, path: &str) -> anyhow::Result<()> {
    if let Some(existing) = table.get("type")
        && existing.as_str() != Some(expected)
    {
        bail!("{path}: shorthand requires type={expected:?}");
    }
    table.insert("type".into(), Value::String(expected.into()));
    Ok(())
}
fn component(value: Value, path: &str, depth: usize) -> anyhow::Result<Value> {
    let mut table = match value {
        Value::String(name) => Table::from_iter([("name".into(), Value::String(name))]),
        Value::Table(table) => table,
        _ => bail!("{path}: expected a component name or component options table"),
    };
    kind(&mut table, "component", path)?;
    table.entry("size").or_insert(Value::String("auto".into()));
    let optional = table
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|n| matches!(n, "hostname" | "uptime" | "shell" | "cpu_cores"));
    table.entry("optional").or_insert(Value::Boolean(optional));
    node(&mut table, path, depth, None)?;
    Ok(Value::Table(table))
}
fn module(value: Value, path: &str, depth: usize, default_size: &str) -> anyhow::Result<Value> {
    let mut table = match value {
        Value::String(preset) => Table::from_iter([("preset".into(), Value::String(preset))]),
        Value::Table(table) => table,
        _ => bail!("{path}: expected a module preset name or module options table"),
    };
    kind(&mut table, "module", path)?;
    table
        .entry("size")
        .or_insert(Value::String(default_size.into()));
    node(&mut table, path, depth, None)?;
    Ok(Value::Table(table))
}
fn node(
    table: &mut Table,
    path: &str,
    depth: usize,
    defaults: Option<&crate::layout::node::Node>,
) -> anyhow::Result<()> {
    if depth > 64 {
        bail!("{path}: excessive layout depth");
    }
    if let Some(align) = table.remove("align") {
        table.entry("align_h").or_insert_with(|| align.clone());
        table.entry("align_v").or_insert(align);
    }
    if let Some(Value::String(image)) = table.get("image").cloned() {
        kind(table, "image", path)?;
        table.remove("image");
        if table.contains_key("path") {
            bail!("{path}: specify image or path, not both");
        }
        if image != "builtin" {
            table.insert("path".into(), Value::String(image));
        }
    }
    if let Some(modules) = table.remove("modules") {
        if table.contains_key("children") {
            bail!("{path}: specify modules or children, not both");
        }
        let direction = table
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("vertical");
        let horizontal = direction == "horizontal";
        kind(table, if horizontal { "row" } else { "column" }, path)?;
        let modules = modules
            .as_array()
            .with_context(|| format!("{path}.modules must be an array"))?;
        let children = modules
            .iter()
            .enumerate()
            .map(|(i, v)| {
                module(
                    v.clone(),
                    &format!("{path}.modules[{i}]"),
                    depth + 1,
                    if horizontal { "fill" } else { "auto" },
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        table.insert("children".into(), Value::Array(children));
    }
    if let Some(components) = table.remove("components") {
        if table.contains_key("children") {
            bail!("{path}: specify components or children, not both");
        }
        kind(table, "module", path)?;
        table.entry("size").or_insert(Value::String("auto".into()));
        let components = components
            .as_array()
            .with_context(|| format!("{path}.components must be an array"))?;
        let children = components
            .iter()
            .enumerate()
            .map(|(i, v)| component(v.clone(), &format!("{path}.components[{i}]"), depth + 1))
            .collect::<anyhow::Result<Vec<_>>>()?;
        table.insert("children".into(), Value::Array(children));
    }
    if let Some(swatches) = table.remove("swatches") {
        let show = swatches
            .as_bool()
            .with_context(|| format!("{path}.swatches must be true or false"))?;
        if !matches!(
            table.get("type").and_then(Value::as_str),
            Some("row" | "column") | None
        ) {
            bail!("{path}: swatches requires a row or column");
        }
        if !table.contains_key("type") {
            kind(table, "column", path)?;
        }
        if !table.contains_key("children") {
            let children = if let Some(default) = defaults.filter(|d| {
                matches!(
                    d.kind,
                    crate::layout::node::Kind::Row | crate::layout::node::Kind::Column
                )
            }) {
                toml::Value::try_from(&default.children)?
            } else {
                Value::Array(vec![])
            };
            table.insert("children".into(), children);
        }
        let children = table
            .get_mut("children")
            .and_then(Value::as_array_mut)
            .with_context(|| format!("{path}.children must be an array"))?;
        children.retain(|c| c.get("type").and_then(Value::as_str) != Some("swatches"));
        if show {
            children.push(toml::Value::try_from(crate::layout::node::Node {
                kind: crate::layout::node::Kind::Swatches,
                size: crate::layout::Size::Auto,
                optional: true,
                ..Default::default()
            })?);
        }
    }
    if let Some(Value::String(size)) = table.get("size") {
        if let Some(percent) = size.strip_suffix('%') {
            let percent = percent
                .parse::<u8>()
                .with_context(|| format!("{path}.size: invalid percentage {size:?}"))?;
            table.insert(
                "size".into(),
                Value::Table(Table::from_iter([(
                    "percent".into(),
                    Value::Integer(i64::from(percent)),
                )])),
            );
        }
    } else if let Some(Value::Integer(size)) = table.get("size") {
        table.insert(
            "size".into(),
            Value::Table(Table::from_iter([("fixed".into(), Value::Integer(*size))])),
        );
    }
    if let Some(Value::Array(children)) = table.get_mut("children") {
        for (i, child) in children.iter_mut().enumerate() {
            let child = child
                .as_table_mut()
                .with_context(|| format!("{path}.children[{i}] must be a node table"))?;
            node(child, &format!("{path}.children[{i}]"), depth + 1, None)?;
        }
    }
    Ok(())
}
pub fn normalize(value: &mut Value) -> anyhow::Result<()> {
    let root = value
        .as_table_mut()
        .context("Configuration must be a table")?;
    if let Some(background) = root.remove("background") {
        let background = background
            .as_str()
            .context("background must be a color or 'transparent'")?;
        let styles = root
            .entry("styles")
            .or_insert(Value::Table(Table::new()))
            .as_table_mut()
            .context("styles must be a table")?;
        let window = styles
            .entry("window")
            .or_insert(Value::Table(Table::new()))
            .as_table_mut()
            .context("styles.window must be a table")?;
        if window.contains_key("background") {
            bail!("Set background or styles.window.background, not both");
        }
        window.insert("background".into(), Value::String(background.into()));
    }
    if let Some(Value::Table(layout)) = root.get_mut("layout") {
        let defaults = super::Layout::default();
        for (side, default) in [("first", &defaults.first), ("second", &defaults.second)] {
            if let Some(value) = layout.get_mut(side) {
                let table = value
                    .as_table_mut()
                    .with_context(|| format!("layout.{side} must be a table"))?;
                // Changing only a section's direction keeps its default content.
                if !table.contains_key("type")
                    && !["modules", "children", "image", "components"]
                        .iter()
                        .any(|k| table.contains_key(*k))
                    && matches!(
                        default.kind,
                        crate::layout::node::Kind::Column | crate::layout::node::Kind::Row
                    )
                    && let Some(direction) = table.get("direction").and_then(Value::as_str)
                {
                    let orientation = if direction == "horizontal" {
                        "row"
                    } else {
                        "column"
                    };
                    kind(table, orientation, &format!("layout.{side}"))?;
                    table.insert("children".into(), Value::try_from(&default.children)?);
                    table
                        .entry("gap")
                        .or_insert(Value::Integer(i64::from(default.gap)));
                }
                node(table, &format!("layout.{side}"), 0, Some(default))?;
            }
        }
    }
    Ok(())
}
