// MyBatis-Plus 集成：依赖添加、配置类生成、代码生成器模板适配、Long ID 序列化。
//
// 设计：幂等（已存在则跳过，不重复添加）；不破坏原 mybatis 配置；模板改造保守替换。
// 依赖优先加到公共模块（common/framework），配置类放 admin 模块新包路径下。

use crate::core::CustomizeParams;
use crate::utils::path::package_to_path;
use std::path::Path;

/// MyBatis-Plus 依赖版本（默认）
const MP_VERSION: &str = "3.5.7";

/// Boot 2.x 用 starter（面向 Spring 5）
const MP_STARTER_BOOT2: &str = "mybatis-plus-boot-starter";
/// Boot 3.x 用 starter（面向 Spring 6 / Jakarta EE）
const MP_STARTER_BOOT3: &str = "mybatis-plus-spring-boot3-starter";

/// 检测项目使用的 Spring Boot 大版本。
///
/// 扫描根 pom（含子模块 pom）的 `spring-boot-starter-parent` 版本及
/// `<spring-boot.version>` 属性，返回主版本号（如 2 / 3）。检测不到返回 None。
///
/// 用途：MyBatis-Plus 的 starter 必须与 Boot 大版本匹配，否则会带入与 Spring 不兼容的
/// `mybatis-spring`（Boot 2 starter 带 mybatis-spring 2.x，与 Spring 6 不兼容，登录全挂）。
pub fn detect_boot_major_version(root: &Path) -> Option<u32> {
    // 候选 pom：根 pom + 一级子模块 pom
    let mut pom_paths: Vec<std::path::PathBuf> = vec![root.join("pom.xml")];
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path().join("pom.xml");
            if p.is_file() {
                pom_paths.push(p);
            }
        }
    }
    for pom in &pom_paths {
        let content = match std::fs::read_to_string(pom) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // 1) <spring-boot.version>3.x</spring-boot.version> 属性
        if let Some(v) = extract_version_after(&content, "<spring-boot.version>") {
            return major_of(&v);
        }
        // 2) spring-boot-starter-parent 的 <version>
        //    形如 <parent>...<artifactId>spring-boot-starter-parent</artifactId><version>3.2.4</version>
        if let Some(idx) = content.find("spring-boot-starter-parent") {
            let tail = &content[idx..];
            if let Some(v) = extract_version_after(tail, "<version>") {
                return major_of(&v);
            }
        }
    }
    None
}

/// 在 content 中找到 tag 后，提取紧随其后的版本号文本（到下一个 < 为止）
fn extract_version_after(content: &str, tag: &str) -> Option<String> {
    let idx = content.find(tag)?;
    let after = &content[idx + tag.len()..];
    let end = after.find('<')?;
    Some(after[..end].trim().to_string())
}

/// 从版本号字符串取主版本号（如 "3.2.4" → 3）
fn major_of(version: &str) -> Option<u32> {
    version.split('.').next()?.parse::<u32>().ok()
}

/// 按 Boot 大版本选择 MyBatis-Plus starter artifactId。
/// 检测不到版本时默认 Boot 3（现代若依多为 Boot 3，避免重复踩坑）。
fn select_starter(boot_major: Option<u32>) -> &'static str {
    match boot_major {
        Some(major) if major < 3 => MP_STARTER_BOOT2,
        _ => MP_STARTER_BOOT3, // >=3 或检测不到
    }
}

