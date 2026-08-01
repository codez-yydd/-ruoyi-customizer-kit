// 全局雪花 ID 集成：Hutool 依赖注入 + ServiceImpl insert 方法 setId + domain 主键 IdType.INPUT。
//
// 设计（与 mybatis_plus 同构的"模板驱动 + 源码扫描"模式）：
// - 依赖：cn.hutool:hutool-all 加到公共模块 pom（幂等，已有则跳过）
// - 模板：serviceImpl.java.vm 的 insert 方法体首行注入 ${className}.setId(IdUtil.getSnowflakeNextId())
// - 源码：扫描 *ServiceImpl.java，对 public int insertXxx(Xxx xxx) { 注入 xxx.setId(...)
// - 幂等：文件已含 IdUtil 则整体跳过，不重复注入
//
// 与 MyBatis-Plus 共存时：domain 主键注解标 @TableId(type = IdType.INPUT)，
// 表示主键由代码手动赋值，避免 MP 的 ASSIGN_ID 与 Hutool 雪花重复分配。

use std::path::Path;

/// Hutool 依赖版本（默认）
const HUTOOL_VERSION: &str = "5.8.32";

/// 添加 Hutool 依赖到公共模块 pom（幂等：项目任意 pom 已有 hutool 则跳过）。
/// 返回是否实际添加。
pub fn add_hutool_dependency(
    root: &Path,
    backend_modules: &[String],
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    // 幂等：扫描所有候选 pom，任一已含 hutool 则跳过
    if any_pom_has(root, backend_modules, "cn.hutool") {
        log("Hutool 依赖已存在，跳过");
        return Ok(false);
    }
    // 候选模块优先级：common > framework > admin > 任意
    let candidates = prioritize_modules(backend_modules);
    for module in &candidates {
        let pom = root.join(module).join("pom.xml");
        if !pom.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&pom)
            .map_err(|e| format!("读取 {} 失败：{e}", pom.display()))?;
        let dep_block = format!(
            "\n    <dependency>\n        <groupId>cn.hutool</groupId>\n        <artifactId>hutool-all</artifactId>\n        <version>{ver}</version>\n    </dependency>\n",
            ver = HUTOOL_VERSION
        );
        let new_content = if let Some(idx) = content.find("<dependencies>") {
            let mut s = String::with_capacity(content.len() + dep_block.len());
            s.push_str(&content[..idx + "<dependencies>".len()]);
            s.push_str(&dep_block);
            s.push_str(&content[idx + "<dependencies>".len()..]);
            s
        } else {
            content.replace(
                "</project>",
                &format!("    <dependencies>{dep_block}    </dependencies>\n</project>"),
            )
        };
        std::fs::write(&pom, new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", pom.display()))?;
        log(&format!(
            "已在 {module}/pom.xml 添加 hutool-all:{HUTOOL_VERSION}"
        ));
        return Ok(true);
    }
    Err("找不到合适的 pom.xml 来添加 Hutool 依赖".into())
}

/// 改造代码生成器模板 serviceImpl.java.vm：insert 方法体首行注入雪花 ID 赋值。
/// 幂等：模板已含 IdUtil 则返回 None。
pub fn inject_snowflake_to_service_impl_vm(content: &str) -> Option<String> {
    if content.contains("IdUtil") {
        return None;
    }
    // 若依代码生成器模板的 insert 方法形如：
    //   public int insert${ClassName}(${ClassName} ${className}) {
    // 在方法体 { 后插入赋值
    let re = regex::Regex::new(
        r"(public\s+int\s+insert\$\{ClassName\}\(\$\{ClassName\}\s+\$\{className\}\)\s*\{)",
    )
    .ok()?;
    // 仅替换首个 insert 方法（每个 ServiceImpl 模板通常只有一个 insert）
    if !re.is_match(content) {
        return None;
    }
    let new_content = re.replace(
        content,
        // $$ 转义为字面 $，使 ${className} 作为模板变量原样输出
        "$1\n        $${className}.setId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());",
    ).to_string();
    Some(new_content)
}

