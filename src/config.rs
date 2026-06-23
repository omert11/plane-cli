use anyhow::{bail, Result};

/// Runtime configuration loaded from environment variables.
///
/// Plane's REST API is workspace-scoped: every path begins with the workspace
/// slug, so `workspace` is required alongside the base URL and API key.
pub struct Config {
    pub url: String,
    pub api_key: String,
    pub workspace: String,
}

pub fn load() -> Result<Config> {
    let url = std::env::var("PLANE_URL").ok().filter(|s| !s.is_empty());
    let api_key = std::env::var("PLANE_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let workspace = std::env::var("PLANE_WORKSPACE_SLUG")
        .ok()
        .filter(|s| !s.is_empty());
    match (url, api_key, workspace) {
        (Some(url), Some(api_key), Some(workspace)) => Ok(Config {
            url,
            api_key,
            workspace,
        }),
        _ => bail!(
            "PLANE_URL, PLANE_API_KEY and PLANE_WORKSPACE_SLUG must be set.\n\
             Example: export PLANE_URL=https://support.diji.tech\n\
                      export PLANE_API_KEY=plane_api_...   # Settings -> API Tokens\n\
                      export PLANE_WORKSPACE_SLUG=your-workspace"
        ),
    }
}
