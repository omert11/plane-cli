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
        /// Comment body in HTML format (optional when --image is given)
        #[arg(long)]
        comment_html: Option<String>,
        /// Attach an image file and embed it inline in the comment
        #[arg(long)]
        image: Option<String>,
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
            image,
        } => add(client, &project, &issue, comment_html, image, json).await,
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
    comment_html: Option<String>,
    image: Option<String>,
    json: bool,
) -> Result<()> {
    // Require at least one of --comment-html / --image. An explicitly-passed
    // empty body (`--comment-html ""` → Some("")) is still honoured for
    // backward compatibility; only the both-absent case is rejected.
    if comment_html.is_none() && image.is_none() {
        anyhow::bail!("Provide --comment-html and/or --image");
    }

    // Build the body: optional text plus, when --image is given, the uploaded
    // asset embedded as an inline image-component node.
    let mut html = comment_html.unwrap_or_default();
    if let Some(file) = image.as_deref() {
        let up = super::attachment::upload_file(client, project, issue, file).await?;
        html.push_str(&util::image_component_html(&up.asset_id));

        // The image is already attached to the issue; if the comment POST
        // fails, roll it back so we don't leave an orphan attachment behind.
        let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/comments"));
        let body = json!({ "comment_html": html });
        let value = match client.post(&path, Some(&body)).await {
            Ok(v) => v,
            Err(e) => {
                super::attachment::delete_asset(client, project, issue, &up.asset_id).await;
                return Err(e.context("Comment failed; rolled back the uploaded image"));
            }
        };
        return finish(value, html, json);
    }

    let path = client.ws_path(&format!("projects/{project}/work-items/{issue}/comments"));
    let body = json!({ "comment_html": html });
    let value = client.post(&path, Some(&body)).await?;
    finish(value, html, json)
}

fn finish(value: Value, html: String, json: bool) -> Result<()> {
    if json {
        output::emit_value(&value)
    } else {
        let item: Comment = serde_json::from_value(value).unwrap_or_else(|_| Comment {
            id: String::new(),
            comment_html: Some(html),
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
