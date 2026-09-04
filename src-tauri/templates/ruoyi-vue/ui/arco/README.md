# arco

基于 Arco Design Vue 适配若依后端的预置后台 UI 模板（npm 单包工程，非 monorepo）。

由 `scripts/snapshot-arco-ui.sh`（Windows 用 `scripts/snapshot-arco-ui.ps1`）从 `dev/arco-ui` 快照生成。
执行改造时，`replace_ui` 会复制本目录到 `{prefix}-ui/` 并替换占位符。

## 技术栈

Vue 3 + TypeScript + Vite 6 + Pinia + Arco Design Vue（Node >= 20，npm 单包工程）。

## 常用命令

在 `{prefix}-ui` 目录下：

```bash
npm install
npm run dev
npm run build:prod
```

也可在项目根目录使用锻造台生成的 `run-ui` / `build` 脚本（无 pnpm-workspace.yaml 时自动走 npm 分支）。

## 环境变量

| 文件 | 变量 | 说明 |
|------|------|------|
| `.env` | `VITE_APP_TITLE` | 应用标题 |
| `.env` | `VITE_APP_REGISTER` | 注册入口开关：`true` 登录页显示注册入口（后端注册开关由参数 `sys.account.registerUser` 控制，关闭时提交注册会提示「当前系统没有开启注册功能」）；`false` 强制隐藏注册入口且 `/register` 重定向登录页 |
| `.env.development` | `VITE_APP_PORT` / `VITE_APP_BASE_API` | 开发端口（5778）/ 接口前缀（`/api`，经 vite proxy 转发到后端） |
| `.env.production` | `VITE_APP_BASE_API` | 生产接口前缀（`/prod-api`，由 nginx 反代到后端） |

## 占位符清单

| 占位符 | 含义 |
|--------|------|
| `{{FRONTEND_TITLE}}` | 前端标题（VITE_APP_TITLE） |
| `{{MODULE_PREFIX}}` | 模块前缀（package.json name） |
| `{{API_BASE_URL_DEV}}` | 开发环境后端地址（vite proxy：`/api`、`/v3/api-docs`、`/webjars` 三段） |
| `{{COPYRIGHT_HOLDER}}` | 版权方（页脚） |
| `{{COPYRIGHT_YEAR}}` | 版权年份（页脚） |
| `{{PROJECT_NAME}}` / `{{SERVER_PORT}}` | 其它通用占位 |
