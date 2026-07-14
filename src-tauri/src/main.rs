// Windows release 下隐藏控制台窗口，请勿删除
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ruoyi_forge_lib::run()
}
