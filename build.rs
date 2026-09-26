//! Windows 构建步骤：嵌入应用图标并写入可执行文件的版本资源信息。

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets\\app.ico");
        res.set("ProductName", "rust-win-tool");
        res.set("FileDescription", "Windows System Tool");
        res.set("Subsystem", "windows");
        res.compile().unwrap();
    }
}
