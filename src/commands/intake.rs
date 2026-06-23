use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::WorkItem;
use crate::util;

#[derive(Subcommand)]
pub enum IntakeCmd {
    /// List intake work items in a project
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
    /// Get an intake work item by ID
    Get {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
    /// Create an intake work item
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item name (title)
        name: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update an intake work item
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete an intake work item
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        id: String,
    },
}

pub async fn run(cmd: IntakeCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        IntakeCmd::List { project } => list(client, &project, json).await,
        IntakeCmd::Get { project, id } => get(client, &project, &id, json).await,
        IntakeCmd::Create {
            project,
            name,
            description,
        } => create(client, &project, name, description, json).await,
        IntakeCmd::Update {
            project,
            id,
            name,
            description,
        } => update(client, &project, &id, name, description, json).await,
        IntakeCmd::Delete { project, id } => delete(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/intake-issues"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<WorkItem> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |v| output::print_work_item_table(v))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/intake-issues/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
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
}

async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    util::insert_opt_str(&mut body, "description", description);
    let path = client.ws_path(&format!("projects/{project}/intake-issues"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: WorkItem = serde_json::from_value(value).unwrap_or_else(|_| WorkItem {
            id: String::new(),
            name,
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
}

async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "description", description);
    let path = client.ws_path(&format!("projects/{project}/intake-issues/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
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
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/intake-issues/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted intake work item {id}"));
    }
    Ok(())
}
