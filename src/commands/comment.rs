use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::Comment;
use crate::util;

#[derive(Subcommand)]
pub enum CommentCmd {
    /// List all comments for a work item
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
    },
    /// Get a single comment by id
    Get {
        /// Comment UUID
        id: String,
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
    },
    /// Add a comment to a work item
    Add {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
        /// Comment body in HTML format
        #[arg(long)]
        comment_html: String,
    },
    /// Update an existing comment
    Update {
        /// Comment UUID
        id: String,
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
        /// New comment body in HTML format
        #[arg(long)]
        comment_html: Option<String>,
    },
    /// Delete a comment
    Delete {
        /// Comment UUID
        id: String,
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
    },
}

pub async fn run(cmd: CommentCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        CommentCmd::List { project, issue } => list(client, &project, &issue, json).await,
        CommentCmd::Get { id, project, issue } => get(client, &project, &issue, &id, json).await,
        CommentCmd::Add {
            project,
            issue,
            comment_html,
        } => add(client, &project, &issue, comment_html, json).await,
        CommentCmd::Update {
            id,
            project,
            issue,
            comment_html,
        } => update(client, &project, &issue, &id, comment_html, json).await,
        CommentCmd::Delete { id, project, issue } => {
            delete(client, &project, &issue, &id, json).await
        }
    }
}

async fn list(client: &PlaneClient, project: &str, issue: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/comments"));
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Comment> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |c| output::print_comment_table(c))
}

async fn get(client: &PlaneClient, project: &str, issue: &str, id: &str, json: bool) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/work-items/{issue}/comments/{id}"
    ));
    let value = client.get::<()>(&path, None).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Comment = serde_json::from_value(value).unwrap_or_else(|_| Comment {
            id: id.to_string(),
            comment_html: None,
            comment_stripped: None,
            actor: None,
            created_at: None,
        });
        output::print_comment_table(&[item]);
        Ok(())
    }
}

async fn add(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    comment_html: String,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/comments"));
    let body = json!({ "comment_html": comment_html });
    let value = client.post(&path, Some(&body)).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Comment = serde_json::from_value(value).unwrap_or_else(|_| Comment {
            id: String::new(),
            comment_html: Some(comment_html),
            comment_stripped: None,
            actor: None,
            created_at: None,
        });
        output::print_comment_table(&[item]);
        Ok(())
    }
}

async fn update(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    comment_html: Option<String>,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/work-items/{issue}/comments/{id}"
    ));
    let mut body = Map::new();
    util::insert_opt_str(&mut body, "comment_html", comment_html);
    let value = client.patch(&path, Some(&Value::Object(body))).await?;
    if json {
        output::emit_value(&value)
    } else {
        let item: Comment = serde_json::from_value(value).unwrap_or_else(|_| Comment {
            id: id.to_string(),
            comment_html: None,
            comment_stripped: None,
            actor: None,
            created_at: None,
        });
        output::print_comment_table(&[item]);
        Ok(())
    }
}

async fn delete(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    json: bool,
) -> Result<()> {
    let path = client.ws_path(&format!(
        "projects/{project}/work-items/{issue}/comments/{id}"
    ));
    client.delete(&path).await?;
    if !json {
        output::print_message(&format!("Deleted comment {id}"));
    }
    Ok(())
}
