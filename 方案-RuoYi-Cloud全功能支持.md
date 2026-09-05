# 方案：RuoYi-Cloud 全功能支持（Spring Boot 2 / 3 / 4 三档）

> 本方案基于提交 `6a65935` 的代码现状编写（2026-09-05），供实施 Agent（Cursor）直接执行。
> 已敲定的范围决策：**全功能对齐分离版**（含站点设置/页脚 ICP/UniApp/微信支付/替换 UI/Nginx）+ **微服务模块裁剪本期做** + **Boot 3/4 样本从官方仓库分支拉取核实**。
> 生态事实（2026-09 已核实）：RuoYi-Cloud 官方最新 v3.6.7（2025-12），官方 **Boot 2.x / 3.x / 4.x 三分支并行维护**（master = Boot 4 + Nacos 3.x；3.6.x = Boot 2.7 + Spring Cloud 2021.0.x）。

---

## 一、现状断点清单（均已核实到代码）

| # | 功能 | Cloud 下现状 | 断点位置 |
|---|------|--------------|----------|
| 0 | 识别/包名/模块/Maven/残留扫描 | ✅ 可用 | detect.json 已对照官方校准 |
| 1 | Boot 大版本检测 | ✅ 可用 | `spring_boot_major` 机制通用 |
| 2 | 配置重构 | ⏭️ 静默跳过 | `find_resources_dir` 只找 `*-admin`（executor.rs:581）；Cloud 配置在 Nacos（`sql/ry_config_*.sql` 的 config_info 表文本），本地仅 bootstrap.yml |
| 3 | SQL 定制 | ⚠️ 半残 | `replace_db_name` 只匹配 `CREATE DATABASE/USE`（sql_customize.rs:124）；Cloud 双库（业务库 `ry-cloud` + 配置库 `ry-config`），且 config_info 文本里的 jdbc url 库名改不到 |
| 4 | MyBatis-Plus | ❌ 注错位置 | `first_writable_module`/`find_starter_module` 可能落到 gateway/auth；Mapper/Service 继承改造按分离版结构扫描 |
| 5 | 页脚 ICP / webInfo / 站点设置 | ❌ 任务失败 | web_footer 找 `framework` 模块 SecurityConfig（web_footer.rs:341）返回 Err；Cloud 认证在 Gateway + Auth（网关白名单机制） |
| 6 | 雪花 / Long ID | ⚠️ 未核实 | 模块定位逻辑按分离版假设（实施时核实 first_writable_module 类逻辑） |
| 7 | UniApp / 微信支付 | ❌ 接口不匹配 | uniapp 模板接口 `/app/{prefix}/auth/*`（分离版后端假设）；Cloud 登录走网关 `/auth/login` |
| 8 | 替换 UI（vben/arco） | ❌ 认证流不匹配 | vben 已深度适配分离版（`POST /login` 返回 `{token}`、`/prod-api` 前缀，见 ui/vben-web-ele/apps/web-ele/src/api/core/auth.ts）；Cloud 是 `/auth/login` 返回 `access_token/expires_in`，getInfo 走 `/system/user/getInfo` |
| 9 | Nginx / 启动脚本 | ❌ 分离版假设 | scripts 面向单体 admin；nginx upstream 指向 admin 8080 |
| 10 | 前端功能裁剪表 | ⚠️ 虚假开放 | `template-capabilities.ts` 中 cloud 仅禁 `db_type`，其余开关全开但实际会失败/跳过 |
| 11 | PG 方言 | 已禁用 | planner.rs:996 明确 cloud 不规划方言任务（本期维持，见六） |

---

## 二、总体设计

### 2.1 架构原则

1. **保持 Nacos 配置中心架构不变**：配置定制通过改写 `sql/ry_config_*.sql` 中 config_info 的配置文本实现（用户导入 nacos 库后生效，符合官方部署流程）。不给服务生成本地 application 三件套。
2. **cloud 专属逻辑集中 + 就地小分支**：新建 `core/nacos_config.rs`（Nacos 配置文本解析/改写引擎，见 3.2）承载最复杂的共性能力；其余模块（MP/web_footer/oss/wechat/scripts/nginx）在现有函数内按 `info.template_dir == "ruoyi-cloud"` 或模块结构探测做分支，不新建平行模块。
3. **Boot 三档复用现有机制**：`spring_boot_major` + MP starter 三分支直接生效；Cloud 特有差异（Nacos 2/3、SCA 版本、bootstrap 写法）进核实清单后按锚点兼容处理。
4. **能力矩阵驱动 UI**：`template-capabilities.ts` 为 cloud 建立完整支持表，ParamConfig 按 cloud 显示专属参数区。

