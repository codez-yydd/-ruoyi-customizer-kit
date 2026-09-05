// MyBatis-Plus 集成：依赖添加、配置类生成、代码生成器模板适配、Long ID 序列化。
//
// 设计：幂等（已存在则跳过，不重复添加）；不破坏原 mybatis 配置；模板改造保守替换。
// 依赖优先加到公共模块（common/framework），配置类放 admin 模块新包路径下。

use crate::core::CustomizeParams;
use crate::utils::file::read_text;
use crate::utils::path::package_to_path;
use std::path::Path;

/// MyBatis-Plus 依赖版本。
///
/// 2026-09-05 已在 Maven Central 核实 3.5.15 同时提供三个 starter：
/// - `com.baomidou:mybatis-plus-boot-starter:3.5.15`
/// - `com.baomidou:mybatis-plus-spring-boot3-starter:3.5.15`
/// - `com.baomidou:mybatis-plus-spring-boot4-starter:3.5.15`
/// 来源：https://repo1.maven.org/maven2/com/baomidou/.../3.5.15/
/// 不要改成 3.5.16/3.5.17（三个 starter 的最高公共版本以核实结果为准）。
const MP_VERSION: &str = "3.5.15";

/// Boot 2.x 用 starter（面向 Spring 5）
const MP_STARTER_BOOT2: &str = "mybatis-plus-boot-starter";
/// Boot 3.x 用 starter（面向 Spring 6 / Jakarta EE）
const MP_STARTER_BOOT3: &str = "mybatis-plus-spring-boot3-starter";
/// Boot 4.x 用 starter（面向 Spring 7；旧 starter 会因自动配置类失效而启动报错，见 baomidou/mybatis-plus#7009）
const MP_STARTER_BOOT4: &str = "mybatis-plus-spring-boot4-starter";

/// 分页插件 jsqlparser 模块（Boot 3 / 4 / 检测不到）。
///
/// 2026-09-05 核实：MyBatis-Plus 3.5.9+ 将 `PaginationInnerInterceptor` 拆到可选模块，
/// starter 不再传递该依赖。来源：官方安装文档
/// https://baomidou.com/en/getting-started/install/
/// 与 Maven Central（`mybatis-plus-jsqlparser` / `mybatis-plus-jsqlparser-4.9` 的 3.5.15 均存在）。
const MP_JSQLPARSER: &str = "mybatis-plus-jsqlparser";
/// JDK 8 / Boot 2 用（jsqlparser 5+ 不支持 JDK 8）
const MP_JSQLPARSER_JDK8: &str = "mybatis-plus-jsqlparser-4.9";

/// 精确 artifactId 标签，禁止用 `contains("mybatis-plus-jsqlparser")` 判断现代档
/// （它是 `mybatis-plus-jsqlparser-4.9` 的前缀）。
fn artifact_id_tag(artifact: &str) -> String {
    format!("<artifactId>{artifact}</artifactId>")
}

/// 版本检测已上移至 `detector`；此处 re-export，oss.rs 与原位单测调用路径不变。
pub use crate::core::detector::detect_boot_major_version;

/// 按 Boot 大版本选择 MyBatis-Plus starter artifactId。
/// 检测不到版本时默认 Boot 4（兜底跟随最新生态）。
fn select_starter(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3 => MP_STARTER_BOOT2, // 2.x
        Some(3) => MP_STARTER_BOOT3,                   // 3.x
        _ => MP_STARTER_BOOT4,                         // >=4 及检测不到（默认现代版本）
    }
}

/// 按 Boot 大版本选择 jsqlparser 分页模块 artifactId。
/// Boot 2（major < 3）→ `jsqlparser-4.9`（JDK 8）；其余（3 / 4 / None）→ 现代档。
fn select_jsqlparser(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3 => MP_JSQLPARSER_JDK8,
        _ => MP_JSQLPARSER,
    }
}

