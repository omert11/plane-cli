use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::Page;
use crate::util;

#[derive(Subcommand)]
pub enum PageCmd {
    /// List pages in a project
    List {
        #[arg(long)]
        project: String,
    },
    /// Get a page by ID
    Get {
        #[arg(long)]
        project: String,
        id: String,
    },
    /// Create a page in a project
    Create {
        #[arg(long)]
        project: String,
        name: String,
        #[arg(long)]
        description_html: Option<String>,
    },
    /// Update a page's name
    Update {
        #[arg(long)]
        project: String,
        id: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a page
    Delete {
        #[arg(long)]
        project: String,
        id: String,
    },
}

pub async fn run(cmd: PageCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        PageCmd::List { project } => list(client, &project, json).await,
        PageCmd::Get { project, id } => get(client, &project, &id, json).await,
        PageCmd::Create {
            project,
            name,
            description_html,
        } => create(client, &project, name, description_html, json).await,
        PageCmd::Update { project, id, name } => update(client, &project, &id, name, json).await,
        PageCmd::Delete { project, id } => delete(client, &project, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/pages"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Page> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |p| output::print_page_table(p))
}

async fn get(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/pages/{id}"));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Page = serde_json::from_value(value).unwrap_or(Page {
            id: id.to_string(),
            name: String::new(),
            created_at: None,
        });
        output::print_page_table(&[item]);
        Ok(())
    }
}

async fn create(
    client: &PlaneClient,
    project: &str,
    name: String,
    description_html: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    body.insert("name".into(), json!(name));
    util::insert_opt_str(&mut body, "description_html", description_html);
    let path = client.ws_path(&format!("projects/{project}/pages"));
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Page = serde_json::from_value(value).unwrap_or(Page {
            id: String::new(),
            name,
            created_at: None,
        });
        output::print_page_table(&[item]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    project: &str,
    id: &str,
    name: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "name", name);
    let path = client.ws_path(&format!("projects/{project}/pages/{id}"));
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Page = serde_json::from_value(value).unwrap_or(Page {
            id: id.to_string(),
            name: String::new(),
            created_at: None,
        });
        output::print_page_table(&[item]);
        Ok(())
    }
}

async fn delete(client: &PlaneClient, project: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/pages/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted page {id}"));
    }
    Ok(())
}
