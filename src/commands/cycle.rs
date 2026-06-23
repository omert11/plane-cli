use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::{Cycle, WorkItem};
use crate::util;

#[derive(Subcommand)]
pub enum CycleCmd {
    /// List all cycles in a project
    List {
        #[arg(long)]
        project: String,
    },
    /// Get a cycle by ID
    Get {
        #[arg(long)]
        project: String,
        id: String,
    },
    /// Create a new cycle
    Create {
        #[arg(long)]
        project: String,
        name: String,
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        end_date: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a cycle
    Update {
        #[arg(long)]
        project: String,
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        end_date: Option<String>,
    },
    /// Delete a cycle
    Delete {
        #[arg(long)]
        project: String,
        id: String,
    },
    /// List work items in a cycle
    ListItems {
        #[arg(long)]
        project: String,
        id: String,
    },
    /// Add work items to a cycle (comma-separated issue UUIDs)
    AddItems {
        #[arg(long)]
        project: String,
        id: String,
        issues: String,
    },
    /// Archive a cycle
    Archive {
        #[arg(long)]
        project: String,
        id: String,
    },
    /// Unarchive a cycle
    Unarchive {
        #[arg(long)]
        project: String,
        id: String,
    },
}

pub async fn run(cmd: CycleCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        CycleCmd::List { project } => list(client, &project, json).await,
        CycleCmd::Get { project, id } => get(client, &project, &id, json).await,
        CycleCmd::Create {
            project,
            name,
            start_date,
            end_date,
            description,
        } => {
            create(
                client,
                &project,
                name,
                start_date,
                end_date,
                description,
                json,
            )
            .await
        }
        CycleCmd::Update {
            project,
            id,
            name,
            start_date,
            end_date,
        } => update(client, &project, &id, name, start_date, end_date, json).await,
        CycleCmd::Delete { project, id } => delete(client, &project, &id, json).await,
        CycleCmd::ListItems { project, id } => list_items(client, &project, &id, json).await,
        CycleCmd::AddItems {
            project,
            id,
            issues,
        } => add_items(client, &project, &id, &issues, json).await,
        CycleCmd::Archive { project, id } => archive(client, &project, &id, json).await,
        CycleCmd::Unarchive { project, id } => unarchive(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/cycles"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Cycle> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |c| output::print_cycle_table(c))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Cycle = serde_json::from_value(value).unwrap_or(Cycle {
            id: id.to_string(),
            name: String::new(),
            start_date: None,
            end_date: None,
            archived_at: None,
        });
        output::print_cycle_table(&[item]);
        Ok(())
    }
}

async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    start_date: Option<String>,
    end_date: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "end_date", end_date);
    util::insert_opt_str(&mut body, "description", description);
    let path = client.ws_path(&format!("projects/{project}/cycles"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Cycle = serde_json::from_value(value).unwrap_or(Cycle {
            id: String::new(),
            name: name.clone(),
            start_date: None,
            end_date: None,
            archived_at: None,
        });
        output::print_cycle_table(&[item]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "end_date", end_date);
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Cycle = serde_json::from_value(value).unwrap_or(Cycle {
            id: id.to_string(),
            name: String::new(),
            start_date: None,
            end_date: None,
            archived_at: None,
        });
        output::print_cycle_table(&[item]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted cycle {id}"));
    }
    Ok(())
}

async fn list_items(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}/cycle-issues"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<WorkItem> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |w| output::print_work_item_table(w))
}

async fn add_items(
    client: &PlaneClient,
    project: &str,
    id: &str,
    issues_csv: &str,
    json: bool,
) -> Result<()> {
    let issue_ids: Vec<Value> = util::split_csv(issues_csv)
        .into_iter()
        .map(Value::String)
        .collect();
    let body = json!({ "issues": issue_ids });
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}/cycle-issues"));
    let value = client.post(&path, Some(&body)).await?;
    if json {
        output::emit_value(&value)
    } else {
        output::print_message(&format!("Added {} issue(s) to cycle {id}", issue_ids.len()));
        Ok(())
    }
}

async fn archive(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/cycles/{id}/archive"));
    let value = client.post(&path, Some(&json!({}))).await?;
    if json {
        output::emit_value(&value)
    } else {
        output::print_message(&format!("Archived cycle {id}"));
        Ok(())
    }
}

async fn unarchive(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    // Plane unarchive: DELETE .../archived-cycles/{id}/unarchive
    let path = client.ws_path(&format!(
        "projects/{project}/archived-cycles/{id}/unarchive"
    ));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Unarchived cycle {id}"));
    }
    Ok(())
}