/// 添加 MyBatis-Plus 依赖到公共模块 pom。
/// 一次写入同时注入 starter + 对应 jsqlparser；幂等拆开：
/// 1. 三个 starter 任一已存在 → 不再加 starter
/// 2. 两个 jsqlparser 精确标签任一已存在 → 不再加 jsqlparser
/// 3. starter 已有、jsqlparser 缺失 → 只补 jsqlparser，返回 `Ok(true)`
/// 4. 两者都有 → `Ok(false)`
///
/// `boot_major` 为 `Some(x)` 时直接使用，不再扫 pom；为 `None` 时现场检测（自测 / 集成测试旧路径）。
pub fn add_dependency(
    root: &Path,
    backend_modules: &[String],
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let starter_markers = [MP_STARTER_BOOT2, MP_STARTER_BOOT3, MP_STARTER_BOOT4];
    let boot_major = boot_major.or_else(|| detect_boot_major_version(root));
    let starter = select_starter(boot_major);
    let jsql = select_jsqlparser(boot_major);

    let has_starter = starter_markers
        .iter()
        .any(|m| any_pom_has(root, backend_modules, m));
    let has_jsql = any_pom_has(root, backend_modules, &artifact_id_tag(MP_JSQLPARSER))
        || any_pom_has(root, backend_modules, &artifact_id_tag(MP_JSQLPARSER_JDK8));

    if has_starter && has_jsql {
        log("MyBatis-Plus starter 与 jsqlparser 分页模块均已存在，跳过");
        return Ok(false);
    }
    if has_starter {
        log("MyBatis-Plus starter 已存在，跳过");
    }
    if has_jsql {
        log("MyBatis-Plus jsqlparser 分页模块已存在，跳过");
    }

    let mut added = false;
    if !has_starter && !has_jsql {
        let module = first_writable_module(root, backend_modules)?;
        write_mp_artifacts(root, &module, &[starter, jsql], boot_major, log)?;
        return Ok(true);
    }
    if !has_starter {
        let module = first_writable_module(root, backend_modules)?;
        write_mp_artifacts(root, &module, &[starter], boot_major, log)?;
        added = true;
    }
    if !has_jsql {
        let module = find_starter_module(root, backend_modules)
            .or_else(|| first_writable_module(root, backend_modules).ok())
            .ok_or_else(|| "找不到合适的 pom.xml 来添加 MyBatis-Plus 依赖".to_string())?;
        write_mp_artifacts(root, &module, &[jsql], boot_major, log)?;
        added = true;
    }
    Ok(added)
}

/// 构造单条 MyBatis-Plus 依赖 XML。
fn mp_dep_xml(artifact: &str) -> String {
    format!(
        "\n    <dependency>\n        <groupId>com.baomidou</groupId>\n        <artifactId>{artifact}</artifactId>\n        <version>{ver}</version>\n    </dependency>\n",
        ver = MP_VERSION
    )
}

/// 在已有 `<dependencies>` 后插入依赖块；若无该节点则在 `</project>` 前包一层。
fn insert_dep_block(content: &str, dep_block: &str) -> String {
    if let Some(idx) = content.find("<dependencies>") {
        let mark = "<dependencies>";
        let mut s = String::with_capacity(content.len() + dep_block.len());
        s.push_str(&content[..idx + mark.len()]);
        s.push_str(dep_block);
        s.push_str(&content[idx + mark.len()..]);
        s
    } else {
        content.replace(
            "</project>",
            &format!("    <dependencies>{dep_block}    </dependencies>\n</project>"),
        )
    }
}

/// 将若干 artifact 写入指定模块 pom，并分别打日志。
fn write_mp_artifacts(
    root: &Path,
    module: &str,
    artifacts: &[&str],
    boot_major: Option<u32>,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    let pom = root.join(module).join("pom.xml");
    let content = read_text(&pom)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", pom.display()))?;
    let dep_block: String = artifacts.iter().copied().map(mp_dep_xml).collect();
    let new_content = insert_dep_block(&content, &dep_block);
    std::fs::write(&pom, new_content).map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
    let boot_label = boot_major.map_or("版本未知，默认按 Boot 4".to_string(), |m| format!("{m}.x"));
    for artifact in artifacts {
        if *artifact == MP_JSQLPARSER || *artifact == MP_JSQLPARSER_JDK8 {
            log(&format!(
                "已在 {module}/pom.xml 添加 {artifact}:{MP_VERSION}（分页插件 jsqlparser 模块）"
            ));
        } else {
            log(&format!(
                "已在 {module}/pom.xml 添加 {artifact}:{MP_VERSION}（Spring Boot {boot_label}）"
            ));
        }
    }
    Ok(())
}