/// 扫描项目中已有的 *ServiceImpl.java 源码，对 insert 方法注入雪花 ID 赋值。
/// 返回修改的文件数。幂等：文件已含 IdUtil 则跳过。
pub fn inject_snowflake_to_existing_sources(
    root: &Path,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut count = 0usize;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy().to_string();
                !matches!(
                    name.as_str(),
                    "target" | "node_modules" | ".git" | ".idea" | "dist"
                )
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !path.is_file()
            || !path
                .file_name()
                .map(|n| n.to_string_lossy().ends_with("ServiceImpl.java"))
                .unwrap_or(false)
        {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(new_content) = inject_snowflake_to_source(&content) {
            if new_content != content {
                std::fs::write(path, &new_content)
                    .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
                count += 1;
                log(&format!("已注入雪花 ID：{}", path.display()));
            }
        }
    }
    Ok(count)
}

/// 改造单个 ServiceImpl 源码：对 public int insertXxx(Xxx xxx) { 注入 xxx.setId(...)。
/// 幂等：文件已含 IdUtil 则返回 None。
pub fn inject_snowflake_to_source(content: &str) -> Option<String> {
    if content.contains("IdUtil") {
        return None;
    }
    // 匹配：public int insertXxx(Xxx xxx) {
    // group1 = 整个方法签名+{
    // group2 = 参数变量名（xxx）
    let re = regex::Regex::new(
        r"(public\s+int\s+insert\w+\(\s*\w+\s+(\w+)\s*\)\s*\{)",
    )
    .ok()?;
    if !re.is_match(content) {
        return None;
    }
    let new_content = re.replace_all(content, |caps: &regex::Captures| {
        let sig = &caps[1];
        let var = &caps[2];
        format!("{sig}\n        {var}.setId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());")
    }).to_string();
    Some(new_content)
}

/// 把 domain 模板/源码里 Long 主键的 @TableId 标记为 IdType.INPUT。
/// 仅当雪花 ID 与 MyBatis-Plus 同时开启时调用，避免 MP 自动分配与手动 setId 冲突。
/// 幂等：已含 IdType.INPUT 则跳过。返回是否修改。
pub fn mark_domain_idtype_input(content: &str) -> Option<String> {
    // 已是 INPUT 策略则跳过
    if content.contains("IdType.INPUT") {
        return None;
    }
    // 分两种情况：
    // 1) 已有 @TableId（MP 已加）→ 改为 @TableId(type = IdType.INPUT)
    // 2) 仅有 @TableId(type = ...) 其他类型 → 替换为 INPUT
    // 同时确保 import 了 IdType
    let mut out = content.to_string();
    let changed;
    // 情况：@TableId(type = ASSIGN_ID/xxx) 或 @TableId(type=AUTO)
    let re_typed = regex::Regex::new(r"@TableId\s*\(\s*type\s*=\s*\w+(\.\w+)?\s*\)").ok()?;
    if re_typed.is_match(&out) {
        out = re_typed
            .replace_all(&out, "@TableId(type = IdType.INPUT)")
            .to_string();
        changed = true;
    } else if out.contains("@TableId") {
        // 裸 @TableId（无 type）→ 改为带 type
        out = out.replace("@TableId\n", "@TableId(type = IdType.INPUT)\n");
        changed = true;
    } else {
        changed = false;
    }
    if !changed {
        return None;
    }
    // 补 IdType import（若缺）
    if !out.contains("import com.baomidou.mybatisplus.annotation.IdType;") {
        // 在首个 import 前插入
        if let Some(idx) = out.find("import ") {
            let mut s = String::with_capacity(out.len() + 64);
            s.push_str(&out[..idx]);
            s.push_str("import com.baomidou.mybatisplus.annotation.IdType;\n");
            s.push_str(&out[idx..]);
            out = s;
        }
    }
    Some(out)
}

