use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::Label;
use crate::util;

#[derive(Subcommand)]
pub enum LabelCmd {
    /// List all labels in a project
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
    /// Get a label by ID
    Get {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Label UUID
        id: String,
    },
    /// Create a new label
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Label name
        name: String,
        /// Hex color code (e.g. #ff0000)
        #[arg(long)]
        color: Option<String>,
        /// Label description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a label
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Label UUID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New hex color code
        #[arg(long)]
        color: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a label
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Label UUID
        id: String,
    },
}

pub async fn run(cmd: LabelCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        LabelCmd::List { project } => list(client, &project, json).await,
        LabelCmd::Get { project, id } => get(client, &project, &id, json).await,
        LabelCmd::Create {
            project,
            name,
            color,
            description,
        } => create(client, &project, name, color, description, json).await,
        LabelCmd::Update {
            project,
            id,
            name,
            color,
            description,
        } => update(client, &project, &id, name, color, description, json).await,
        LabelCmd::Delete { project, id } => delete(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/labels"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Label> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |v| output::print_label_table(v))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/labels/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Label = serde_json::from_value(value).unwrap_or(Label {
            id: id.to_string(),
            name: String::new(),
            color: None,
            description: None,
        });
        output::print_label_table(&[item]);
        Ok(())
    }
}

async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    color: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    util::insert_opt_str(&mut body, "color", color);
    util::insert_opt_str(&mut body, "description", description);
    let path = client.ws_path(&format!("projects/{project}/labels"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Label = serde_json::from_value(value).unwrap_or(Label {
            id: String::new(),
            name,
            color: None,
            description: None,
        });
        output::print_label_table(&[item]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    color: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "color", color);
    util::insert_opt_str(&mut body, "description", description);
    let path = client.ws_path(&format!("projects/{project}/labels/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Label = serde_json::from_value(value).unwrap_or(Label {
            id: id.to_string(),
            name: String::new(),
            color: None,
            description: None,
        });
        output::print_label_table(&[item]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/labels/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted label {id}"));
    }
    Ok(())
}