### 2.2 新增参数（CustomizeParams）

| 参数 | 类型 | serde 默认 | 用途 |
|------|------|-----------|------|
| `config_db_name` | `String` | `""` | Nacos 配置库名（空则 `{new_module_prefix}-config`，与业务库 `{prefix}` 呼应；替换 ry-config） |
| `remove_modules` | `Vec<String>` | `[]` | 微服务模块裁剪，合法值 `gen` / `job` / `file` / `monitor`（对应 ruoyi-gen/job/file/ruoyi-visual 下 monitor；`api`/`common`/`gateway`/`auth`/`system` 不可裁剪，validate 拒绝） |

复用现有参数的 Cloud 语义（不加新字段，文档与 UI 文案注明）：
- `server_port` → 网关端口（RuoYi-Cloud gateway 默认 8080，与分离版 admin 一致，nginx/scripts 对齐复用）
- `db_name` → 业务库；`jwt_secret`/`jwt_expire_minutes` → 注入 Nacos 的 auth 配置文本；OSS/微信支付/站点设置字段 → 注入位置变为 Nacos 配置文本或 system 服务

**明确不加**：`nacos_server_addr`（bootstrap.yml 里的 nacos 地址是部署期信息，改造期改写无意义，保持 localhost:8848 原样）。

---

## 三、功能改造明细

### 3.1 基础层收口（低成本，先行）

1. `planner.rs`：cloud 任务的 created_files 预估、任务描述按 cloud 实际形态调整（如「配置重构」→「Nacos 配置定制」）。
2. `validator.rs`：cloud 场景校验项适配——旧包名残留扫描已通用；新增「双库脚本均存在且已改库名」「bootstrap.yml 未被破坏（仍含 nacos 配置）」校验。
3. logback：detect.json 已声明六个服务的 logback 文件，核实 `logback.rs` 是否按模板声明列表逐文件处理（若只处理 admin 则补多文件循环）。

### 3.2 Nacos 配置定制引擎（新模块 `core/nacos_config.rs`，本方案技术核心）

**难点**：config_info 的 content 在 SQL 文件里是**转义后的长文本**（单引号 `''` 转义、换行为 `\r\n` 字面量），必须先反转义成 yaml 文本、改写后再按原格式重新转义写回，不能对 SQL 原文做正则盲替换。

接口设计：

```rust
/// 从 ry_config_*.sql 解析出的一个服务配置
pub struct NacosServiceConfig {
    pub service: String,      // ruoyi-system / ruoyi-gateway / ...
    pub profile: String,      // dev / prod
    pub content: String,      // 反转义后的 yaml 全文
    /// 写回 SQL 文件所需的定位信息（原始 INSERT 行片段或字节区间）
    ...
}

pub fn parse_config_sql(sql_path: &Path) -> Result<Vec<NacosServiceConfig>, String>;
pub fn write_back(sql_path: &Path, configs: &[NacosServiceConfig]) -> Result<(), String>;
```

改写能力（均操作反转义后的 yaml 文本，复用 config_rewrite.rs 已有的行级手术函数风格，保留注释）：

1. **库名**：`jdbc:mysql://.../{业务库名}` 全量替换（含 master/slave）。
2. **Redis**：host/port/password/database 键值替换（Boot 2 的 `spring.redis` 与 Boot 3/4 的 `spring.data.redis` 两种键位都处理——读取服务 pom 或按内容特征判断，实施时按官方两分支实际配置核实）。
3. **JWT**：`token.secret` / `token.expireTime`（位于 auth 与 system 服务配置，核实实际归属后定点替换；secret 为空则随机生成，复用现有随机生成逻辑）。
4. **MP 注入**（enable_mybatis_plus 时）：把 `mybatis:` 配置块替换为 `mybatis-plus:` 块（mapper 路径按 cloud 实际值保留）。
5. **OSS / 微信支付配置注入**：追加到 system 服务配置（键结构复用现有 `{prefix}.oss` / `{prefix}.wechat.pay` 生成逻辑）。
6. **网关白名单**（页脚/站点设置用，见 3.5）：向 gateway 服务配置的白名单列表追加 `/webInfo`（键名与结构以官方分支实际配置为准，进核实清单）。