// ---------- 辅助 ----------

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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- 模板注入 ----------

    #[test]
    fn vm_inject_adds_setid_into_insert_body() {
        let vm = "package ${packageName}.service.impl;\npublic class ${ClassName}ServiceImpl {\n    public int insert${ClassName}(${ClassName} ${className}) {\n        ${className}.setCreateTime(new Date());\n        return ${className}Mapper.insert${ClassName}(${className});\n    }\n}\n";
        let out = inject_snowflake_to_service_impl_vm(vm).unwrap();
        assert!(out.contains("${className}.setId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"));
        // 注入位置：在 { 之后、setCreateTime 之前
        let idx_brace = out.find("insert${ClassName}(${ClassName} ${className}) {").unwrap()
            + "insert${ClassName}(${ClassName} ${className}) {".len();
        let idx_setid = out.find("setId").unwrap();
        assert!(idx_setid > idx_brace, "setId 应在 insert 方法体 {{ 之后");
    }

    #[test]
    fn vm_inject_is_idempotent() {
        let vm = "public int insert${ClassName}(${ClassName} ${className}) {\n}\n";
        let once = inject_snowflake_to_service_impl_vm(vm).unwrap();
        // 已含 IdUtil → 第二次返回 None
        assert!(inject_snowflake_to_service_impl_vm(&once).is_none());
    }

    #[test]
    fn vm_inject_returns_none_when_no_insert() {
        let vm = "public class FooServiceImpl {\n    public List<Foo> selectList() { return null; }\n}\n";
        assert!(inject_snowflake_to_service_impl_vm(vm).is_none());
    }

    // ---------- 源码注入 ----------

    #[test]
    fn source_inject_extracts_param_var_and_injects() {
        let src = "public class SysUserServiceImpl {\n    public int insertSysUser(SysUser user) {\n        user.setCreateTime(new Date());\n        return userMapper.insertSysUser(user);\n    }\n}\n";
        let out = inject_snowflake_to_source(src).unwrap();
        // 变量名 user 应被正确提取并用于 setId
        assert!(out.contains("user.setId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"));
        assert!(!out.contains("xxx.setId"), "不应硬编码变量名");
    }

    #[test]
    fn source_inject_handles_different_param_name() {
        let src = "public int insertOrder(Order orderEntity) {\n    return orderMapper.insertOrder(orderEntity);\n}\n";
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("orderEntity.setId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"));
    }

    #[test]
    fn source_inject_is_idempotent() {
        let src = "public int insertSysUser(SysUser user) {\n    return userMapper.insertSysUser(user);\n}\n";
        let once = inject_snowflake_to_source(src).unwrap();
        // 已含 IdUtil → 第二次返回 None
        assert!(inject_snowflake_to_source(&once).is_none());
    }

    #[test]
    fn source_inject_skips_when_no_insert_method() {
        let src = "public class SysUserServiceImpl {\n    public List<SysUser> selectList() { return null; }\n}\n";
        assert!(inject_snowflake_to_source(src).is_none());
    }

    // ---------- domain IdType.INPUT ----------

    #[test]
    fn mark_input_replaces_typed_tableid() {
        let domain = "import com.baomidou.mybatisplus.annotation.TableId;\n@TableId(type = IdType.AUTO)\nprivate Long id;\n";
        let out = mark_domain_idtype_input(domain).unwrap();
        assert!(out.contains("@TableId(type = IdType.INPUT)"));
        assert!(out.contains("import com.baomidou.mybatisplus.annotation.IdType;"));
        assert!(!out.contains("IdType.AUTO"));
    }

    #[test]
    fn mark_input_replaces_plain_tableid() {
        let domain = "@TableId\nprivate Long userId;\n";
        let out = mark_domain_idtype_input(domain).unwrap();
        assert!(out.contains("@TableId(type = IdType.INPUT)"));
    }

    #[test]
    fn mark_input_is_idempotent() {
        let domain = "@TableId(type = IdType.INPUT)\nprivate Long id;\n";
        assert!(mark_domain_idtype_input(domain).is_none());
    }

    // ---------- hutool 依赖 ----------

    #[test]
    fn add_hutool_dependency_writes_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let common = tmp.path().join("demo-common");
        std::fs::create_dir_all(&common).unwrap();
        std::fs::write(
            common.join("pom.xml"),
            "<project>\n  <dependencies>\n  </dependencies>\n</project>",
        )
        .unwrap();
        let modules = vec!["demo-common".to_string()];

        let first = add_hutool_dependency(tmp.path(), &modules, &|_| {}).unwrap();
        assert!(first, "首次应添加");
        let content = std::fs::read_to_string(common.join("pom.xml")).unwrap();
        assert!(content.contains("<artifactId>hutool-all</artifactId>"));
        assert!(content.contains("cn.hutool"));

        // 幂等：第二次应跳过
        let second = add_hutool_dependency(tmp.path(), &modules, &|_| {}).unwrap();
        assert!(!second, "已存在 hutool 应跳过");
    }
}
