use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

use crate::client::{is_not_found, unwrap_results, PlaneClient};
use crate::output;
use crate::types::Attachment;
use crate::util;

#[derive(Subcommand)]
pub enum AttachmentCmd {
    /// Upload a file to a work item (presign → upload → confirm in one step)
    Add {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
        /// Path to the file to upload
        #[arg(long)]
        file: String,
        /// Also embed the uploaded image inline into the issue description
        #[arg(long)]
        inline: bool,
    },
    /// List attachments on a work item
    List {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
    },
    /// Download an attachment to a local path
    Download {
        /// Attachment UUID
        id: String,
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
        /// Destination path (defaults to the attachment's original name in cwd)
        #[arg(long)]
        out: Option<String>,
        /// Overwrite the destination file if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Delete an attachment from a work item (DESTRUCTIVE)
    Delete {
        /// Attachment UUID
        id: String,
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
    },
}

pub async fn run(cmd: AttachmentCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        AttachmentCmd::Add {
            project,
            issue,
            file,
            inline,
        } => add(client, &project, &issue, &file, inline, json).await,
        AttachmentCmd::List { project, issue } => list(client, &project, &issue, json).await,
        AttachmentCmd::Download {
            id,
            project,
            issue,
            out,
            force,
        } => download(client, &project, &issue, &id, out, force, json).await,
        AttachmentCmd::Delete { id, project, issue } => {
            delete(client, &project, &issue, &id, json).await
        }
    }
}

// ---- Endpoint paths (Plane versions disagree on the resource name) ----
//
// Newer instances expose `…/issues/{id}/issue-attachments/`, others
// `…/work-items/{id}/attachments/`. Every collection call tries the first and
// falls back to the second on a real 404 (status-based, see [`with_fallback`]).

fn primary_collection(project: &str, issue: &str) -> String {
    format!("projects/{project}/issues/{issue}/issue-attachments")
}

fn fallback_collection(project: &str, issue: &str) -> String {
    format!("projects/{project}/work-items/{issue}/attachments")
}

/// Run `op` against the primary collection suffix; on a 404, retry against the
/// fallback suffix. Returns the operation result plus the suffix that worked
/// (callers need it to build the per-item confirm/delete path). Collapses the
/// previously-triplicated primary→fallback→context scaffolding into one place.
async fn with_fallback<'a, F, Fut, T>(
    project: &'a str,
    issue: &'a str,
    op: F,
) -> Result<(T, String)>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let primary = primary_collection(project, issue);
    match op(primary.clone()).await {
        Ok(v) => Ok((v, primary)),
        Err(e) if is_not_found(&e) => {
            let fallback = fallback_collection(project, issue);
            let v = op(fallback.clone())
                .await
                .context("Both issue-attachments and work-items/attachments paths failed")?;
            Ok((v, fallback))
        }
        Err(e) => Err(e),
    }
}

async fn post_collection(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    body: &Value,
) -> Result<(Value, String)> {
    with_fallback(project, issue, |suffix| async move {
        client.post(&client.ws_path(&suffix), Some(body)).await
    })
    .await
}

async fn get_collection(client: &PlaneClient, project: &str, issue: &str) -> Result<Value> {
    // Request a large page so a single call covers virtually all work items.
    // If the response still reports a further page, warn rather than silently
    // truncate (a missing attachment on a later page would otherwise read as
    // "not found"). Following the full cursor is left out until verified against
    // a real instance; `per_page=100` is the pragmatic guard.
    let query = [("per_page", "100")];
    let (value, _) = with_fallback(project, issue, |suffix| async move {
        client.get(&client.ws_path(&suffix), Some(&query)).await
    })
    .await?;
    if let Some(next) = value.get("next_page_number").and_then(|v| v.as_i64()) {
        if next > 0 {
            eprintln!(
                "warning: this work item has more attachments than one page; \
                 some may be missing from the result"
            );
        }
    }
    Ok(value)
}

/// Best-effort deletion of a stored asset, used to roll back an orphaned upload
/// when a later step (e.g. the comment POST) fails. Errors are swallowed — the
/// caller is already in an error path and rollback is a courtesy, not a guarantee.
pub async fn delete_asset(client: &PlaneClient, project: &str, issue: &str, asset_id: &str) {
    let _ = with_fallback(project, issue, |suffix| async move {
        client
            .delete(&client.ws_path(&format!("{suffix}/{asset_id}")))
            .await
    })
    .await;
}

/// Result of a successful upload — the stored asset UUID and the confirm
/// response. Reused by `comment --image` to embed the same asset.
pub struct Uploaded {
    pub asset_id: String,
    pub name: String,
    pub size: usize,
    pub confirm: Value,
}

