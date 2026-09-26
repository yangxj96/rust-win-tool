//! Windows 桌面程序入口。

#![windows_subsystem = "windows"]

mod app;
mod features;
mod platform;
mod shared;
mod support;
mod ui;

fn main() {
    app::run();
}
