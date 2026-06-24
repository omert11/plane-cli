use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Method, Response, StatusCode};
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

const UA: &str = concat!("plane-cli/", env!("CARGO_PKG_VERSION"));

/// A typed error carrying the HTTP status, so callers can branch on the actual
/// status code (e.g. fall back on a real 404) instead of string-matching the
/// formatted message — the message embeds the raw server body and is not stable.
#[derive(Debug)]
pub struct PlaneApiError {
    pub status: StatusCode,
    pub message: String,
}

impl std::fmt::Display for PlaneApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PlaneApiError {}

impl PlaneApiError {
    pub fn is_not_found(&self) -> bool {
        self.status == StatusCode::NOT_FOUND
    }
}

/// True when an `anyhow::Error` is a Plane API 404. Callers use this to decide
/// whether to retry against an alternate endpoint path.
pub fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<PlaneApiError>()
        .is_some_and(|e| e.is_not_found())
}

/// HTTP client for the Plane REST API (`/api/v1`).
///
/// Auth is the `X-Api-Key` header. All resource paths are workspace-scoped, so
/// helpers ([`ws_path`](PlaneClient::ws_path)) prepend `workspaces/{slug}/`.
/// Plane requires a **trailing slash** on every endpoint (the official SDK
/// appends one); [`request`](PlaneClient::request) enforces it.
pub struct PlaneClient {
    http: Client,
    base_url: String,
    workspace: String,
}

impl PlaneClient {
    pub fn new(cfg: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Api-Key",
            HeaderValue::from_str(&cfg.api_key).context("Invalid API key characters")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));
        let http = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            http,
            // {base}/api/v1 — paths are joined with a leading slash.
            base_url: format!("{}/api/v1", cfg.url.trim_end_matches('/')),
            workspace: cfg.workspace.clone(),
        })
    }

    /// The configured workspace slug. Part of the public client surface;
    /// modules go through [`ws_path`](Self::ws_path) so this is rarely called directly.
    #[allow(dead_code)]
    pub fn workspace(&self) -> &str {
        &self.workspace
    }

    /// Build a workspace-scoped path: `workspaces/{slug}/{suffix}`.
    /// `suffix` should NOT start with a slash.
    pub fn ws_path(&self, suffix: &str) -> String {
        format!("/workspaces/{}/{}", self.workspace, suffix)
    }

    /// Resolve a possibly-relative asset URL against the instance origin.
    /// Plane sometimes returns an absolute URL and sometimes a relative path/key
    /// (e.g. `/plane-uploads/...`); absolute inputs are returned unchanged.
    pub fn absolute_url(&self, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }
        // base_url is `{origin}/api/v1`; strip the api suffix to get the origin.
        let origin = self
            .base_url
            .strip_suffix("/api/v1")
            .unwrap_or(&self.base_url);
        format!(
            "{}/{}",
            origin.trim_end_matches('/'),
            url.trim_start_matches('/')
        )
    }

    pub async fn request<Q: Serialize + ?Sized, B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: Option<&Q>,
        body: Option<&B>,
    ) -> Result<Value> {
        // Plane mandates a trailing slash on every endpoint.
        let path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{path}/")
        };
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.request(method, &url);
        if let Some(q) = query {
            req = req.query(q);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.context("Plane request failed")?;
        handle_response(resp).await
    }

    pub async fn get<Q: Serialize + ?Sized>(&self, path: &str, query: Option<&Q>) -> Result<Value> {
        self.request::<Q, ()>(Method::GET, path, query, None).await
    }

    pub async fn post<B: Serialize + ?Sized>(&self, path: &str, body: Option<&B>) -> Result<Value> {
        self.request::<(), B>(Method::POST, path, None, body).await
    }

    pub async fn patch<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<Value> {
        self.request::<(), B>(Method::PATCH, path, None, body).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        self.request::<(), ()>(Method::DELETE, path, None, None)
            .await
    }

    /// Upload raw bytes to an object-storage (MinIO / S3) presigned POST.
    ///
    /// Plane does not receive the file itself: an attachment request returns a
    /// presigned `url` plus signed `fields`, and the file is POSTed straight to
    /// storage as `multipart/form-data`. The order matters — every signed field
    /// goes first and `file` **last** (S3 policies are order-sensitive). This
    /// call must NOT carry the Plane `X-Api-Key` / JSON `Content-Type` headers,
    /// so it uses a bare request (no `default_headers` auth is sent because the
    /// target host differs and storage rejects unexpected signed headers).
    pub async fn upload_to_storage(
        &self,
        url: &str,
        fields: &serde_json::Map<String, Value>,
        file_name: &str,
        mime: &str,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let mut form = reqwest::multipart::Form::new();
        // Signed fields first, in the order the server provided them.
        for (k, v) in fields {
            let val = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            form = form.text(k.clone(), val);
        }
        // `file` part last; name + content-type echo what we declared on presign.
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str(mime)
            .context("Invalid MIME type for upload")?;
        form = form.part("file", part);

        // Fresh client: storage rejects the JSON Content-Type / X-Api-Key that
        // our default headers carry; only the UA is worth keeping for CF.
        let resp = reqwest::Client::new()
            .post(url)
            .header(USER_AGENT, UA)
            .multipart(form)
            .send()
            .await
            .context("Storage upload request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Storage upload failed ({status}){}",
                if body.is_empty() {
                    String::new()
                } else {
                    format!(": {body}")
                }
            ));
        }
        Ok(())
    }

    /// Fetch raw bytes from an absolute URL (e.g. a presigned download URL).
    /// Carries the UA but no Plane auth headers — presigned URLs are
    /// self-authenticating.
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = reqwest::Client::new()
            .get(url)
            .header(USER_AGENT, UA)
            .send()
            .await
            .context("Download request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Download failed ({status}){}",
                if body.is_empty() {
                    String::new()
                } else {
                    format!(": {body}")
                }
            ));
        }
        Ok(resp
            .bytes()
            .await
            .context("Failed to read download body")?
            .to_vec())
    }
}

