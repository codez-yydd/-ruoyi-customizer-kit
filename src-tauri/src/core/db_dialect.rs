// 数据库方言元数据与切换任务。
//
// 集中定义 MySQL / PostgreSQL 连接、驱动、分页方言，避免字符串散落。
// SwitchDatabaseDialect 任务职责：pom 驱动替换、初始化脚本替换、代码生成器 mapper 适配。
// 数据源 YAML 由 config_rewrite 按方言直接生成，不走本任务。

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

/// 本期仅 ruoyi-vue 支持 PostgreSQL。`template_dir` 为空视为未识别，允许规划（由管线加载后再拦截）。
pub fn supports_postgresql_template(template_dir: &str) -> bool {
    let t = template_dir.trim();
    t.is_empty() || t.eq_ignore_ascii_case("ruoyi-vue")
}

/// `db_type=postgresql` 且模板不是 ruoyi-vue 时返回中文错误，供管线在规划前拦截。
pub fn postgresql_unsupported_template_error(template_dir: &str, db_type: &str) -> Option<String> {
    if !db_type.trim().eq_ignore_ascii_case("postgresql") {
        return None;
    }
    let t = template_dir.trim();
    if supports_postgresql_template(t) {
        return None;
    }
    Some(format!(
        "本期仅 ruoyi-vue 支持 PostgreSQL，当前模板为 {t}"
    ))
}

/// 执行数据库方言切换（pom / SQL 资产 / 生成器 mapper）。
pub fn switch(
    root: &Path,
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
            "已替换 pom 驱动为 {}:{}（{} 个文件）",
            dialect.pom_group_id, dialect.pom_artifact_id, pom_n
        ));
    }

    match replace_sql_scripts(root, log) {
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

/// 定位全部 pom 中的 MySQL 驱动 `<dependency>` 块并删除，再在原声明处（或 admin 模块）插入 PG 驱动。
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
    let mut first_removed_path: Option<std::path::PathBuf> = None;

    for pom in &poms {
        let Some(content) = read_text(pom) else {
            continue;
        };
        let (new_content, removed) = strip_mysql_driver_blocks(&content);
        if removed == 0 {
            continue;
        }
        if first_removed_path.is_none() {
            first_removed_path = Some(pom.clone());
        }
        // 在原声明处插入 PG 驱动（第一个被删块的位置：dependencies 内靠前）
        let written = if !inserted {
            inserted = true;
            insert_dependency_block(&new_content, &pg_block)
        } else {
            new_content
        };
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

/// 把项目 sql/ 下 MySQL 版 ry_*.sql、quartz.sql 备份为 *.mysql.sql.bak，再复制 PG 版。
fn replace_sql_scripts(root: &Path, log: &dyn Fn(&str)) -> Result<(usize, usize, String), String> {
    let sql_dir = root.join("sql");
    if !sql_dir.is_dir() {
        std::fs::create_dir_all(&sql_dir).map_err(|e| format!("创建 sql 目录失败：{e}"))?;
    }

    let ry_src = crate::core::paths::resolve("templates/ruoyi-vue/sql/postgresql/ry.sql");
    let quartz_src = crate::core::paths::resolve("templates/ruoyi-vue/sql/postgresql/quartz.sql");
    if !ry_src.is_file() && !quartz_src.is_file() {
        return Err("请自行准备 PostgreSQL 初始化脚本（内置资产缺失：templates/ruoyi-vue/sql/postgresql/）".into());
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
        log("已写入 PostgreSQL 初始化脚本 sql/ry.sql");
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

    #[test]
    fn postgresql_rejected_for_non_vue_template() {
        assert!(supports_postgresql_template(""));
        assert!(supports_postgresql_template("ruoyi-vue"));
        assert!(!supports_postgresql_template("ruoyi"));
        assert!(!supports_postgresql_template("ruoyi-cloud"));
        assert!(postgresql_unsupported_template_error("ruoyi", "postgresql")
            .unwrap()
            .contains("本期仅 ruoyi-vue"));
        assert!(postgresql_unsupported_template_error("ruoyi-vue", "postgresql").is_none());
        assert!(postgresql_unsupported_template_error("ruoyi", "mysql").is_none());
    }
}
