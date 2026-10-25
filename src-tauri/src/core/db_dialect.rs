// 数据库方言元数据与切换任务。
//
// 集中定义 MySQL / PostgreSQL 连接、驱动、分页方言，避免字符串散落。
// SwitchDatabaseDialect 任务职责：pom 驱动替换、初始化脚本替换、代码生成器 mapper 适配、
// PageHelper helperDialect 切换（单体/分离版）。
// 三个模板（ruoyi-vue / ruoyi / ruoyi-cloud）均支持 PostgreSQL：
// - vue / 单体：数据源 YAML 由 config_rewrite 按方言直接生成；
// - Cloud：数据源落在 Nacos ry_config SQL，由 nacos_config 按方言改写（不走本任务）。

use crate::core::CustomizeParams;
use crate::utils::file::read_text;
use std::path::Path;

/// 一种受支持的数据库方言。
#[derive(Debug, Clone, Copy)]
pub struct DbDialect {
    pub name: &'static str,
    pub display_name: &'static str,
    pub driver_class: &'static str,
    pub url_scheme: &'static str,
    pub default_port: u16,
    pub url_params: &'static str,
    pub validation_query: &'static str,
    pub mp_db_type: &'static str,
    pub pom_group_id: &'static str,
    pub pom_artifact_id: &'static str,
}

const MYSQL: DbDialect = DbDialect {
    name: "mysql",
    display_name: "MySQL",
    driver_class: "com.mysql.cj.jdbc.Driver",
    url_scheme: "jdbc:mysql",
    default_port: 3306,
    url_params: "useUnicode=true&characterEncoding=utf8&zeroDateTimeBehavior=convertToNull&useSSL=true&serverTimezone=GMT%2B8",
    validation_query: "SELECT 1 FROM DUAL",
    mp_db_type: "MYSQL",
    pom_group_id: "com.mysql",
    pom_artifact_id: "mysql-connector-j",
};

const POSTGRESQL: DbDialect = DbDialect {
    name: "postgresql",
    display_name: "PostgreSQL",
    driver_class: "org.postgresql.Driver",
    url_scheme: "jdbc:postgresql",
    default_port: 5432,
    url_params: "currentSchema=public",
    validation_query: "SELECT 1",
    mp_db_type: "POSTGRE_SQL",
    pom_group_id: "org.postgresql",
    pom_artifact_id: "postgresql",
};

/// 按名称取方言。仅 mysql / postgresql。
pub fn from_name(name: &str) -> Option<&'static DbDialect> {
    match name.trim().to_ascii_lowercase().as_str() {
        "mysql" => Some(&MYSQL),
        "postgresql" | "postgres" | "pg" => Some(&POSTGRESQL),
        _ => None,
    }
}

/// 从改造参数取方言，非法值回退 MySQL（validate 已拦截非法值）。
pub fn from_params(params: &CustomizeParams) -> &'static DbDialect {
    from_name(&params.db_type).unwrap_or(&MYSQL)
}

/// 是否为 PostgreSQL 方言。
pub fn is_postgresql(params: &CustomizeParams) -> bool {
    from_params(params).name == "postgresql"
}

