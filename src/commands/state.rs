use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::State;
use crate::util;

#[derive(Subcommand)]
pub enum StateCmd {
    /// List all states in a project
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
    },
    /// Get a state by ID
    Get {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// State UUID
        id: String,
    },
    /// Create a new state
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// State name
        name: String,
        /// Hex color code (e.g. #ff0000)
        color: String,
        /// State group: backlog, unstarted, started, completed, cancelled
        #[arg(long)]
        group: Option<String>,
        /// Mark as the default state
        #[arg(long)]
        default: Option<bool>,
        /// State description
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a state by ID
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// State UUID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New hex color code
        #[arg(long)]
        color: Option<String>,
        /// New group: backlog, unstarted, started, completed, cancelled
        #[arg(long)]
        group: Option<String>,
    },
    /// Delete a state by ID
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// State UUID
        id: String,
    },
}

pub async fn run(cmd: StateCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        StateCmd::List { project } => list(client, &project, json).await,
        StateCmd::Get { project, id } => get(client, &project, &id, json).await,
        StateCmd::Create {
            project,
            name,
            color,
            group,
            default,
            description,
        } => {
            create(
                client,
                &project,
                name,
                color,
                group,
                default,
                description,
                json,
            )
            .await
        }
        StateCmd::Update {
            project,
            id,
            name,
            color,
            group,
        } => update(client, &project, &id, name, color, group, json).await,
        StateCmd::Delete { project, id } => delete(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/states"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<State> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |s| output::print_state_table(s))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/states/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let state: State = serde_json::from_value(value).unwrap_or(State {
            id: id.to_string(),
            name: String::new(),
            color: None,
            group: None,
            default: false,
            sequence: None,
        });
        output::print_state_table(&[state]);
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    color: String,
    group: Option<String>,
    default: Option<bool>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    body.insert("color".into(), json!(color));
    util::insert_opt_str(&mut body, "group", group);
    util::insert_opt_bool(&mut body, "default", default);
    util::insert_opt_str(&mut body, "description", description);

    let path = client.ws_path(&format!("projects/{project}/states"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let state: State = serde_json::from_value(value).unwrap_or(State {
            id: String::new(),
            name,
            color: Some(color),
            group: None,
            default: false,
            sequence: None,
        });
        output::print_state_table(&[state]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    color: Option<String>,
    group: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "color", color);
    util::insert_opt_str(&mut body, "group", group);

    let path = client.ws_path(&format!("projects/{project}/states/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let state: State = serde_json::from_value(value).unwrap_or(State {
            id: id.to_string(),
            name: String::new(),
            color: None,
            group: None,
            default: false,
            sequence: None,
        });
        output::print_state_table(&[state]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/states/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted state {id}"));
    }
    Ok(())
}