规划为独立任务 `TaskType::RewriteNacosConfig`，跳过条件：找不到 ry_config SQL（合法跳过并日志说明）。

### 3.3 SQL 双库定制（sql_customize.rs 扩展）

1. 库名替换新增分支：`CREATE DATABASE`/`USE` 之外，追加对 config_info 文本内 jdbc url 的库名替换（由 3.2 引擎完成，本模块只处理语句级）。
2. 双库语义：业务库 `ry-cloud` → `{db_name 或 prefix}`；配置库 `ry-config` → `config_db_name`。两者正则分开、日志分开计数。
3. admin 密码 / 清演示 / 清 quartz：对业务库脚本照旧（已有逻辑通用）。

### 3.4 MyBatis-Plus / 雪花 / Long ID 的 Cloud 适配

1. **starter 落点**：cloud 分支固定写入 `ruoyi-common`（所有业务服务依赖它；gateway/auth 不依赖 MP）。jsqlparser 同步。幂等检查扫描范围改为 common + modules 下全部 pom。
2. **分页配置类**：生成到 `ruoyi-modules/ruoyi-system` 的 config 包（路径与包名按新包名生成）；`DbType` 复用方言逻辑（含 PG 分支，虽然 cloud 暂禁 PG，代码保持一致）。
3. **Mapper/Service 继承改造**：扫描范围 cloud 分支改为 `ruoyi-modules/ruoyi-system`、`ruoyi-modules/ruoyi-job` 的 src 目录。
4. **生成器适配**：cloud 的 gen 服务模板（`ruoyi-modules/ruoyi-gen/src/main/resources/vm/**`，detect.json 已声明）走现有 generator 改造逻辑，路径已通用。
5. **雪花 hutool**：依赖加 common；`insert` 注入与 `IdType.INPUT` 改造扫描 modules。
6. **Long ID**：`@JsonSerialize` 注入扫描范围同上。

### 3.5 页脚 ICP / webInfo / 站点设置（Gateway 架构实现）

1. **放行机制**：不再找 SecurityConfig。向 Nacos gateway 配置的白名单追加 `/webInfo`（3.2 引擎能力）；同时核实 gateway 路由表是否需要为 `/webInfo` 增加到 system 服务的路由（官方路由 `/system/**` → system 服务，WebInfoController 放 system 则天然可达，**优先此方案**：接口路径定为 `/system/webInfo`，白名单放行该路径，前端请求路径统一）。
2. **Java 模板**：`WebInfoController` / `SiteSettingsController` 生成到 `ruoyi-modules/ruoyi-system`（包名 `{new_package}.system` 体系，模板复用 `templates/ruoyi-vue/java/*.tmpl`，生成路径参数化）。
3. **sys_config 种子 / 菜单 SQL**：复用现有生成逻辑，写入业务库脚本。
4. **前端**：经典 ruoyi-ui 的 webInfo.js/Copyright 模板复用（前端结构与分离版一致）；vben/arco 走 3.8 的 cloud 适配层。
5. `web_footer.rs` 增加分支时保留分离版路径不动；cloud 分支失败语义改为明确 Err 提示（找不到 system 模块）。

### 3.6 UniApp + 微信支付（Cloud 接口适配）

1. uniapp 模板 `config/env.js` 的 baseUrl 指向网关（dev: `http://localhost:8080`）；登录接口 cloud 分支改 `/auth/login`（请求体/响应结构差异在模板 `api/auth.js` 内适配：token 取 `access_token`，有效期 `expires_in`）；用户信息 `/system/user/getInfo`（核实小程序是否已有账号密码登录页——现有骨架是微信登录 `/app/{prefix}/auth/wechat-login`，该后端接口在分离版由工具生成，Cloud 版本期一并生成到 system 服务，路径经网关 `/system/app/auth/**`，白名单按需放行——实施时以现有 uniapp 后端生成为准逐一移植）。
2. 微信支付：依赖加 system 服务 pom；`WechatPayConfig` 生成到 system；properties 配置注入 Nacos system 配置文本（3.2 引擎）。

### 3.7 替换 UI：vben / arco 的 Cloud 认证适配层

方案：**不复制整套模板目录**，在两个 UI 模板内建 `cloud-overlay/` 覆盖文件集（API 层 + 环境文件差异），`replace_ui.rs` 按 `template_dir == ruoyi-cloud` 时复制 overlay 覆盖：

