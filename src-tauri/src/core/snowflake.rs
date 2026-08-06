// 全局雪花 ID 集成：Hutool 依赖注入 + ServiceImpl 主 insert 注入主键 setter + domain 主键 IdType.INPUT。
//
// 设计（与 mybatis_plus 同构的"模板驱动 + 源码扫描"模式）：
// - 依赖：cn.hutool:hutool-all 加到公共模块 pom（幂等，已有则跳过）
// - 模板：serviceImpl.java.vm 的 insert 方法体注入
//         ${className}.set${pkColumn.capJavaField}(IdUtil.getSnowflakeNextId())
// - 源码：仅对「主实体 insert」（如 insertUser / insertSysUser）注入；
//         setter 取自同文件 selectXxx(Long pkField) 的参数名（如 userId → setUserId），
//         禁止误用通用 setId（若依实体主键是 userId/configId 等业务字段）
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

/// 改造代码生成器模板 serviceImpl.java.vm：insert 方法体首行注入雪花主键赋值。
/// 使用若依模板变量 ${pkColumn.capJavaField}，对应 setUserId / setConfigId 等真实 setter。
/// 幂等：模板已含 IdUtil 则返回 None。
pub fn inject_snowflake_to_service_impl_vm(content: &str) -> Option<String> {
    if content.contains("IdUtil") {
        return None;
    }
    // 若依代码生成器模板的 insert 方法形如：
    //   public int insert${ClassName}(${ClassName} ${className}) {
    let re = regex::Regex::new(
        r"(public\s+int\s+insert\$\{ClassName\}\(\$\{ClassName\}\s+\$\{className\}\)\s*\{)",
    )
    .ok()?;
    if !re.is_match(content) {
        return None;
    }
    let new_content = re.replace(
        content,
        // $$ 转义为字面 $；pkColumn.capJavaField → UserId / ConfigId
        "$1\n        $${className}.set$${pkColumn.capJavaField}(cn.hutool.core.util.IdUtil.getSnowflakeNextId());",
    )
    .to_string();
    Some(new_content)
}

/// 扫描项目中已有的 *ServiceImpl.java 源码，对主 insert 方法注入雪花主键赋值。
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

/// 改造单个 ServiceImpl 源码：仅对主实体 insert 注入 `{var}.set{Pk}(...)`。
///
/// 规则：
/// 1. 方法名必须是 insert{Type} 或 insert{Type去掉Sys前缀}（insertRole ✓，insertRoleMenu ✗）
/// 2. 主键 setter 按「当前实体」从同文件 `selectXxxById(Long pk)` / `public Entity selectXxx(Long pk)` 推导
///    （禁止取第一个任意 select(Long)，否则会把 selectDeptListByRoleId 的 roleId 误当成部门主键）
/// 3. 推导失败则跳过该 insert（避免写入错误 setter）
///
/// 幂等：文件已含 IdUtil 则返回 None。
pub fn inject_snowflake_to_source(content: &str) -> Option<String> {
    if content.contains("IdUtil") {
        return None;
    }
    // group1 = 签名+{；group2 = 方法名；group3 = 参数类型；group4 = 参数变量名
    let re = regex::Regex::new(
        r"(public\s+int\s+(insert\w+)\(\s*(\w+)\s+(\w+)\s*\)\s*\{)",
    )
    .ok()?;
    if !re.is_match(content) {
        return None;
    }
    let mut any = false;
    let new_content = re
        .replace_all(content, |caps: &regex::Captures| {
            let sig = &caps[1];
            let method = &caps[2];
            let type_name = &caps[3];
            let var = &caps[4];
            if !is_primary_insert(method, type_name) {
                return sig.to_string();
            }
            match resolve_pk_setter_for_entity(content, type_name) {
                Some(pk_setter) => {
                    any = true;
                    format!(
                        "{sig}\n        {var}.{pk_setter}(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"
                    )
                }
                None => sig.to_string(),
            }
        })
        .to_string();
    if any {
        Some(new_content)
    } else {
        None
    }
}

/// 判断是否为「主实体新增」方法（需写雪花主键），排除 insertRoleMenu / insertUserPost 等关联插入。
fn is_primary_insert(method: &str, type_name: &str) -> bool {
    let short = type_name.strip_prefix("Sys").unwrap_or(type_name);
    method == format!("insert{type_name}") || method == format!("insert{short}")
}