/// 添加 MyBatis-Plus 依赖到公共模块 pom（幂等：已存在则跳过）。
/// 返回是否实际添加。
pub fn add_dependency(root: &Path, backend_modules: &[String], log: &dyn Fn(&str)) -> Result<bool, String> {
    // 两个 starter 名都视为「已有依赖」（幂等检查兼容老项目可能已注入 Boot 2 starter）
    let dep_markers = [MP_STARTER_BOOT2, MP_STARTER_BOOT3];
    let boot_major = detect_boot_major_version(root);
    let artifact = select_starter(boot_major);
    // 候选模块优先级：common > framework > admin > 任意
    let candidates = prioritize_modules(backend_modules);
    for module in &candidates {
        let pom = root.join(module).join("pom.xml");
        if !pom.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&pom).map_err(|e| format!("读取 {} 失败：{e}", pom.display()))?;
        // 幂等：项目任意 pom 已有任一 MyBatis-Plus starter 则不再添加
        if dep_markers.iter().any(|m| any_pom_has(root, backend_modules, m)) {
            log(&format!("MyBatis-Plus 依赖已存在，跳过"));
            return Ok(false);
        }
        // 在 <dependencies> 后插入（若无 <dependencies> 则在 </project> 前插入整个块）
        let dep_block = format!(
            "\n    <dependency>\n        <groupId>com.baomidou</groupId>\n        <artifactId>{artifact}</artifactId>\n        <version>{ver}</version>\n    </dependency>\n",
            artifact = artifact,
            ver = MP_VERSION
        );
        let new_content = if let Some(idx) = content.find("<dependencies>") {
            let mut s = String::with_capacity(content.len() + dep_block.len());
            s.push_str(&content[..idx + "<dependencies>".len()]);
            s.push_str(&dep_block);
            s.push_str(&content[idx + "<dependencies>".len()..]);
            s
        } else {
            // 无 dependencies 节点，插在 </project> 前
            content.replace("</project>", &format!("    <dependencies>{dep_block}    </dependencies>\n</project>"))
        };
        std::fs::write(&pom, new_content).map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
        log(&format!(
            "已在 {module}/pom.xml 添加 {artifact}:{MP_VERSION}（Spring Boot {}）",
            boot_major.map_or("版本未知，默认按 Boot 3".to_string(), |m| format!("{m}.x"))
        ));
        return Ok(true);
    }
    Err("找不到合适的 pom.xml 来添加 MyBatis-Plus 依赖".into())
}

/// 生成 MybatisPlusConfig.java（幂等：已存在则跳过）
pub fn add_config_class(
    root: &Path,
    params: &CustomizeParams,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let admin = backend_modules.iter().find(|m| m.ends_with("-admin"))
        .or_else(|| backend_modules.first())
        .ok_or("无后端模块可放置配置类")?;
    let pkg_path = package_to_path(&params.new_package);
    let config_dir = root.join(admin).join("src/main/java").join(&pkg_path).join("framework/config");
    let config_file = config_dir.join("MybatisPlusConfig.java");

    if config_file.exists() {
        log("MybatisPlusConfig.java 已存在，跳过");
        return Ok(false);
    }
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败：{e}"))?;
    let java = format!(
        "package {pkg}.framework.config;\n\nimport com.baomidou.mybatisplus.annotation.DbType;\nimport com.baomidou.mybatisplus.extension.plugins.MybatisPlusInterceptor;\nimport com.baomidou.mybatisplus.extension.plugins.inner.PaginationInnerInterceptor;\nimport org.springframework.context.annotation.Bean;\nimport org.springframework.context.annotation.Configuration;\n\n/**\n * MyBatis-Plus 配置\n */\n@Configuration\npublic class MybatisPlusConfig\n{{\n    /**\n     * 分页插件\n     */\n    @Bean\n    public MybatisPlusInterceptor mybatisPlusInterceptor()\n    {{\n        MybatisPlusInterceptor interceptor = new MybatisPlusInterceptor();\n        interceptor.addInnerInterceptor(new PaginationInnerInterceptor(DbType.MYSQL));\n        return interceptor;\n    }}\n}}\n",
        pkg = params.new_package
    );
    std::fs::write(&config_file, java).map_err(|e| format!("写入配置类失败：{e}"))?;
    log(&format!("已生成 {admin}/.../framework/config/MybatisPlusConfig.java"));
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
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
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
pub fn adapt_existing_sources(
    root: &Path,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut count = 0usize;
    // 扫描所有 .java 文件
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
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let new_content = if file_name.ends_with("Mapper.java") {
            adapt_existing_mapper(&content, &file_name)
        } else if file_name.ends_with("ServiceImpl.java") {
            adapt_current_service_impl(&content, &file_name)
        } else if file_name.starts_with("I") && file_name.ends_with("Service.java") {
            adapt_current_service(&content, &file_name)
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

/// 改造已有 Service 接口：加 extends IService<Entity>
fn adapt_current_service(content: &str, file_name: &str) -> Option<String> {
    if content.contains("extends IService") {
        return None;
    }
    let entity = extract_entity_name(file_name)?;
    let iface_name = file_name.strip_suffix(".java").unwrap_or("");
    let mut out = String::with_capacity(content.len() + 128);
    let mut import_added = false;

    for line in content.lines() {
        if !import_added && line.trim_start().starts_with("import ") {
            out.push_str("import com.baomidou.mybatisplus.extension.service.IService;\n");
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
        if let Ok(c) = std::fs::read_to_string(&pom) {
            if c.contains(marker) {
                return true;
            }
        }
    }
    false
}