/// 执行数据库方言切换（pom / SQL 资产 / 生成器 mapper / PageHelper 方言）。
/// `template_dir`：识别出的模板目录名（ruoyi-vue / ruoyi / ruoyi-cloud），决定 SQL 资产取哪套。
pub fn switch(
    root: &Path,
    template_dir: &str,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<SwitchOutcome, String> {
    let dialect = from_params(params);
    if dialect.name == "mysql" {
        return Ok(SwitchOutcome {
            modified_files: 0,
            created_files: 0,
            summary: vec!["当前为 MySQL，无需切换方言".into()],
        });
    }

    let mut summary = Vec::new();
    let mut modified = 0usize;
    let mut created = 0usize;

    let pom_n = replace_pom_driver(root, dialect, log)?;
    modified += pom_n;
    if pom_n > 0 {
        summary.push(format!(
            "已替换 pom 驱动为 {}:{}（{} 个文件，每个原 MySQL 声明处各插入一份 PG 驱动）",
            dialect.pom_group_id, dialect.pom_artifact_id, pom_n
        ));
    }

    match replace_sql_scripts(root, template_dir, log) {
        Ok((n_mod, n_new, msg)) => {
            modified += n_mod;
            created += n_new;
            summary.push(msg);
        }
        Err(e) => return Err(e),
    }

    let mapper_n = adapt_generator_mappers(root, log)?;
    modified += mapper_n;
    if mapper_n > 0 {
        summary.push(format!("已适配代码生成器 mapper（{mapper_n} 个文件）"));
    }

    let ph_n = switch_pagehelper_dialect(root, log);
    modified += ph_n;
    if ph_n > 0 {
        summary.push(format!("已切换 PageHelper 分页方言 helperDialect → postgresql（{ph_n} 个文件）"));
    } else {
        // Cloud 官方 Nacos 配置不显式配置 pagehelper（PageHelper 按 JDBC url 自动探测方言），
        // 找不到配置属于正常情况；vue / 单体的 application yml 均应命中。
        log("未找到 pagehelper.helperDialect 配置（Cloud 官方配置依赖 url 自动探测，跳过）");
    }

    log("集群模式下建议设置 org.quartz.impl.jdbcjobstore.PostgreSQLDelegate");
    summary.push(
        "集群模式下建议设置 org.quartz.impl.jdbcjobstore.PostgreSQLDelegate（当前 ScheduleConfig 未显式配置 Delegate，未改动）".into(),
    );

    Ok(SwitchOutcome {
        modified_files: modified,
        created_files: created,
        summary,
    })
}

/// 方言切换结果
pub struct SwitchOutcome {
    pub modified_files: usize,
    pub created_files: usize,
    pub summary: Vec<String>,
}

// ---------- pom 驱动替换 ----------

const MYSQL_ARTIFACTS: &[&str] = &["mysql-connector-j", "mysql-connector-java"];

/// 定位全部 pom 中的 MySQL 驱动 `<dependency>` 块并删除，再在每个原声明处插入 PG 驱动。
///
/// 官方三模板的驱动声明位置：vue / 单体仅 `*-admin/pom.xml` 一处；
/// Cloud 在 system / gen / job 三个模块 pom 各声明一份——逐 pom 插入保证
/// 每个连库微服务都拿到 PG 驱动（vue / 单体仅一处声明，行为不变）。
fn replace_pom_driver(
    root: &Path,
    dialect: &DbDialect,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let poms = collect_poms(root);
    if poms.is_empty() {
        log("未找到 pom.xml，跳过驱动替换");
        return Ok(0);
    }

    let pg_block = format!(
        "        <dependency>\n            <groupId>{}</groupId>\n            <artifactId>{}</artifactId>\n        </dependency>\n",
        dialect.pom_group_id, dialect.pom_artifact_id
    );

    let mut modified = 0usize;
    let mut inserted = false;

    for pom in &poms {
        let Some(content) = read_text(pom) else {
            continue;
        };
        let (new_content, removed) = strip_mysql_driver_blocks(&content);
        if removed == 0 {
            continue;
        }
        // 在每个被删块的原声明处插入 PG 驱动（dependencies 内靠前）。
        // Cloud 多模块各自持有驱动依赖，逐 pom 插入；vue / 单体仅一处声明，行为不变。
        inserted = true;
        let written = insert_dependency_block(&new_content, &pg_block);
        std::fs::write(pom, written)
            .map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
        modified += 1;
        log(&format!(
            "已从 {} 移除 MySQL 驱动并写入 PostgreSQL 驱动",
            pom.display()
        ));
    }

    if !inserted {
        log("未在 pom 中找到 mysql-connector-j / mysql-connector-java，仍尝试插入到 admin 模块");
        if let Some(admin) = find_admin_pom(root, &poms) {
            let Some(content) = read_text(&admin) else {
                return Ok(modified);
            };
            if content.contains(&format!("<artifactId>{}</artifactId>", dialect.pom_artifact_id)) {
                log("admin 模块已含 PostgreSQL 驱动，跳过插入");
                return Ok(modified);
            }
            let written = insert_dependency_block(&content, &pg_block);
            std::fs::write(&admin, written)
                .map_err(|e| format!("写入 {} 失败：{e}", admin.display()))?;
            modified += 1;
            log(&format!("已在 {} 插入 PostgreSQL 驱动", admin.display()));
        }
    }

    Ok(modified)
}

/// 删除包含 MySQL 驱动 artifact 的整个 `<dependency>` 块。
fn strip_mysql_driver_blocks(content: &str) -> (String, usize) {
    let re = match regex::Regex::new(
        r"(?s)[ \t]*<dependency>\s*<groupId>[\w.]+</groupId>\s*<artifactId>(mysql-connector-j|mysql-connector-java)</artifactId>.*?</dependency>\s*",
    ) {
        Ok(r) => r,
        Err(_) => return (content.to_string(), 0),
    };
    let count = re.find_iter(content).count();
    if count == 0 {
        return (content.to_string(), 0);
    }
    (re.replace_all(content, "").to_string(), count)
}

fn insert_dependency_block(content: &str, block: &str) -> String {
    if let Some(idx) = content.find("<dependencies>") {
        let mut s = String::with_capacity(content.len() + block.len());
        s.push_str(&content[..idx + "<dependencies>".len()]);
        s.push('\n');
        s.push_str(block);
        s.push_str(&content[idx + "<dependencies>".len()..]);
        s
    } else {
        content.replace(
            "</project>",
            &format!("    <dependencies>\n{block}    </dependencies>\n</project>"),
        )
    }
}

fn find_admin_pom(root: &Path, poms: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    poms.iter()
        .find(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().ends_with("-admin"))
                .unwrap_or(false)
        })
        .cloned()
        .or_else(|| {
            if let Ok(entries) = std::fs::read_dir(root) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with("-admin") {
                        let p = e.path().join("pom.xml");
                        if p.is_file() {
                            return Some(p);
                        }
                    }
                }
            }
            None
        })
}

