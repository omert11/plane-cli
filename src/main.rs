use anyhow::Result;
use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;
mod output;
mod types;
mod util;

use commands::{
    comment, cycle, intake, issue, label, link, member, module, page, project, state, worklog,
};

#[derive(Parser)]
#[command(name = "plane-cli")]
#[command(version, about = "CLI for Plane project management", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Project operations
    Project {
        #[command(subcommand)]
        cmd: project::ProjectCmd,
    },
    /// Work item (issue) operations
    Issue {
        #[command(subcommand)]
        cmd: issue::IssueCmd,
    },
    /// State operations (per project)
    State {
        #[command(subcommand)]
        cmd: state::StateCmd,
    },
    /// Label operations (per project)
    Label {
        #[command(subcommand)]
        cmd: label::LabelCmd,
    },
    /// Work item comment operations
    Comment {
        #[command(subcommand)]
        cmd: comment::CommentCmd,
    },
    /// Cycle (sprint) operations
    Cycle {
        #[command(subcommand)]
        cmd: cycle::CycleCmd,
    },
    /// Module operations
    Module {
        #[command(subcommand)]
        cmd: module::ModuleCmd,
    },
    /// Intake (triage inbox) operations
    Intake {
        #[command(subcommand)]
        cmd: intake::IntakeCmd,
    },
    /// Page operations
    Page {
        #[command(subcommand)]
        cmd: page::PageCmd,
    },
    /// Work log operations
    Worklog {
        #[command(subcommand)]
        cmd: worklog::WorklogCmd,
    },
    /// Work item link / relation operations
    Link {
        #[command(subcommand)]
        cmd: link::LinkCmd,
    },
    /// Member operations (workspace / project) + current user (`member me`)
    Member {
        #[command(subcommand)]
        cmd: member::MemberCmd,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load()?;
    let client = client::PlaneClient::new(&cfg)?;

    match cli.command {
        Commands::Project { cmd } => project::run(cmd, &client, cli.json).await,
        Commands::Issue { cmd } => issue::run(cmd, &client, cli.json).await,
        Commands::State { cmd } => state::run(cmd, &client, cli.json).await,
        Commands::Label { cmd } => label::run(cmd, &client, cli.json).await,
        Commands::Comment { cmd } => comment::run(cmd, &client, cli.json).await,
        Commands::Cycle { cmd } => cycle::run(cmd, &client, cli.json).await,
        Commands::Module { cmd } => module::run(cmd, &client, cli.json).await,
        Commands::Intake { cmd } => intake::run(cmd, &client, cli.json).await,
        Commands::Page { cmd } => page::run(cmd, &client, cli.json).await,
        Commands::Worklog { cmd } => worklog::run(cmd, &client, cli.json).await,
        Commands::Link { cmd } => link::run(cmd, &client, cli.json).await,
        Commands::Member { cmd } => member::run(cmd, &client, cli.json).await,
    }
}