/// 按实体类型推导主键 setter。
///
/// 优先顺序（避免误用 ByRoleId / ByUserId 等关联查询参数）：
/// 1. `select{Entity|Short}ById(Long field)` —— 若依标准按主键查询
/// 2. `public {Entity} selectXxx(Long field)` —— 返回实体本身的单 Long 参数查询
fn resolve_pk_setter_for_entity(content: &str, type_name: &str) -> Option<String> {
    let short = type_name.strip_prefix("Sys").unwrap_or(type_name);

    // 1) selectDeptById / selectSysDeptById / selectDictDataById
    let re_by_id = regex::Regex::new(&format!(
        r"select(?:{type_name}|{short})ById\(\s*Long\s+(\w+)\s*\)"
    ))
    .ok()?;
    if let Some(caps) = re_by_id.captures(content) {
        return Some(field_to_setter(&caps[1]));
    }

    // 2) 返回类型就是该实体：public SysDept selectXxx(Long field)
    let re_ret = regex::Regex::new(&format!(
        r"public\s+{type_name}\s+select\w+\(\s*Long\s+(\w+)\s*\)"
    ))
    .ok()?;
    if let Some(caps) = re_ret.captures(content) {
        return Some(field_to_setter(&caps[1]));
    }

    None
}

fn field_to_setter(field: &str) -> String {
    format!("set{}", capitalize_first(field))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 把 domain 模板/源码里 Long 主键的 @TableId 标记为 IdType.INPUT。
/// 仅当雪花 ID 与 MyBatis-Plus 同时开启时调用，避免 MP 自动分配与手动 setId 冲突。
/// 幂等：已含 IdType.INPUT 则跳过。返回是否修改。
pub fn mark_domain_idtype_input(content: &str) -> Option<String> {
    // 已是 INPUT 策略则跳过
    if content.contains("IdType.INPUT") {
        return None;
    }
    let mut out = content.to_string();
    let changed;
    let re_typed = regex::Regex::new(r"@TableId\s*\(\s*type\s*=\s*\w+(\.\w+)?\s*\)").ok()?;
    if re_typed.is_match(&out) {
        out = re_typed
            .replace_all(&out, "@TableId(type = IdType.INPUT)")
            .to_string();
        changed = true;
    } else if out.contains("@TableId") {
        out = out.replace("@TableId\n", "@TableId(type = IdType.INPUT)\n");
        changed = true;
    } else {
        changed = false;
    }
    if !changed {
        return None;
    }
    if !out.contains("import com.baomidou.mybatisplus.annotation.IdType;") {
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
    sorted.sort_by_key(|m| {
        if m.contains("common") {
            0
        } else if m.contains("framework") {
            1
        } else if m.contains("admin") {
            2
        } else {
            3
        }
    });
    sorted
}

fn any_pom_has(root: &Path, modules: &[String], needle: &str) -> bool {
    let mut paths = vec![root.join("pom.xml")];
    for m in modules {
        paths.push(root.join(m).join("pom.xml"));
    }
    for p in paths {
        if let Ok(c) = std::fs::read_to_string(p) {
            if c.contains(needle) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- VM 注入 ----------

    #[test]
    fn vm_inject_uses_pk_column_setter() {
        let vm = "package ${packageName}.service.impl;\npublic class ${ClassName}ServiceImpl {\n    public int insert${ClassName}(${ClassName} ${className}) {\n        ${className}.setCreateTime(new Date());\n        return ${className}Mapper.insert${ClassName}(${className});\n    }\n}\n";
        let out = inject_snowflake_to_service_impl_vm(vm).unwrap();
        assert!(out.contains(
            "${className}.set${pkColumn.capJavaField}(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"
        ));
        assert!(!out.contains(".setId("), "不应再使用通用 setId");
    }

    #[test]
    fn vm_inject_is_idempotent() {
        let vm = "public int insert${ClassName}(${ClassName} ${className}) {\n}\n";
        let once = inject_snowflake_to_service_impl_vm(vm).unwrap();
        assert!(inject_snowflake_to_service_impl_vm(&once).is_none());
    }

    #[test]
    fn vm_inject_returns_none_when_no_insert() {
        let vm = "public class FooServiceImpl {\n    public List<Foo> selectList() { return null; }\n}\n";
        assert!(inject_snowflake_to_service_impl_vm(vm).is_none());
    }

    // ---------- 源码注入 ----------

    #[test]
    fn source_inject_uses_real_pk_setter() {
        let src = r#"
public class SysUserServiceImpl {
    public SysUser selectUserById(Long userId) { return null; }
    public int insertUser(SysUser user) {
        user.setCreateTime(new Date());
        return userMapper.insertUser(user);
    }
}
"#;
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("user.setUserId(cn.hutool.core.util.IdUtil.getSnowflakeNextId());"));
        assert!(!out.contains("user.setId("));
    }

    #[test]
    fn source_inject_prefers_by_id_over_association_select() {
        // selectDeptListByRoleId 排在 selectDeptById 前面时，仍应取 deptId
        let src = r#"
public class SysDeptServiceImpl {
    public List<Long> selectDeptListByRoleId(Long roleId) { return null; }
    public SysDept selectDeptById(Long deptId) { return null; }
    public int insertDept(SysDept dept) {
        return deptMapper.insertDept(dept);
    }
}
"#;
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("dept.setDeptId("));
        assert!(!out.contains("dept.setRoleId("));
    }

    #[test]
    fn source_inject_menu_ignores_user_id_selects() {
        let src = r#"
public class SysMenuServiceImpl {
    public List<SysMenu> selectMenuList(Long userId) { return null; }
    public SysMenu selectMenuById(Long menuId) { return null; }
    public int insertMenu(SysMenu menu) {
        return menuMapper.insertMenu(menu);
    }
}
"#;
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("menu.setMenuId("));
        assert!(!out.contains("menu.setUserId("));
    }

    #[test]
    fn source_inject_skips_association_inserts() {
        let src = r#"
public class SysRoleServiceImpl {
    public List<SysRole> selectRolesByUserId(Long userId) { return null; }
    public SysRole selectRoleById(Long roleId) { return null; }
    public int insertRole(SysRole role) {
        return roleMapper.insertRole(role);
    }
    public int insertRoleMenu(SysRole role) {
        return 1;
    }
}
"#;
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("role.setRoleId("));
        assert!(!out.contains("role.setUserId("));
        let after_menu = out.split("insertRoleMenu").nth(1).unwrap_or("");
        assert!(
            !after_menu.contains("IdUtil"),
            "关联 insert 不应注入雪花: {after_menu}"
        );
    }

    #[test]
    fn source_inject_handles_config_id() {
        let src = r#"
public class SysConfigServiceImpl {
    public SysConfig selectConfigById(Long configId) { return null; }
    public int insertConfig(SysConfig config) {
        return configMapper.insertConfig(config);
    }
}
"#;
        let out = inject_snowflake_to_source(src).unwrap();
        assert!(out.contains("config.setConfigId("));
    }

    #[test]
    fn source_inject_is_idempotent() {
        let src = r#"
public class SysUserServiceImpl {
    public SysUser selectUserById(Long userId) { return null; }
    public int insertUser(SysUser user) {
        return userMapper.insertUser(user);
    }
}
"#;
        let once = inject_snowflake_to_source(src).unwrap();
        assert!(inject_snowflake_to_source(&once).is_none());
    }

    #[test]
    fn source_inject_skips_when_no_pk_hint() {
        // 没有 selectXxxById / 返回实体的 select(Long) 时无法安全推导
        let src = "public int insertOrder(Order orderEntity) {\n    return orderMapper.insertOrder(orderEntity);\n}\n";
        assert!(inject_snowflake_to_source(src).is_none());
    }

    #[test]
    fn is_primary_insert_rules() {
        assert!(is_primary_insert("insertRole", "SysRole"));
        assert!(is_primary_insert("insertSysRole", "SysRole"));
        assert!(is_primary_insert("insertUser", "SysUser"));
        assert!(!is_primary_insert("insertRoleMenu", "SysRole"));
        assert!(!is_primary_insert("insertUserPost", "SysUser"));
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
        let modules = vec!["demo-common".into()];
        let log = |_m: &str| {};
        assert!(add_hutool_dependency(tmp.path(), &modules, &log).unwrap());
        let pom = std::fs::read_to_string(common.join("pom.xml")).unwrap();
        assert!(pom.contains("hutool-all"));
        assert!(!add_hutool_dependency(tmp.path(), &modules, &log).unwrap());
    }
}