fn collect_poms(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !matches!(
                    name.as_ref(),
                    "target" | "node_modules" | ".git" | ".idea" | "dist"
                )
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if path.is_file() && path.file_name().map(|n| n == "pom.xml").unwrap_or(false) {
            out.push(path.to_path_buf());
        }
    }
    out
}

/// 扫描全部 pom 是否仍含 MySQL 驱动坐标（校验用）。
pub fn poms_still_have_mysql_driver(root: &Path) -> Vec<String> {
    let mut hits = Vec::new();
    for pom in collect_poms(root) {
        let Some(content) = read_text(&pom) else {
            continue;
        };
        if MYSQL_ARTIFACTS.iter().any(|a| content.contains(a)) {
            hits.push(pom.to_string_lossy().to_string());
        }
    }
    hits
}

// ---------- SQL 资产替换 ----------

/// 解析某模板的 PG 资产路径。`kind` 为文件名（ry.sql / quartz.sql）。
/// quartz 三模板表结构一致（官方三仓 quartz.sql 逐字节相同），共用 ruoyi-vue 资产；
/// ry.sql 各模板种子数据不同（Cloud 有 Sentinel/Nacos 控制台菜单、单体多 sys_user_online），各自维护。
fn resolve_pg_asset(template_dir: &str, kind: &str) -> std::path::PathBuf {
    let tpl = template_dir.trim();
    let primary = crate::core::paths::resolve(&format!("templates/{tpl}/sql/postgresql/{kind}"));
    if primary.is_file() {
        return primary;
    }
    crate::core::paths::resolve(&format!("templates/ruoyi-vue/sql/postgresql/{kind}"))
}

