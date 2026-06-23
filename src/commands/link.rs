use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::WorkItemLink;
use crate::util;

#[derive(Subcommand)]
pub enum LinkCmd {
    /// List links for a work item
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
    },
    /// Add a link to a work item
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
        /// URL of the link
        url: String,
        /// Optional display title for the link
        #[arg(long)]
        title: Option<String>,
    },
    /// Remove a link from a work item
    Remove {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
        /// Link UUID
        id: String,
    },
}

pub async fn run(cmd: LinkCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        LinkCmd::List { project, issue } => list(client, &project, &issue, json).await,
        LinkCmd::Create {
            project,
            issue,
            url,
            title,
        } => create(client, &project, &issue, url, title, json).await,
        LinkCmd::Remove { project, issue, id } => remove(client, &project, &issue, &id, json).await,
    }
}

async fn list(client: &PlaneClient, project: &str, issue: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/links"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<WorkItemLink> =
        serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |v| print_link_table(v))
}

async fn create(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    url: String,
    title: Option<String>,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/links"));
    let mut body = Map::new();
    body.insert("url".into(), json!(url));
    util::insert_opt_str(&mut body, "title", title);
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let link: WorkItemLink = serde_json::from_value(value).unwrap_or(WorkItemLink {
            id: "-".into(),
            url: None,
            title: None,
            created_at: None,
        });
        output::print_message(&format!(
            "Created link {} → {}",
            link.id,
            link.url.as_deref().unwrap_or("-")
        ));
        Ok(())
    }
}

async fn remove(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/links/{id}"));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Removed link {id}"));
    }
    Ok(())
}

fn print_link_table(items: &[WorkItemLink]) {
    use crate::util::truncate;
    use colored::Colorize;
    use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["ID", "Title", "URL", "Created"]);
    for l in items {
        table.add_row(vec![
            Cell::new(truncate(&l.id, 36)),
            Cell::new(truncate(l.title.as_deref().unwrap_or("-"), 30)),
            Cell::new(truncate(l.url.as_deref().unwrap_or("-"), 60)),
            Cell::new(l.created_at.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    println!("{} {}", items.len().to_string().bold(), "links".dimmed());
}
