use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::{Member, Project};
use crate::util;

#[derive(Subcommand)]
pub enum ProjectCmd {
    /// List all projects in the workspace
    List,
    /// Get a project by ID
    Get { id: String },
    /// Create a new project
    Create {
        /// Project display name
        name: String,
        /// Short uppercase identifier (e.g. "MP")
        identifier: String,
        #[arg(long)]
        description: Option<String>,
        /// UUID of the project lead user
        #[arg(long)]
        lead: Option<String>,
        /// Network visibility: 0=secret, 2=public
        #[arg(long)]
        network: Option<i64>,
    },
    /// Update a project
    Update {
        /// Project UUID
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        identifier: Option<String>,
    },
    /// Delete a project
    Delete {
        /// Project UUID
        id: String,
    },
    /// List members of a project
    Members {
        /// Project UUID
        id: String,
    },
    /// Get features/settings of a project
    Features {
        /// Project UUID
        id: String,
    },
    /// Archive a project
    Archive {
        /// Project UUID
        id: String,
    },
    /// Unarchive a project
    Unarchive {
        /// Project UUID
        id: String,
    },
}

pub async fn run(cmd: ProjectCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        ProjectCmd::List => list(client, json).await,
        ProjectCmd::Get { id } => get(client, &id, json).await,
        ProjectCmd::Create {
            name,
            identifier,
            description,
            lead,
            network,
        } => create(client, name, identifier, description, lead, network, json).await,
        ProjectCmd::Update {
            id,
            name,
            description,
            identifier,
        } => update(client, &id, name, description, identifier, json).await,
        ProjectCmd::Delete { id } => delete(client, &id, json).await,
        ProjectCmd::Members { id } => members(client, &id, json).await,
        ProjectCmd::Features { id } => features(client, &id, json).await,
        ProjectCmd::Archive { id } => archive(client, &id, json).await,
        ProjectCmd::Unarchive { id } => unarchive(client, &id, json).await,
    }
}

async fn list(client: &PlaneClient, json: bool) -> Result<()> {
    let value = client.get::<()>(&client.ws_path("projects"), None).await?;
    let items: Vec<Project> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |p| output::print_project_table(p))
}

async fn get(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    let value = client
        .get::<()>(&client.ws_path(&format!("projects/{id}")), None)
        .await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Project = serde_json::from_value(value).unwrap_or_else(|_| Project {
            id: id.to_string(),
            name: String::new(),
            identifier: String::new(),
            description: None,
            network: None,
            archived_at: None,
            created_at: None,
        });
        output::print_project_table(&[item]);
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &PlaneClient,
    name: String,
    identifier: String,
    description: Option<String>,
    lead: Option<String>,
    network: Option<i64>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    body.insert("identifier".into(), json!(identifier));
    util::insert_opt_str(&mut body, "description", description);
    util::insert_opt_str(&mut body, "project_lead", lead);
    if let Some(n) = network {
        body.insert("network".into(), json!(n));
    }
    let value = client
        .post(&client.ws_path("projects"), Some(&Value::Object(body)))
        .await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Project = serde_json::from_value(value).unwrap_or_else(|_| Project {
            id: String::new(),
            name,
            identifier,
            description: None,
            network: None,
            archived_at: None,
            created_at: None,
        });
        output::print_project_table(&[item]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    identifier: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    util::insert_opt_str(&mut body, "description", description);
    util::insert_opt_str(&mut body, "identifier", identifier);
    let value = client
        .patch(
            &client.ws_path(&format!("projects/{id}")),
            Some(&Value::Object(body)),
        )
        .await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Project = serde_json::from_value(value).unwrap_or_else(|_| Project {
            id: id.to_string(),
            name: String::new(),
            identifier: String::new(),
            description: None,
            network: None,
            archived_at: None,
            created_at: None,
        });
        output::print_project_table(&[item]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    client
        .delete(&client.ws_path(&format!("projects/{id}")))
        .await?;
    if !json {
        output::print_message(&format!("Deleted project {id}"));
    }
    Ok(())
}

async fn members(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    let value = client
        .get::<()>(&client.ws_path(&format!("projects/{id}/members")), None)
        .await?;
    // Members endpoint returns a plain array (no results envelope)
    let items: Vec<Member> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |m| output::print_member_table(m))
}

async fn features(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    let value = client
        .get::<()>(&client.ws_path(&format!("projects/{id}/features")), None)
        .await?;
    output::emit_value(&value)?;
    if !json {
        // emit_value always prints JSON; no human table for feature flags
    }
    Ok(())
}

async fn archive(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    client
        .post::<Value>(
            &client.ws_path(&format!("projects/{id}/archive")),
            Some(&json!({})),
        )
        .await?;
    if !json {
        output::print_message(&format!("Archived project {id}"));
    }
    Ok(())
}

async fn unarchive(client: &PlaneClient, id: &str, json: bool) -> Result<()> {
    client
        .delete(&client.ws_path(&format!("projects/{id}/archive")))
        .await?;
    if !json {
        output::print_message(&format!("Unarchived project {id}"));
    }
    Ok(())
}
