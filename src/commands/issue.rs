use anyhow::{anyhow, Result};
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::WorkItem;
use crate::util;

#[derive(Subcommand)]
pub enum IssueCmd {
    /// List work items in a project
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Comma-separated relations to expand into full objects
        /// (e.g. labels,state,assignees)
        #[arg(long)]
        expand: Option<String>,
        /// Comma-separated fields to return (server-side projection,
        /// e.g. id,name,labels)
        #[arg(long)]
        fields: Option<String>,
        /// Filter by state UUID (client-side; the API ignores state query params)
        #[arg(long)]
        state: Option<String>,
        /// Filter by label UUID (client-side)
        #[arg(long)]
        label: Option<String>,
    },
    /// Get a work item by UUID
    Get {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
    /// Get a work item by human identifier (e.g. PROJ-123)
    GetId {
        /// Work item identifier in PROJECT-N format
        ident: String,
    },
    /// Create a new work item
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item name (required)
        name: String,
        /// Description (plain text; wrapped into HTML)
        #[arg(long)]
        description: Option<String>,
        /// Description as raw HTML (sent verbatim; takes precedence over --description)
        #[arg(long)]
        description_html: Option<String>,
        /// Priority: urgent, high, medium, low, none
        #[arg(long)]
        priority: Option<String>,
        /// State UUID
        #[arg(long)]
        state: Option<String>,
        /// Comma-separated assignee user UUIDs
        #[arg(long)]
        assignees: Option<String>,
        /// Comma-separated label UUIDs
        #[arg(long)]
        labels: Option<String>,
        /// Start date (ISO 8601)
        #[arg(long)]
        start_date: Option<String>,
        /// Target date (ISO 8601)
        #[arg(long)]
        target_date: Option<String>,
    },
    /// Update a work item
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// Description (plain text; wrapped into HTML)
        #[arg(long)]
        description: Option<String>,
        /// Description as raw HTML (sent verbatim; takes precedence over --description)
        #[arg(long)]
        description_html: Option<String>,
        /// Priority: urgent, high, medium, low, none
        #[arg(long)]
        priority: Option<String>,
        /// State UUID
        #[arg(long)]
        state: Option<String>,
        /// Comma-separated assignee user UUIDs (replaces list)
        #[arg(long)]
        assignees: Option<String>,
        /// Comma-separated label UUIDs (replaces list)
        #[arg(long)]
        labels: Option<String>,
        /// Start date (ISO 8601)
        #[arg(long)]
        start_date: Option<String>,
        /// Target date (ISO 8601)
        #[arg(long)]
        target_date: Option<String>,
    },
    /// Delete a work item
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
    /// Search work items across the workspace
    Search {
        /// Free-text search query
        query: String,
        /// Maximum results to display (default: 25)
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Count work items in a project
    Count {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
    /// Add or remove assignees on a work item
    Assignee {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
        /// Comma-separated user UUIDs to add
        #[arg(long)]
        add: Option<String>,
        /// Comma-separated user UUIDs to remove
        #[arg(long)]
        remove: Option<String>,
    },
    /// Add or remove labels on a work item
    Label {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
        /// Comma-separated label UUIDs to add
        #[arg(long)]
        add: Option<String>,
        /// Comma-separated label UUIDs to remove
        #[arg(long)]
        remove: Option<String>,
    },
    /// Archive a work item (must be in completed/cancelled state)
    Archive {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
    /// Unarchive a work item
    Unarchive {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
    /// List archived work items in a project
    ListArchived {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
}

pub async fn run(cmd: IssueCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        IssueCmd::List {
            project,
            expand,
            fields,
            state,
            label,
        } => list(client, &project, expand, fields, state, label, json).await,
        IssueCmd::Get { project, id } => get(client, &project, &id, json).await,
        IssueCmd::GetId { ident } => get_by_ident(client, &ident, json).await,
        IssueCmd::Create {
            project,
            name,
            description,
            description_html,
            priority,
            state,
            assignees,
            labels,
            start_date,
            target_date,
        } => {
            create(
                client,
                &project,
                name,
                description,
                description_html,
                priority,
                state,
                assignees,
                labels,
                start_date,
                target_date,
                json,
            )
            .await
        }
        IssueCmd::Update {
            project,
            id,
            name,
            description,
            description_html,
            priority,
            state,
            assignees,
            labels,
            start_date,
            target_date,
        } => {
            update(
                client,
                &project,
                &id,
                name,
                description,
                description_html,
                priority,
                state,
                assignees,
                labels,
                start_date,
                target_date,
                json,
            )
            .await
        }
        IssueCmd::Delete { project, id } => delete(client, &project, &id, json).await,
        IssueCmd::Search { query, limit } => search(client, &query, limit, json).await,
        IssueCmd::Count { project } => count(client, &project, json).await,
        IssueCmd::Assignee {
            project,
            id,
            add,
            remove,
        } => manage_assignees(client, &project, &id, add, remove, json).await,
        IssueCmd::Label {
            project,
            id,
            add,
            remove,
        } => manage_labels(client, &project, &id, add, remove, json).await,
        IssueCmd::Archive { project, id } => archive(client, &project, &id).await,
        IssueCmd::Unarchive { project, id } => unarchive(client, &project, &id).await,
        IssueCmd::ListArchived { project } => list_archived(client, &project, json).await,
    }
}

// ---- list ----

async fn list(
    client: &PlaneClient,
    project: &str,
    expand: Option<String>,
    fields: Option<String>,
    state: Option<String>,
    label: Option<String>,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items"));

    // A --fields projection must not silently strip fields the table renderer
    // (`id`) or the client-side filters (`state`/`labels`) depend on — that
    // would render an empty result with no hint why.
    let mut required: Vec<&str> = Vec::new();
    if !json {
        required.push("id");
    }
    if state.is_some() {
        required.push("state");
    }
    if label.is_some() {
        required.push("labels");
    }
    let fields = fields.map(|f| ensure_fields(f, &required));

    let mut base_query: Vec<(&str, String)> = Vec::new();
    if let Some(e) = expand {
        base_query.push(("expand", e));
    }
    if let Some(f) = fields {
        base_query.push(("fields", f));
    }

    // Follow the pagination cursor so multi-page responses return every item.
    // Page size is server-determined (self-hosted Plane serves up to 1000 per
    // page, so a single request usually suffices).
    let mut items: Vec<Value> = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut query = base_query.clone();
        if let Some(c) = &cursor {
            query.push(("cursor", c.clone()));
        }
        let value = client.get(&path, Some(&query)).await?;
        let has_next = value
            .get("next_page_results")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let next_cursor = value
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(String::from);
        if let Value::Array(mut page) = unwrap_results(value) {
            items.append(&mut page);
        }
        match (has_next, next_cursor) {
            // Only continue while the cursor actually advances — a server
            // echoing the same cursor with next_page_results=true would
            // otherwise loop forever.
            (true, Some(c)) if cursor.as_ref() != Some(&c) => cursor = Some(c),
            _ => break,
        }
    }

    apply_filters(&mut items, state.as_deref(), label.as_deref());

    if json {
        // Raw payload: keeps every field the API returned (labels, assignees,
        // expanded relations, …) instead of the trimmed WorkItem projection.
        return output::emit_value(&Value::Array(items));
    }
    let items: Vec<WorkItem> = items
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    output::print_work_item_table(&items);
    Ok(())
}

/// Append `required` field names missing from a `--fields` projection.
fn ensure_fields(fields: String, required: &[&str]) -> String {
    let mut parts = util::split_csv(&fields);
    for r in required {
        if !parts.iter().any(|p| p == r) {
            parts.push((*r).to_string());
        }
    }
    parts.join(",")
}

/// Client-side `--state` / `--label` filtering. The community API ignores
/// these as query params on the list endpoint, so they are applied after
/// fetching — still a single list round-trip for the caller.
fn apply_filters(items: &mut Vec<Value>, state: Option<&str>, label: Option<&str>) {
    if let Some(sid) = state {
        items.retain(|v| matches_id(v.get("state"), sid));
    }
    if let Some(lid) = label {
        items.retain(|v| {
            v.get("labels")
                .and_then(Value::as_array)
                .is_some_and(|arr| arr.iter().any(|l| matches_id(Some(l), lid)))
        });
    }
}

/// True when `value` references the given UUID — either as a bare string or as
/// an object carrying an `id` (the shape `expand` switches relations to).
fn matches_id(value: Option<&Value>, id: &str) -> bool {
    match value {
        Some(Value::String(s)) => s == id,
        Some(Value::Object(map)) => map.get("id").and_then(Value::as_str) == Some(id),
        _ => false,
    }
}

// ---- get ----

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        return output::emit_value(&value);
    }
    let item: WorkItem = serde_json::from_value(value).unwrap_or_else(|_| WorkItem {
        id: id.to_string(),
        name: String::new(),
        sequence_id: None,
        priority: None,
        state: None,
        start_date: None,
        target_date: None,
        completed_at: None,
        created_at: None,
        project: None,
    });
    output::print_work_item_table(&[item]);
    Ok(())
}

