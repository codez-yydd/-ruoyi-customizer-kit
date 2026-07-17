// 前后端分离：把前端目录移动到输出根目录同级，与后端平级。
//
// 设计：
// - 前端目录：{prefix}-ui（若依默认 ruoyi-ui，改造后已重命名）
// - 移动目标：{output_root}/../{prefix}-ui-frontend —— 即输出目录同级
//   但更稳妥的做法是放到 {output_root}/{prefix}-ui-frontend（输出目录内平级子目录），
//   避免跨目录权限问题。这里采用后者：输出目录下出现「后端根内容 + 前端目录」两个平级项。
// - 生成根 README 说明结构
// - 此任务必须最后执行（移动目录后，后续任务无法再访问前端目录）

use crate::core::CustomizeParams;
use std::path::Path;

/// 执行前后端分离：移动前端目录 + 生成根 README。
/// 返回是否实际移动。
pub fn split_frontend(
    output_root: &Path,
    params: &CustomizeParams,
    log: &dyn Fn(&str),
) -> Result<bool, String> {
    let prefix = &params.new_module_prefix;
    // 候选前端目录名：优先新前缀，回退旧前缀
    let candidates = [
        format!("{}-ui", prefix),
        "ruoyi-ui".to_string(),
    ];
    let mut frontend_dir = None;
    for name in &candidates {
        let p = output_root.join(name);
        if p.is_dir() {
            frontend_dir = Some((name.clone(), p));
            break;
        }
    }
    let (fe_name, fe_path) = match frontend_dir {
        Some(x) => x,
        None => {
            log("未找到前端目录（ruoyi-ui / {prefix}-ui），跳过前后端分离");
            return Ok(false);
        }
    };

    // 移动目标：输出根目录下的 {prefix}-ui-frontend
    let new_fe_name = format!("{}-ui-frontend", prefix);
    let new_fe_path = output_root.join(&new_fe_name);
    if new_fe_path.exists() {
        return Err(format!(
            "目标前端目录已存在，拒绝覆盖：{}",
            new_fe_path.display()
        ));
    }

    std::fs::rename(&fe_path, &new_fe_path)
        .map_err(|e| format!("移动 {} → {} 失败：{e}", fe_path.display(), new_fe_path.display()))?;
    log(&format!("前后端分离：{} → {}", fe_name, new_fe_name));

    // 生成根 README 说明结构
    let readme_path = output_root.join("README.md");
    let readme = render_split_readme(params, &new_fe_name);
    std::fs::write(&readme_path, readme)
        .map_err(|e| format!("写入根 README 失败：{e}"))?;
    log(&format!("已生成根目录 README：{}", readme_path.display()));

    Ok(true)
}

/// 渲染前后端分离后的根 README
fn render_split_readme(params: &CustomizeParams, fe_name: &str) -> String {
    let prefix = &params.new_module_prefix;
    let title = if params.frontend_title.is_empty() {
        &params.new_project_name
    } else {
        &params.frontend_title
    };
    format!(
        "# {title}\n\n本仓库已做前后端分离改造，目录结构如下：\n\n```\n.\n├── {prefix}-admin/        # 后端主模块（Spring Boot 启动入口）\n├── {prefix}-common/       # 后端公共模块\n├── {prefix}-framework/    # 后端框架模块\n├── {prefix}-system/       # 后端业务模块\n├── ...                    # 其余后端模块\n├── {fe_name}/    # 前端工程（Vue，独立运行）\n└── pom.xml               # 后端 Maven 聚合 pom\n```\n\n## 后端\n\n```bash\n# 在仓库根目录\nmvn clean package\njava -jar {prefix}-admin/target/{prefix}-admin.jar\n```\n\n## 前端\n\n```bash\ncd {fe_name}\nnpm install\nnpm run dev\n```\n\n## 部署\n\n前后端可分别独立部署：\n- 后端打 jar 包运行\n- 前端 `npm run build` 后产物在 `{fe_name}/dist`，交由 nginx 托管\n\n---\n\n> 本 README 由若依锻造台（RuoYi Forge）前后端分离功能自动生成。\n",
        prefix = prefix,
        fe_name = fe_name,
        title = title
    )
}