/// 候选模块中第一个存在 pom.xml 的模块。
/// Vue：common > framework > admin > 其余（成功语义不变）。
/// Cloud：固定写入 `{prefix}-common/{prefix}-common-datasource`（叶子，不是聚合 ruoyi-common）。
fn first_writable_module(root: &Path, modules: &[String]) -> Result<String, String> {
    if crate::core::detector::is_cloud_layout(root) {
        if let Some(m) = crate::core::detector::find_module_by_leaf_suffix(root, modules, "common-datasource")
        {
            return Ok(m);
        }
        return Err("Cloud 未找到 ruoyi-common-datasource（或改名后的 *-common-datasource）".into());
    }
    for module in &prioritize_modules(modules) {
        if root.join(module).join("pom.xml").is_file() {
            return Ok(module.clone());
        }
    }
    Err("找不到合适的 pom.xml 来添加 MyBatis-Plus 依赖".into())
}

/// 在优先级顺序中查找已声明任一 MP starter 的模块。
fn find_starter_module(root: &Path, modules: &[String]) -> Option<String> {
    let starters = [MP_STARTER_BOOT2, MP_STARTER_BOOT3, MP_STARTER_BOOT4];
    for module in &prioritize_modules(modules) {
        let pom = root.join(module).join("pom.xml");
        if let Some(c) = read_text(&pom) {
            if starters.iter().any(|s| c.contains(s)) {
                return Some(module.clone());
            }
        }
    }
    None
}

/// 生成 MybatisPlusConfig.java（幂等：已存在则跳过）
pub fn add_config_class(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let cloud = crate::core::detector::is_cloud_layout(root);
    let (module, pkg_suffix, java_pkg) = if cloud {
        let system = crate::core::detector::find_module_by_leaf_suffix(root, backend_modules, "system")
            .ok_or("Cloud 未找到 system 模块，无法放置 MybatisPlusConfig")?;
        (
            system,
            "system/config".to_string(),
            format!("{}.system.config", params.new_package),
        )
    } else {
        let admin = backend_modules.iter().find(|m| m.ends_with("-admin"))
            .or_else(|| backend_modules.first())
            .ok_or("无后端模块可放置配置类")?;
        (
            admin.clone(),
            "framework/config".to_string(),
            format!("{}.framework.config", params.new_package),
        )
    };
    let pkg_path = package_to_path(&params.new_package);
    let config_dir = root.join(&module).join("src/main/java").join(&pkg_path).join(&pkg_suffix);
    let config_file = config_dir.join("MybatisPlusConfig.java");

    if config_file.exists() {
        log("MybatisPlusConfig.java 已存在，跳过");
        return Ok(false);
    }
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let dialect = crate::core::db_dialect::from_params(params);
    let java = format!(
        "package {java_pkg};\n\nimport com.baomidou.mybatisplus.annotation.DbType;\nimport com.baomidou.mybatisplus.extension.plugins.MybatisPlusInterceptor;\nimport com.baomidou.mybatisplus.extension.plugins.inner.PaginationInnerInterceptor;\nimport org.springframework.context.annotation.Bean;\nimport org.springframework.context.annotation.Configuration;\n\n/**\n * MyBatis-Plus 配置\n */\n@Configuration\npublic class MybatisPlusConfig\n{{\n    /**\n     * 分页插件\n     */\n    @Bean\n    public MybatisPlusInterceptor mybatisPlusInterceptor()\n    {{\n        MybatisPlusInterceptor interceptor = new MybatisPlusInterceptor();\n        interceptor.addInnerInterceptor(new PaginationInnerInterceptor(DbType.{mp_db_type}));\n        return interceptor;\n    }}\n}}\n",
        mp_db_type = dialect.mp_db_type
    );
    std::fs::write(&config_file, java).map_err(|e| format!("写入配置类失败：{e}"))?;
    log(&format!("已生成 {module}/.../{pkg_suffix}/MybatisPlusConfig.java"));
    Ok(true)
}

/// 适配代码生成器模板（Mapper/Service/ServiceImpl/Domain/XML）
/// 返回修改的文件数。幂等：已适配的文件跳过。
pub fn adapt_generator_templates(
    root: &Path,
    generator_files: &[String],
    enable_long_id: bool,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut count = 0usize;
    for rel in generator_files {
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let content = match read_text(&path) {
            Some(c) => c,
            None => continue,
        };
        let new_content = match name.as_str() {
            "mapper.java.vm" => adapt_mapper_vm(&content),
            "service.java.vm" => adapt_service_vm(&content),
            "serviceImpl.java.vm" => adapt_service_impl_vm(&content),
            "domain.java.vm" => adapt_domain_vm(&content, enable_long_id),
            "mapper.xml.vm" => adapt_mapper_xml_vm(&content),
            _ => None,
        };
        if let Some(nc) = new_content {
            if nc != content {
                std::fs::write(&path, nc).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                count += 1;
                log(&format!("已适配代码生成器模板：{rel}"));
            }
        }
    }
    Ok(count)
}

