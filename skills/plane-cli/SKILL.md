---
name: plane-cli
description: Plane project management için CLI wrapper skill. plane-cli Rust binary'sini doğru komutlarla çağırır - project/issue/state/label/comment/cycle/module/intake/page/worklog/link/member/attachment. Her zaman --json kullanıp parse eder.
when_to_use: Plane iş takibi - issue/proje yönetimi, work item oluşturma/güncelleme, cycle/module yönetimi, üye arama. Tetikleme - "issue oluştur", "work item listele", "PROJ-123 getir", "issue ara", "cycle'a ekle", "yorum ekle", "plane projeleri", "/plane-cli".
allowed-tools: Bash(plane-cli *) Read
---

# plane-cli Workflow Skill

[Plane](https://plane.so) iş takibini terminal'den yönet. `plane-cli` binary'sini wrap eder,
`--json` çıktısını parse edip kullanıcıya özetler. ~70 operasyon, nested subcommand yapısı.

## Önkoşul: Binary + Env

```bash
plane-cli --version || (echo "plane-cli not installed" && exit 1)
test -n "$PLANE_URL" && test -n "$PLANE_API_KEY" && test -n "$PLANE_WORKSPACE_SLUG" || echo "env missing"
```

Üç env zorunlu:

| Env | Açıklama |
|-----|----------|
| `PLANE_URL` | Plane instance URL'i (örn. `https://support.diji.tech`) — `/api/v1` EKLEME |
| `PLANE_API_KEY` | Settings → API Tokens'tan alınan `plane_api_...` token |
| `PLANE_WORKSPACE_SLUG` | Workspace slug'ı |

Eksikse repo README'deki kurulum + env satırlarını kullanıcıya göster.

## ID Semantiği — KRİTİK

Plane API **UUID tabanlı**. Çoğu komut `--project <UUID>` ister; issue/cycle/module
işlemleri de UUID alır. İnsan-okunur kısayollar:

- **`PROJ-123`** (human identifier) → sadece `issue get-id PROJ-123` kabul eder, UUID'ye çevirir
- **`--project <UUID>`** → her domain'de zorunlu (member dışında). Önce `project list` ile bul
- Diğer her şey (state, label, assignee, cycle id, issue id) **UUID** ister

**Tipik akış**: Kullanıcı `PROJ-123` veya proje adıyla konuşur → önce `project list` /
`issue get-id` ile UUID çöz → sonra hedef komutu UUID'lerle çağır.

## Komut Şablonu

**Her zaman `--json` ile çağır**, çıktıyı parse et, kullanıcıya tablo gibi özetle.
`--json` global flag — komuttan ÖNCE de sonra da gelebilir, tutarlılık için sonra koy.

### Project

```bash
plane-cli project list                                  # tüm projeler (UUID + identifier)
plane-cli project get <UUID>
plane-cli project members <UUID>
plane-cli project features <UUID>
plane-cli project create "Marketing" "MKT" --description "..." --network 2   # 0=secret, 2=public
plane-cli project update <UUID> ...
plane-cli project archive <UUID>      # DESTRUCTIVE — onay al
plane-cli project unarchive <UUID>
plane-cli project delete <UUID>       # DESTRUCTIVE — onay al
```

### Issue (work item)

```bash
plane-cli issue list --project <UUID>
plane-cli issue get <issue-UUID> --project <UUID>
plane-cli issue get-id PROJ-123                         # human ident → UUID çözer
plane-cli issue search "login bug" --limit 50          # workspace genelinde free-text
plane-cli issue count --project <UUID>

# Create (DESTRUCTIVE — onay al)
plane-cli issue create "Fix login redirect" --project <UUID> \
  --description "düz metin — escape edilip <p> içine sarılır" \   # düz metin
  --priority high \                                     # urgent|high|medium|low|none
  --state <state-UUID> \
  --assignees <user-UUID>,<user-UUID> \
  --labels <label-UUID>,<label-UUID> \
  --start-date 2026-06-23 --target-date 2026-06-30

# Zengin biçimli açıklama (başlık/kod bloğu/kalın) → --description-html (HTML escape ETMEZ)
plane-cli issue create "GSA komisyon hatası" --project <UUID> \
  --description-html "<h2>Sorun</h2><p>GSA <b>%0 komisyon</b> tanımlayamıyor.</p><pre><code>parseFloat(x) === 0</code></pre>"

# Update (DESTRUCTIVE — onay al; assignees/labels LİSTEYİ DEĞİŞTİRİR, eklemez)
plane-cli issue update <issue-UUID> --project <UUID> --priority urgent --state <state-UUID>
plane-cli issue update <issue-UUID> --project <UUID> --description-html "<h2>Güncelleme</h2><p>...</p>"

# Assignee / Label — incremental ekle/çıkar (update'in aksine listeyi korur)
plane-cli issue assignee <issue-UUID> --project <UUID> --add <user-UUID> --remove <user-UUID>
plane-cli issue label    <issue-UUID> --project <UUID> --add <label-UUID>

# Archive (yalnızca completed/cancelled state'te) / Delete (DESTRUCTIVE)
plane-cli issue archive   <issue-UUID> --project <UUID>
plane-cli issue unarchive <issue-UUID> --project <UUID>
plane-cli issue list-archived --project <UUID>
plane-cli issue delete <issue-UUID> --project <UUID>   # DESTRUCTIVE — onay al
```

### State / Label (per project)

```bash
plane-cli state list --project <UUID>
plane-cli state create "In Review" "#ffaa00" --project <UUID> \
  --group started                                       # backlog|unstarted|started|completed|cancelled
plane-cli state update <UUID> --project <UUID> ...
plane-cli state delete <UUID> --project <UUID>          # DESTRUCTIVE

plane-cli label list --project <UUID>
plane-cli label create "Bug" --project <UUID> --color "#ff0000"
plane-cli label update <UUID> --project <UUID> ...
plane-cli label delete <UUID> --project <UUID>          # DESTRUCTIVE
```

### Comment (work item)

```bash
plane-cli comment list --project <UUID> --issue <issue-UUID>
plane-cli comment add  --project <UUID> --issue <issue-UUID> --comment-html "<p>Merhaba</p>"   # HTML!
plane-cli comment add  --project <UUID> --issue <issue-UUID> --image /path/shot.png            # görseli yorum içine göm
plane-cli comment update <comment-UUID> ...
plane-cli comment delete <comment-UUID> ...             # DESTRUCTIVE
```

> `--comment-html` body **HTML** ister — düz metni `<p>...</p>` ile sar.
> `--image` görseli önce issue'ya attachment olarak yükler, sonra yorum gövdesine
> inline gömer. `--comment-html` ve `--image` birlikte ya da tek tek verilebilir
> (ikisi de yoksa hata). Yorum POST başarısız olursa yüklenen görsel geri alınır.

### Cycle (sprint) / Module

```bash
plane-cli cycle list --project <UUID>
plane-cli cycle create "Sprint 12" --project <UUID> --start-date 2026-06-23 --end-date 2026-07-07
plane-cli cycle list-items --project <UUID> <cycle-UUID>
plane-cli cycle add-items <cycle-UUID> "<issue-UUID>,<issue-UUID>" --project <UUID>
plane-cli cycle archive <cycle-UUID> --project <UUID>

plane-cli module list --project <UUID>
plane-cli module create "Auth" --project <UUID> --lead <user-UUID> --target-date 2026-07-01
plane-cli module list-items --project <UUID> <module-UUID>
plane-cli module add-items <module-UUID> "<issue-UUID>,<issue-UUID>" --project <UUID>
```

> `add-items` issue UUID'leri **virgülle ayrılmış tek string** (positional arg) olarak ister.

### Intake (triage inbox)

```bash
plane-cli intake list --project <UUID>
plane-cli intake get <UUID> --project <UUID>
plane-cli intake create "Yeni talep" --project <UUID> --description "..."
plane-cli intake update <UUID> --project <UUID> ...
plane-cli intake delete <UUID> --project <UUID>         # DESTRUCTIVE
```

### Page / Worklog / Link

```bash
plane-cli page list --project <UUID>
plane-cli page create "Notlar" --project <UUID> --description-html "<p>...</p>"
plane-cli page update <UUID> --project <UUID> ...

plane-cli worklog list --project <UUID> --issue <issue-UUID>
plane-cli worklog create 90 --project <UUID> --issue <issue-UUID> --description "..."   # 90 = dakika
plane-cli worklog update <UUID> ...

plane-cli link list   --project <UUID> --issue <issue-UUID>
plane-cli link create "https://..." --project <UUID> --issue <issue-UUID> --title "Tasarım"
plane-cli link remove <link-UUID> --project <UUID> --issue <issue-UUID>
```

### Attachment (dosya eki / inline görsel)

```bash
plane-cli attachment add  --project <UUID> --issue <issue-UUID> --file /path/shot.png            # yükle (presign→upload→confirm)
plane-cli attachment add  --project <UUID> --issue <issue-UUID> --file /path/shot.png --inline    # + issue açıklamasına göm
plane-cli attachment list --project <UUID> --issue <issue-UUID>
plane-cli attachment download <attachment-UUID> --project <UUID> --issue <issue-UUID> [--out path] [--force]
plane-cli attachment download-inline --project <UUID> --issue <issue-UUID> [--out-dir dir] [--force]   # açıklamadaki gömülü görseller
plane-cli attachment delete   <attachment-UUID> --project <UUID> --issue <issue-UUID>             # DESTRUCTIVE
```

> `add` 3 adımı (presigned URL → object storage upload → confirm) tek komutta yapar;
> MIME uzantıdan tespit edilir. `--inline` asset'i `<image-component>` ile issue
> `description_html`'ine ekler (mevcut gövde korunur). `download` varsayılan olarak
> mevcut dosyanın üzerine yazmaz — `--force` veya `--out` ile yön ver.
>
> **`download-inline`** issue açıklamasına gömülü görselleri indirir. Bunlar
> attachment DEĞİL ayrı bir asset tipi (`<image-component>` node'ları,
> entity_type `ISSUE_DESCRIPTION`) — `attachment list`'te GÖRÜNMEZ. Komut
> `description_html`'i okur, her gömülü asset'i `GET /workspaces/{slug}/assets/{id}/`
> ile çözer (presigned URL → indir) ve `<out-dir>/<uuid-kısa>-<isim>` olarak kaydeder.
> NOT: Bazı self-hosted instance'larda asset download endpoint'i 500 verir
> (bundled `S3Storage` API'nin geçtiği `is_server` argümanını reddediyor — sunucu
> tarafı düzeltmesi gerekir). Komut bu durumda çıplak 500 yerine kök nedeni
> açıklayan net bir hata döndürür.

### Member / Me

```bash
plane-cli member list                       # workspace üyeleri (UUID çözmek için ANA kaynak)
plane-cli member list --project <UUID>      # proje üyeleri
plane-cli member me                          # mevcut authenticated kullanıcı
plane-cli --json member me | jq .email
```

> `member list` assignee/lead UUID'lerini bulmanın yoludur — kullanıcı isim/email
> verdiğinde önce burada eşleştir.

## Roller

`member list` çıktısında `role`: **Admin=20, Member=15, Guest=5** (binary `role_name` çevirir).

## Akış Örneği — "PROJ-123 ne durumda?"

1. `plane-cli --json issue get-id PROJ-123` → UUID + state + assignee + detay
2. Tablo gibi sun; yorumlar isteniyorsa `comment list --project <UUID> --issue <UUID>`

## Akış Örneği — "Bu sorun için issue aç"

1. Proje belli değilse `project list` → doğru proje UUID'sini bul (gerekirse `AskUserQuestion`)
2. Title + açıklama çıkar. **Açıklama biçimi seçimi (KRİTİK)**:
   - Başlık/kod bloğu/kalın/liste gibi **zengin biçimlendirme** gerekiyorsa → `--description-html`
     ile **HTML gövdesi** ver (`<h2>...</h2><p>...</p><pre><code>...</code></pre>`).
   - **Sadece düz metin** ise → `--description` (CLI escape edip `<p>` ile sarar).
   - **ASLA `--description`'a HTML markup'ı yazma** — escape edilip `&lt;h2&gt;` gibi ham etiket
     olarak görünür (render olmaz). HTML her zaman `--description-html` ile gider.
3. `AskUserQuestion`:
   - header: "Issue"
   - question: "Plane'de yeni work item oluşturayım mı?"
   - options: ["Evet, oluştur", "Hayır"]
4. Onay → düz metin: `plane-cli issue create "..." --project <UUID> --priority medium --description "..."`
   zengin: `plane-cli issue create "..." --project <UUID> --priority medium --description-html "<h2>...</h2><p>...</p>"`
5. Dönen `id` (UUID) + identifier'ı (`PROJ-N`) kullanıcıya bildir

## Akış Örneği — "Sprint'e şu issue'ları ekle"

1. `cycle list --project <UUID>` → doğru cycle UUID
2. Issue UUID'lerini topla (`issue get-id` / `issue list`)
3. `plane-cli cycle add-items <cycle-UUID> "<uuid1>,<uuid2>" --project <UUID>`

## Akış Örneği — "Bana ata"

1. `plane-cli --json member me | jq .id` → kendi user UUID
2. `plane-cli issue assignee <issue-UUID> --project <UUID> --add <user-UUID>`
   (update DEĞİL — assignee komutu listeyi korur, sadece ekler)

## Hata Durumları

- `Not found (404)` → UUID yanlış, trailing-slash gerektiren endpoint, veya kaynak yok
- `Unauthorized (401)` → `PLANE_API_KEY` geçersiz/süresi dolmuş, Settings → API Tokens'tan yenile
- `Permission denied (403)` → Token'ın workspace/proje yetkisi yok (Guest rolü olabilir)
- `Bad request (400)` → Parametre formatı (priority/state-group enum'ları, tarih ISO 8601)
- `Unprocessable (422)` → Geçersiz alan değeri (örn. archive için state completed/cancelled değil)
- `Error: PLANE_URL...` → env eksik

## İpuçları

- **UUID önce çöz**: assignee/state/label/cycle hep UUID — isim verildiğinde önce
  `member list` / `state list` / `label list` / `cycle list` ile eşleştir.
- **Priority enum'u**: `urgent`, `high`, `medium`, `low`, `none` (tam küçük harf).
- **State group enum'u**: `backlog`, `unstarted`, `started`, `completed`, `cancelled`.
- **Tarihler ISO 8601**: `2026-06-23` formatı.
- **assignee/label flag farkı**: `issue update --assignees` listeyi **değiştirir** (replace),
  `issue assignee --add/--remove` listeyi **korur** (incremental). Tek kişi eklerken `assignee` kullan.
- **HTML alanları**: `comment add --comment-html`, `page create --description-html`,
  `issue create/update --description-html` HTML'i **escape ETMEDEN** gönderir (render olur).
  Buna karşılık `issue create/update --description` düz metindir — CLI escape edip `<p>` ile sarar,
  bu yüzden **HTML'i `--description`'a yazma** (ham `&lt;tag&gt;` olarak görünür).
- **worklog dakika**: `worklog create <dakika>` — pozitif tam sayı (dakika cinsinden).
- **DESTRUCTIVE komutlar**: create/update/delete/archive — her zaman `AskUserQuestion` ile onay al.

## İlgili Kaynaklar

- Repo README: `${CLAUDE_SKILL_DIR}/../../README.md` (yoksa proje CLAUDE.md)
- Plane API docs: https://developers.plane.so/api-reference/introduction
- Resmi SDK: https://github.com/makeplane/plane-python-sdk
- MCP server: https://github.com/makeplane/plane-mcp-server
