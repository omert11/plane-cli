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
        IssueCmd::List { project } => list(client, &project, json).await,
        IssueCmd::Get { project, id } => get(client, &project, &id, json).await,
        IssueCmd::GetId { ident } => get_by_ident(client, &ident, json).await,
        IssueCmd::Create {
            project,
            name,
            description,
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

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<WorkItem> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |v| output::print_work_item_table(v))
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

    // Plane stores description as HTML; wrap plain text in a paragraph.
    if let Some(desc) = description {
        let html = format!("<p>{}</p>", html_escape(&desc));
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

    if let Some(desc) = description {
        let html = format!("<p>{}</p>", html_escape(&desc));
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

/// Minimal HTML entity escaping for plain-text → HTML conversion.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