// ---------- 模板改写 ----------

/// Mapper 模板 → 继承 BaseMapper，移除默认 CRUD
fn adapt_mapper_vm(content: &str) -> Option<String> {
    if content.contains("extends BaseMapper") {
        return None; // 已适配
    }
    Some(
        "package ${packageName}.mapper;\n\nimport com.baomidou.mybatisplus.core.mapper.BaseMapper;\nimport ${packageName}.domain.${ClassName};\n\n/**\n * ${functionName}Mapper接口\n */\npublic interface ${ClassName}Mapper extends BaseMapper<${ClassName}>\n{\n}\n".to_string(),
    )
}

/// Service 模板 → 继承 IService
fn adapt_service_vm(content: &str) -> Option<String> {
    if content.contains("extends IService") {
        return None;
    }
    Some(
        "package ${packageName}.service;\n\nimport java.util.List;\nimport ${packageName}.domain.${ClassName};\nimport com.baomidou.mybatisplus.extension.service.IService;\n\n/**\n * ${functionName}Service接口\n */\npublic interface I${ClassName}Service extends IService<${ClassName}>\n{\n}\n".to_string(),
    )
}

/// ServiceImpl 模板 → 继承 ServiceImpl
fn adapt_service_impl_vm(content: &str) -> Option<String> {
    if content.contains("extends ServiceImpl") {
        return None;
    }
    Some(
        "package ${packageName}.service.impl;\n\nimport org.springframework.stereotype.Service;\nimport com.baomidou.mybatisplus.extension.service.impl.ServiceImpl;\nimport ${packageName}.domain.${ClassName};\nimport ${packageName}.mapper.${ClassName}Mapper;\nimport ${packageName}.service.I${ClassName}Service;\n\n/**\n * ${functionName}Service业务层处理\n */\n@Service\npublic class ${ClassName}ServiceImpl extends ServiceImpl<${ClassName}Mapper, ${ClassName}> implements I${ClassName}Service\n{\n}\n".to_string(),
    )
}

