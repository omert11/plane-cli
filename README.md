# plane-cli

Single-binary Rust CLI for [Plane](https://plane.so) project management — manage
projects, work items (issues), cycles, modules, states, labels, comments and more
from the terminal. Works great with AI agents and shell pipelines.

Mirrors the surface of Plane's official [REST API](https://developers.plane.so) and
[MCP server](https://github.com/makeplane/plane-mcp-server), in the same spirit as
[`zammad-cli`](https://github.com/omert11/zammad-cli).

## Features

- **~70 operations** as nested subcommands (`issue search`, `project create`,
  `cycle add-items`, `state list`, `member me`, …)
- **Pretty colored tables** by default, `--json` for piping into `jq`
- **Human identifiers** — `issue get-id PROJ-123` resolves a work item by its
  project identifier + sequence
- **Workspace-scoped** — one workspace per invocation via `PLANE_WORKSPACE_SLUG`
- **Single static binary** (~3 MB, no runtime)

## Install

### Prebuilt binaries (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/omert11/plane-cli/releases/latest):

| Platform | Archive |
|----------|---------|
| Linux x86_64 | `plane-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `plane-cli-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 (Intel) | `plane-cli-x86_64-apple-darwin.tar.gz` |
| macOS aarch64 (Apple Silicon) | `plane-cli-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `plane-cli-x86_64-pc-windows-msvc.zip` |

### From source

```bash
cargo build --release
cp target/release/plane-cli ~/.local/bin/plane-cli
```

## Configuration

Three environment variables are required:

```bash
export PLANE_URL=https://support.diji.tech        # base URL (no /api/v1)
export PLANE_API_KEY=plane_api_...              # Settings → API Tokens
export PLANE_WORKSPACE_SLUG=your-workspace      # workspace slug from the URL
```

- **API key:** in Plane, go to **Settings → API Tokens**, create a token, copy it.
- **Workspace slug:** the path segment after the host in the web UI
  (`https://support.diji.tech/<slug>/projects`).
- Auth uses the `X-Api-Key` header (Plane REST API standard).

## Usage

```bash
# Projects
plane-cli project list
plane-cli project get <project-uuid>
plane-cli project create "Mobile App" MP --description "iOS + Android"

# Work items (issues)
plane-cli issue list --project <project-uuid>
plane-cli issue get-id DSTK-42
plane-cli issue create --project <pid> "Fix login bug" --priority high --labels bug,urgent
plane-cli issue search "payment timeout" --json | jq '.[].name'
plane-cli issue assignee --project <pid> <issue-uuid> --add <user-uuid>

# Cycles & modules
plane-cli cycle list --project <pid>
plane-cli cycle add-items --project <pid> <cycle-uuid> <issue-uuid-1>,<issue-uuid-2>
plane-cli module list --project <pid>

# Metadata
plane-cli state list --project <pid>
plane-cli label create --project <pid> "blocked" --color "#ff0000"
plane-cli comment add --project <pid> --issue <wid> "Looks good to me"

# Who am I
plane-cli member me
plane-cli member list                      # workspace members
plane-cli member list --project <pid>      # project members
```

`--json` is a global flag and works on every command.

## Command reference

| Domain | Subcommands |
|--------|-------------|
| `project` | list · get · create · update · delete · members · features · archive · unarchive |
| `issue` | list · get · get-id · create · update · delete · search · count · assignee · label · archive · unarchive · list-archived |
| `state` | list · get · create · update · delete |
| `label` | list · get · create · update · delete |
| `comment` | list · get · add · update · delete |
| `cycle` | list · get · create · update · delete · list-items · add-items · archive · unarchive |
| `module` | list · get · create · update · delete · list-items · add-items · archive · unarchive |
| `intake` | list · get · create · update · delete |
| `page` | list · get · create · update · delete |
| `worklog` | list · create · update · delete |
| `link` | list · create · remove |
| `member` | list · me |

Run `plane-cli <domain> --help` for arguments of each subcommand.

### Roadmap

Not yet wrapped (available via the REST API / MCP server): attachments,
custom work-item properties, estimates, initiatives, milestones, relations,
work-item types, advanced PQL search. Each maps cleanly onto a new
`src/commands/*.rs` module — contributions welcome.

## License

MIT. plane-cli is a REST API client and contains no Plane source code.
