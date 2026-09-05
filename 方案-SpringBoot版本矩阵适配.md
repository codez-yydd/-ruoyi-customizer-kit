# 方案：Spring Boot 2 / 3 / 4 版本矩阵适配

> 本方案基于提交 `6b01d06` 的代码现状编写（2026-09-05），供实施 Agent（Cursor）直接执行。
> 目标：锻造台对若依前后端分离版（RuoYi-Vue）的改造能力覆盖 Spring Boot 2.5（若依 3.8.x）/ 3.x（若依 3.9.x）/ 4.x（未来版本）三档，**且覆盖工具注入的全部新功能**（MyBatis-Plus、配置重构、雪花 ID、OSS、微信支付等），不只是配置文件改写。
> 生态事实（2026-09 已核实）：Spring Boot 4.0 已发布；MyBatis-Plus 自 3.5.13 起提供 `mybatis-plus-spring-boot4-starter`（最新 3.5.15），**Boot 4 下使用旧 starter 会因自动配置类失效而启动报错**（官方 issue baomidou/mybatis-plus#7009）。

---

## 一、现状盘点（均已核实到代码）

### 1.1 版本敏感点全景

| # | 位置 | 现状 | Boot 2 | Boot 3 | Boot 4 |
|---|------|------|--------|--------|--------|
| 1 | `src-tauri/src/core/mybatis_plus.rs:12` | `MP_VERSION = "3.5.7"` | ✅（boot starter 存在） | ✅ | ❌ **3.5.7 无 boot4 starter，注入后项目无法启动** |
| 2 | `src-tauri/src/core/mybatis_plus.rs:73-77` `select_starter()` | 两分支（boot2 / boot3+） | ✅ | ✅ | ❌ 检测到 4 会误选 boot3 starter |
| 3 | `src-tauri/src/core/config_rewrite.rs:743-745` | 生成的 dev/prod 硬编码 `spring.data.redis`（Boot 3 键位） | ❌ **键位错位**：Boot 2 绑定 `spring.redis`，模板 redis 配置全部不生效，静默回退默认值 | ✅ | ✅（Boot 4 延续 `spring.data.redis`，实施时官方文档复核） |
| 4 | `src-tauri/src/core/mybatis_plus.rs:26` `detect_boot_major_version()` | 已能识别 2/3（含 4.x 数字也会返回 4，天然前向兼容） | ✅ | ✅ | ✅ |
| 5 | `src-tauri/src/core/web_footer.rs:337` SecurityConfig 补丁 | 锚定 captchaImage 行，注释明确兼容 SB2 antMatchers / SB3 requestMatchers | ✅ | ✅ | ✅（Security 7 保留 requestMatchers） |
| 6 | 生成的 Java 模板（`templates/ruoyi-vue/java/*.tmpl`） | 纯 Spring 注解 + 若依自有类，无 javax/jakarta import | ✅ | ✅ | ✅ |
| 7 | `snowflake.rs:19` hutool-all 5.8.32 / `wechat.rs:18` wechatpay-java 0.2.17 / `oss.rs:67-70` 四家 OSS SDK | 纯 Java SDK，不依赖 Spring 自动装配（OssProperties 用标准 `@ConfigurationProperties`） | ✅ | ✅ | ✅ |
| 8 | logback 彩色注入、JWT、BCrypt、SQL 定制、Nginx、脚本 | 与 Boot 版本无关 | ✅ | ✅ | ✅ |

**结论**：需要改动的只有 #1/#2（MP starter 三分支 + 版本升级）和 #3（Redis 键位），其余能力天然三档兼容；另需把版本检测上移为全局信息（见 3.1），并补齐版本相关校验与展示。

### 1.2 现有版本检测的局限

`detect_boot_major_version(root)` 目前是 `mybatis_plus.rs` 的私有能力，只有 MP 依赖注入用到了。配置重构（#3）完全不知道 Boot 版本；识别结果 `ProjectInfo`（`core/mod.rs:471`）也不含版本信息，前端和报告无从展示。

---

## 二、总体设计

**一次检测、全局共享**：把 Boot 大版本检测上移为一等公民——识别阶段（detector）解析一次，存入 `ProjectInfo.spring_boot_major`，planner/executor/各 core 模块按需取用；`mybatis_plus.rs` 的原函数保留为内部实现（或迁移到 `core/detector.rs`，二选一，推荐迁移并在原处 re-export 保持测试兼容）。

---

## 三、改造项明细

### 3.1 版本检测上移（基础项，先行）

1. `detect_boot_major_version(root) -> Option<u32>` 从 `mybatis_plus.rs:26` 迁移到 `core/detector.rs` 并 `pub`；`mybatis_plus.rs` 内 `pub use` 或薄包装，**现有单测（mybatis_plus.rs:575-640 四个用例）保持原位且不改断言**。
2. `ProjectInfo`（`core/mod.rs:471`）新增字段：

```rust
/// 识别到的 Spring Boot 大版本（如 2 / 3 / 4）；未识别到为 None
#[serde(default)]
pub spring_boot_major: Option<u32>,
```

   `#[serde(default)]` 保证旧的前端 localStorage 持久化识别结果与旧配置 JSON 反序列化不报错。
