// 配置文件重构：将若依 application.yml + application-druid.yml 重构为
// application.yaml + application-dev.yaml + application-prod.yaml 三件套。
//
// 关键设计原则（修复早期版本的三个缺陷）：
// 1. 【保留注释】对 application 原文做「行级外科手术」，绝不用 serde_yaml 全量重序列化
//    （YAML 库会丢失所有注释，若依配置注释很重要）。
// 2. 【不留 .bak 垃圾】旧 druid 文件直接删除（内容已迁移到 dev/prod），不生成 .bak。
// 3. 【环境相关配置抽到 dev/prod】datasource、redis、上传路径、profiles 等环境差异配置
//    从 base 移除，放进 dev（明文 localhost）和 prod（环境变量占位）。
//    base 只保留与环境无关的公共配置（server、token、mybatis-plus、ruoyi 业务配置等）。

use crate::core::CustomizeParams;
use regex::Regex;
use std::path::{Path, PathBuf};

/// 配置重构结果
pub struct RewriteOutcome {
    pub base_path: PathBuf,
    pub dev_path: PathBuf,
    pub prod_path: PathBuf,
}

/// 顶层 YAML 键，用于按顶层块切分 application 内容
fn is_top_level_key_line(line: &str) -> Option<String> {
    // 形如 "key:" 或 "key: value"，不以空格开头，非注释，非空
    let trimmed = line.trim_end();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    if line.starts_with(' ') || line.starts_with('\t') {
        return None; // 缩进行，不是顶层
    }
    // 取冒号前的 key（去掉文档标记 ---）
    if trimmed == "---" || trimmed == "..." {
        return None;
    }
    let key = trimmed.split(':').next().unwrap_or("").trim();
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

/// 将 application 原文按顶层块切分：键名 → 该块所有行。
/// 关键：块前的注释/空行归属于「下一个」块（而非上一个），这样注释跟随它所注释的配置。
fn split_top_blocks(content: &str) -> Vec<(String, Vec<String>)> {
    let lines: Vec<&str> = content.lines().collect();
    let n = lines.len();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = 0;
    while i < n {
        let line = lines[i];
        if let Some(key) = is_top_level_key_line(line) {
            // 收集本块：键行 + 后续缩进行/块内空行/块内注释，直到下一个顶层键或文件末尾
            let mut block_lines = vec![line.to_string()];
            let mut j = i + 1;
            // 但要把「下一块的块前注释+空行」留给下一块：从后往前扫，遇到连续的顶层注释/空行就提前结束
            while j < n {
                let l = lines[j];
                // 缩进行 → 属于本块
                if l.starts_with(' ') || l.starts_with('\t') {
                    block_lines.push(l.to_string());
                    j += 1;
                    continue;
                }
                // 顶层空行或顶层注释 → 可能是块间分隔或下一块的块前注释
                break;
            }
            // 现在 j 指向第一个非缩进的顶层行；从 block_lines 末尾去掉连续的顶层空行/注释，
            // 它们留给下一块。但块内的空行（缩进）已 push，保留。
            // 注意：block_lines 目前只含缩进行，不包含顶层空行/注释（上面 break 了），
            // 所以无需回退。但块与块之间的空行会被跳过——下面补上下一块的块前注释/空行。
            blocks.push((key, block_lines));
            i = j;
            // 跳过并收集「块前空行」——丢弃（块间分隔），但顶层注释要留给下一块
            while i < n {
                let l = lines[i];
                if l.trim().is_empty() {
                    i += 1; // 丢弃块间空行
                    continue;
                }
                if l.starts_with('#') {
                    // 顶层注释：属于下一块，作为下一块的块前注释先行收集
                    // 找到下一块
                    let mut prelude: Vec<String> = Vec::new();
                    while i < n && lines[i].starts_with('#') {
                        prelude.push(lines[i].to_string());
                        i += 1;
                    }
                    // 跳过注释后的空行
                    while i < n && lines[i].trim().is_empty() {
                        i += 1;
                    }
                    // 现在应指向下一个顶层键；把注释并入该块的开头
                    if i < n {
                        if let Some(key2) = is_top_level_key_line(lines[i]) {
                            // 递归收集该块（复用逻辑：直接进下一轮 while）
                            // 这里把 prelude 存起来，下一轮处理 lines[i] 时追加
                            // 简化：直接构造该块并合并 prelude
                            let mut block_lines2 = prelude;
                            block_lines2.push(lines[i].to_string());
                            let mut k = i + 1;
                            while k < n {
                                let l2 = lines[k];
                                if l2.starts_with(' ') || l2.starts_with('\t') {
                                    block_lines2.push(l2.to_string());
                                    k += 1;
                                } else {
                                    break;
                                }
                            }
                            blocks.push((key2, block_lines2));
                            i = k;
                            continue;
                        }
                    }
                    // 注释后没有顶层键（文件末尾注释）：作为独立块保留
                    blocks.push(("<trailing-comments>".into(), prelude));
                    break;
                }
                break;
            }
        } else {
            // 文件开头未进入任何块的内容（如文档标记 ---、首部注释）
            blocks.push(("<prelude>".into(), vec![line.to_string()]));
            i += 1;
        }
    }
    blocks
}

/// 环境相关配置的顶层键名（这些从 base 移除，进 dev/prod）
const ENV_KEYS: &[&str] = &["spring", "redis", "druid", "datasource", "log"];
// 注：spring 是环境相关的（含 datasource/redis/profiles），整体移到 dev/prod；
// base 只保留 server / token / mybatis-plus / ruoyi / 自定义业务配置。

/// 执行配置重构。resources_dir 指向含 application*.yml 的目录（通常是 admin/src/main/resources）。
pub fn rewrite(
    resources_dir: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<RewriteOutcome, String> {
    // 1. 读取原始 application.yml/yaml（原文，保留注释）
    let (base_content, base_existed) =
        read_first(resources_dir, &["application.yml", "application.yaml"])?;
    let base_content = if base_existed {
        base_content
    } else {
        "server:\n  port: 8080\n".to_string()
    };

    // 2. 读取 application-druid.yml/yaml：内容用于「未开 SQL 定制时解析原库名」
    //    （必须在末尾删除旧文件之前完成解析，此处读取后一直保留在内存中），
    //    旧文件本身在迁移完成后仍需删除
    let (druid_content, druid_existed) =
        read_first(resources_dir, &["application-druid.yml", "application-druid.yaml"])?;

    // 3. 按顶层块切分 application，挑出公共块（保留注释）；环境相关块（spring/redis 等）
    //    从 base 移除——datasource/redis 已改用标准模板写入 dev/prod。
    //    但 spring 块需特殊处理：保留其中的「运行时子项」（messages/jackson/mvc 等 + 注释），
    //    只丢弃 datasource/redis/data/profiles 等环境相关子项。
    //    （若丢掉 spring.messages.basename，MessageSource 会退化为空 → 登录登出 i18n 全挂）
    let blocks = split_top_blocks(&base_content);
    let mut public_lines: Vec<String> = Vec::new();
    let mut spring_runtime_children: Vec<String> = Vec::new(); // 保留的 spring 运行时子项
    for (key, lines) in &blocks {
        if key == "<prelude>" {
            public_lines.extend(lines.iter().cloned());
            public_lines.push(String::new()); // 空行分隔
            continue;
        }
        if key == "spring" {
            // 从 spring 块抽取运行时子项（排除 datasource/redis/data/profiles）
            spring_runtime_children = extract_spring_runtime_children(lines);
            continue;
        }
        if ENV_KEYS.contains(&key.as_str()) {
            // 其它环境相关块（redis/datasource 等独立顶层块，罕见）从 base 移除
            continue;
        }
        public_lines.extend(lines.iter().cloned());
        public_lines.push(String::new());
    }

    // 4. 构建 base 公共配置：
    //    - server.port 同步为 params.server_port（与 nginx / scripts 对齐）
    //    - 在 server 块之后补 spring.application.name + spring.profiles.active=dev（base 必须有 profiles 激活）
    //    - 若开启 MyBatis-Plus，补 mybatis-plus 块
    let mut base_out = String::new();
    for line in &public_lines {
        base_out.push_str(line);
        base_out.push('\n');
    }
    // 顶部确保 server 存在；若原文无 server，补一个（端口用 params.server_port）
    if !base_content.contains("server:") {
        base_out = format!("server:\n  port: {}\n\n{}", params.server_port, base_out);
    }
    // server.port 同步：把 server 块内的 port 行强制改为 params.server_port
    // （原 application.yml 里常写 8080，需与 nginx upstream / scripts/start.sh 的端口一致）
    base_out = sync_server_port(&base_out, params.server_port);
    // 追加精简 spring 块：
    //   - 先放保留的运行时子项（messages/jackson/mvc 等，含注释）
    //   - 再补 application.name + profiles.active=dev（base 必须有 profiles 激活）
    base_out.push_str("\nspring:\n");
    if !spring_runtime_children.is_empty() {
        for child in &spring_runtime_children {
            base_out.push_str(child);
            base_out.push('\n');
        }
    }
    base_out.push_str(&format!("  application:\n    name: {}\n", params.new_project_name));
    base_out.push_str("  profiles:\n    active: dev\n");

    // 追加 mybatis-plus 块（若开启）
    if params.enable_mybatis_plus {
        base_out.push_str(&format!(
            "\nmybatis-plus:\n  mapper-locations: classpath*:mapper/**/*Mapper.xml\n  type-aliases-package: {pkg}.**.domain\n  configuration:\n    map-underscore-to-camel-case: true\n",
            pkg = params.new_package
        ));
    }

    // 5. 构建 dev / prod：统一用「标准完整模板」明文写入 datasource + redis
    //    （druid 全量连接池参数 + lettuce 连接池）
    //    dev 与 prod 内容完全一致：都明文，无 ${ENV} 占位（由部署人员后续按需替换）
    //
    //    库名三分支决策：
    //    1) 用户填写了数据库名 → 直接使用；
    //    2) 未填写但开启了 SQL 定制 → 用模块前缀（与前端「留空则用模块前缀」提示一致）；
    //    3) 未填写且未开 SQL 定制 → 从原 druid/application 配置解析原库名并保持，
    //       保证数据库层零改动；解析失败才回退模块前缀并提示。
    //       （解析基于函数开头已读入内存的原文，早于第 7 步删除旧 druid 文件，不受影响）
    let db_name: String = if !params.db_name.is_empty() {
        params.db_name.clone()
    } else if params.enable_sql_customize {
        params.new_module_prefix.clone()
    } else {
        // 未开 SQL 定制：优先解析旧 druid 文件，其次尝试 application 原文
        let parsed = if druid_existed {
            parse_master_db_name(&druid_content).or_else(|| parse_master_db_name(&base_content))
        } else {
            parse_master_db_name(&base_content)
        };
        match parsed {
            Some(name) => {
                log(&format!("未填写数据库名，配置重构保持原库名：{name}"));
                name
            }
            None => {
                log(&format!(
                    "未能从原配置解析数据库名，回退使用模块前缀 {prefix}，请手工确认连接配置",
                    prefix = params.new_module_prefix
                ));
                params.new_module_prefix.clone()
            }
        }
    };
    let std_block = build_standard_datasource_redis(&db_name);
    let dev = std_block.clone();
    let prod = std_block;

    // 6. 写出文件
    let base_path = resources_dir.join("application.yaml");
    let dev_path = resources_dir.join("application-dev.yaml");
    let prod_path = resources_dir.join("application-prod.yaml");
    std::fs::write(&base_path, &base_out).map_err(|e| format!("写入 application.yaml 失败：{e}"))?;
    std::fs::write(&dev_path, &dev).map_err(|e| format!("写入 application-dev.yaml 失败：{e}"))?;
    std::fs::write(&prod_path, &prod).map_err(|e| format!("写入 application-prod.yaml 失败：{e}"))?;

    // 7. 删除旧 druid 文件（内容已迁移，不留 .bak）
    if druid_existed {
        for name in &["application-druid.yml", "application-druid.yaml"] {
            let p = resources_dir.join(name);
            if p.exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
        log("已删除旧 application-druid 配置（datasource 已迁移到 dev/prod）");
    }
    // 删除旧 application.yml（非 .yaml），其内容已重写为 application.yaml
    let old_app_yml = resources_dir.join("application.yml");
    if old_app_yml.exists() {
        let _ = std::fs::remove_file(&old_app_yml);
    }

    log("配置重构完成：application.yaml(公共) + application-dev.yaml + application-prod.yaml");
    Ok(RewriteOutcome {
        base_path,
        dev_path,
        prod_path,
    })
}

/// 从 druid 内容中提取 spring.datasource 块（含缩进），返回可直接挂在 spring: 下的子内容
///
/// 注：datasource 已改用标准模板生成（`build_standard_datasource_redis`），此函数保留备用，
/// 当前 dev/prod 生成路径不再调用。
#[allow(dead_code)]
fn extract_spring_datasource_block(druid_content: &str) -> Option<String> {
    // druid 文件通常是：spring:\n  datasource:\n    ...
    // 我们要取出 "  datasource:" 及其所有更深层缩进行
    let lines: Vec<&str> = druid_content.lines().collect();
    let mut start: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start() == "datasource:" || line.starts_with("  datasource:") {
            // 确认在 spring 块下（两层缩进）
            if line.starts_with("  ") && !line.starts_with("   ") {
                start = Some(i);
                break;
            }
        }
    }
    let start = start?;
    let mut block_lines: Vec<String> = Vec::new();
    // 从 datasource 行开始，收集后续缩进 >= 4 空格的行（datasource 的子项）
    block_lines.push(lines[start].to_string());
    for line in lines.iter().skip(start + 1) {
        if line.trim().is_empty() || line.starts_with('#') {
            block_lines.push(line.to_string());
            continue;
        }
        // 仍是 datasource 的子项：缩进 >= 4（datasource 本身是 2，其子项 >= 4）
        if line.starts_with("    ") || line.starts_with("     ") || line.starts_with("\t") {
            block_lines.push(line.to_string());
        } else {
            break;
        }
    }
    Some(block_lines.join("\n"))
}

/// 从 spring 块的行中抽取「运行时子项」——保留除 datasource/redis/data/profiles 之外的所有子项及其注释。
///
/// 输入 lines 形如：
/// ```text
/// spring:
///   # 国际化
///   messages:
///     basename: i18n/messages
///   jackson:
///     date-format: yyyy-MM-dd
///   profiles:
///     active: druid
///   datasource:
///     ...
/// ```
/// 输出（每个元素一行，可直接拼到新 spring 块下，2 空格缩进）：
/// - 保留：messages / jackson / mvc / servlet / main / http 等运行时配置 + 它们的注释 + 更深层级内容
/// - 丢弃：datasource / data（redis 在 data 下）/ redis / profiles（active 已在 base 单独写）
///
/// 这样 base 仍能正确加载 i18n（spring.messages.basename）、Jackson 日期格式等运行时行为。
fn extract_spring_runtime_children(lines: &[String]) -> Vec<String> {
    // 环境相关子项（按 key 名匹配，冒号结尾）：整体丢弃其子树
    const ENV_CHILD_KEYS: &[&str] = &["datasource:", "data:", "redis:", "profiles:"];

    let mut out: Vec<String> = Vec::new();
    let mut seen_spring_header = false;
    let mut skip_current = false; // 当前正在丢弃某个环境子项的子树
    let mut pending_comment: Option<String> = None; // 暂存块前注释，归属下一个顶层子项

    for line in lines {
        // 跳过 spring: 头本身（调用方会重新写 spring:）
        let trimmed_end = line.trim_end();
        if !seen_spring_header {
            if trimmed_end == "spring:" {
                seen_spring_header = true;
                continue;
            }
            // 头之前的行（理论上 split_top_blocks 不会给，防御性跳过）
            continue;
        }

        // 注释行（#）：可能是某子项的块前注释或行内说明
        let is_comment = trimmed_end.starts_with('#');
        let is_blank = trimmed_end.is_empty();

        // spring 下的两层缩进顶层子项（恰好 2 空格，非注释）
        if !is_comment && !is_blank
            && line.starts_with("  ")
            && !line.starts_with("   ")
        {
            let key = trimmed_end.trim_start(); // 去掉 2 空格缩进，得到 "messages:" / "profiles:" 等
            if ENV_CHILD_KEYS.contains(&key) {
                skip_current = true;
                pending_comment = None; // 该注释属于被丢弃的子项
                continue;
            }
            // 保留的顶层子项：先吐出归属它的块前注释
            skip_current = false;
            if let Some(c) = pending_comment.take() {
                out.push(c);
            }
            out.push(line.clone());
            continue;
        }

        // 三层及以上缩进（子项的子项）
        if !is_blank && line.starts_with("   ") {
            if skip_current {
                continue;
            }
            // 归属当前保留子项（或其块前注释段），保留
            out.push(line.clone());
            continue;
        }

        // 空行 / 注释行
        if is_comment {
            // 注释可能属于下一个子项（块前注释）或当前保留子项的行内说明
            if skip_current {
                // 属于被丢弃子项，丢弃
                continue;
            }
            // 暂存为下一个顶层子项的块前注释；但若已有保留子项在产出中，
            // 可能是当前子项内部的说明注释 —— 判断缩进层级无法可靠区分，
            // 保守策略：注释一律暂存 pending_comment，遇到保留顶层子项时吐出；
            // 若之后遇到的是被丢弃子项则丢弃。多行注释累加。
            if let Some(existing) = pending_comment.take() {
                pending_comment = Some(format!("{existing}\n{line}"));
            } else {
                pending_comment = Some(line.clone());
            }
            continue;
        }

        // 空行：保留子项之间的分隔——若 pending 有注释或已有产出则保留一个空行
        if is_blank && !out.is_empty() {
            out.push(String::new());
        }
    }

    // 末尾若有未归属的注释且产出处于保留区，附加（避免丢失块尾说明）
    if let Some(c) = pending_comment {
        if !out.is_empty() {
            out.push(c);
        }
    }

    // 去掉末尾多余空行
    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    out
}

/// 从 base 抽出的环境块行中，收集 spring 下的子配置（redis、profiles 等已删 active）
///
/// 注：datasource/redis 已改用标准模板生成，此函数保留备用，当前路径不再调用。
#[allow(dead_code)]
fn collect_env_spring_children(env_lines: &[String]) -> Vec<String> {
    // env_lines 形如 ["spring:", "  redis:", "    host: localhost", ...]
    // 我们要 spring 下除 datasource/profiles 之外的子项（主要是 redis）
    let mut out = Vec::new();
    let mut in_spring = false;
    let mut in_redis = false;
    let mut skip_datasource = false;
    for line in env_lines {
        if line == "spring:" {
            in_spring = true;
            continue;
        }
        if !in_spring {
            continue;
        }
        // spring 下的两层缩进子项
        if line.starts_with("  ") && !line.starts_with("   ") {
            let key = line.trim_end();
            if key == "datasource:" {
                skip_datasource = true;
                in_redis = false;
                continue;
            }
            if key == "profiles:" {
                skip_datasource = false;
                in_redis = false;
                continue; // active 已在 base 单独处理
            }
            if key == "redis:" {
                in_redis = true;
                skip_datasource = false;
                out.push(line.clone());
                continue;
            }
            // 其它 spring 子项（如 mvc、servlet 等）保留
            skip_datasource = false;
            in_redis = false;
            out.push(line.clone());
        } else if line.starts_with("   ") {
            // 三层及以上：归属当前子项
            if skip_datasource {
                continue;
            }
            if in_redis {
                out.push(line.clone());
            } else if !out.is_empty() {
                // 其它保留的子项的子项
                out.push(line.clone());
            }
        }
    }
    out
}

/// 给 prod 的 datasource / redis 应用环境变量占位（用户名/密码/URL/redis host）
///
/// 注：dev/prod 现统一用明文标准模板（无 ${ENV} 占位），此函数保留备用，当前路径不再调用。
#[allow(dead_code)]
fn apply_prod_placeholders(content: &str) -> String {
    let mut out = content.to_string();
    // datasource master 的 username/password/url
    out = replace_value_line(&out, "username:", "MYSQL_USERNAME");
    out = replace_value_line(&out, "password:", "MYSQL_PASSWORD");
    // redis host/port/password
    let re_redis_host = Regex::new(r"(host:\s*)(\S+)").unwrap();
    out = re_redis_host
        .replace_all(&out, |caps: &regex::Captures| {
            let v = &caps[2];
            if v.contains("${") {
                caps[0].to_string()
            } else {
                format!("{}${{REDIS_HOST:{}}}", &caps[1], v)
            }
        })
        .to_string();
    out
}

/// 把形如 "key: value" 的行值替换为 ${ENV_VAR:原值}（仅 prod 用）
///
/// 注：dev/prod 现统一用明文标准模板，此函数保留备用，当前路径不再调用。
#[allow(dead_code)]
fn replace_value_line(content: &str, key: &str, env_var: &str) -> String {
    let mut out = String::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key) {
            // 提取原值
            let after = &trimmed[key.len()..];
            let orig = after.trim();
            if orig.is_empty() || orig.contains("${") {
                out.push_str(line);
            } else {
                // 保留缩进
                let indent = &line[..line.len() - trimmed.len()];
                out.push_str(&format!("{indent}{key} ${{{env_var}:{orig}}}"));
            }
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // 末尾可能多一个换行，去掉
    if out.ends_with('\n') && content.ends_with('\n') {
        // 保持一致
    }
    out
}

/// 把 base 配置中 server 块内的 port 行强制改为 `port: <target_port>`。
///
/// 用于让 application.yaml 的 server.port 与 nginx upstream / scripts/start.sh 的端口一致。
/// 仅匹配 server 块下第一个 `port: <数字>` 行（缩进 2 空格）；其它 port（如 server.port 注释）不受影响。
fn sync_server_port(content: &str, target_port: i32) -> String {
    // 找到顶层 "server:" 行，在其块内替换第一个 "  port: 数字" 行
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_server = false;
    let mut port_replaced = false;
    for line in &lines {
        // 顶层 server: 键（不以空格开头）
        if !line.starts_with(' ') && !line.starts_with('\t') {
            let key = line.split(':').next().unwrap_or("").trim();
            in_server = key == "server";
            out.push((*line).to_string());
            continue;
        }
        if in_server && !port_replaced {
            // 匹配 "  port: 数字"（恰好两层缩进）
            let trimmed = line.trim_start();
            if trimmed.starts_with("port:") {
                let after = trimmed["port:".len()..].trim();
                // 仅替换纯数字值（保留行内注释）
                let num_part = after.split('#').next().unwrap_or("").trim();
                if !num_part.is_empty()
                    && num_part.chars().all(|c| c.is_ascii_digit())
                {
                    let indent = &line[..line.len() - trimmed.len()];
                    out.push(format!("{indent}port: {}", target_port));
                    port_replaced = true;
                    continue;
                }
            }
        }
        out.push((*line).to_string());
    }
    out.join("\n")
}

/// 从 YAML 文本解析主库（master）数据源 url 中的数据库名。
///
/// 规则：
/// 1. 优先定位 `master:` 行，仅在其块内查找 `url:` 行；找不到 `master:` 时，
///    回退为全文第一个 `jdbc:mysql://` 开头的 url 行（兼容 datasource 直写在 application.yml 的项目）。
/// 2. url 为空、无 `/` 路径段、库名段为空，或库名段含非法字符（`/`、空白）均视为解析失败，返回 None。
fn parse_master_db_name(yaml: &str) -> Option<String> {
    let lines: Vec<&str> = yaml.lines().collect();

    // 定位 master: 行（行内注释不影响识别），记录缩进用于判断块结束
    let mut master: Option<(usize, usize)> = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();
        let key_part = trimmed.split('#').next().unwrap_or("").trim();
        if key_part == "master:" {
            master = Some((i, indent_width(trimmed)));
            break;
        }
    }

    match master {
        Some((mi, mindent)) => {
            // 仅在 master 块内查找 url（直到出现缩进 <= master 的非注释行）
            for line in lines.iter().skip(mi + 1) {
                let trimmed = line.trim_end();
                let t = trimmed.trim_start();
                if t.is_empty() || t.starts_with('#') {
                    continue;
                }
                if indent_width(trimmed) <= mindent {
                    break; // 已离开 master 块
                }
                if let Some(db) = db_name_from_url_line(trimmed) {
                    return Some(db);
                }
            }
            None
        }
        None => {
            // 无 master 块：回退为全文第一个 jdbc:mysql url 行
            for line in &lines {
                let trimmed = line.trim_end();
                let t = trimmed.trim_start();
                if t.is_empty() || t.starts_with('#') {
                    continue;
                }
                if let Some(db) = db_name_from_url_line(trimmed) {
                    return Some(db);
                }
            }
            None
        }
    }
}

/// 行首缩进宽度（字符数；仅用于同文件内的块范围比较）
fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// 从单行 `url: jdbc:mysql://host:port/<db>?params` 中提取库名；非 jdbc:mysql url 行返回 None。
fn db_name_from_url_line(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("url:")?;
    // 去掉行内注释后取值（值两侧空白一并去除）
    let value = rest.split('#').next().unwrap_or("").trim();
    let after = value.strip_prefix("jdbc:mysql://")?;
    // 去掉查询参数，得到 host:port/<db> 路径段；无 `/` 视为解析失败
    let path = after.split('?').next().unwrap_or("");
    let (_host, db) = path.split_once('/')?;
    // 库名段为空或含非法字符（`/`、空白）均视为解析失败
    if db.is_empty() || db.contains('/') || db.contains(char::is_whitespace) {
        return None;
    }
    Some(db.to_string())
}

/// 构建标准完整的 spring.datasource + spring.data.redis 配置块（明文）。
///
/// 用于 dev / prod：druid 全量连接池参数 + lettuce 连接池。dev 与 prod 内容完全一致，
/// 不注入 ${ENV} 占位（由部署人员后续按需替换）。
///
/// `db_name`：数据库名（url 中 jdbc:mysql://localhost:3306/<db_name>）。
fn build_standard_datasource_redis(db_name: &str) -> String {
    format!(
        r#"# ===== 数据源 + Redis 配置（dev/prod 明文，部署时按需替换密码/地址） =====
spring:
  datasource:
    type: com.alibaba.druid.pool.DruidDataSource
    driverClassName: com.mysql.cj.jdbc.Driver
    druid:
      # 主库数据源
      master:
        url: jdbc:mysql://localhost:3306/{db_name}?useUnicode=true&characterEncoding=utf8&zeroDateTimeBehavior=convertToNull&useSSL=true&serverTimezone=GMT%2B8
        username: root
        password: 123456
      # 从库数据源
      slave:
        # 从数据源开关/默认关闭
        enabled: false
        url:
        username:
        password:
      # 初始连接数
      initialSize: 5
      # 最小连接池数量
      minIdle: 10
      # 最大连接池数量
      maxActive: 20
      # 配置获取连接等待超时的时间
      maxWait: 60000
      # 配置连接超时时间
      connectTimeout: 30000
      # 配置网络超时时间
      socketTimeout: 60000
      # 配置间隔多久才进行一次检测，检测需要关闭的空闲连接，单位是毫秒
      timeBetweenEvictionRunsMillis: 60000
      # 配置一个连接在池中最小生存的时间，单位是毫秒
      minEvictableIdleTimeMillis: 300000
      # 配置一个连接在池中最大生存的时间，单位是毫秒
      maxEvictableIdleTimeMillis: 900000
      # 配置检测连接是否有效
      validationQuery: SELECT 1 FROM DUAL
      testWhileIdle: true
      testOnBorrow: false
      testOnReturn: false
      webStatFilter:
        enabled: true
      statViewServlet:
        enabled: true
        # 设置白名单，不填则允许所有访问
        allow:
        url-pattern: /druid/*
        # 控制台管理用户名和密码
        login-username: admin
        login-password: wauio@(*&d
      filter:
        stat:
          enabled: true
          # 慢SQL记录
          log-slow-sql: true
          slow-sql-millis: 1000
          merge-sql: true
        wall:
          config:
            multi-statement-allow: true
  data:
    # redis 配置
    redis:
      # 地址
      host: localhost
      # 端口，默认为6379
      port: 6379
      # 数据库索引
      database: 1
      # 密码
      password:
      # 连接超时时间
      timeout: 10s
      lettuce:
        pool:
          # 连接池中的最小空闲连接
          min-idle: 0
          # 连接池中的最大空闲连接
          max-idle: 8
          # 连接池的最大数据库连接数
          max-active: 8
          # #连接池最大阻塞等待时间（使用负值表示没有限制）
          max-wait: -1ms
"#
    )
}

/// 按候选名顺序读取第一个存在的文件，返回 (内容, 是否存在)
fn read_first(dir: &Path, candidates: &[&str]) -> Result<(String, bool), String> {
    for name in candidates {
        let p = dir.join(name);
        if p.is_file() {
            let content = crate::utils::file::read_text(&p)
                .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", p.display()))?;
            return Ok((content, true));
        }
    }
    Ok((String::new(), false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_db_name_standard_druid_url() {
        // 标准 druid 配置：master url 带查询参数 → 解析出 ry-vue
        let yaml = "# 数据源配置\nspring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry-vue?useUnicode=true&characterEncoding=utf8&serverTimezone=GMT%2B8\n        username: root\n        password: password\n      slave:\n        enabled: false\n        url:\n";
        assert_eq!(parse_master_db_name(yaml).as_deref(), Some("ry-vue"));
    }

    #[test]
    fn parse_db_name_url_without_query() {
        // url 无查询参数（行尾结束）→ 解析出 ry-vue
        let yaml = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry-vue\n";
        assert_eq!(parse_master_db_name(yaml).as_deref(), Some("ry-vue"));
    }

    #[test]
    fn parse_db_name_url_empty_fails() {
        // master url 为空（未开启从库时的典型占位写法）→ 解析失败
        let yaml = "spring:\n  datasource:\n    druid:\n      master:\n        url:\n        username: root\n        password: password\n      slave:\n        enabled: false\n";
        assert_eq!(parse_master_db_name(yaml), None);
    }

    #[test]
    fn parse_db_name_no_path_segment_fails() {
        // 无 `/` 路径段 → 解析失败
        let yaml = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306\n";
        assert_eq!(parse_master_db_name(yaml), None);
        // 库名段为空（/ 后直接跟查询参数）→ 解析失败
        let yaml2 = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/?useUnicode=true\n";
        assert_eq!(parse_master_db_name(yaml2), None);
    }

    #[test]
    fn parse_db_name_illegal_chars_fail() {
        // 库名段含空格 → 解析失败
        let yaml = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/ry vue?x=1\n";
        assert_eq!(parse_master_db_name(yaml), None);
        // 库名段含 `/`（多级路径）→ 解析失败
        let yaml2 = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:mysql://localhost:3306/a/b?x=1\n";
        assert_eq!(parse_master_db_name(yaml2), None);
    }

    #[test]
    fn parse_db_name_falls_back_to_first_url_without_master() {
        // 无 master 块（datasource 直写 application.yml）→ 回退全文第一个 jdbc:mysql url
        let yaml = "spring:\n  datasource:\n    url: jdbc:mysql://localhost:3306/rydb?useSSL=false\n";
        assert_eq!(parse_master_db_name(yaml).as_deref(), Some("rydb"));
    }

    #[test]
    fn parse_db_name_non_mysql_url_ignored() {
        // 非 jdbc:mysql url → 解析失败
        let yaml = "spring:\n  datasource:\n    druid:\n      master:\n        url: jdbc:postgresql://localhost:5432/pgdb\n";
        assert_eq!(parse_master_db_name(yaml), None);
    }

    #[test]
    fn parse_db_name_slave_url_not_picked_when_master_present() {
        // master url 可解析时，不应取到 slave 的空 url 或其他值
        let yaml = "spring:\n  datasource:\n    druid:\n      slave:\n        enabled: false\n        url:\n      master:\n        url: jdbc:mysql://localhost:3306/masterdb?x=1\n        username: root\n";
        assert_eq!(parse_master_db_name(yaml).as_deref(), Some("masterdb"));
    }
}
