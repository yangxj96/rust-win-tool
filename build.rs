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
