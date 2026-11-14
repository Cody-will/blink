//! Read older schemas in memory. The source configuration is never rewritten.
use anyhow::{Context, bail};
use toml::{Table, Value};
pub fn convert(value: &mut Value) -> anyhow::Result<()> {
    let table = value
        .as_table_mut()
        .context("Configuration must be a TOML table")?;
    let legacy = table
        .get("layout")
        .and_then(Value::as_table)
        .is_some_and(|t| t.contains_key("horizontal") || t.contains_key("vertical"))
        || [
            "window",
            "first_section",
            "second_section",
            "module",
            "component",
        ]
        .iter()
        .any(|k| table.contains_key(*k));
    if !legacy {
        return Ok(());
    }
    if table.contains_key("styles") {
        bail!("Cannot mix legacy window/module styles with the new styles table");
    }
    let mut styles = Table::new();
    for (old, new) in [
        ("window", "window"),
        ("module", "module"),
        ("first_section", "section"),
        ("second_section", "section"),
    ] {
        if let Some(Value::Table(mut t)) = table.remove(old) {
            let container = t.remove("container").unwrap_or(Value::Table(Table::new()));
            if new == "section" && styles.get(new).is_some_and(|s| s != &container) {
                bail!(
                    "Legacy sections have different styles; move those overrides to layout.first.style and layout.second.style"
                );
            }
            styles.insert(new.into(), container);
            if let Some(title) = t.remove("title") {
                styles.insert("title".into(), title);
            }
            if let Some(gap) = t.remove("gap")
                && gap.as_integer() != Some(0)
            {
                bail!("Legacy global module gap must be moved to each module node's gap field");
            }
            if !t.is_empty() {
                bail!(
                    "Unrecognized legacy {old} fields: {:?}",
                    t.keys().collect::<Vec<_>>()
                );
            }
        }
    }
    if let Some(Value::Table(mut component)) = table.remove("component") {
        for key in ["icon", "label", "content"] {
            if let Some(v) = component.remove(key) {
                styles.insert(if key == "content" { "text" } else { key }.into(), v);
            }
        }
        if let Some(padding) = component.remove("padding") {
            styles.insert(
                "component".into(),
                Value::Table(Table::from_iter([("padding".into(), padding)])),
            );
        }
        if !component.is_empty() {
            bail!("Unrecognized legacy component fields");
        }
    }
    table.insert("styles".into(), Value::Table(styles));
    if let Some(Value::Table(layout)) = table.get_mut("layout") {
        let direction = if layout.contains_key("horizontal") {
            Some(("horizontal", "left", "right"))
        } else if layout.contains_key("vertical") {
            Some(("vertical", "top", "bottom"))
        } else {
            None
        };
        if let Some((direction, first, second)) = direction {
            let mut split = layout
                .remove(direction)
                .and_then(|v| v.as_table().cloned())
                .context("Invalid legacy layout")?;
            for (old, new) in [(first, "first"), (second, "second")] {
                let mut section = split
                    .remove(old)
                    .and_then(|v| v.as_table().cloned())
                    .with_context(|| format!("Missing legacy {old} section"))?;
                let mut child = section
                    .remove("child")
                    .and_then(|v| v.as_table().cloned())
                    .context("Missing legacy section child")?;
                if let Some(size) = section.remove("size") {
                    child.insert("size".into(), size);
                }
                if !section.is_empty() {
                    bail!("Unmapped legacy section fields");
                }
                layout.insert(new.into(), Value::Table(child));
            }
            if !split.is_empty() {
                bail!("Unmapped legacy layout fields");
            }
            layout.insert("direction".into(), Value::String(direction.into()));
        }
    }
    fn nodes(v: &mut Value) -> anyhow::Result<()> {
        if let Value::Table(t) = v {
            if t.get("type").and_then(Value::as_str) == Some("module")
                && !t.contains_key("children")
            {
                let name = t.remove("name").and_then(|v| v.as_str().map(str::to_owned));
                if let Some(name) = name {
                    if crate::components::preset(&name.to_lowercase()).is_some() {
                        t.insert("preset".into(), Value::String(name.to_lowercase()));
                    } else {
                        bail!("Legacy module {name:?} needs an explicit component list");
                    }
                }
            }
            if t.get("type").and_then(Value::as_str) == Some("icon") {
                bail!(
                    "Legacy icon path nodes must become image nodes or component icon_path fields"
                );
            }
            for (_, v) in t.iter_mut() {
                nodes(v)?;
            }
        } else if let Value::Array(a) = v {
            for v in a {
                nodes(v)?;
            }
        }
        Ok(())
    }
    nodes(value)
}
