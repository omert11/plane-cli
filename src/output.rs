use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};
use serde::Serialize;
use serde_json::Value;

use crate::types::{
    Attachment, Comment, Cycle, Label, Member, Module, Page, Project, State, WorkItem,
};
use crate::util::truncate;

pub fn emit_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn emit_value(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Render `value` as JSON when `json`, else invoke the human-readable closure.
pub fn render<T: Serialize>(value: &T, json: bool, human: impl FnOnce(&T)) -> Result<()> {
    if json {
        emit_json(value)
    } else {
        human(value);
        Ok(())
    }
}

pub fn print_message(msg: &str) {
    println!("{} {}", "→".bold().green(), msg);
}

fn base_table() -> Table {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    t
}

fn footer(n: usize, label: &str) {
    println!("{} {}", n.to_string().bold(), label.dimmed());
}

// ---- Projects ----

pub fn print_project_table(items: &[Project]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Identifier", "Name", "Archived", "Created"]);
    for p in items {
        table.add_row(vec![
            Cell::new(truncate(&p.id, 36)),
            Cell::new(&p.identifier),
            Cell::new(truncate(&p.name, 50)),
            Cell::new(if p.archived_at.is_some() { "✓" } else { "" }),
            Cell::new(p.created_at.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    footer(items.len(), "projects");
}

// ---- Work items ----

fn state_name(v: &Option<Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(m)) => m
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("-")
            .to_string(),
        _ => "-".to_string(),
    }
}

pub fn print_work_item_table(items: &[WorkItem]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "#", "Name", "State", "Priority", "Target"]);
    for w in items {
        table.add_row(vec![
            Cell::new(truncate(&w.id, 36)),
            Cell::new(w.sequence_id.map(|s| s.to_string()).unwrap_or_default()),
            Cell::new(truncate(&w.name, 50)),
            Cell::new(state_name(&w.state)),
            Cell::new(w.priority.as_deref().unwrap_or("-")),
            Cell::new(w.target_date.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    footer(items.len(), "work items");
}

// ---- States ----

pub fn print_state_table(items: &[State]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Name", "Group", "Color", "Default"]);
    for s in items {
        table.add_row(vec![
            Cell::new(truncate(&s.id, 36)),
            Cell::new(&s.name),
            Cell::new(s.group.as_deref().unwrap_or("-")),
            Cell::new(s.color.as_deref().unwrap_or("-")),
            Cell::new(if s.default { "✓" } else { "" }),
        ]);
    }
    println!("{table}");
    footer(items.len(), "states");
}

// ---- Labels ----

pub fn print_label_table(items: &[Label]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Name", "Color", "Description"]);
    for l in items {
        table.add_row(vec![
            Cell::new(truncate(&l.id, 36)),
            Cell::new(&l.name),
            Cell::new(l.color.as_deref().unwrap_or("-")),
            Cell::new(truncate(l.description.as_deref().unwrap_or("-"), 40)),
        ]);
    }
    println!("{table}");
    footer(items.len(), "labels");
}

// ---- Comments ----

pub fn print_comment_table(items: &[Comment]) {
    for c in items {
        let body = c
            .comment_stripped
            .as_deref()
            .or(c.comment_html.as_deref())
            .unwrap_or("");
        println!(
            "{} {} {}",
            "comment".bold(),
            truncate(&c.id, 36).cyan(),
            c.created_at.as_deref().unwrap_or("-").dimmed()
        );
        for line in body.lines() {
            println!("  {line}");
        }
        println!("{}", "---".dimmed());
    }
    footer(items.len(), "comments");
}

// ---- Cycles ----

pub fn print_cycle_table(items: &[Cycle]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Name", "Start", "End", "Archived"]);
    for c in items {
        table.add_row(vec![
            Cell::new(truncate(&c.id, 36)),
            Cell::new(truncate(&c.name, 40)),
            Cell::new(c.start_date.as_deref().unwrap_or("-")),
            Cell::new(c.end_date.as_deref().unwrap_or("-")),
            Cell::new(if c.archived_at.is_some() { "✓" } else { "" }),
        ]);
    }
    println!("{table}");
    footer(items.len(), "cycles");
}

// ---- Modules ----

pub fn print_module_table(items: &[Module]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Name", "Status", "Start", "Target"]);
    for m in items {
        table.add_row(vec![
            Cell::new(truncate(&m.id, 36)),
            Cell::new(truncate(&m.name, 40)),
            Cell::new(m.status.as_deref().unwrap_or("-")),
            Cell::new(m.start_date.as_deref().unwrap_or("-")),
            Cell::new(m.target_date.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    footer(items.len(), "modules");
}

// ---- Pages ----

pub fn print_page_table(items: &[Page]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Name", "Created"]);
    for p in items {
        table.add_row(vec![
            Cell::new(truncate(&p.id, 36)),
            Cell::new(truncate(&p.name, 50)),
            Cell::new(p.created_at.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    footer(items.len(), "pages");
}

// ---- Attachments ----

pub fn print_attachment_table(items: &[Attachment]) {
    let mut table = base_table();
    table.set_header(vec!["ID", "Asset", "Name", "Size", "Type", "Uploaded"]);
    for a in items {
        let (name, size, mime) = match &a.attributes {
            Some(attr) => (
                attr.name.as_deref().unwrap_or("-").to_string(),
                attr.size.map(human_size).unwrap_or_else(|| "-".into()),
                attr.mime_type.as_deref().unwrap_or("-").to_string(),
            ),
            None => ("-".into(), "-".into(), "-".into()),
        };
        table.add_row(vec![
            Cell::new(truncate(&a.id, 36)),
            Cell::new(truncate(a.asset_id.as_deref().unwrap_or("-"), 36)),
            Cell::new(truncate(&name, 40)),
            Cell::new(size),
            Cell::new(mime),
            Cell::new(if a.is_uploaded.unwrap_or(false) {
                "✓"
            } else {
                ""
            }),
        ]);
    }
    println!("{table}");
    footer(items.len(), "attachments");
}

/// Render a byte count compactly (e.g. 12896 → "12.6 KB").
pub fn human_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 0 {
        return bytes.to_string();
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

// ---- Members ----

pub fn print_member_table(items: &[Member]) {
    let mut table = base_table();
    table.set_header(vec!["Member ID", "Name", "Email", "Role"]);
    for m in items {
        let name = m.display_name.clone().unwrap_or_else(|| {
            format!(
                "{} {}",
                m.first_name.as_deref().unwrap_or(""),
                m.last_name.as_deref().unwrap_or("")
            )
            .trim()
            .to_string()
        });
        let id = m.member_id.as_deref().or(m.id.as_deref()).unwrap_or("-");
        table.add_row(vec![
            Cell::new(truncate(id, 36)),
            Cell::new(if name.is_empty() { "-".into() } else { name }),
            Cell::new(m.email.as_deref().unwrap_or("-")),
            Cell::new(role_name(m.role)),
        ]);
    }
    println!("{table}");
    footer(items.len(), "members");
}

/// Map Plane role integers to readable names (Admin=20, Member=15, Guest=5).
pub fn role_name(role: Option<i64>) -> String {
    match role {
        Some(20) => "Admin".into(),
        Some(15) => "Member".into(),
        Some(5) => "Guest".into(),
        Some(n) => n.to_string(),
        None => "-".into(),
    }
}
