use anyhow::Result;
use clap::Subcommand;

use crate::client::{unwrap_results, PlaneClient};
use crate::output;
use crate::types::{Me, Member};

#[derive(Subcommand)]
pub enum MemberCmd {
    /// List workspace members, or project members when --project is given
    List {
        /// Project UUID — omit for workspace-level members
        #[arg(long)]
        project: Option<String>,
    },
    /// Get the currently authenticated user
    Me,
}

pub async fn run(cmd: MemberCmd, client: &PlaneClient, json: bool) -> Result<()> {
    match cmd {
        MemberCmd::List { project } => list(client, project.as_deref(), json).await,
        MemberCmd::Me => me(client, json).await,
    }
}

/// List workspace members or, when `project_id` is supplied, project members.
async fn list(client: &PlaneClient, project_id: Option<&str>, json: bool) -> Result<()> {
    let path = match project_id {
        Some(pid) => client.ws_path(&format!("projects/{pid}/members")),
        None => client.ws_path("members"),
    };
    let value = client.get::<()>(&path, None).await?;
    let items: Vec<Member> = serde_json::from_value(unwrap_results(value)).unwrap_or_default();
    output::render(&items, json, |m| output::print_member_table(m))
}

/// Print the currently authenticated user's profile.
async fn me(client: &PlaneClient, json: bool) -> Result<()> {
    let value = client.get::<()>("/users/me", None).await?;
    if json {
        return output::emit_value(&value);
    }
    // Human-readable: deserialise into Me and print key fields.
    let me: Me = serde_json::from_value(value).unwrap_or(Me {
        id: None,
        email: None,
        display_name: None,
        first_name: None,
        last_name: None,
    });
    let name = me.display_name.clone().unwrap_or_else(|| {
        format!(
            "{} {}",
            me.first_name.as_deref().unwrap_or(""),
            me.last_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string()
    });
    println!("id:    {}", me.id.as_deref().unwrap_or("-"));
    println!("email: {}", me.email.as_deref().unwrap_or("-"));
    println!("name:  {}", if name.is_empty() { "-" } else { &name });
    Ok(())
}