// ---- get-id ----

async fn get_by_ident(client: &PlaneClient, ident: &str, json: bool) -> Result<()> {
    let parsed = util::parse_work_item_ident(ident).ok_or_else(|| {
        anyhow!("Invalid work item identifier {ident:?}. Expected PROJECT-N format.")
    })?;
    let path = client.ws_path(&format!(
        "work-items/{}-{}",
        parsed.project_identifier, parsed.sequence
    ));
    let value = client.get::<()>(&path, None).await?;
    if json {
        return output::emit_value(&value);
    }
    let item: WorkItem = serde_json::from_value(value).unwrap_or_else(|_| WorkItem {
        id: ident.to_string(),
        name: String::new(),
        sequence_id: None,
        priority: None,
        state: None,
        start_date: None,
        target_date: None,
        completed_at: None,
        created_at: None,
        project: None,
    });
    output::print_work_item_table(&[item]);
    Ok(())
}

// ---- create ----

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    description: Option<String>,
    description_html: Option<String>,
    priority: Option<String>,
    state: Option<String>,
    assignees: Option<String>,
    labels: Option<String>,
    start_date: Option<String>,
    target_date: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));

    if let Some(html) = resolve_description_html(description, description_html) {
        body.insert("description_html".into(), json!(html));
    }

    util::insert_opt_str(&mut body, "priority", priority);
    util::insert_opt_str(&mut body, "state", state);
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "target_date", target_date);
    util::insert_opt_csv_array(&mut body, "assignees", assignees);
    util::insert_opt_csv_array(&mut body, "labels", labels);

    let path = client.ws_path(&format!("projects/{project}/work-items"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        return output::emit_value(&value);
    }
    let item: WorkItem = serde_json::from_value(value).unwrap_or_else(|_| WorkItem {
        id: String::new(),
        name: name.clone(),
        sequence_id: None,
        priority: None,
        state: None,
        start_date: None,
        target_date: None,
        completed_at: None,
        created_at: None,
        project: None,
    });
    output::print_work_item_table(&[item]);
    Ok(())
}