/// 把项目 sql/ 下 MySQL 版 ry_*.sql、quartz.sql 备份为 *.mysql.sql.bak，再复制 PG 版。
///
/// Cloud 排除 `ry_config*`（Nacos 配置库脚本，保持 MySQL）与 `ry_seata*`（Seata Server
/// 自身存储库，不属于业务库切换范围）。
fn replace_sql_scripts(
    root: &Path,
    template_dir: &str,
    log: &dyn Fn(&str),
) -> Result<(usize, usize, String), String> {
    let sql_dir = root.join("sql");
    if !sql_dir.is_dir() {
        std::fs::create_dir_all(&sql_dir).map_err(|e| format!("创建 sql 目录失败：{e}"))?;
    }

    let ry_src = resolve_pg_asset(template_dir, "ry.sql");
    let quartz_src = resolve_pg_asset(template_dir, "quartz.sql");
    if !ry_src.is_file() && !quartz_src.is_file() {
        return Err(format!(
            "请自行准备 PostgreSQL 初始化脚本（内置资产缺失：templates/{tpl}/sql/postgresql/）",
            tpl = template_dir.trim()
        ));
    }

    let mut backed = 0usize;
    let mut created = 0usize;

    if let Ok(entries) = std::fs::read_dir(&sql_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if !path.is_file() {
                continue;
            }
            let lower = name.to_ascii_lowercase();
            // Cloud 专属脚本保持 MySQL：Nacos 配置库 / Seata Server 存储库不随业务库切换
            if lower.starts_with("ry_config") || lower.starts_with("ry_seata") {
                log(&format!("保持 MySQL 不转换（非业务库脚本）：{name}"));
                continue;
            }
            let is_ry = name.starts_with("ry") && name.ends_with(".sql") && !name.contains(".mysql.");
            let is_quartz = name == "quartz.sql";
            if !is_ry && !is_quartz {
                continue;
            }
            let bak_name = if let Some(stem) = name.strip_suffix(".sql") {
                format!("{stem}.mysql.sql.bak")
            } else {
                format!("{name}.mysql.sql.bak")
            };
            let bak = sql_dir.join(&bak_name);
            std::fs::rename(&path, &bak)
                .map_err(|e| format!("备份 {} 失败：{e}", path.display()))?;
            backed += 1;
            log(&format!("已备份 MySQL 脚本：{name} → {bak_name}"));
        }
    }

    if ry_src.is_file() {
        let dest = sql_dir.join("ry.sql");
        std::fs::copy(&ry_src, &dest).map_err(|e| format!("复制 PG ry.sql 失败：{e}"))?;
        created += 1;
        log(&format!(
            "已写入 PostgreSQL 初始化脚本 sql/ry.sql（模板资产：{}）",
            ry_src.display()
        ));
    }
    if quartz_src.is_file() {
        let dest = sql_dir.join("quartz.sql");
        std::fs::copy(&quartz_src, &dest).map_err(|e| format!("复制 PG quartz.sql 失败：{e}"))?;
        created += 1;
        log("已写入 PostgreSQL quartz 脚本 sql/quartz.sql");
    }

    if created == 0 {
        return Err("请自行准备 PostgreSQL 初始化脚本".into());
    }
    Ok((
        backed,
        created,
        format!("已替换 PostgreSQL 初始化脚本（备份 {backed} 个 MySQL 脚本，写入 {created} 个 PG 脚本）"),
    ))
}

// ---------- PageHelper 分页方言 ----------

/// 把 application*.yml / application*.yaml 中的 `pagehelper.helperDialect: mysql` 切为 postgresql。
///
/// 官方落点：vue / 单体的 `application.yml`（配置重构后为 `application.yaml`）显式配置
/// helperDialect=mysql，PG 下必须切换，否则 PageHelper 生成 MySQL 方言 LIMIT 语法；
/// Cloud 官方 Nacos 配置不显式配置 pagehelper（按 JDBC url 自动探测），通常无命中。
/// 仅 PG 模式调用；MySQL 模式零改动。
fn switch_pagehelper_dialect(root: &Path, log: &dyn Fn(&str)) -> usize {
    // 行尾处理（CRLF 兼容）：多行模式下 `$` 只匹配 `\n` 之前，CRLF 文件行尾的
    // `\r` 若不被模式吞掉，整行不命中（PG 方言静默漏切）。`\r` 必须放进捕获组 2
    // 内——replace_all 替换的是整个匹配串（含 `\r`），放捕获组外会被闭包输出
    // 丢弃、行尾 `\r` 意外丢失；注释用 `[^\r\n]*` 避免 `.` 吞 `\r` 后错位。
    // LF 文件 `\r?` 匹配空串，行为与原先完全一致。
    let re = regex::Regex::new(
        r"(?im)^([ \t]*helperDialect[ \t]*:[ \t]*)mysql([ \t]*(?:#[^\r\n]*)?\r?)$",
    )
    .unwrap();
    let mut n = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !matches!(
                    name.as_ref(),
                    "target" | "node_modules" | ".git" | ".idea" | "dist"
                )
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_app_yml = (name.starts_with("application") || name.starts_with("bootstrap"))
            && (name.ends_with(".yml") || name.ends_with(".yaml"));
        if !is_app_yml {
            continue;
        }
        let Some(content) = read_text(path) else {
            continue;
        };
        if !re.is_match(&content) {
            continue;
        }
        let new_content = re
            .replace_all(&content, |caps: &regex::Captures| {
                format!("{}postgresql{}", &caps[1], &caps[2])
            })
            .to_string();
        if std::fs::write(path, &new_content).is_ok() {
            n += 1;
            log(&format!("已切换 PageHelper 分页方言：{}", path.display()));
        }
    }
    n
}