/// Domain 模板 → 加 @TableName，Long 主键加 @TableId + @JsonSerialize
fn adapt_domain_vm(content: &str, enable_long_id: bool) -> Option<String> {
    if content.contains("@TableName") {
        return None;
    }
    // 简化：在类声明上方加 @TableName 注解行
    // 定位 "public class ${ClassName}" 行，在其前插入注解与 import
    let mut out = String::new();
    let mut inserted_imports = false;
    let mut inserted_annotation = false;
    for line in content.lines() {
        // 在首个 import 前补充 import（仅一次）
        if !inserted_imports && line.trim_start().starts_with("import ") {
            out.push_str("import com.baomidou.mybatisplus.annotation.TableName;\n");
            out.push_str("import com.baomidou.mybatisplus.annotation.TableId;\n");
            if enable_long_id {
                out.push_str("import com.fasterxml.jackson.databind.annotation.JsonSerialize;\n");
                out.push_str("import com.fasterxml.jackson.databind.ser.std.ToStringSerializer;\n");
            }
            inserted_imports = true;
        }
        // 在 public class 前插入 @TableName
        if !inserted_annotation && line.contains("public class ${ClassName}") {
            out.push_str("@TableName(\"${tableName}\")\n");
            inserted_annotation = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Long 主键注解：在 ${pkColumn} 相关主键字段上添加（保守处理：仅给 Long 主键字段行追加注解）
    if enable_long_id {
        // 对包含 private ${pkColumn.javaType} ${pkColumn.javaField}; 且类型为 Long 的行加注解
        let re = regex::Regex::new(r"(private\s+Long\s+\w+;)").unwrap();
        out = re.replace_all(&out, "@TableId\n@JsonSerialize(using = ToStringSerializer.class)\n    $1").to_string();
    }
    Some(out)
}

/// Mapper XML 模板 → 移除默认 CRUD SQL（保留 resultMap 与自定义查询扩展区）
fn adapt_mapper_xml_vm(content: &str) -> Option<String> {
    if content.contains("<!-- MyBatis-Plus 适配：默认 CRUD 由 BaseMapper 提供") {
        return None;
    }
    // 保守处理：保留文件，仅在头部追加说明注释，不删原有 SQL（避免破坏自定义查询）
    let note = "<!-- MyBatis-Plus 适配：默认 CRUD（insert/update/delete/selectById/selectList）由 BaseMapper 提供，下方保留的 SQL 可作为自定义查询扩展 -->\n";
    Some(format!("{note}{content}"))
}

// ---------- 辅助 ----------

/// 扫描项目中已有的 Mapper/Service/ServiceImpl 源码，改造为 MyBatis-Plus 继承体系。
/// 返回修改的文件数。幂等：已适配的跳过。
///
/// 注意：仅当存在对应 `{Entity}Mapper.java` 时才改造 ServiceImpl
/// （如 SysUserOnline 无 Mapper、数据在 Redis，强行 extends ServiceImpl 会编译失败）。
pub fn adapt_existing_sources(
    root: &Path,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    // 先收集项目中真实存在的 Mapper 实体名，供 ServiceImpl 改造门禁
    let mapper_entities = collect_mapper_entities(root);

    let mut count = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".idea" | "dist")
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() || !path.file_name().map(|n| n.to_string_lossy().ends_with(".java")).unwrap_or(false) {
            continue;
        }
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let content = match read_text(path) {
            Some(c) => c,
            None => continue,
        };

        let new_content = if file_name.ends_with("Mapper.java") {
            adapt_existing_mapper(&content, &file_name)
        } else if file_name.ends_with("ServiceImpl.java") {
            let entity = match extract_entity_name(&file_name) {
                Some(e) => e,
                None => continue,
            };
            // 无对应 Mapper 的 ServiceImpl（如在线用户）跳过，避免引入不存在的 XxxMapper
            if !mapper_entities.contains(&entity) {
                log(&format!(
                    "跳过 MyBatis-Plus ServiceImpl 适配（无 {entity}Mapper）：{}",
                    path.display()
                ));
                None
            } else {
                adapt_current_service_impl(&content, &file_name)
            }
        } else if file_name.starts_with("I") && file_name.ends_with("Service.java") {
            let entity = match extract_entity_name(&file_name) {
                Some(e) => e,
                None => continue,
            };
            // 无对应 Mapper 的 Service（如在线用户）不继承 IService，否则 Impl 必须实现 MP 抽象方法
            if !mapper_entities.contains(&entity) {
                log(&format!(
                    "跳过 MyBatis-Plus Service 适配（无 {entity}Mapper）：{}",
                    path.display()
                ));
                None
            } else {
                adapt_current_service(&content, &file_name, root)
            }
        } else {
            None
        };

        if let Some(nc) = new_content {
            if nc != content {
                std::fs::write(path, &nc).map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                count += 1;
                log(&format!("已适配 MyBatis-Plus：{}", path.display()));
            }
        }
    }
    Ok(count)
}

/// 收集项目中已有的 `*Mapper.java` 对应实体名（SysUserMapper → SysUser）
fn collect_mapper_entities(root: &Path) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".idea" | "dist")
            } else {
                true
            }
        })
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with("Mapper.java") {
            if let Some(entity) = extract_entity_name(&name) {
                set.insert(entity);
            }
        }
    }
    set
}

/// 从文件名提取实体名：SysUserMapper → SysUser, SysUserServiceImpl → SysUser, ISysUserService → SysUser
fn extract_entity_name(file_name: &str) -> Option<String> {
    let stem = file_name.strip_suffix(".java")?;
    if stem.ends_with("Mapper") {
        Some(stem.strip_suffix("Mapper")?.to_string())
    } else if stem.ends_with("ServiceImpl") {
        Some(stem.strip_suffix("ServiceImpl")?.to_string())
    } else if stem.starts_with("I") && stem.ends_with("Service") {
        Some(stem[1..].strip_suffix("Service")?.to_string())
    } else {
        None
    }
}

