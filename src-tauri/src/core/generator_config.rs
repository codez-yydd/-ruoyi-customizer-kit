// 代码生成器配置定制：generator.yml 字段改写 + Vue3 模板升级。
//
// 设计：
// - generator.yml：若依 ruoyi-generator/src/main/resources/generator.yml，含 author/packageName/tablePrefix
//   用保守正则匹配每行的值并替换，匹配不到跳过
// - Vue3 模板：generator 的 vm/vue/ 下的 .vm 模板，把 Element UI（Vue2）写法改为 Element Plus（Vue3）
//   保守替换：仅精确匹配的语法才改，匹配不到跳过，不破坏模板

use crate::core::CustomizeParams;
use std::path::Path;

/// 生成器配置定制结果
pub struct GeneratorOutcome {
    pub modified_files: usize,
    pub summary: Vec<String>,
}

/// 执行代码生成器配置定制。
pub fn customize_generator(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<GeneratorOutcome, String> {
    let mut modified = 0usize;
    let mut summary = Vec::new();

    // 1. generator.yml 字段定制
    let n = customize_generator_yml(root, params, log)?;
    if n > 0 {
        modified += n;
        let author = if params.generator_author.is_empty() {
            "（保留默认）"
        } else {
            &params.generator_author
        };
        summary.push(format!(
            "代码生成器配置：作者={author}，包名={}",
            params.new_package
        ));
    }

    // 2. Vue3 模板升级
    if params.generator_vue3 {
        let vn = upgrade_vue3_templates(root, log)?;
        if vn > 0 {
            modified += vn;
            summary.push(format!("Vue3 模板升级：改造 {vn} 个模板"));
        }
    }

    Ok(GeneratorOutcome {
        modified_files: modified,
        summary,
    })
}

/// 定位 generator.yml（兼顾模块改名后场景）
fn find_generator_yml(root: &Path) -> Option<std::path::PathBuf> {
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
        if path.is_file() && path.file_name().map(|n| n == "generator.yml").unwrap_or(false) {
            return Some(path.to_path_buf());
        }
    }
    None
}

/// 改写 generator.yml 的 author / packageName / tablePrefix。返回修改的文件数（0 或 1）。
fn customize_generator_yml(
    root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let yml_path = match find_generator_yml(root) {
        Some(p) => p,
        None => {
            log("未找到 generator.yml，跳过代码生成器配置定制");
            return Ok(0);
        }
    };
    let content = crate::utils::file::read_text(&yml_path)
        .ok_or_else(|| format!("读取 {} 失败（UTF-8/GBK 均无法识别）", yml_path.display()))?;
    let mut new_content = content.clone();
    let mut changed = false;

    // author: ruoyi → 自定义（留空则用版权方或 frontend_title，都空则不改）
    if !params.generator_author.is_empty() {
        if replace_yml_value(&mut new_content, "author", &params.generator_author) {
            changed = true;
            log(&format!("generator.yml author → {}", params.generator_author));
        }
    }
    // packageName: com.ruoyi → new_package
    if !params.new_package.is_empty() {
        if replace_yml_value(&mut new_content, "packageName", &params.new_package) {
            changed = true;
            log(&format!("generator.yml packageName → {}", params.new_package));
        }
    }
    // tablePrefix: sys_ → 自定义
    if !params.generator_table_prefix.is_empty() {
        if replace_yml_value(&mut new_content, "tablePrefix", &params.generator_table_prefix) {
            changed = true;
            log(&format!("generator.yml tablePrefix → {}", params.generator_table_prefix));
        }
    }

    if changed {
        std::fs::write(&yml_path, &new_content)
            .map_err(|e| format!("写入 {} 失败：{e}", yml_path.display()))?;
        Ok(1)
    } else {
        Ok(0)
    }
}

/// 替换 yml 某行的值（保守匹配 `key: value`，值可为引号或裸值）。
fn replace_yml_value(content: &mut String, key: &str, new_value: &str) -> bool {
    // 匹配 key: xxx 或 key: 'xxx' 或 key: "xxx"
    // 用 r#"..."# 避免 \" 转义问题
    let pattern = format!(r#"(?m)({}\s*:\s*)['"]?[^'"\n#]+['"]?"#, regex::escape(key));
    let re = match regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let want = new_value.to_string();
    let mut changed = false;
    let new = re
        .replace_all(content, |caps: &regex::Captures| {
            changed = true;
            format!("{}{}", &caps[1], want)
        })
        .to_string();
    if changed {
        *content = new;
    }
    changed
}

/// 升级 Vue3 模板：把 generator 的 vm/vue/*.vm 里 Element UI（Vue2）写法改为 Element Plus（Vue3）。
/// 保守替换，匹配不到跳过。返回修改的文件数。
fn upgrade_vue3_templates(root: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
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
        if !path.is_file() {
            continue;
        }
        // 仅处理 vm/vue/ 目录下的 .vm 文件
        let path_str = path.to_string_lossy();
        if !path_str.contains("/vm/vue/") && !path_str.contains("\\vm\\vue\\") {
            continue;
        }
        if !path.extension().map(|e| e == "vm").unwrap_or(false) {
            continue;
        }
        let content = match crate::utils::file::read_text(path) {
            Some(c) => c,
            None => continue,
        };
        let mut new_content = content.clone();

        // 1. :visible.sync="x" → v-model="x"（Element Plus 弹窗）
        let n1 = replace_all_count(&mut new_content, |s| {
            let re = regex::Regex::new(r#":visible\.sync="(\w+)""#).unwrap();
            let c = std::cell::Cell::new(0usize);
            let r = re.replace_all(s, |caps: &regex::Captures| {
                c.set(c.get() + 1);
                format!("v-model=\"{}\"", &caps[1])
            }).to_string();
            (r, c.get())
        });
        // 2. el-tag :  type="success" 等 Vue2 写法保留（Element Plus 兼容）
        // 3. .sync 修饰符其他场景（如 size.sync）→ 移除 .sync
        let n2 = replace_all_count(&mut new_content, |s| {
            let re = regex::Regex::new(r#"\.sync=""#).unwrap();
            let c = std::cell::Cell::new(0usize);
            let r = re.replace_all(s, |_caps: &regex::Captures| {
                c.set(c.get() + 1);
                "=\"".to_string()
            }).to_string();
            (r, c.get())
        });
        // 4. el-image 的 lazy 属性等无需改

        let total = n1 + n2;
        if total > 0 && new_content != content {
            std::fs::write(path, &new_content)
                .map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
            count += 1;
            log(&format!("Vue3 模板升级：{}", path.display()));
        }
    }
    Ok(count)
}

/// 通用：对字符串应用一个返回（新字符串, 替换次数）的闭包
fn replace_all_count<F>(s: &mut String, f: F) -> usize
where
    F: FnOnce(&str) -> (String, usize),
{
    let (new, n) = f(s);
    if n > 0 {
        *s = new;
    }
    n
}
