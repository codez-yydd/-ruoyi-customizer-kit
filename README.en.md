# RuoYi Forge

[中文](README.md) | English

A desktop tool for rapid initialization and customization of [RuoYi-Vue](https://gitee.com/y_project/RuoYi-Vue) projects. Automates package renaming, module restructuring, config file refactoring, MyBatis-Plus integration, UniApp mini-program scaffolding, and more — so you don't have to do it all by hand.

> This project is co-developed with [GLM 5.2](https://zhipuai.cn).

## Features

- **Package Renaming** — Full-text Java package replacement with automatic directory migration
- **Module Restructuring** — Backend modules + frontend directories renamed by prefix (e.g. `ruoyi-admin` → `demo-admin`)
- **Maven Coordinate Update** — Batch replacement of groupId / artifactId / module dependency references
- **Config File Refactoring** — Splits `application.yml` + `application-druid.yml` into `application.yaml` + `application-dev.yaml` + `application-prod.yaml`
- **MyBatis-Plus Integration** — Adds dependency, generates pagination config, refactors existing Mapper/Service/ServiceImpl to inherit MP base classes, adapts code generator templates
- **Long ID Precision Fix** — Adds `@JsonSerialize(using = ToStringSerializer.class)` to Long primary key fields
- **UniApp Scaffolding** — Optionally generates `{prefix}-uniapp` project with request utilities, login framework, env config; auto-appends WeChat config to backend application files
- **Deferred Extraction** — ZIP archives are extracted at execution time to a user-chosen output directory; original files are never modified
- **Execution Preview** — Preview task list, impact scope, and high-risk items before executing
- **Residue Scanning** — Post-execution validation for leftover old package/module names
- **Execution Report** — Generates a Markdown transformation report

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
