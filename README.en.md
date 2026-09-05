# RuoYi Forge

[中文](README.md) | English

A desktop tool for rapid initialization and customization of **RuoYi-Vue** (separated frontend/backend) and **RuoYi-Cloud** (microservices) projects. Official repositories can be pulled from the home screen. Automates package renaming, module restructuring, config file refactoring, MyBatis-Plus integration, UniApp mini-program scaffolding, and more — so you don't have to do it all by hand.

> This project is co-developed with [GLM 5.2](https://zhipuai.cn).

## Features

- **Package Renaming** — Full-text Java package replacement with automatic directory migration
- **Module Restructuring** — Backend modules + frontend directories renamed by prefix (e.g. `ruoyi-admin` → `demo-admin`)
- **Maven Coordinate Update** — Batch replacement of groupId / artifactId / module dependency references
- **Config File Refactoring** — For the separated edition, splits `application.yml` + `application-druid.yml` into `application.yaml` + `application-dev.yaml` + `application-prod.yaml`. For Cloud, rewrites Nacos `sql/ry_config*.sql` and does not generate the application.yaml trio
- **RuoYi-Cloud** — Rewrites Nacos datasource / Redis / Token and per-service ports (including bootstrap.yml); optional trim of gen / job / file / monitor; replacing the UI applies a cloud-overlay (login `/auth/login`, jobs `/schedule/**`, code gen `/code/**`, logs / online users, etc.)
- **MyBatis-Plus Integration** — Adds dependency, generates pagination config, refactors existing Mapper/Service/ServiceImpl to inherit MP base classes, adapts code generator templates
- **PostgreSQL Dialect** — For RuoYi-Vue, switch datasource, driver, pagination, init scripts, and code-generator queries to PostgreSQL (standalone / microservice not supported in this release)
- **Long ID Precision Fix** — Adds `@JsonSerialize(using = ToStringSerializer.class)` to Long primary key fields
- **Dev Startup Scripts** — Cloud project root `run.bat` / `run.sh` is an arrow-key checklist (↑↓ move, Space toggle, A select all, N select none, Enter start, Esc quit; all selected by default). Scans `run-*.bat` (excludes `run-ui`); copy `run-xxx.bat` for a new module to appear in the menu. Runs a single Maven install for the whole repo before starting services, to avoid parallel `mvn clean`. Jar deploy scripts remain `scripts/start.bat` (generated when “startup scripts” is enabled) and are not the same as the dev `run.bat`
- **Replace Admin UI** — Optionally replace original `ruoyi-ui` with a preset Vben Admin (Element Plus, pnpm monorepo) or Arco Design (Arco Design Vue, npm single package) template; title / port / copyright are filled via placeholders. Available for `ruoyi-vue` and `ruoyi-cloud`; disabled for standalone `ruoyi`
- **OSS** — Aliyun / Tencent Cloud / MinIO / Qiniu. Separated edition: `POST /common/oss/upload`. Cloud: `POST /system/oss/upload` (via gateway `/system/**`, login required). Does not replace the official local upload `/common/upload` or Cloud `/file/upload`
- **JWT** — Separated edition writes yaml `token.*`; Cloud writes Java `TokenConstants` / `CacheConstants`
- **UniApp Scaffolding** — Optionally generates `{prefix}-uniapp` project with request utilities, login framework, env config; auto-appends WeChat config to backend application files
- **Official Source Fetch** — From the home screen, pull official RuoYi-Vue or RuoYi-Cloud backends via Gitee (shallow git clone, no login) or GitHub (archive zip) and continue into detection
- **Deferred Extraction** — ZIP archives are extracted at execution time to a user-chosen output directory; original files are never modified
- **Execution Preview** — Preview task list, impact scope, and high-risk items before executing
- **Residue Scanning** — Post-execution validation for leftover old package/module names
- **Execution Report** — Generates a Markdown transformation report
- **CLI Mode** — Standalone `forge-cli` binary, config-file driven, unattended / CI-friendly (not bundled into the GUI installer)

## RuoYi-Cloud notes

Config rewriting goes through Nacos SQL, not the admin application.yaml trio. The Nacos address stays `127.0.0.1:8848` and the Sentinel port stays `8718`; neither is changed. Registration names and dataIds stay official `ruoyi-*`: only the pom module directory prefix is renamed, not the Nacos service name.

When Replace UI is enabled, log-menu `component` values are rewritten from `system/operlog` and `system/logininfor` to `monitor/...`; `perms` are left unchanged.

The gateway port is `server_port`. You may trim gen / job / file / monitor; gateway / auth / system cannot be trimmed.

For development, start Nacos first, then double-click `run.bat` (or `run.sh`) at the project root and select services. For production / packaged jars, use `scripts/start.bat` (requires “startup scripts”).

Note: docker-compose and the Nacos namespace are left unchanged. Redis is currently written as localhost:6379 / db1. Upload filesystem paths and Cloud `demoEnabled` are not configurable.

## Screenshots

A wizard-style workflow — customize a project in five steps: select project → auto-detect → configure → preview → execute.

### Home · Select Project

![Home](docs/img/01-home.png)

### Detect · Auto Recognition

![Detect](docs/img/02-detect.png)

### Configure · Package / Module / Frontend

![Configure - Basic](docs/img/03-config-basic.png)

### Configure · Integration Switches

![Configure - Features](docs/img/04-config-features.png)

### Configure · OSS / JWT / Deploy

![Configure - Deploy](docs/img/05-config-deploy.png)

### Preview · Task List

![Preview](docs/img/06-preview.png)

### Execute · Result Overview

![Execute](docs/img/07-execute.png)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Framework | [Tauri 2](https://tauri.app) |
| Frontend | [Vue 3](https://vuejs.org) + [Vite 6](https://vite.dev) + [Element Plus](https://element-plus.org) + [Pinia](https://pinia.vuejs.org) |
| Backend | [Rust](https://www.rust-lang.org) stable |
| Type Checking | [TypeScript 5.6](https://www.typescriptlang.org) + [vue-tsc](https://github.com/vuejs/language-tools) |
| AI Assisted | GLM 5.2 + Qwen 3.7 Plus |

## Prerequisites

- **Node.js** >= 20 (use npm; pnpm is not supported)
- **Rust** stable (install via [rustup](https://rustup.rs))
- **System Dependencies**:
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
  - Linux: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, etc. (see [Tauri docs](https://v2.tauri.app/start/prerequisites/))

## Install Dependencies

```bash
npm install
```

Rust dependencies are downloaded automatically on first build.

## Development

```bash
# Start dev mode (launches Vite dev server + Tauri window)
npm run tauri dev
```

```bash
# Type check frontend
npm run typecheck
```

## CLI Mode

Standalone `forge-cli` binary, driven by a config file, for unattended runs and CI.

### Build and Run

Development:

```bash
cd src-tauri
cargo run --bin forge-cli -- --help
```

Release build:

```bash
npm run build:cli
# Artifact: src-tauri/target/release/forge-cli (forge-cli.exe on Windows)
```

Release layout: place the `forge-cli` executable **alongside** `templates/` and `agents/` (it does not use GUI resource injection). CLI is **not** bundled into the `npm run tauri build` installer.

### Subcommands

Only `templates`, `detect`, `init-config`, `preview`, and `run` exist. There is no interactive wizard, no `doctor`, and no daemon.

#### templates

`forge-cli templates` — list built-in templates (name, loadable, detection notes).

#### detect

`forge-cli detect <path> [--json]` — detect an **already extracted** project directory. `--json` prints the full detection result as JSON.

#### init-config

Generate a prefilled config file. After detection, `original_*` is filled in; `new_project_name` = `--prefix` (same as the GUI). The written JSON is **not** redacted and includes `_comment` and `_source`. Secrets are stored in plaintext — do not commit the file to a public repository.

Required: `--source` `--package` `--prefix` `--title` `--output`

Optional: `--out` (default `forge.json`), `--set k=v` (repeatable)

```bash
forge-cli init-config --source ./ruoyi-vue.zip --package com.demo --prefix demo --title Demo --output ./out
```

#### preview

`forge-cli preview --config forge.json [--json]` — preview transform tasks without writing files. `_source` in the config is required (this subcommand has no `--source`).

#### run

```text
forge-cli run --config forge.json [--source <zip-or-dir>] [--set k=v ...] [--json] [--quiet]
```

- Without `--source`, `_source` in the config is required
- A path ending with `.zip` is treated as a zip; otherwise as a directory
- `--json`: progress as NDJSON (`{"type":"log",...}`), last line `{"type":"result",...}`; `result` redacts the JWT secret
- `--quiet`: print only the final summary
- `--set`: overlay `CustomizeParams` by serde field name using dotted paths; values are parsed as JSON (bool / number / string). Unknown fields or type mismatches exit with code 2
- Env `FORGE_SET="k=v;k2=v2"` uses the same format as `--set` and can be combined (`init-config` / `preview` / `run`)

`--set` examples:

```text
--set enable_sql_customize=true
--set db_type=postgresql
--set db_name=demo_db
--set jwt_expire_minutes=60
--set ui_template=arco
--set enable_cloud_custom_ports=true
```

Full workflow (`init-config` → `preview` → `run --set db_type=postgresql --set db_name=demo_db`):

PowerShell:

```powershell
.\forge-cli.exe init-config `
  --source D:\dl\ruoyi-vue.zip `
  --package com.demo `
  --prefix demo `
  --title Demo `
  --output D:\out\demo
.\forge-cli.exe preview --config forge.json
.\forge-cli.exe run --config forge.json --set db_type=postgresql --set db_name=demo_db
```

bash (one line):

```bash
forge-cli init-config --source ./ruoyi-vue.zip --package com.demo --prefix demo --title Demo --output ./out && forge-cli preview --config forge.json && forge-cli run --config forge.json --set db_type=postgresql --set db_name=demo_db
```

### Exit Codes

- `0` all succeeded
- `1` finished but some tasks failed or a check is Fail
- `2` usage / argument / config error

### Configuration Parameters

Field names match `CustomizeParams` in `src/types/index.ts`. Defaults match CLI `default_params()` / GUI `defaults()`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| original_package | string | `com.ruoyi` | Original Java package. `init-config` overwrites this from detection |
| new_package | string | `""` | New Java package. `init-config` fills from `--package` |
| original_module_prefix | string | `ruoyi` | Original module prefix. `init-config` overwrites this from detection |
| new_module_prefix | string | `""` | New module prefix. `init-config` fills from `--prefix` |
| original_project_name | string | `ruoyi` | Original project name. `init-config` overwrites from the detected module prefix |
| new_project_name | string | `""` | New project name. `init-config` sets this to `--prefix` (same as GUI) |
| frontend_title | string | `""` | Frontend title. `init-config` fills from `--title` |
| copyright_year | string | `""` | Copyright year (e.g. `2024-2026`); empty skips copyright replacement |
| copyright_holder | string | `""` | Copyright holder; empty skips copyright replacement |
| enable_footer_icp | boolean | `true` | Footer copyright and ICP: always show footer, extend year, read ICP from backend yaml |
| enable_site_settings | boolean | `true` | Admin site-settings page: runtime title / logo / ICP (stored in `sys_config`) |
| enable_mybatis_plus | boolean | `true` | Integrate MyBatis-Plus |
| enable_config_rewrite | boolean | `true` | Separated edition: rewrite admin into the application.yaml trio. Cloud: rewrite Nacos `sql/ry_config*.sql`; does not generate the application.yaml trio |
| enable_logback_rewrite | boolean | `true` | Rewrite logback paths |
| enable_generator_mybatis_plus | boolean | `true` | Adapt code-generator templates for MyBatis-Plus |
| enable_long_id_json_string | boolean | `true` | Serialize Long primary keys as strings to avoid JS precision loss |
| enable_snowflake_id | boolean | `false` | Global snowflake IDs: setId on insert, disable auto-increment |
| enable_report | boolean | `true` | Generate a Markdown transformation report |
| enable_clear_home | boolean | `true` | Clear the RuoYi frontend home page to a blank page |
| enable_remove_github | boolean | `true` | Remove GitHub / Gitee links from the top bar |
| enable_remove_docs | boolean | `true` | Remove documentation links from the top bar |
| output_dir | string | `""` | Output directory. `init-config` fills from `--output` |
| enable_uniapp | boolean | `false` | Generate a UniApp mini-program project |
| wx_appid | string | `""` | WeChat mini-program AppID (meaningful only when `enable_uniapp=true`) |
| wx_appsecret | string | `""` | WeChat mini-program AppSecret |
| pay_included | boolean | `false` | Include WeChat Pay (config block + SDK dependency + config class) |
| pay_enabled | boolean | `false` | Enable WeChat Pay (yml `enabled`) |
| pay_mode | string | `public-key` | Pay mode: `public-key` (V3 public key) / `certificate` (V3 platform cert) / `v2` |
| pay_mch_id | string | `""` | Merchant ID |
| pay_mch_serial_no | string | `""` | Merchant certificate serial (V3) |
| pay_api_v3_key | string | `""` | API V3 key |
| pay_private_key_path | string | `classpath:cert/apiclient_key.pem` | Merchant API private key path (V3) |
| pay_public_key_id | string | `""` | WeChat Pay platform public key ID (V3 public-key mode) |
| pay_public_key_path | string | `classpath:cert/wxp_pub.pem` | WeChat Pay platform public key path (V3 public-key mode) |
| pay_api_key | string | `""` | API V2 key |
| pay_cert_path | string | `classpath:cert/apiclient_cert.p12` | Merchant cert path `apiclient_cert.p12` (V2) |
| pay_notify_url | string | `""` | Payment notify URL (shared by dev / prod) |
| enable_security | boolean | `false` | Security hardening (admin password, disable register, clean demo users, etc.) |
| admin_password | string | `""` | New admin password in plaintext; empty leaves it unchanged |
| clean_demo_users | boolean | `false` | Remove demo user data (ry / ryadmin, etc.) |
| enable_sql_customize | boolean | `false` | Customize SQL init scripts |
| db_name | string | `""` | New database name. Separated edition: empty uses the module prefix. Cloud: empty keeps official `ry-cloud`; if set, the business DB uses that name and the config DB defaults to `{name}-config` |
| db_host | string | `127.0.0.1` | Database host; written into the datasource only when SQL customize is on |
| db_port | number | `3306` | Database port; falls back to `5432` when PostgreSQL and the value is 0 |
| db_username | string | `root` | Database username |
| db_password | string | `""` | Database password; may be empty |
| config_db_name | string | `""` | Cloud config-database name. Empty: `{db_name}-config` if `db_name` is set, otherwise `ry-config` |
| remove_modules | string[] | `[]` | Cloud modules to trim; only `gen` / `job` / `file` / `monitor` |
| enable_cloud_custom_ports | boolean | `false` | Custom Cloud module ports. Off: increment from the gateway port; trimmed modules do not consume a port |
| cloud_port_auth | number | `0` | Cloud auth port; `0` = auto-increment |
| cloud_port_system | number | `0` | Cloud system port; `0` = auto-increment |
| cloud_port_gen | number | `0` | Cloud gen port; `0` = auto-increment |
| cloud_port_job | number | `0` | Cloud job port; `0` = auto-increment |
| cloud_port_file | number | `0` | Cloud file port; `0` = auto-increment |
| cloud_port_monitor | number | `0` | Cloud monitor port; `0` = auto-increment |
| db_type | string | `mysql` | Database type: `mysql` or `postgresql` only; `postgresql` is ruoyi-vue only |
| admin_username | string | `""` | Admin login name; empty keeps `admin` (only the `user_id=1` seed row) |
| admin_nickname | string | `""` | Admin nickname; empty keeps `若依` |
| clean_quartz | boolean | `false` | Remove quartz tables and data |
| enable_frontend_split | boolean | `false` | Split frontend out of the project root, sibling to the backend |
| enable_ai_rules | boolean | `true` | Generate AI rule files (`AGENTS.md` + `CLAUDE.md`) |
| enable_sub_agents | boolean | `false` | Inject sub-agent collaboration notes into `AGENTS.md` |
| sub_agents_description | string | `""` | Sub-agent notes for `AGENTS.md` (can be generated by scanning `agents/`) |
| enable_oss | boolean | `false` | Enable OSS (Aliyun / Tencent Cloud / MinIO / Qiniu). Separated edition: `POST /common/oss/upload`. Cloud: `POST /system/oss/upload` (via gateway `/system/**`, login required). Does not replace official local upload `/common/upload` or Cloud `/file/upload` |
| oss_provider | string | `aliyun` | OSS provider: `aliyun` / `tencent` / `minio` / `qiniu` |
| oss_endpoint | string | `""` | Endpoint (Aliyun/Tencent region, MinIO address, or Qiniu domain) |
| oss_bucket | string | `""` | Bucket name |
| oss_access_key | string | `""` | accessKey |
| oss_secret_key | string | `""` | secretKey |
| oss_custom_domain | string | `""` | Custom domain (CDN); empty uses the default domain |
| enable_jwt | boolean | `false` | Customize JWT. Separated edition writes yaml `token.*`; Cloud writes Java `TokenConstants` / `CacheConstants` |
| jwt_secret | string | `""` | JWT secret; if enabled and empty, a random secret is generated |
| jwt_expire_minutes | number | `30` | Token TTL in minutes |
| enable_generator_config | boolean | `false` | Customize code-generator config (`generator.yml`) |
| generator_author | string | `""` | Generated-code author name |
| generator_table_prefix | string | `""` | Table prefixes to strip (comma-separated, e.g. `sys_,tb_`) |
| generator_vue3 | boolean | `false` | Upgrade generator templates to Vue3 |
| enable_nginx_config | boolean | `false` | Generate an Nginx reverse-proxy config |
| server_port | number | `8080` | Separated edition: admin port. Cloud: gateway port; other services default to incrementing from this |
| server_name | string | `""` | Public hostname; empty uses `localhost` |
| use_https | boolean | `false` | Enable HTTPS (certificate placeholder block) |
| enable_startup_scripts | boolean | `false` | Generate jar deploy scripts `scripts/start.bat` (and `.sh`). Not the same as the project-root dev scripts `run.bat` / `run.sh` |
| enable_replace_ui | boolean | `false` | Replace original `ruoyi-ui` with a preset admin UI (`ruoyi-vue` and `ruoyi-cloud`; disabled for standalone `ruoyi`) |
| ui_template | string | `vben-web-ele` | Admin UI template: `vben-web-ele` or `arco` |

The config file may also contain `_comment` and `_source` (not `CustomizeParams` fields; CLI uses them to record notes and the source path).

Constraints:

- Older JSON without `db_type` is treated as `mysql`
- `ruoyi` / `ruoyi-cloud` cannot use `postgresql`
- PostgreSQL init scripts do not include `CREATE DATABASE`; create the database yourself
- Cloud keeps official `ruoyi-*` dataIds / registration names
- Nacos `8848` and Sentinel `8718` are not changed
- `remove_modules` accepts only `gen` / `job` / `file` / `monitor`; illegal values are rejected
- Passwords / secrets in the config are plaintext by design
- GUI `save_config_json` redacts secrets; CLI `init-config` does not

## Build & Package

```bash
# Build for production (macOS .app/.dmg, Windows .exe/.msi, Linux .deb/.AppImage)
npm run tauri build
```

Build artifacts are located in `src-tauri/target/release/bundle/`.

## Project Structure

```
ruoyi-forge/
├── src/                     # Vue 3 frontend
├── src-tauri/               # Rust backend
│   ├── src/bin/forge_cli.rs # forge-cli entry
│   ├── src/core/            # Core engine (scan/detect/plan/execute/validate/report)
│   ├── templates/ruoyi-vue/ # Rule JSON files + UniApp template
│   └── tests/               # Integration tests
├── docs/                    # Documentation
└── dist/                    # Frontend build output
```

## Running Tests

```bash
cd src-tauri
cargo test
```

## License

[MIT](LICENSE)
