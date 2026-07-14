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

    // 2. 读取 application-druid.yml/yaml（datasource 来源，原文）
    let (druid_content, druid_existed) =
        read_first(resources_dir, &["application-druid.yml", "application-druid.yaml"])?;

    // 3. 按顶层块切分 application，挑出公共块（保留注释）+ 环境块
    let blocks = split_top_blocks(&base_content);
    let mut public_lines: Vec<String> = Vec::new();
    let mut env_lines_from_base: Vec<String> = Vec::new(); // 来自 base 的 spring/redis 等环境块
    for (key, lines) in &blocks {
        if key == "<prelude>" {
            public_lines.extend(lines.iter().cloned());
            public_lines.push(String::new()); // 空行分隔
            continue;
        }
        if ENV_KEYS.contains(&key.as_str()) {
            env_lines_from_base.extend(lines.iter().cloned());
            env_lines_from_base.push(String::new());
        } else {
            public_lines.extend(lines.iter().cloned());
            public_lines.push(String::new());
        }
    }

    // 4. 构建 base 公共配置：
    //    - 在 server 块之后补 spring.application.name + spring.profiles.active=dev（base 必须有 profiles 激活）
    //    - 若开启 MyBatis-Plus，补 mybatis-plus 块
    let mut base_out = String::new();
    for line in &public_lines {
        base_out.push_str(line);
        base_out.push('\n');
    }
    // 顶部确保 server 存在；若原文无 server，补一个
    if !base_content.contains("server:") {
        base_out = format!("server:\n  port: 8080\n\n{}", base_out);
    }
    // 追加精简 spring 块（base 必须有 profiles 激活 + 应用名）
    base_out.push_str("\nspring:\n");
    base_out.push_str(&format!("  application:\n    name: {}\n", params.new_project_name));
    base_out.push_str("  profiles:\n    active: dev\n");

    // 追加 mybatis-plus 块（若开启）
    if params.enable_mybatis_plus {
        base_out.push_str(&format!(
            "\nmybatis-plus:\n  mapper-locations: classpath*:mapper/**/*Mapper.xml\n  type-aliases-package: {pkg}.**.domain\n  configuration:\n    map-underscore-to-camel-case: true\n",
            pkg = params.new_package
        ));
    }

    // 5. 构建 dev / prod：合并「base 里抽出的 spring/redis 环境块」+「druid 的 datasource」
    //    dev：明文（localhost）；prod：密码/连接串用环境变量占位
    let mut dev = String::new();
    let mut prod = String::new();

    // 5.1 先放 druid 的 datasource（若有）
    if druid_existed {
        let druid_ds = extract_spring_datasource_block(&druid_content);
        if let Some(ds_block) = druid_ds {
            dev.push_str("spring:\n");
            dev.push_str(&ds_block);
            dev.push('\n');
            prod.push_str("spring:\n");
            prod.push_str(&apply_prod_placeholders(&ds_block));
            prod.push('\n');
        }
    }
    // 5.2 再放从 base 抽出的 redis / 环境相关 spring 子配置（合并到已有的 spring 块下）
    //     若 dev 已有 spring 块（来自 datasource），追加到其后；否则新建
    let env_spring = collect_env_spring_children(&env_lines_from_base);
    if !env_spring.is_empty() {
        let has_spring_dev = dev.contains("spring:");
        if !has_spring_dev {
            dev.push_str("spring:\n");
            prod.push_str("spring:\n");
        }
        for child in &env_spring {
            dev.push_str(child);
            dev.push('\n');
            prod.push_str(child);
            prod.push('\n');
        }
    }
    // prod 末尾的环境占位也应用到 redis（如有）
    prod = apply_prod_placeholders(&prod);

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

/// 从 base 抽出的环境块行中，收集 spring 下的子配置（redis、profiles 等已删 active）
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

/// 按候选名顺序读取第一个存在的文件，返回 (内容, 是否存在)
fn read_first(dir: &Path, candidates: &[&str]) -> Result<(String, bool), String> {
    for name in candidates {
        let p = dir.join(name);
        if p.is_file() {
            let content = std::fs::read_to_string(&p).map_err(|e| format!("读取 {} 失败：{e}", p.display()))?;
            return Ok((content, true));
        }
    }
    Ok((String::new(), false))
}