/// Run the full presign → storage upload → confirm flow for one file against a
/// work item. Public so other commands (e.g. `comment --image`) can attach the
/// same way and reuse the returned `asset_id` for inline embedding.
pub async fn upload_file(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    file: &str,
) -> Result<Uploaded> {
    // 1. Read file + derive metadata.
    let bytes = tokio::fs::read(file)
        .await
        .with_context(|| format!("Failed to read file {file:?}"))?;
    let size = bytes.len();
    let name = util::file_name_of(file);
    let mime = util::mime_from_path(file);

    // 2. Request a presigned upload URL.
    let presign_body = json!({ "name": name, "type": &mime, "size": size });
    let (presign, collection) = post_collection(client, project, issue, &presign_body).await?;

    let upload = presign
        .get("upload_data")
        .ok_or_else(|| anyhow!("Presign response missing upload_data:\n{presign}"))?;
    let upload_url = upload
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Presign response missing upload_data.url"))?;
    let fields = upload
        .get("fields")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("Presign response missing upload_data.fields"))?;
    let asset_id = presign
        .get("asset_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Presign response missing asset_id"))?
        .to_string();

    // 3. Upload the bytes straight to object storage (MinIO / S3).
    client
        .upload_to_storage(upload_url, fields, &name, &mime, bytes)
        .await?;

    // 4. Confirm the upload so Plane marks it complete (otherwise it stays hidden).
    let confirm_path = client.ws_path(&format!("{collection}/{asset_id}"));
    let confirm = client
        .patch(&confirm_path, Some(&json!({ "is_uploaded": true })))
        .await?;

    Ok(Uploaded {
        asset_id,
        name,
        size,
        confirm,
    })
}

async fn add(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    file: &str,
    inline: bool,
    json: bool,
) -> Result<()> {
    let up = upload_file(client, project, issue, file).await?;

    // Optionally embed the asset inline in the issue description.
    if inline {
        embed_inline(client, project, issue, &up.asset_id).await?;
    }

    if json {
        output::emit_value(&up.confirm)
    } else {
        output::print_message(&format!(
            "Uploaded {} ({}) → asset {}{}",
            up.name,
            output::human_size(up.size as i64),
            up.asset_id,
            if inline { " (embedded inline)" } else { "" }
        ));
        Ok(())
    }
}

/// GET the issue, append an `image-component` node to its `description_html`,
/// then PATCH it back — preserving the existing editor markup.
///
/// Note: this is an unguarded read-modify-write. Plane's REST API exposes no
/// optimistic-concurrency token for the issue description, so a concurrent edit
/// landing between the GET and the PATCH would be overwritten. The window is
/// small and the alternative (not embedding) is worse for the single-user CLI
/// case this targets; documented here so the limitation is explicit.
async fn embed_inline(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    asset_id: &str,
) -> Result<()> {
    let issue_path = client.ws_path(&format!("projects/{project}/work-items/{issue}"));
    let current = client.get::<()>(&issue_path, None).await?;
    let existing = current
        .get("description_html")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let updated = format!("{existing}{}", util::image_component_html(asset_id));
    client
        .patch(&issue_path, Some(&json!({ "description_html": updated })))
        .await
        .context("Failed to embed image inline")?;
    Ok(())
}

async fn list(client: &PlaneClient, project: &str, issue: &str, json: bool) -> Result<()> {
    let value = get_collection(client, project, issue).await?;
    let items: Vec<Attachment> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |a| output::print_attachment_table(a))
}

async fn download(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    id: &str,
    out: Option<String>,
    force: bool,
    json: bool,
) -> Result<()> {
    // Resolve the attachment record to learn its presigned download URL + name.
    let value = get_collection(client, project, issue).await?;
    let items: Vec<Value> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    let record = items
        .into_iter()
        .find(|v| v.get("id").and_then(|x| x.as_str()) == Some(id))
        .ok_or_else(|| anyhow!("Attachment {id} not found on this work item"))?;

    // Plane exposes the link under one of these keys depending on version. It
    // may be absolute or a relative path/key — resolve relative ones against the
    // instance base so reqwest doesn't choke on a base-less relative URL.
    let raw_url = record
        .get("asset_url")
        .or_else(|| record.get("asset"))
        .or_else(|| record.get("download_url"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Attachment record has no downloadable URL:\n{record}"))?;
    let url = client.absolute_url(raw_url);

    // Destination: explicit --out, else the server name's BASENAME only (never
    // honour an embedded path like `../foo` — strip to a bare file name).
    let default_name = record
        .get("attributes")
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str())
        .map(util::file_name_of)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "attachment".to_string());
    let dest = out.unwrap_or(default_name);

    // Refuse to clobber an existing file unless --force.
    if !force && tokio::fs::try_exists(&dest).await.unwrap_or(false) {
        return Err(anyhow!(
            "{dest} already exists — pass --force to overwrite or --out to choose another path"
        ));
    }

    let bytes = client.download_bytes(&url).await?;
    tokio::fs::write(&dest, &bytes)
        .await
        .with_context(|| format!("Failed to write {dest:?}"))?;

    if json {
        output::emit_value(&json!({ "saved": dest, "bytes": bytes.len() }))
    } else {
        output::print_message(&format!("Saved {dest} ({} bytes)", bytes.len()));
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
    with_fallback(project, issue, |suffix| async move {
        client
            .delete(&client.ws_path(&format!("{suffix}/{id}")))
            .await
    })
    .await?;
    if !json {
        output::print_message(&format!("Deleted attachment {id}"));
    }
    Ok(())
}
