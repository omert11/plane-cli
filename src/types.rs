use serde::{Deserialize, Serialize};

// All models use `#[serde(default)]` on optional fields so partial API
// responses (Plane varies fields by `expand`/`fields` params) deserialize
// cleanly. `id` is a UUID string in Plane (not an integer).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub identifier: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub network: Option<i64>,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub sequence_id: Option<i64>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub target_date: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub project: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub sequence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub comment_html: Option<String>,
    #[serde(default)]
    pub comment_stripped: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cycle {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub target_date: Option<String>,
    #[serde(default)]
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub member_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub role: Option<i64>,
}

/// A URL link attached to a work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemLink {
    pub id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Me {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
}
