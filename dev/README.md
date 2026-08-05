# dev/ 开发工作区

本目录是「替换后台 UI（vben-web-ele 适配若依）」功能的开发环境，包含前端适配工程与联调用的若依后端。
**这是开发工作区，非工具产物**——最终适配好的 vben 工程会快照到 `src-tauri/templates/ruoyi-vue/ui/vben-web-ele/` 成为工具的预置模板。

## 目录结构

```
dev/
├── run.bat / run.sh        后端启动脚本（若依 SpringBoot3）
├── run-ui.bat / run-ui.sh  前端启动脚本（vben web-ele）
├── README.md
├── vben-ui/                vben-admin 适配工程（基于 gitee annsion/vue-vben-admin，已裁剪 + 适配若依）
└── ruoyi-backend/          若依后端（gitee y_project/RuoYi-Vue springboot3 分支，已改本地配置）
```

> 说明：`vben-ui` 与 `ruoyi-backend` 均克隆自上游，已删除各自 `.git`，作为本仓库普通目录管理。

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
```

### 3. 启动 Redis
确保本地 6379 运行（db15，无密码）。

### 4. 启动两端
```bash
# 终端1：启动后端（http://localhost:14001）
cd dev && ./run.sh          # 或 Windows 双击 run.bat

# 终端2：启动前端（http://localhost:5777）
cd dev && ./run-ui.sh       # 或 Windows 双击 run-ui.bat
```

浏览器访问 http://localhost:5777 ，用 `admin / admin123` 登录。

## 配置说明
| 项 | 值 |
|---|---|
| 后端端口 | 14001 |
| 前端端口 | 5777 |
| MySQL | localhost:3306 / ruoyi / root / 123456 |
| Redis | localhost:6379 / db=15 / 无密码 |
| vite proxy | `/api` → `http://localhost:14001`（rewrite 去掉 /api 前缀，匹配若依无前缀接口） |

配置文件位置：
- 后端：`ruoyi-backend/ruoyi-admin/src/main/resources/application.yml`（端口/Redis）
- 后端：`ruoyi-backend/ruoyi-admin/src/main/resources/application-druid.yml`（MySQL）
- 前端：`vben-ui/apps/web-ele/vite.config.mts`（proxy）
- 前端：`vben-ui/apps/web-ele/.env.development`（环境变量）

## 适配进度
- [x] 对接层（auth/user/menu/captcha/request）
- [x] 字典系统 + DictTag + v-hasPermi 权限指令
- [x] system: user/role/menu/dept/post/dict/config/notice
- [x] monitor: operlog/logininfor/online/server/cache/job
- [ ] 联调验证（需真实后端 + 数据库）
- [ ] UI 细节打磨（边看边调）