3. `detector::detect()` 内部调用检测并填充该字段；`commands/execute.rs` 执行前重新 detect 时自然获得。
4. 前端镜像 `src/types/index.ts` 的 `ProjectInfo` 加 `spring_boot_major?: number`。

### 3.2 MyBatis-Plus：starter 三分支 + 版本升级（Boot 4 的关键项）

`mybatis_plus.rs` 改动：

1. `MP_VERSION` 从 `"3.5.7"` 升级为 `"3.5.15"`（该版本线同时提供三个 starter；实施时到 Maven Central 核实 `mybatis-plus-boot-starter` / `mybatis-plus-spring-boot3-starter` / `mybatis-plus-spring-boot4-starter` 三个 artifact 的 3.5.15 均存在，任一缺失则按实际存在的最高公共版本调整）。
2. `select_starter()` 改为三分支：

```rust
const MP_STARTER_BOOT4: &str = "mybatis-plus-spring-boot4-starter";

fn select_starter(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3  => MP_STARTER_BOOT2,   // 2.x
        Some(3)                   => MP_STARTER_BOOT3,   // 3.x
        _                         => MP_STARTER_BOOT4,   // >=4 及检测不到（默认现代版本）
    }
}
```

   默认兜底从「Boot 3」改为「Boot 4」的理由：检测不到版本的项目越来越罕见，兜底应跟随最新生态（与原注释「现代若依多为 Boot 3」同一逻辑的延续）；日志文案同步更新。
3. 幂等检查的 `dep_markers` 数组（mybatis_plus.rs:84）加入 boot4 starter 名——三个名字都视为「已有依赖」。
4. `add_dependency()` 的调用处：优先使用调用链传入的版本（executor 持有 `ProjectInfo`，将 `info.spring_boot_major` 显式传入；不要在函数内部重复扫 pom——迁移后函数签名加参数 `boot_major: Option<u32>`，executor 调用点回填，`mybatis_plus.rs` 内部自测路径传 None 时现场检测保持行为）。
5. 分页配置类 `MybatisPlusConfig.java`（mybatis_plus.rs:147）的 `PaginationInnerInterceptor` API 在三个 starter 间一致，**无需改动**，但需确认生成代码不变（现有单测覆盖）。

### 3.3 Redis 配置键位按版本生成（Boot 2 修复项）

`config_rewrite.rs`：

1. `build_standard_datasource_redis(db_name)`（:680）签名加参数 `boot_major: Option<u32>`，redis 段键位分支：

```text
boot_major == Some(2)            → spring.redis.{host,port,database,password,timeout,lettuce...}
boot_major 为 3/4/None           → spring.data.redis.{...}   （现状，保持不变）
```

   注意 Boot 2 分支只改键的层级路径（`spring.redis` 少一层 `data`），子项（lettuce 连接池等）内容不变；datasource 段两版键位相同，不动。
2. `rewrite()` 签名加 `boot_major: Option<u32>` 参数（executor 调用点从 `info.spring_boot_major` 传入）。
3. `extract_spring_runtime_children` 的 `ENV_CHILD_KEYS`（:353）已同时含 `redis:` 与 `data:`，两种原始形态都会被正确丢弃重建，**无需改动**（加注释说明该设计意图即可）。
4. `parse_master_db_name` / `db_name_from_url_line`（:604/:659）与 Boot 版本无关，不动。

### 3.4 执行后校验：新增版本一致性校验（validator.rs）

新增 `CheckItem`（仅当开启对应功能时校验）：

| 校验项 | 条件 | 级别 |
|--------|------|------|
| MP starter 与 Boot 大版本匹配 | `enable_mybatis_plus` | 失败级：Boot 4 项目 pom 中不得出现 boot2/boot3 starter 名；Boot 3 项目不得出现 boot2/boot4；Boot 2 同理（检测不到版本时跳过本项） |
| Redis 键位与 Boot 大版本匹配 | `enable_config_rewrite` | 失败级：Boot 2 项目 dev/prod yaml 中不得出现 `spring.data.redis`；Boot 3/4 项目不得出现 `spring.redis:` 直挂键 |

### 3.5 前端与报告展示

1. `ProjectDetect.vue` 识别结果区新增「Spring Boot 版本」行：`2.x` / `3.x` / `4.x` / 「未识别」（样式沿用现有字段行）。
2. `report.rs` 的「项目信息」节追加 `- Spring Boot 大版本：{2/3/4/未识别}`。
3. `Preview.vue` 高风险项：当检测到 Boot 4 且开启 MP 时，planner 在高风险清单中追加「Spring Boot 4 项目：已注入 mybatis-plus-spring-boot4-starter（3.5.15+），如遇自动装配问题参考官方 issue #7009」——提示性，不阻塞。

### 3.6 明确无需改动项（防止实施者画蛇添足）

