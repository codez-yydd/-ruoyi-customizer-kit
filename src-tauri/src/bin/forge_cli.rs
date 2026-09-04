// 若依锻造台命令行入口。不要加 windows_subsystem = "windows"（须保留控制台）。

fn main() -> std::process::ExitCode {
    ruoyi_forge_lib::cli::run()
}
