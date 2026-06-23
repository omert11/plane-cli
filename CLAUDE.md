# plane-cli

Single-binary Rust CLI for [Plane](https://plane.so) project management. ~70 operations,
nested subcommand structure (`issue search`, `project create`, `cycle add-items`,
`member me`, …). Built in the same spirit as `zammad-cli`.

## Stack

- **Dil**: Rust 2021
- **Build**: `cargo` (rustc 1.80+)
- **Bağımlılıklar**:
  - `clap` 4.6 (derive) — argparse + nested subcommand
  - `reqwest` 0.12 (rustls + json) — HTTP client
  - `tokio` 1 (rt-multi-thread, macros) — async runtime
  - `serde` + `serde_json` — JSON
  - `comfy-table` 7.1 — tables
  - `colored` 2.1 — terminal colors
  - `anyhow` — error wrapping

## Dil

Türkçe iletişim, İngilizce kod yorumu + commit mesajı.

## Komutlar

```bash
cargo build                                # debug
cargo build --release                      # release (~3 MB binary)
cargo run -- project list                  # local çalıştır (env gerekli)
cargo clippy --all-targets -- -D warnings  # lint (temiz olmalı)
cargo fmt --all                            # format
cargo test                                 # test (util parse + iso vb.)
```

**Kurulum (ZORUNLU adım)**: Shell oturumları ve skill'ler PATH'teki
`~/.local/bin/plane-cli`'ı çağırır — repo'daki `target/release/plane-cli`'ı DEĞİL.
Kod değişip release derlendikten sonra kurulu binary mutlaka güncellenmeli:

```bash
cargo build --release
cp target/release/plane-cli ~/.local/bin/plane-cli   # ZORUNLU — atlanırsa yeni komutlar görünmez
which plane-cli                                       # doğrula
```

Binary kullanımı:

```bash
export PLANE_URL=https://support.diji.tech
export PLANE_API_KEY=plane_api_...          # Settings → API Tokens
export PLANE_WORKSPACE_SLUG=your-workspace

plane-cli project list
plane-cli issue list --project <pid>
plane-cli issue get-id DSTK-42
plane-cli --json member me | jq .email
```

## Proje Yapısı

```
src/
├── main.rs       clap parser + tokio runtime, 12 domain dispatch
├── config.rs     env var (PLANE_URL / PLANE_API_KEY / PLANE_WORKSPACE_SLUG) reader
├── client.rs     reqwest wrapper — X-Api-Key auth, /api/v1 base, ws_path() workspace
│                 scoping, mandatory trailing slash, unwrap_results(), error format
├── types.rs      serde structs (Project, WorkItem, State, Label, Comment, Cycle,
│                 Module, Page, Member, Me, WorkItemLink) — id is a UUID String
├── output.rs     render() dispatch + per-type table printers + role_name()
├── util.rs       truncate, split_csv, insert_opt_*, parse_work_item_ident (PROJ-123)
└── commands/     bir domain = bir modül; enum XCmd + pub async fn run(...)
    ├── project.rs  issue.rs  state.rs  label.rs  comment.rs
    ├── cycle.rs    module.rs  intake.rs  page.rs  worklog.rs  link.rs  member.rs
```

## Kod Konvansiyonları

- `cargo fmt --all` ile formatla; `cargo clippy --all-targets -- -D warnings` temiz olmalı
- `anyhow::Result` + `?` ile hata zinciri; `#[cfg(test)]` dışında `unwrap()`/`panic!` yok
- **Path kurma**: HER ZAMAN `client.ws_path("projects/...")` (workspace slug + leading slash).
  Tek istisna `member me` → `client.get::<()>("/users/me", None)`.
- **Trailing slash**: client otomatik ekler — path'lere elle EKLEME.
- **Liste yanıtları**: `serde_json::from_value(unwrap_results(value))` — paginated
  `{results: [...]}` zarfını açar; düz dizide no-op.
- **Body kurma**: `serde_json::Map` + `util::insert_opt_str/bool/csv_array` + `json!(...)`,
  sonra `client.post/patch(&path, Some(&Value::Object(body)))`.
- **Output**: `output::render(&items, json, |v| output::print_X_table(v))`; detay/raw için
  `output::emit_value(&value)`; aksiyon sonrası `output::print_message("...")`.
- Yeni domain eklemek = yeni `commands/<x>.rs` + `mod.rs` + `main.rs` dispatch satırı.

## API Notları

- **Base**: `{PLANE_URL}/api/v1` — `PLANE_URL`'e `/api/v1` EKLEME.
- **Auth**: `X-Api-Key: <token>` header (Bearer DEĞİL; SDK standardı).
- **Workspace-scoped**: tüm resource path'leri `workspaces/{slug}/...` ile başlar.
- **Trailing slash ZORUNLU** — yoksa bazı endpoint'ler 404/redirect verir.
- **work-items = issues**: REST'te kaynak adı `work-items` (eski `issues` değil).
  Alt-kaynak isimleri tutarsız: cycle/module/intake item path'leri
  `cycle-issues` / `module-issues` / `intake-issues` (work-items DEĞİL — SDK'dan doğrulandı).
- **By-identifier**: `workspaces/{slug}/work-items/{PROJ}-{SEQ}` (proje değil workspace altında).
- **Roller**: Admin=20, Member=15, Guest=5 (`output::role_name` çevirir).
- **Kaynak referansı**: resmi SDK (`makeplane/plane-python-sdk`) ve MCP server
  (`makeplane/plane-mcp-server`) endpoint/tool kontratının kaynağıdır.

## Release

Tag push → GitHub Actions multi-target build (Linux x86_64/aarch64, macOS
x86_64/aarch64, Windows x86_64) + GitHub Release.

```bash
git tag v0.1.0
git push origin v0.1.0
```
