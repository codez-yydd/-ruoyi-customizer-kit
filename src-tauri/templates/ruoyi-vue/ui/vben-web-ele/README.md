# vben-web-ele

基于 vue-vben-admin（web-ele）适配若依后端的预置后台 UI 模板。

由 `scripts/snapshot-vben-ui.ps1` 从 `dev/vben-ui` 快照生成。
执行改造时，`replace_ui` 会复制本目录到 `{prefix}-ui/` 并替换占位符。

## 常用命令

在 `{prefix}-ui` 目录下：

```bash
pnpm install
pnpm run dev:ele
pnpm run build:ele
```

也可在项目根目录使用锻造台生成的 `run-ui` / `build` 脚本（已自动识别 Vben monorepo）。