/// Plane list endpoints return a paginated envelope
/// (`{ results, total_count, next_page_number, ... }`). Detail/create endpoints
/// return the bare object. This unwraps `results` when present, else returns the
/// value as-is — so callers can always `serde_json::from_value::<Vec<T>>` a list
/// response and `<T>` a single response.
pub fn unwrap_results(value: Value) -> Value {
    match value {
        Value::Object(ref map) if map.contains_key("results") => {
            map.get("results").cloned().unwrap_or(Value::Null)
        }
        other => other,
    }
}

async fn handle_response(resp: Response) -> Result<Value> {
    let status = resp.status();
    if status.is_success() {
        if status == StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let text = resp.text().await.context("Failed to read response body")?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        return serde_json::from_str(&text).context("Failed to parse response JSON");
    }

    let body = resp.text().await.unwrap_or_default();
    let parsed = serde_json::from_str::<Value>(&body).ok();
    let msg = parsed
        .as_ref()
        .and_then(|v| {
            v.get("error")
                .or_else(|| v.get("detail"))
                .or_else(|| v.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| body.clone());

    let prefix = match status {
        StatusCode::NOT_FOUND => "Not found (404) — check workspace slug / id / path",
        StatusCode::FORBIDDEN => "Permission denied (403)",
        StatusCode::BAD_REQUEST => "Bad request (400)",
        StatusCode::UNAUTHORIZED => "Unauthorized (401) — check PLANE_API_KEY",
        StatusCode::UNPROCESSABLE_ENTITY => "Unprocessable (422)",
        _ => "Plane API error",
    };
    let message = if !body.is_empty() && body.trim() != msg.trim() {
        format!("{prefix}: {msg}\n--- raw: {body}")
    } else {
        format!("{prefix}: {msg}")
    };
    Err(anyhow!(PlaneApiError { status, message }))
}