1. `api/core/auth.ts` cloud 版：`POST /auth/login`（响应 `{access_token, expires_in}` → 适配为 vben 会话结构）、刷新 `/auth/refresh`?（核实官方 cloud 是否有刷新接口，无则复用重新登录语义）、登出 `DELETE /auth/logout`。
2. 用户信息与权限：`/system/user/getInfo`、`/system/menu/getRouters`（响应结构与分离版一致，路径前缀差异）。
3. 验证码：`/code`（cloud 路径）。
4. 环境文件：`VITE_GLOB_API_URL=/prod-api`（生产经 nginx 反代网关，见 3.9；开发 `/api` → vite proxy 指向 8080 网关）。
5. arco 模板同思路（已有双模板体系，overlay 各自维护）。

### 3.8 微服务模块裁剪（remove_modules）

执行为独立任务 `TaskType::TrimCloudModules`：

1. 删除模块目录（`ruoyi-modules/ruoyi-{gen|job|file}`、`ruoyi-visual`（monitor 时整目录，核实 monitor 独占 visual））。
2. 根 pom `<modules>` 中移除对应 `<module>` 声明；`ruoyi-visual/pom.xml` 若只剩 monitor 需同步处理。
3. Nacos 配置文本（3.2 引擎）：删除对应服务的 dev/prod 配置条目。
4. gateway 路由：删除对应服务路由（gateway 配置文本内）。
5. 业务库 SQL 种子：删除 gen/job 相关菜单与权限行（`sys_menu` 中定时任务/代码生成菜单；按菜单名锚点删除，复用 sql_customize 的行删除风格）；quartz 表块一并清理（job 被裁剪时联动 clean_quartz 语义）。
6. 前端 ruoyi-ui：菜单为运行时数据，前端不需改；vben/arco 无硬编码菜单，不需改。
7. validator：裁剪后校验全 pom 无被裁模块引用残留、nacos 配置无对应服务条目残留。

### 3.9 Nginx / 脚本

1. **Nginx**：cloud 变体模板（`templates/ruoyi-vue/nginx/cloud/` 或同目录 nginx.conf.cloud.tmpl）：upstream 指向 `gateway:8080`，`location /prod-api/` 反代网关（strip `/prod-api` 前缀——核实 cloud 前端请求路径是否带服务前缀经网关转发，以此定 rewrite 规则）；HTTPS 条件段复用现有 `{{#HTTPS}}` 机制。
2. **脚本**：cloud 版 start/stop（先检查 nacos 可达的提示逻辑、按服务顺序启动：gateway → auth → system → 其余；Windows .bat / Unix .sh 双份，风格复用 scripts.rs 现有模板）；run-ui/build 脚本复用；admin finalName 逻辑替换为各服务 jar 打包脚本。docker-compose 不做（超出范围，PR 说明）。

### 3.10 前端能力矩阵与参数页

1. `template-capabilities.ts`：cloud 从「仅禁 db_type」更新为完整支持表——禁用仅剩 `db_type`；`TEMPLATE_META` 的 cloud 描述更新为「微服务版（Spring Cloud + Nacos），已支持全功能改造」。
2. ParamConfig：cloud 时「安全 & SQL」区显示「配置库名」输入框 + 「裁剪微服务模块」多选框（gen/job/file/monitor）；单体 ruoyi 的禁用表不变。
3. `types/index.ts` 镜像新字段；配置导入导出脱敏清单不变（新字段非敏感）。

---

## 四、Spring Boot 2 / 3 / 4 三档适配点（Cloud 特有）

1. **直接复用**：`spring_boot_major` 检测、MP starter 三分支、redis 键位分支（cloud 场景在 3.2 引擎内同样按大版本处理）。
2. **需按分支核实的差异**（官方三分支拉取后逐一确认，写成代码注释留档）：
   - Nacos 客户端版本与 bootstrap 形态（Boot2: nacos 2.x + spring-cloud-starter-bootstrap；Boot3/4: SCA 2023.x/2025.x，是否改用 `spring.config.import` —— 若是，3.2 引擎的「bootstrap 完整性校验」按两种锚点兼容）；
   - gateway 白名单配置键名与结构（Boot2/3/4 的 gateway 配置文本差异）；
   - auth 返回结构（access_token 语义三档是否一致）；
   - sql 目录文件名（ry_config SQL 的命名随版本变化，解析入口按 `ry_config*.sql` 通配）。