/// 改造已有 Mapper：加 extends BaseMapper<Entity>
fn adapt_existing_mapper(content: &str, file_name: &str) -> Option<String> {
    if content.contains("extends BaseMapper") {
        return None; // 已适配
    }
    let entity = extract_entity_name(file_name)?;
    let mut out = String::with_capacity(content.len() + 128);
    let mut import_added = false;

    for line in content.lines() {
        // 在首个 import 行前插入 BaseMapper import
        if !import_added && line.trim_start().starts_with("import ") {
            out.push_str("import com.baomidou.mybatisplus.core.mapper.BaseMapper;\n");
            import_added = true;
        }
        // 改造接口声明
        if line.contains(&format!("interface {}", file_name.strip_suffix(".java").unwrap_or(""))) {
            // 替换接口声明，加 extends
            let new_line = line.replace(
                &format!("interface {}", file_name.strip_suffix(".java").unwrap_or("")),
                &format!("interface {} extends BaseMapper<{}>", file_name.strip_suffix(".java").unwrap_or(""), entity),
            );
            out.push_str(&new_line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    Some(out)
}

/// 改造已有 Service 接口：加 extends IService<Entity>，并补齐实体 import。
fn adapt_current_service(content: &str, file_name: &str, root: &Path) -> Option<String> {
    if content.contains("extends IService") {
        return None;
    }
    let entity = extract_entity_name(file_name)?;
    let iface_name = file_name.strip_suffix(".java").unwrap_or("");
    let mut out = String::with_capacity(content.len() + 192);
    let mut import_added = false;
    // 是否已有实体 import（如 import xxx.domain.SysNoticeRead;）
    let has_entity_import = content.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("import ") && t.ends_with(&format!(".{entity};"))
    });
    let entity_import = if has_entity_import {
        None
    } else {
        find_java_package(root, &entity).map(|pkg| format!("import {pkg}.{entity};\n"))
    };

    for line in content.lines() {
        if !import_added && line.trim_start().starts_with("import ") {
            out.push_str("import com.baomidou.mybatisplus.extension.service.IService;\n");
            if let Some(ref ei) = entity_import {
                out.push_str(ei);
            }
            import_added = true;
        }
        if line.contains(&format!("interface {iface_name}")) {
            let new_line = line.replace(
                &format!("interface {iface_name}"),
                &format!("interface {iface_name} extends IService<{entity}>"),
            );
            out.push_str(&new_line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    Some(out)
}

/// 在项目中查找 `{class_name}.java` 的 package 声明
fn find_java_package(root: &Path, class_name: &str) -> Option<String> {
    let target = format!("{class_name}.java");
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(name.as_str(), "target" | "node_modules" | ".git" | ".idea" | "dist")
            } else {
                true
            }
        })
        .flatten()
    {
        if entry.file_name().to_string_lossy() != target.as_str() {
            continue;
        }
        let content = read_text(entry.path())?;
        for line in content.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("package ") {
                return Some(rest.trim_end_matches(';').trim().to_string());
            }
        }
    }
    None
}

/// 改造已有 ServiceImpl：加 extends ServiceImpl<Mapper, Entity> implements IService
fn adapt_current_service_impl(content: &str, file_name: &str) -> Option<String> {
    if content.contains("extends ServiceImpl") {
        return None;
    }
    let entity = extract_entity_name(file_name)?;
    let class_name = file_name.strip_suffix(".java").unwrap_or("");
    // 推断 Mapper 名：SysUserServiceImpl → SysUserMapper
    let mapper_name = format!("{entity}Mapper");
    // 推断 IService 名：SysUser → ISysUserService
    let service_iface = format!("I{entity}Service");

    let mut out = String::with_capacity(content.len() + 256);
    let mut imports_added = false;

    for line in content.lines() {
        if !imports_added && line.trim_start().starts_with("import ") {
            out.push_str("import com.baomidou.mybatisplus.extension.service.impl.ServiceImpl;\n");
            out.push_str(&format!("import com.baomidou.mybatisplus.extension.service.IService;\n"));
            imports_added = true;
        }
        if line.contains(&format!("class {class_name}")) {
            // 查找 implements 子句
            let new_line = if line.contains("implements") {
                // 已有 implements，替换为 extends ServiceImpl + 保留原 implements
                let re = regex::Regex::new(&format!(
                    r"(class\s+{class_name})\s+implements\s+(\w+)"
                )).unwrap();
                re.replace(line, format!("$1 extends ServiceImpl<{mapper_name}, {entity}> implements {service_iface}")).to_string()
            } else {
                // 无 implements，直接加 extends + implements
                let re = regex::Regex::new(&format!(
                    r"(class\s+{class_name})(\s*\{{)"
                )).unwrap();
                re.replace(line, format!("$1 extends ServiceImpl<{mapper_name}, {entity}> implements {service_iface} $2")).to_string()
            };
            out.push_str(&new_line);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    Some(out)
}

/// 模块优先级排序：common > framework > admin > 其余
fn prioritize_modules(modules: &[String]) -> Vec<String> {
    let mut sorted: Vec<String> = modules.to_vec();
    sorted.sort_by_key(|m| match m.as_str() {
        m if m.ends_with("-common") => 0,
        m if m.ends_with("-framework") => 1,
        m if m.ends_with("-admin") => 2,
        _ => 3,
    });
    sorted
}

/// 检查任一 pom 是否已含某关键字
fn any_pom_has(root: &Path, modules: &[String], marker: &str) -> bool {
    for m in modules {
        let pom = root.join(m).join("pom.xml");
        if let Some(c) = read_text(&pom) {
            if c.contains(marker) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造临时项目根，写入给定内容的根 pom.xml
    fn mk_root(root_pom: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), root_pom).unwrap();
        dir
    }

    /// 若依 SB2：根 pom 用 <spring-boot.version> 属性
    #[test]
    fn detect_sb2_via_property() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <properties>
    <spring-boot.version>2.5.15</spring-boot.version>
  </properties>
</project>"#;
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), Some(2));
    }

    /// 若依 SB3：根 pom 用 <spring-boot.version> 属性
    #[test]
    fn detect_sb3_via_property() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <properties>
    <spring-boot.version>3.2.4</spring-boot.version>
  </properties>