- hutool-all 5.8.32、wechatpay-java 0.2.17、四家 OSS SDK、BCrypt、JWT：纯 SDK / 标准 Spring 注解，三档通用。
- `java/*.tmpl` 两个 Controller 模板：无 servlet 命名空间 import，三档通用。
- web_footer 的 SecurityConfig 补丁：已有 SB2/SB3 双形态测试。
- druid：依赖坐标由若依源项目自带（Boot 3 用 druid-spring-boot-3-starter），工具只写 `spring.datasource.druid.*` 配置键，键位由项目自带 starter 绑定，**不注入 druid 依赖，无需处理**。
- quartz、logback、generator VM 模板、Nginx、脚本：版本无关。

---

## 四、测试与验收

### 4.1 单元测试（新增，全部放对应模块 tests）

1. `select_starter` 三分支：Some(2)/Some(3)/Some(4)/None → 三个 starter 名。
2. `build_standard_datasource_redis` 键位：Some(2) 产出含 `spring.redis:` 且不含 `spring.data.redis`；Some(3)/Some(4)/None 产出含 `spring.data.redis`。
3. `detect_boot_major_version` 补 Boot 4 用例：`<spring-boot.version>4.0.0</spring-boot.version>` → Some(4)；parent 形式 `4.0.1` → Some(4)。
4. 旧 ProjectInfo JSON（无 spring_boot_major 字段）反序列化不报错且为 None。
5. validator 版本一致性：构造 Boot 4 + boot3 starter 残留 → 校验失败项。

### 4.2 集成测试

新增 `src-tauri/tests/boot_versions.rs`（仿 `e2e_pipeline.rs` fixture 构造方式），构造三个 fixture 项目（根 pom 仅 `<spring-boot.version>` 不同：2.5.15 / 3.5.14 / 4.0.0），各自开启 `enable_mybatis_plus + enable_config_rewrite` 执行后断言：

- Boot 2：pom 含 `mybatis-plus-boot-starter`、dev/prod 含 `spring.redis:`；
- Boot 3：pom 含 `mybatis-plus-spring-boot3-starter`、dev/prod 含 `spring.data.redis:`；
- Boot 4：pom 含 `mybatis-plus-spring-boot4-starter`、dev/prod 含 `spring.data.redis:`；
- 三者校验项全部通过、无对方 starter 名残留。

### 4.3 手工冒烟

1. `npm run tauri dev`，分别用 Boot 2（若依 3.8.x zip）与 Boot 3（若依 3.9.x zip）源项目走完整向导，识别页显示版本，改造后 `mvn compile` 通过（本机有 JDK 环境时）。
2. Boot 4 源项目暂无官方若依版本，可用「把 3.9.2 zip 的根 pom `spring-boot.version` 改为 4.0.0」的模拟项目验证注入与校验路径（**模拟项目仅用于验证工具行为，不保证能真实启动**——此限制写入任务日志提示）。

### 4.4 回归验收线

- 现有全部测试（`src-tauri/tests/` 16 个文件）不改断言全部通过。
- `npm run typecheck` 通过。

---

## 五、实施前需核实的外部事实（不得凭猜测硬编码）

1. Maven Central 上 3.5.15（或所选版本）的三个 starter artifact 是否齐全（见 3.2 第 1 条）。
2. Spring Boot 4 是否延续 `spring.data.redis` 键位（查官方 Spring Boot 4 配置文档；若已确认，删除 3.3 中的「实施时复核」注释）。
3. 若依官方是否已发布 Boot 4 版本及其根 pom 版本写法（影响 detect 锚点是否需要扩展；当前 `<spring-boot.version>` 属性 + parent 两种锚点已覆盖常规写法）。

## 六、明确不做

1. 不做 Boot 4 项目的真实启动验证（无官方若依 Boot 4 源，工具侧只保证注入内容与校验正确）。
2. 不主动升级 hutool / wechatpay-java / OSS SDK 版本（与 Boot 版本无关，避免无关回归）。
3. 不处理 ruoyi（单体）与 ruoyi-cloud 模板的版本差异（它们与 ruoyi-vue 共用同一套 MP/config_rewrite 代码路径，改动自动受益，但不为它们新增专属逻辑）。
4. 不做 MP 3.5.7 → 3.5.15 的行为差异兜底（如分页 API 微调）——如集成测试暴露问题，先在 PR 中记录再决定。

## 七、建议提交拆分

1. `feat:识别结果新增SpringBoot大版本字段并上移检测逻辑`
2. `fix:配置重构按Boot版本生成redis键位修复Boot2失效`
3. `feat:MyBatis-Plus支持Boot4并升级starter三分支选型`
4. `feat:执行后校验新增starter与redis键位版本一致性检查`
5. `feat:识别页与执行报告展示SpringBoot版本`

## 八、全局约束

- 所有文件 UTF-8；中文注释正常显示；Commit Message 简体中文（Conventional Commits，冒号后不加空格）。
- 不重构无关代码；现有测试断言一律不改；GUI 既有流程零变化（仅识别页加一行展示）。
- 涉及外部坐标与配置键的事实必须先核实再写入代码，核实结论以代码注释形式留档（注明来源与日期）。