3. **升级类任务不做**：不帮用户把 Boot2 cloud 项目升级到 Boot3/4（改造保持原 Boot 版本）。

---

## 五、实施前核实清单（从官方仓库拉取，不得凭猜测硬编码）

实施第一步：从 gitee.com/y_project/RuoYi-Cloud 拉取三分支（Boot2 的 3.6.x tag / Boot3 分支 / master(Boot4)），确认以下事实并写入代码注释（来源+日期）：

1. ry_config SQL 的 config_info INSERT 具体格式（转义规则、列顺序、dev/prod 条目组织）。
2. gateway 白名单配置键与路由表结构（三档分别记录）。
3. auth 登录/登出/刷新接口路径与响应字段。
4. system 服务配置文本中 token/redis/mybatis 配置的实际键位（三档）。
5. sql 目录文件命名与双库脚本划分。
6. ruoyi-ui 的 BASE_API 与请求路径前缀。
7. monitor 与 visual 的目录从属关系（裁剪边界）。
8. 根 pom 的 Boot/Cloud 版本属性写法（`spring-boot.version` 等锚点在三档的形态）。

若官方 Boot3/4 分支结构与预期差异过大（如配置不再走 nacos SQL 导入），停下并在 PR 中报告差异与调整建议，不强行实现。

## 六、明确不做

1. PG 方言对 cloud 开放（双库配置文本方言化复杂度高，留二期；`db_type` 在 cloud 保持禁用）。
2. docker-compose / k8s 部署产物。
3. Boot 版本升级迁移（改造保持源项目 Boot 版本）。
4. 模块裁剪不支持 gateway/auth/system/common/api（核心骨架）。
5. seata/sentinel 相关改造（保持源项目现状）。

## 七、实施顺序与提交拆分（建议）

1. `feat:Cloud识别与基础校验收口(任务规划/校验/logback多服务)` —— 3.1
2. `feat:Nacos配置定制引擎(ry_config解析改写与双库名)` —— 3.2 + 3.3
3. `feat:MyBatisPlus雪花LongID适配Cloud模块结构` —— 3.4
4. `feat:页脚ICP与站点设置走网关白名单架构` —— 3.5
5. `feat:UniApp与微信支付适配Cloud接口` —— 3.6
6. `feat:vben与arco新增cloud认证适配层` —— 3.7
7. `feat:Nginx与启动脚本Cloud变体` —— 3.9
8. `feat:微服务模块裁剪参数与执行` —— 3.8
9. `feat:参数页与能力矩阵支持Cloud全功能` —— 3.10
10. `test:Cloud三档Boot版本全功能集成测试` —— 补全

## 八、测试与验收

1. **单元测试**：nacos_config 引擎（SQL 转义/反转义往返、yaml 行级改写保留注释、多服务条目定位写回）；模块裁剪（pom modules 移除、菜单 SQL 行删除）；参数校验（remove_modules 非法值）。
2. **集成测试**：`src-tauri/tests/cloud_pipeline.rs`，fixture 按官方三分支真实结构构造（核实清单产物），Boot2/3/4 × {全功能开} × {裁剪 gen+job} 组合断言：双库库名、nacos 文本改写点、starter 落点、白名单、前端 overlay、裁剪残留。
3. **回归线**：现有全部测试不改断言通过（分离版/单体行为零变化）；`npm run typecheck` 通过。
4. **手工冒烟**：官方 3.6.x（Boot2）zip 全功能改造 → 导入双库 SQL 到 MySQL + 启动 nacos → 服务全起 → 登录正常、页脚/站点设置/生成器可用。Boot3/4 分支样本走相同清单（本地无 nacos 环境时至少完成 SQL/yaml/pom 级核对并在 PR 记录）。

## 九、全局约束

- 所有文件 UTF-8；中文注释正常显示；Commit Message 简体中文（Conventional Commits，冒号后不加空格）。
- 分离版（ruoyi-vue）与单体版（ruoyi）行为零回归；cloud 分支逻辑不得反向污染分离版路径。
- 涉及官方仓库结构的锚点（SQL 格式、白名单键、接口路径）必须先核实再写实现，核实结论以代码注释留档；核实不通时宁可报错也不静默产出错误内容。
- UI 模板 overlay 文件与主模板同步维护，差异点集中在 overlay 内，不整目录复制。