</project>"#;
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), Some(3));
    }

    /// parent 继承形式：spring-boot-starter-parent 的 version
    #[test]
    fn detect_via_parent_version() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>2.7.18</version>
  </parent>
</project>"#;
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), Some(2));
    }

    /// 无版本信息 → None（oss 会默认 jakarta）
    #[test]
    fn detect_returns_none_when_no_version() {
        let pom = "<?xml version=\"1.0\"?>\n<project><artifactId>x</artifactId></project>";
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), None);
    }

    /// 子模块 pom 也能被扫描（根 pom 无版本，子模块有）
    #[test]
    fn detect_from_submodule_pom() {
        let dir = tempfile::tempdir().unwrap();
        // 根 pom 无版本信息
        std::fs::write(dir.path().join("pom.xml"), "<project><modules><module>ruoyi-admin</module></modules></project>").unwrap();
        // 子模块 pom 带 spring-boot.version
        let admin = dir.path().join("ruoyi-admin");
        std::fs::create_dir_all(&admin).unwrap();
        std::fs::write(admin.join("pom.xml"), "<project><properties><spring-boot.version>2.5.15</spring-boot.version></properties></project>").unwrap();
        assert_eq!(detect_boot_major_version(dir.path()), Some(2));
    }

    #[test]
    fn select_starter_matches_boot_major() {
        assert_eq!(select_starter(Some(2)), MP_STARTER_BOOT2);
        assert_eq!(select_starter(Some(1)), MP_STARTER_BOOT2);
        assert_eq!(select_starter(Some(3)), MP_STARTER_BOOT3);
        assert_eq!(select_starter(Some(4)), MP_STARTER_BOOT4);
        // 检测不到默认 Boot 4
        assert_eq!(select_starter(None), MP_STARTER_BOOT4);
    }

    #[test]
    fn select_jsqlparser_matches_boot_major() {
        assert_eq!(select_jsqlparser(Some(2)), MP_JSQLPARSER_JDK8);
        assert_eq!(select_jsqlparser(Some(1)), MP_JSQLPARSER_JDK8);
        assert_eq!(select_jsqlparser(Some(3)), MP_JSQLPARSER);
        assert_eq!(select_jsqlparser(Some(4)), MP_JSQLPARSER);
        assert_eq!(select_jsqlparser(None), MP_JSQLPARSER);
    }

    /// Boot 4：根 pom 用 <spring-boot.version> 属性
    #[test]
    fn detect_sb4_via_property() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <properties>
    <spring-boot.version>4.0.0</spring-boot.version>
  </properties>
</project>"#;
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), Some(4));
    }

    /// Boot 4 parent 继承形式
    #[test]
    fn detect_sb4_via_parent_version() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>4.0.1</version>
  </parent>
</project>"#;
        let dir = mk_root(pom);
        assert_eq!(detect_boot_major_version(dir.path()), Some(4));
    }
}