// ---- update ----

#[allow(clippy::too_many_arguments)]
async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    description_html: Option<String>,
    priority: Option<String>,
    state: Option<String>,
    assignees: Option<String>,
    labels: Option<String>,
    start_date: Option<String>,
    target_date: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);

    if let Some(html) = resolve_description_html(description, description_html) {
        body.insert("description_html".into(), json!(html));
    }

    util::insert_opt_str(&mut body, "priority", priority);
    util::insert_opt_str(&mut body, "state", state);
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "target_date", target_date);
    util::insert_opt_csv_array(&mut body, "assignees", assignees);
    util::insert_opt_csv_array(&mut body, "labels", labels);

    let path = client.ws_path(&format!("projects/{project}/work-items/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        return output::emit_value(&value);
    }
    let item: WorkItem = serde_json::from_value(value).unwrap_or_else(|_| WorkItem {
        id: id.to_string(),
        name: String::new(),
        sequence_id: None,
        priority: None,
        state: None,
        start_date: None,
        target_date: None,
        completed_at: None,
        created_at: None,
        project: None,
    });
    output::print_work_item_table(&[item]);
    Ok(())
}

// ---- delete ----

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted work item {id}"));
    }
    Ok(())
}

// ---- search ----

async fn search(client: &PlaneClient, query: &str, limit: usize, json: bool) -> Result<()> {
    let path = client.ws_path("work-items/search");
    let q = vec![("search", query.to_string())];
    let value = client.get(&path, Some(&q)).await?;
    // The search endpoint returns a list directly (or wrapped); normalise both.
    let mut items: Vec<WorkItem> =
        serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    items.truncate(limit);
    output::render(&items, json, |v| output::print_work_item_table(v))
}