// ---------- 代码生成器 mapper 适配 ----------

/// 整段替换 4 个 information_schema 查询，并替换 sysdate / date_format。
fn adapt_generator_mappers(root: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let targets = find_generator_mappers(root);
    if targets.is_empty() {
        log("未找到 GenTableMapper.xml / GenTableColumnMapper.xml，跳过生成器适配");
        return Ok(0);
    }
    let mut n = 0usize;
    for path in targets {
        let Some(content) = read_text(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut new_content = content.clone();
        let mut changed = false;
        let mut warns = Vec::new();

        if name == "GenTableMapper.xml" {
            for (id, sql) in [
                ("selectDbTableList", PG_SELECT_DB_TABLE_LIST),
                ("selectDbTableListByNames", PG_SELECT_DB_TABLE_LIST_BY_NAMES),
                ("selectTableByName", PG_SELECT_TABLE_BY_NAME),
            ] {
                match replace_select_block(&new_content, id, sql) {
                    Some(next) => {
                        new_content = next;
                        changed = true;
                    }
                    None => {
                        warns.push(format!("未找到 <select id=\"{id}\">，跳过"));
                    }
                }
            }
        } else if name == "GenTableColumnMapper.xml" {
            match replace_select_block(&new_content, "selectDbTableColumnsByName", PG_SELECT_DB_COLUMNS) {
                Some(next) => {
                    new_content = next;
                    changed = true;
                }
                None => warns.push("未找到 <select id=\"selectDbTableColumnsByName\">，跳过".into()),
            }
        }

        let after_sysdate = new_content.replace("sysdate()", "now()");
        if after_sysdate != new_content {
            new_content = after_sysdate;
            changed = true;
        }
        let after_df = new_content.replace(
            "date_format(create_time,'%Y%m%d')",
            "to_char(create_time,'YYYYMMDD')",
        );
        if after_df != new_content {
            new_content = after_df;
            changed = true;
        }

        for w in &warns {
            log(&format!("⚠️ {}：{w}", path.display()));
        }
        if changed {
            std::fs::write(&path, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            n += 1;
            log(&format!("已适配生成器 mapper：{}", path.display()));
        }
    }
    Ok(n)
}

fn find_generator_mappers(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !matches!(
                    name.as_ref(),
                    "target" | "node_modules" | ".git" | ".idea" | "dist"
                )
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name == "GenTableMapper.xml" || name == "GenTableColumnMapper.xml" {
            out.push(path.to_path_buf());
        }
    }
    out
}

/// 用新 SQL 整段替换 `<select id="...">...</select>`。找不到返回 None。
fn replace_select_block(content: &str, select_id: &str, new_inner: &str) -> Option<String> {
    let open = format!("<select id=\"{select_id}\"");
    let start = content.find(&open)?;
    let after_open = start + open.len();
    let tag_end = content[after_open..].find('>')? + after_open + 1;
    let close = "</select>";
    let close_rel = content[tag_end..].find(close)?;
    let close_at = tag_end + close_rel;
    let mut out = String::with_capacity(content.len() + new_inner.len());
    out.push_str(&content[..tag_end]);
    out.push('\n');
    out.push_str(new_inner);
    if !new_inner.ends_with('\n') {
        out.push('\n');
    }
    out.push('\t');
    out.push_str(&content[close_at..]);
    Some(out)
}

const PG_SELECT_DB_TABLE_LIST: &str = r#"		select table_name, table_comment, create_time, update_time from (
		  select c.relname as table_name,
		         obj_description(c.oid) as table_comment,
		         current_timestamp as create_time,
		         current_timestamp as update_time
		  from pg_class c
		  left join pg_namespace n on n.oid = c.relnamespace
		  where c.relkind = 'r' and n.nspname = 'public'
		) t
		where table_name not like 'qrtz_%' and table_name not like 'gen_%'
		AND table_name NOT IN (select table_name from gen_table)
		<if test="tableName != null and tableName != ''">
			AND lower(table_name) like lower(concat('%', #{tableName}, '%'))
		</if>
		<if test="tableComment != null and tableComment != ''">
			AND lower(table_comment) like lower(concat('%', #{tableComment}, '%'))
		</if>
		<if test="params.beginTime != null and params.beginTime != ''"><!-- 开始时间检索 -->
			AND to_char(create_time,'YYYYMMDD') &gt;= to_char(#{params.beginTime}::timestamp,'YYYYMMDD')
		</if>
		<if test="params.endTime != null and params.endTime != ''"><!-- 结束时间检索 -->
			AND to_char(create_time,'YYYYMMDD') &lt;= to_char(#{params.endTime}::timestamp,'YYYYMMDD')
		</if>
        order by create_time desc
"#;

const PG_SELECT_DB_TABLE_LIST_BY_NAMES: &str = r#"		select table_name, table_comment, create_time, update_time from (
		  select c.relname as table_name,
		         obj_description(c.oid) as table_comment,
		         current_timestamp as create_time,
		         current_timestamp as update_time
		  from pg_class c
		  left join pg_namespace n on n.oid = c.relnamespace
		  where c.relkind = 'r' and n.nspname = 'public'
		) t
		where table_name not like 'qrtz_%' and table_name not like 'gen_%'
		and table_name in
	    <foreach collection="array" item="name" open="(" separator="," close=")">
 			#{name}
        </foreach>
"#;

const PG_SELECT_TABLE_BY_NAME: &str = r#"		select table_name, table_comment, create_time, update_time from (
		  select c.relname as table_name,
		         obj_description(c.oid) as table_comment,
		         current_timestamp as create_time,
		         current_timestamp as update_time
		  from pg_class c
		  left join pg_namespace n on n.oid = c.relnamespace
		  where c.relkind = 'r' and n.nspname = 'public'
		) t
		where coalesce(table_comment, '') &lt;&gt; ''
		and table_name = #{tableName}
"#;

const PG_SELECT_DB_COLUMNS: &str = r#"		select
		  a.attname as column_name,
		  (case when (a.attnotnull and not exists (
		     select 1 from pg_constraint x where x.conrelid = a.attrelid and a.attnum = any(x.conkey) and x.contype = 'p'
		  )) then '1' else '0' end) as is_required,
		  (case when exists (
		     select 1 from pg_constraint x where x.conrelid = a.attrelid and a.attnum = any(x.conkey) and x.contype = 'p'
		  ) then '1' else '0' end) as is_pk,
		  a.attnum as sort,
		  col_description(a.attrelid, a.attnum) as column_comment,
		  (case when a.attidentity in ('a','d') then '1' else '0' end) as is_increment,
		  format_type(a.atttypid, a.atttypmod) as column_type
		from pg_class c
		join pg_namespace n on n.oid = c.relnamespace
		join pg_attribute a on a.attrelid = c.oid
		where n.nspname = 'public' and c.relname = #{tableName} and a.attnum > 0 and not a.attisdropped
		order by a.attnum
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_accepts_aliases() {
        assert_eq!(from_name("mysql").unwrap().mp_db_type, "MYSQL");
        assert_eq!(from_name("PostgreSQL").unwrap().driver_class, "org.postgresql.Driver");
        assert!(from_name("oracle").is_none());
    }

    #[test]
    fn strip_mysql_connector_j_block() {
        let pom = r#"<dependencies>
        <dependency>
            <groupId>com.mysql</groupId>
            <artifactId>mysql-connector-j</artifactId>
        </dependency>
        <dependency>
            <groupId>com.example</groupId>
            <artifactId>keep-me</artifactId>
        </dependency>
    </dependencies>"#;
        let (out, n) = strip_mysql_driver_blocks(pom);
        assert_eq!(n, 1);
        assert!(!out.contains("mysql-connector-j"));
        assert!(out.contains("keep-me"));
    }

    /// 批次 G：PostgreSQL 已放开到全部模板（ruoyi-vue / ruoyi / ruoyi-cloud）。
    /// 原拦截函数已删除，规划与执行侧不再有模板门槛。
    #[test]
    fn postgresql_supported_for_all_templates() {
        for tpl in ["ruoyi-vue", "ruoyi", "ruoyi-cloud"] {
            let mut params = CustomizeParams::default();
            params.db_type = "postgresql".into();
            assert!(
                is_postgresql(&params),
                "{tpl} + postgresql 应视为 PG 方言"
            );
        }
        let mysql = CustomizeParams::default();
        assert!(!is_postgresql(&mysql));
    }

    /// 批次 G：PageHelper helperDialect 切换（PG 模式专用）
    #[test]
    fn switch_pagehelper_dialect_rewrites_mysql_value() {
        let dir = tempfile::tempdir().unwrap();
        let res = dir.path().join("src/main/resources");
        std::fs::create_dir_all(&res).unwrap();
        std::fs::write(
            res.join("application.yml"),
            "pagehelper:\n  helperDialect: mysql\n  supportMethodsArguments: true\n",
        )
        .unwrap();
        let n = switch_pagehelper_dialect(dir.path(), &|_| {});
        assert_eq!(n, 1);
        let out = std::fs::read_to_string(res.join("application.yml")).unwrap();
        assert!(out.contains("helperDialect: postgresql"), "{out}");
        assert!(!out.contains("mysql"));
    }

    /// P2-8 回归：CRLF 行尾的 `helperDialect: mysql\r\n` 必须被切换且 `\r` 原位保留
    ///（多行模式 `$` 只匹配 `\n` 前，行尾锚需显式吞 `\r`，否则整行不命中静默漏切）
    #[test]
    fn switch_pagehelper_dialect_handles_crlf_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let res = dir.path().join("src/main/resources");
        std::fs::create_dir_all(&res).unwrap();
        // Windows 官方源码常见的 CRLF 行尾（带行尾注释与纯值两种形态）
        let content = "pagehelper:\r\n  helperDialect: mysql\r\n  helperDialect: mysql # 分页方言\r\n  supportMethodsArguments: true\r\n";
        std::fs::write(res.join("application.yml"), content).unwrap();

        let n = switch_pagehelper_dialect(dir.path(), &|_| {});
        assert_eq!(n, 1, "计数按文件计，单文件两处命中仍为 1");
        let out = std::fs::read_to_string(res.join("application.yml")).unwrap();
        assert!(
            out.contains("helperDialect: postgresql\r\n"),
            "纯值行应切换且 CRLF 保留：{out:?}"
        );
        assert!(
            out.contains("helperDialect: postgresql # 分页方言\r\n"),
            "带注释行应切换且注释与 CRLF 保留：{out:?}"
        );
        assert!(!out.contains("mysql"));
        // 其余行不受影响
        assert!(out.starts_with("pagehelper:\r\n"));
        assert!(out.contains("supportMethodsArguments: true\r\n"));
    }

    /// 批次 G：Cloud 官方无 pagehelper 配置时切换零命中（不算错误）
    #[test]
    fn switch_pagehelper_dialect_no_config_is_zero() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sql")).unwrap();
        assert_eq!(switch_pagehelper_dialect(dir.path(), &|_| {}), 0);
    }

    /// 批次 G：SQL 资产替换按模板取 ry.sql，且 Cloud 的 ry_config / ry_seata 保持不动
    #[test]
    fn replace_sql_scripts_keeps_cloud_config_and_seata() {
        let dir = tempfile::tempdir().unwrap();
        let sql = dir.path().join("sql");
        std::fs::create_dir_all(&sql).unwrap();
        std::fs::write(sql.join("ry_20260417.sql"), "create database `ry-cloud`;\n").unwrap();
        std::fs::write(
            sql.join("ry_config_20260918.sql"),
            "CREATE DATABASE `ry-config`;\nUSE `ry-config`;\n",
        )
        .unwrap();
        std::fs::write(
            sql.join("ry_seata_20210128.sql"),
            "CREATE DATABASE `ry-seata`;\n",
        )
        .unwrap();
        let (backed, created, _) =
            replace_sql_scripts(dir.path(), "ruoyi-cloud", &|_| {}).unwrap();
        assert_eq!(backed, 1, "仅业务库 ry_*.sql 应被备份");
        assert_eq!(created, 2, "应写入 PG ry.sql + quartz.sql（quartz 走共享回落资产）");
        assert!(sql.join("ry_20260417.mysql.sql.bak").is_file());
        assert!(
            sql.join("ry_config_20260918.sql").is_file(),
            "ry_config 必须保持 MySQL 原样"
        );
        assert!(
            sql.join("ry_seata_20210128.sql").is_file(),
            "ry_seata 必须保持原样"
        );
        let cfg = std::fs::read_to_string(sql.join("ry_config_20260918.sql")).unwrap();
        assert!(cfg.contains("`ry-config`"), "ry_config 内容不得被改写：{cfg}");
        let ry = std::fs::read_to_string(sql.join("ry.sql")).unwrap();
        assert!(
            ry.contains("-- RuoYi-Cloud PostgreSQL 初始化脚本"),
            "应取 Cloud 专属资产：{}",
            ry.lines().take(3).collect::<Vec<_>>().join(" / ")
        );
    }

    /// 批次 G：单体模板取 ruoyi 专属资产
    #[test]
    fn replace_sql_scripts_uses_mono_asset() {
        let dir = tempfile::tempdir().unwrap();
        let sql = dir.path().join("sql");
        std::fs::create_dir_all(&sql).unwrap();
        std::fs::write(sql.join("ry_20260319.sql"), "create table sys_user_online...").unwrap();
        let (_, created, _) = replace_sql_scripts(dir.path(), "ruoyi", &|_| {}).unwrap();
        assert_eq!(created, 2, "ry.sql + quartz.sql（共享资产）");
        let ry = std::fs::read_to_string(sql.join("ry.sql")).unwrap();
        assert!(ry.contains("-- RuoYi PostgreSQL 初始化脚本"), "应取单体资产");
        assert!(
            ry.contains("sys_user_online"),
            "单体资产应含 sys_user_online（Shiro 在线用户表）"
        );
    }

    /// 批次 G：vue 模板行为不变（仍取 ruoyi-vue 资产）
    #[test]
    fn replace_sql_scripts_vue_asset_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let sql = dir.path().join("sql");
        std::fs::create_dir_all(&sql).unwrap();
        std::fs::write(sql.join("ry_20260417.sql"), "-- x").unwrap();
        let (_, created, _) = replace_sql_scripts(dir.path(), "ruoyi-vue", &|_| {}).unwrap();
        assert_eq!(created, 2, "ry.sql + quartz.sql（vue 资产直取）");
        let ry = std::fs::read_to_string(sql.join("ry.sql")).unwrap();
        assert!(ry.contains("-- RuoYi-Vue PostgreSQL 初始化脚本"));
    }

    /// 批次 G：Cloud 三个模块 pom 各有 MySQL 驱动时，逐 pom 插入 PG 驱动
    #[test]
    fn replace_pom_driver_inserts_pg_into_every_removed_pom() {
        let dir = tempfile::tempdir().unwrap();
        let mk = |m: &str| {
            let p = dir.path().join(format!("ruoyi-modules/ruoyi-{m}"));
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(
                p.join("pom.xml"),
                "<project><dependencies>\n        <dependency>\n            <groupId>com.mysql</groupId>\n            <artifactId>mysql-connector-j</artifactId>\n        </dependency>\n</dependencies></project>\n",
            )
            .unwrap();
        };
        for m in ["system", "gen", "job"] {
            mk(m);
        }
        let n = replace_pom_driver(dir.path(), &POSTGRESQL, &|_| {}).unwrap();
        assert_eq!(n, 3, "三个模块 pom 都应被改写");
        for m in ["system", "gen", "job"] {
            let pom = std::fs::read_to_string(
                dir.path().join(format!("ruoyi-modules/ruoyi-{m}/pom.xml")),
            )
            .unwrap();
            assert!(pom.contains("<artifactId>postgresql</artifactId>"), "{m} 应含 PG 驱动：{pom}");
            assert!(!pom.contains("mysql-connector-j"), "{m} 不应残留 MySQL 驱动");
        }
    }
}
