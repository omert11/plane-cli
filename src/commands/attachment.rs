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
    /// Download the inline images embedded in a work item's description.
    ///
    /// Description images (`<image-component>`) are a different asset type than
    /// attachments and never show up in `attachment list`. This reads the issue's
    /// `description_html`, finds each embedded asset, and downloads it via the
    /// workspace asset endpoint.
    DownloadInline {
        /// Project UUID
        #[arg(long)]
        project: String,
        /// Work-item (issue) UUID
        #[arg(long)]
        issue: String,
        /// Directory to save the images into (created if missing; defaults to cwd)
        #[arg(long)]
        out_dir: Option<String>,
        /// Overwrite destination files if they already exist
        #[arg(long)]
        force: bool,
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
        AttachmentCmd::DownloadInline {
            project,
            issue,
            out_dir,
            force,
        } => download_inline(client, &project, &issue, out_dir, force, json).await,
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

/// Resolve a workspace asset's presigned download URL via the public asset
/// endpoint (`GET /workspaces/{slug}/assets/{id}/`). The endpoint returns
/// `{ asset_url, asset_name, asset_type, … }` where `asset_url` is a presigned
/// link; we hand that to [`download_bytes`](PlaneClient::download_bytes).
///
/// Some self-hosted instances ship an asset endpoint that 500s because the
/// bundled `S3Storage` does not accept the `is_server` kwarg the view passes
/// (a server-side version mismatch, not a CLI bug). When that happens the
/// caller can't recover, so we surface a pointed error instead of a bare 500.
async fn fetch_inline_asset_url(client: &PlaneClient, asset_id: &str) -> Result<(String, String)> {
    let path = client.ws_path(&format!("assets/{asset_id}"));
    let value = client.get::<()>(&path, None).await.map_err(|e| {
        if e.downcast_ref::<crate::client::PlaneApiError>()
            .is_some_and(|err| err.status.is_server_error())
        {
            anyhow!(
                "Asset endpoint returned a server error for {asset_id}. This Plane \
                 instance's workspace asset download is broken (the bundled S3Storage \
                 rejects the `is_server` argument the API passes — a server-side fix \
                 is required). Original error:\n{e}"
            )
        } else {
            e
        }
    })?;

    let url = value
        .get("asset_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Asset response for {asset_id} has no asset_url:\n{value}"))?;
    // The download name Plane records for this asset (falls back to the UUID).
    let name = value
        .get("asset_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(util::file_name_of)
        .unwrap_or_else(|| asset_id.to_string());
    Ok((client.absolute_url(url), name))
}

/// Download every inline `image-component` asset embedded in an issue's
/// description into `out_dir`. Files are named after the asset's stored name,
/// prefixed with the asset UUID to guarantee uniqueness (two pasted images can
/// share the name `image.png`).
async fn download_inline(
    client: &PlaneClient,
    project: &str,
    issue: &str,
    out_dir: Option<String>,
    force: bool,
    json: bool,
) -> Result<()> {
    // 1. Read the issue description and pull out the inline asset UUIDs.
    let issue_path = client.ws_path(&format!("projects/{project}/work-items/{issue}"));
    let current = client.get::<()>(&issue_path, None).await?;
    let html = current
        .get("description_html")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let asset_ids = util::extract_inline_asset_ids(html);

    if asset_ids.is_empty() {
        if json {
            return output::emit_value(&json!({ "saved": [], "count": 0 }));
        }
        output::print_message("No inline images found in this work item's description");
        return Ok(());
    }

    // 2. Ensure the destination directory exists.
    let dir = out_dir.unwrap_or_else(|| ".".to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("Failed to create output directory {dir:?}"))?;

    // 3. Resolve + download each asset. Prefix the UUID so same-named pastes
    //    don't collide on disk.
    let mut saved = Vec::new();
    for asset_id in &asset_ids {
        let (url, name) = fetch_inline_asset_url(client, asset_id)
            .await
            .with_context(|| format!("Failed to resolve inline asset {asset_id}"))?;
        let short = asset_id.split('-').next().unwrap_or(asset_id);
        let file_name = format!("{short}-{name}");
        let dest = std::path::Path::new(&dir).join(&file_name);

        if !force && tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            return Err(anyhow!(
                "{} already exists — pass --force to overwrite",
                dest.display()
            ));
        }

        let bytes = client.download_bytes(&url).await?;
        tokio::fs::write(&dest, &bytes)
            .await
            .with_context(|| format!("Failed to write {}", dest.display()))?;
        let dest_str = dest.to_string_lossy().to_string();
        if !json {
            output::print_message(&format!("Saved {dest_str} ({} bytes)", bytes.len()));
        }
        saved.push(json!({ "asset_id": asset_id, "saved": dest_str, "bytes": bytes.len() }));
    }

    if json {
        output::emit_value(&json!({ "saved": saved, "count": asset_ids.len() }))
    } else {
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
