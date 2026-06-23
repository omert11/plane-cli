use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::util;

#[derive(Subcommand)]
pub enum WorklogCmd {
    /// List work logs for a work item
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
    },
    /// Create a work log for a work item
    Create {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
        /// Duration in minutes
        duration: i64,
        /// Optional description of the work performed
        #[arg(long)]
        description: Option<String>,
    },
    /// Update a work log
    Update {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
        /// Work log UUID
        id: String,
        /// Duration in minutes
        #[arg(long)]
        duration: Option<i64>,
        /// Description of the work performed
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a work log
    Delete {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work item UUID
        #[arg(long)]
        issue: String,
        /// Work log UUID
        id: String,
    },
}

pub async fn run(cmd: WorklogCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        WorklogCmd::List { project, issue } => list(client, &project, &issue, json).await,
        WorklogCmd::Create {
            project,
            issue,
            duration,
            description,
        } => create(client, &project, &issue, duration, description, json).await,
        WorklogCmd::Update {
            project,
            issue,
            id,
            duration,
            description,
        } => update(client, &project, &issue, &id, duration, description, json).await,
        WorklogCmd::Delete { project, issue, id } => {
            delete(client, &project, &issue, &id, json).await
        }
    }
}

async fn list(client: &PlaneClient, project: &str, issue: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/worklogs"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Value> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    if json {
        output::emit_json(&items)?;
    } else {
        print_worklog_table(&items);
    }
    Ok(())
}

async fn create(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    duration: i64,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/worklogs"));
    let mut body = Map::new();
    body.insert("duration".into(), json!(duration));
    util::insert_opt_str(&mut body, "description", description);
    let value = client.post(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)?;
    } else {
        output::print_message("Work log created");
    }
    Ok(())
}

async fn update(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    duration: Option<i64>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/work-items/{issue}/worklogs/{id}"
    ));
    let mut body = Map::new();
    if let Some(d) = duration {
        body.insert("duration".into(), json!(d));
    }
    util::insert_opt_str(&mut body, "description", description);
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)?;
    } else {
        output::print_message(&format!("Work log {id} updated"));
    }
    Ok(())
}

async fn delete(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/work-items/{issue}/worklogs/{id}"
    ));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted work log {id}"));
    }
    Ok(())
}

fn print_worklog_table(items: &[Value]) {
    use colored::Colorize;
    use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["ID", "Duration (min)", "Description", "Created"]);
    for item in items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let duration = item
            .get("duration")
            .and_then(|v| v.as_i64())
            .map(|d| d.to_string())
            .unwrap_or_else(|| "-".into());
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let created = item
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        table.add_row(vec![
            Cell::new(crate::util::truncate(id, 36)),
            Cell::new(&duration),
            Cell::new(crate::util::truncate(description, 40)),
            Cell::new(created),
        ]);
    }
    println!("{table}");
    println!(
        "{} {}",
        items.len().to_string().bold(),
        "work logs".dimmed()
    );
}
