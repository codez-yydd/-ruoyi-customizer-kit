// 执行改造命令：薄壳，构造 TransformOptions 并转发 transform:progress 事件。

use crate::core::pipeline::{self, LogEvent, TransformOptions};
use crate::core::{CustomizeParams, ProjectInfo};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

pub use crate::core::pipeline::ExecuteResponse;

/// 执行改造。前端传入识别结果（project_info）+ 改造参数 + 来源信息。
/// 执行过程通过事件 "transform:progress" 实时推送日志。
#[tauri::command]
pub async fn execute_transform(
    app: AppHandle,
    project_info: ProjectInfo,
    params: CustomizeParams,
    source_type: String,
    zip_path: Option<String>,
) -> Result<ExecuteResponse, String> {
    let source_path = if source_type == "zip" {
        PathBuf::from(zip_path.as_deref().unwrap_or(""))
    } else {
        PathBuf::from(&project_info.root_path)
    };
    let template_dir = if project_info.template_dir.is_empty() {
        None
    } else {
        Some(project_info.template_dir.clone())
    };
    let opts = TransformOptions {
        source_type,
        source_path,
        params,
        template_dir,
    };
    // AppHandle 为 Clone + Send + 'static，move 进 emit 闭包转发进度事件；
    // 事件名 "transform:progress" 与载荷结构不变
    run_transform_blocking(opts, move |ev: &LogEvent| {
        let _ = app.emit("transform:progress", ev);
    })
    .await
}

/// 把同步阻塞的改造管线移入 `tauri::async_runtime::spawn_blocking` 执行。
// async 命令壳 + spawn_blocking（勿改回 async 上下文直接同步调用，原因同
// download.rs 的 list_official_versions）：改造管线深处（core/nacos_config.rs 的
// live Nacos 路径）使用 reqwest::blocking，而 async 命令体在 Tauri 的 tokio
// worker 线程 async 上下文执行，reqwest 0.12 的 blocking 在该上下文中 drop 内部
// runtime 会 panic（Cannot drop a runtime in a context where blocking is not
// allowed），命令 panic 后 promise resolver 永不调用，前端 await invoke() 永久
// 挂起（改造执行界面永久转圈）。spawn_blocking 把同步管线放到 tokio 阻塞线程池
// （普通 OS 线程、无 async 上下文）执行，reqwest::blocking 安全，也不长期占用
// worker 线程。spawn_blocking 返回 Result<Result<_, String>, JoinError>，此处
// 展平：JoinError 映射为中文错误，内层 Result 原样透传，最终返回类型不变
// （前端零改动）。emit 回调注入便于单测在不依赖 Tauri AppHandle 的情况下直接
// 驱动本函数（见 mod tests）。
async fn run_transform_blocking<E>(opts: TransformOptions, emit: E) -> Result<ExecuteResponse, String>
where
    E: Fn(&LogEvent) + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || pipeline::run_transform(&opts, &emit))
        .await
        .map_err(|e| format!("改造执行任务失败：{e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归测试：防止「reqwest::blocking 在 async 运行时上下文 panic，命令 promise
    /// 永久挂起」的缺陷回归（改造执行界面永久转圈的根因）。
    /// 缺陷写法是 async 命令体在 async 上下文中直接同步调用 pipeline::run_transform：
    /// 命令体在 Tauri 的 tokio worker 线程 async 上下文执行，改造管线深处
    /// （core/nacos_config.rs 的 live Nacos 路径）用 reqwest::blocking，reqwest 0.12
    /// 的 blocking 在该上下文中 drop 内部 runtime 触发 panic（Cannot drop a runtime
    /// in a context where blocking is not allowed），命令 panic 后 promise resolver
    /// 永不调用，前端 await invoke() 永久挂起。修复后命令壳把管线移入
    /// spawn_blocking 阻塞线程池（普通 OS 线程、无 async 上下文）执行。
    /// 本测试把 execute_transform 命令的核心执行体 run_transform_blocking（含
    /// spawn_blocking 包裹、JoinError 中文映射与内层 Result 透传的全部逻辑）spawn
    /// 到 Tauri 异步运行时的 worker 线程（与 GUI 实际执行环境一致的 async 上下文）
    /// 调用；用默认参数（new_package 为空）让 params.validate() 首个校验即失败，
    /// run_transform 立即返回 Ok(success=false)（core/pipeline.rs 快速失败分支），
    /// 不碰文件系统、不打外网；断言 spawn 的 JoinHandle 正常完成（未 panic /
    /// 未挂起）、返回 Ok 且载荷 success=false。
    ///
    /// 注意：不要把本测试改造成 `tauri::test::mock_app()` 驱动完整命令壳的形态——
    /// mock_app 会经 Builder::build 拉入 muda 的 Common Controls v6 专属导入
    /// （TaskDialogIndirect / SetWindowSubclass 等，来自 tauri 默认 feature
    /// common-controls-v6），而 lib 单测 exe 不经过 tauri-build、不嵌 v6 manifest，
    /// Windows 加载器解析到 v5 版 comctl32.dll 时进程初始化即报
    /// STATUS_ENTRYPOINT_NOT_FOUND（0xc0000139），导致本 crate 全部单测无法运行。
    /// 已在本机用 mt.exe 嵌入 v6 manifest 实验证实该结论。
    #[test]
    fn execute_transform_async_shell_resolves_in_async_context() {
        let opts = TransformOptions {
            source_type: "directory".into(),
            // 快速失败路径不读取 source_path，留空即可（不触碰文件系统）
            source_path: PathBuf::new(),
            // 默认参数 new_package 为空：validate() 首个校验即失败，
            // run_transform 走快速失败分支（core/pipeline.rs 参数校验段）
            params: CustomizeParams::default(),
            template_dir: None,
        };
        let result = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn(run_transform_blocking(opts, |ev: &LogEvent| {
                let _ = ev; // 快速失败路径不产生进度事件
            }))
            .await
            .expect("async 上下文中的命令壳应正常完成而非 panic / 永久挂起")
        });
        // fail 语义在 Ok 载荷中表达（与前端现有分支一致），不应走 Err
        let resp = result.expect("命令壳应返回 Ok（fail 语义在 Ok 载荷中表达）");
        assert!(!resp.success, "参数校验失败应返回 success=false 载荷");
        assert!(
            resp.message.contains("新包名"),
            "默认参数应在新包名校验处失败：{}",
            resp.message
        );
    }
}