// ---- count ----

async fn count(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    // The community REST API has no dedicated count endpoint, but every list
    // response carries a `total_count` (and `count`) in its pagination envelope,
    // so we read it off the project work-items list.
    let path = client.ws_path(&format!("projects/{project}/work-items"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        return output::emit_value(&value);
    }
    let total = value
        .get("total_count")
        .or_else(|| value.get("count"))
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    output::print_message(&format!("Total work items: {total}"));
    Ok(())
}

// ---- assignee management ----

async fn manage_assignees(
    client: &PlaneClient,
    project: &str,
    id: &str,
    add: Option<String>,
    remove: Option<String>,
    json: bool,
) -> Result<()> {
    if add.is_none() && remove.is_none() {
        return Err(anyhow!("Provide --add and/or --remove"));
    }

    // Fetch current item to read existing assignees.
    let path = client.ws_path(&format!("projects/{project}/work-items/{id}"));
    let current = client.get::<()>(&path, None).await?;

    let mut ids: Vec<String> = current
        .get("assignees")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|u| {
                    u.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| u.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(rem) = remove {
        let to_remove: Vec<String> = util::split_csv(&rem);
        ids.retain(|uid| !to_remove.contains(uid));
    }
    if let Some(add_csv) = add {
        for uid in util::split_csv(&add_csv) {
            if !ids.contains(&uid) {
                ids.push(uid);
            }
        }
    }

    let body = json!({ "assignees": ids });
    let value = client.patch(&path, Some(&body)).await?;
    if json {
        return output::emit_value(&value);
    }
    output::print_message(&format!("Updated assignees on {id}"));
    Ok(())
}

// ---- label management ----

async fn manage_labels(
    client: &PlaneClient,
    project: &str,
    id: &str,
    add: Option<String>,
    remove: Option<String>,
    json: bool,
) -> Result<()> {
    if add.is_none() && remove.is_none() {
        return Err(anyhow!("Provide --add and/or --remove"));
    }

    let path = client.ws_path(&format!("projects/{project}/work-items/{id}"));
    let current = client.get::<()>(&path, None).await?;

    let mut ids: Vec<String> = current
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|lb| {
                    lb.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| lb.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(rem) = remove {
        let to_remove: Vec<String> = util::split_csv(&rem);
        ids.retain(|lid| !to_remove.contains(lid));
    }
    if let Some(add_csv) = add {
        for lid in util::split_csv(&add_csv) {
            if !ids.contains(&lid) {
                ids.push(lid);
            }
        }
    }

    let body = json!({ "labels": ids });
    let value = client.patch(&path, Some(&body)).await?;
    if json {
        return output::emit_value(&value);
    }
    output::print_message(&format!("Updated labels on {id}"));
    Ok(())
}

// ---- archive ----

async fn archive(client: &PlaneClient, project: &str, id: &str) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{id}/archive"));
    // Archive is a POST with empty body.
    client.post::<Value>(&path, None).await?;
    output::print_message(&format!("Archived work item {id}"));
    Ok(())
}

// ---- unarchive ----

async fn unarchive(client: &PlaneClient, project: &str, id: &str) -> Result<()> {
    // Unarchive is a DELETE on the /unarchive sub-resource (SDK uses _delete).
    let path = client.ws_path(&format!("projects/{project}/work-items/{id}/unarchive"));
    client.delete(&path).await?;
    output::print_message(&format!("Unarchived work item {id}"));
    Ok(())
}

// ---- list-archived ----

async fn list_archived(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/archived-work-items"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<WorkItem> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |v| output::print_work_item_table(v))
}

// ---- helpers ----

/// Pick the `description_html` body value from the two description flags.
///
/// `--description-html` wins and is sent verbatim (caller already wrote HTML).
/// `--description` is plain text: escaped and wrapped in a paragraph so the
/// markup renders instead of leaking literal `<tags>` into the issue body.
/// Returns `None` when neither flag is set, leaving the field untouched.
fn resolve_description_html(
    description: Option<String>,
    description_html: Option<String>,
) -> Option<String> {
    match description_html {
        Some(html) => Some(html),
        None => description.map(|desc| format!("<p>{}</p>", html_escape(&desc))),
    }
}

/// Minimal HTML entity escaping for plain-text → HTML conversion.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_html_is_sent_verbatim() {
        let out = resolve_description_html(None, Some("<h2>Sorun</h2>".into()));
        assert_eq!(out.as_deref(), Some("<h2>Sorun</h2>"));
    }

    #[test]
    fn description_html_takes_precedence_over_plain() {
        let out = resolve_description_html(Some("plain".into()), Some("<p><b>rich</b></p>".into()));
        assert_eq!(out.as_deref(), Some("<p><b>rich</b></p>"));
    }

    #[test]
    fn plain_description_is_escaped_and_wrapped() {
        let out = resolve_description_html(Some("a < b & \"c\"".into()), None);
        assert_eq!(out.as_deref(), Some("<p>a &lt; b &amp; &quot;c&quot;</p>"));
    }

    #[test]
    fn no_description_leaves_field_untouched() {
        assert_eq!(resolve_description_html(None, None), None);
    }

    #[test]
    fn ensure_fields_appends_missing_only() {
        assert_eq!(
            ensure_fields("name,sequence_id".into(), &["id", "state"]),
            "name,sequence_id,id,state"
        );
        assert_eq!(ensure_fields("id,name".into(), &["id"]), "id,name");
        assert_eq!(ensure_fields("id".into(), &[]), "id");
    }

    #[test]
    fn apply_filters_matches_bare_and_expanded_shapes() {
        let base = vec![
            json!({"id": "1", "state": "s1", "labels": ["l1"]}),
            json!({"id": "2", "state": {"id": "s2"}, "labels": [{"id": "l2"}]}),
            json!({"id": "3", "state": "s1", "labels": []}),
        ];

        let mut items = base.clone();
        apply_filters(&mut items, Some("s2"), None);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], "2");

        let mut items = base.clone();
        apply_filters(&mut items, None, Some("l1"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], "1");

        let mut items = base;
        apply_filters(&mut items, Some("s1"), Some("l1"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], "1");
    }

    #[test]
    fn apply_filters_drops_items_missing_the_filtered_field() {
        // Items lacking the filtered field never match — the documented edge
        // ensure_fields() protects against for --fields projections.
        let mut items = vec![json!({"id": "1"})];
        apply_filters(&mut items, Some("s1"), None);
        assert!(items.is_empty());

        let mut items = vec![json!({"id": "1"})];
        apply_filters(&mut items, None, Some("l1"));
        assert!(items.is_empty());
    }

    #[test]
    fn matches_id_on_bare_string_and_object() {
        let s = json!("abc-123");
        let o = json!({"id": "abc-123", "name": "Onay Bekliyor"});
        assert!(matches_id(Some(&s), "abc-123"));
        assert!(matches_id(Some(&o), "abc-123"));
        assert!(!matches_id(Some(&s), "other"));
        assert!(!matches_id(Some(&o), "other"));
        assert!(!matches_id(None, "abc-123"));
        assert!(!matches_id(Some(&Value::Null), "abc-123"));
    }
}
