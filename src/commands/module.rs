use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::{Module, WorkItem};
use crate::util;

#[derive(Subcommand)]
pub enum ModuleCmd {
    /// List modules in a project
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
    /// Get a module by ID
    Get {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
    },
    /// Create a new module
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module name
        name: String,
        /// Start date (ISO 8601, e.g. 2024-01-15)
        #[arg(long)]
        start_date: Option<String>,
        /// Target/end date (ISO 8601)
        #[arg(long)]
        target_date: Option<String>,
        /// UUID of the lead user
        #[arg(long)]
        lead: Option<String>,
        /// Module description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a module
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New status (backlog, planned, in-progress, paused, completed, cancelled)
        #[arg(long)]
        status: Option<String>,
        /// New start date (ISO 8601)
        #[arg(long)]
        start_date: Option<String>,
        /// New target date (ISO 8601)
        #[arg(long)]
        target_date: Option<String>,
    },
    /// Delete a module
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
    },
    /// List work items in a module
    ListItems {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
    },
    /// Add work items to a module (comma-separated issue UUIDs)
    AddItems {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
        /// Comma-separated work item UUIDs to add
        issues: String,
    },
    /// Archive a module
    Archive {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
    },
    /// Unarchive a module
    Unarchive {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Module UUID
        id: String,
    },
}

pub async fn run(cmd: ModuleCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        ModuleCmd::List { project } => list(client, &project, json).await,
        ModuleCmd::Get { project, id } => get(client, &project, &id, json).await,
        ModuleCmd::Create {
            project,
            name,
            start_date,
            target_date,
            lead,
            description,
        } => {
            create(
                client,
                &project,
                name,
                start_date,
                target_date,
                lead,
                description,
                json,
            )
            .await
        }
        ModuleCmd::Update {
            project,
            id,
            name,
            status,
            start_date,
            target_date,
        } => {
            update(
                client,
                &project,
                &id,
                name,
                status,
                start_date,
                target_date,
                json,
            )
            .await
        }
        ModuleCmd::Delete { project, id } => delete(client, &project, &id, json).await,
        ModuleCmd::ListItems { project, id } => list_items(client, &project, &id, json).await,
        ModuleCmd::AddItems {
            project,
            id,
            issues,
        } => add_items(client, &project, &id, &issues, json).await,
        ModuleCmd::Archive { project, id } => archive(client, &project, &id, json).await,
        ModuleCmd::Unarchive { project, id } => unarchive(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/modules"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Module> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |m| output::print_module_table(m))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/modules/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Module = serde_json::from_value(value).unwrap_or_else(|_| Module {
            id: id.to_string(),
            name: String::new(),
            status: None,
            start_date: None,
            target_date: None,
            archived_at: None,
        });
        output::print_module_table(&[item]);
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    start_date: Option<String>,
    target_date: Option<String>,
    lead: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "target_date", target_date);
    util::insert_opt_str(&mut body, "lead", lead);
    util::insert_opt_str(&mut body, "description", description);

    let path = client.ws_path(&format!("projects/{project}/modules"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Module = serde_json::from_value(value).unwrap_or_else(|_| Module {
            id: String::new(),
            name,
            status: None,
            start_date: None,
            target_date: None,
            archived_at: None,
        });
        output::print_module_table(&[item]);
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    status: Option<String>,
    start_date: Option<String>,
    target_date: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "status", status);
    util::insert_opt_str(&mut body, "start_date", start_date);
    util::insert_opt_str(&mut body, "target_date", target_date);

    let path = client.ws_path(&format!("projects/{project}/modules/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Module = serde_json::from_value(value).unwrap_or_else(|_| Module {
            id: id.to_string(),
            name: String::new(),
            status: None,
            start_date: None,
            target_date: None,
            archived_at: None,
        });
        output::print_module_table(&[item]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/modules/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted module {id}"));
    }
    Ok(())
}

async fn list_items(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/modules/{id}/module-issues"));
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

    let path = client.ws_path(&format!("projects/{project}/modules/{id}/module-issues"));
    let value = client.post(&path, Some(&body)).await?;
    if json {
        output::emit_value(&value)
    } else {
        output::print_message(&format!(
            "Added {} issue(s) to module {id}",
            issue_ids.len()
        ));
        Ok(())
    }
}

async fn archive(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/modules/{id}/archive"));
    let value = client.post(&path, Some(&json!({}))).await?;
    if json {
        output::emit_value(&value)
    } else {
        output::print_message(&format!("Archived module {id}"));
        Ok(())
    }
}

async fn unarchive(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/archived-modules/{id}/unarchive"
    ));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Unarchived module {id}"));
    }
    Ok(())
}
