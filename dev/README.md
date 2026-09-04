# dev/ 开发工作区

本目录是「替换后台 UI（vben-web-ele 适配若依）」功能的开发环境，包含前端适配工程与联调用的若依后端。
**这是开发工作区，非工具产物**——最终适配好的 vben 工程会快照到 `src-tauri/templates/ruoyi-vue/ui/vben-web-ele/` 成为工具的预置模板。

## 目录结构

```
dev/
├── run.bat / run.sh        后端启动脚本（若依 SpringBoot3）
├── run-ui.bat / run-ui.sh  前端启动脚本（vben web-ele）
├── run-arco.bat / run-arco.sh  前端启动脚本（arco-ui）
├── README.md
├── vben-ui/                vben-admin 适配工程（基于 gitee annsion/vue-vben-admin，已裁剪 + 适配若依）
├── arco-ui/                Arco Admin 适配工程（Vue3 + TS + Vite + Pinia + Arco Design Vue，从零搭建）
└── ruoyi-backend/          若依后端（gitee y_project/RuoYi-Vue springboot3 分支，已改本地配置）
```

> 说明：`vben-ui` 与 `ruoyi-backend` 均克隆自上游，已删除各自 `.git`，作为本仓库普通目录管理。

## 目录约定

| 路径 | 用途 |
|------|------|
| `dev/vben-ui/` | **适配开发区**：本地联调改页面、对接若依接口，不直接作为改造产物 |
| `dev/arco-ui/` | **适配开发区**：Arco Design Vue 版后台（单包 vite 工程），对接同一若依后端 |
| `src-tauri/templates/ruoyi-vue/ui/vben-web-ele/` | **工具内置模板**：锻造台执行「替换后台 UI」时从此复制到 `{prefix}-ui/` |
| `src-tauri/templates/ruoyi-vue/ui/arco/` | **工具内置模板**：arco-ui 的快照产物（由 `scripts/snapshot-arco-ui.sh` 生成），供锻造台复制到 `{prefix}-ui/` |

适配完成后（或大改后）在仓库根目录快照：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/snapshot-vben-ui.ps1
```

```bash
bash scripts/snapshot-arco-ui.sh
```

## 启动步骤

### 1. 准备数据库（仅首次）
本地 MySQL 创建 `ruoyi` 库并导入 SQL：
```bash
mysql -uroot -p123456 -e "CREATE DATABASE IF NOT EXISTS ruoyi DEFAULT CHARSET utf8mb4"
mysql -uroot -p123456 ruoyi < dev/ruoyi-backend/sql/ry_20260417.sql
mysql -uroot -p123456 ruoyi < dev/ruoyi-backend/sql/quartz.sql
```

### 2. 安装前端依赖（仅首次）
```bash
cd dev/vben-ui
pnpm install

# arco-ui（二选一或都装，两者共用同一后端）
cd dev/arco-ui
npm install
```

### 3. 启动 Redis
确保本地 6379 运行（db15，无密码）。

### 4. 启动两端
```bash
# 终端1：启动后端（http://localhost:14001）
cd dev && ./run.sh          # 或 Windows 双击 run.bat

# 终端2：启动前端（http://localhost:5777）
cd dev && ./run-ui.sh       # 或 Windows 双击 run-ui.bat

# 或启动 Arco 版前端（http://localhost:5778）
cd dev && ./run-arco.sh     # 或 Windows 双击 run-arco.bat
```

浏览器访问 http://localhost:5777 （vben 版）或 http://localhost:5778 （arco 版），用 `admin / admin123` 登录。

## 配置说明
| 项 | 值 |
|---|---|
| 后端端口 | 14001 |
| vben 前端端口 | 5777 |
| arco 前端端口 | 5778 |
| MySQL | localhost:3306 / ruoyi / root / 123456 |
| Redis | localhost:6379 / db=15 / 无密码 |
| vite proxy | `/api` → `http://localhost:14001`（rewrite 去掉 /api 前缀，匹配若依无前缀接口） |
| 日志/上传路径 | 已本地化为运行目录下 `./logs` 与 `./uploadPath`（logback.xml / application.yml）；原值 `/home/ruoyi/xxx` 仅适用于 Linux |

配置文件位置：
- 后端：`ruoyi-backend/ruoyi-admin/src/main/resources/application.yml`（端口/Redis）
- 后端：`ruoyi-backend/ruoyi-admin/src/main/resources/application-druid.yml`（MySQL）
- 前端（vben）：`vben-ui/apps/web-ele/vite.config.mts`（proxy）
- 前端（vben）：`vben-ui/apps/web-ele/.env.development`（环境变量）
- 前端（arco）：`arco-ui/vite.config.ts`（proxy，端口读 `.env.development` 的 `VITE_APP_PORT`）

## arco-ui 说明

Arco Design Vue 版后台（`dev/arco-ui/`），单包普通 vite 工程（非 monorepo，npm 管理），后端协议与 vben 版完全一致：

- token 存 localStorage `Admin-Token`，请求头 `Authorization: Bearer {token}`
- `/login` 响应 token 在顶层；`/getInfo` 的 user/roles/permissions 在顶层；`/captchaImage` 的 img 为裸 base64（前端补前缀），captchaEnabled=false 时隐藏验证码
- 动态路由：`/getRouters` 的 `Layout`/`ParentView` 渲染为嵌套 router-view，`InnerLink` 为 iframe 内嵌（URL 取 meta.link），外链菜单新窗口打开；noCache 取反为 keep-alive 缓存标记
- 权限指令 `v-hasPermi` / `v-hasRole`（`*:*:*` 与 admin 全通过）
- 当前进度：工程骨架 + 认证体系 + 动态路由 + 布局（侧边栏/顶栏/多标签页）+ 登录页 + 首页 + 错误页；system / monitor / tool 业务页面已全部完成，typecheck 与 build:prod 通过，并已快照接入锻造台模板（`src-tauri/templates/ruoyi-vue/ui/arco/`）

## 适配进度
- [x] 对接层（auth/user/menu/captcha/request）
- [x] 登录页验证码图片显示（form schema suffix 渲染）+ 隐藏手机/扫码/第三方登录入口
- [x] 字典系统 + DictTag + v-hasPermi 权限指令
- [x] system: user/role/menu/dept/post/dict/config/notice
- [x] monitor: operlog/logininfor/online/server/cache/job
- [x] 外链菜单适配（若依 path=完整URL → 移到 meta.link，避免 addRoute 崩溃）
- [x] 登录后跳首页（homePath 指向真实菜单 /system/user，无 dashboard 页）
- [x] 顶部栏用户信息（头像/昵称接 /getInfo；头像补 API 前缀）
- [x] 个人中心页（/user/profile：基本资料/修改密码/头像上传，隐藏路由）
- [ ] 联调验证（需真实后端 + 数据库）
- [ ] UI 细节打磨（边看边调）
- [x] 快照脚本：`scripts/snapshot-vben-ui.ps1` → `src-tauri/templates/ruoyi-vue/ui/vben-web-ele/`
- [x] 占位符：标题 / 开发代理 / 版权（由锻造台 replace_ui 写入）

## 快照到工具模板

适配完成后（或每次大改前端后），在仓库根目录执行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/snapshot-vben-ui.ps1
```

会排除 `node_modules` 等目录，并把本地联调值替换为 `{{FRONTEND_TITLE}}` 等占位符。
