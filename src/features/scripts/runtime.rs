//! 通过隐藏命令行进程执行用户命令和脚本。

use std::process::Command;

use crate::features::scripts::ScriptKind;
use crate::shared::BackendError;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 通过 `cmd.exe` 运行命令并捕获标准输出和标准错误。
///
/// 在 Windows 上隐藏子进程控制台窗口。此调用会阻塞，必须放在 GPUI 后台任务中。
pub fn run_command(command: &str) -> Result<String, BackendError> {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let output = cmd
        .output()
        .map_err(|error| BackendError::Command(error.to_string()))?;
    let mut text = decode_console(&output.stdout);
    let stderr = decode_console(&output.stderr);
    if !stderr.trim().is_empty() {
        text.push('\n');
        text.push_str(&stderr);
    }
    if !output.status.success() {
        return Err(BackendError::Command(text.trim().to_string()));
    }
    Ok(text.trim().to_string())
}

/// 将用户脚本写入临时文件，再调用对应解释器执行。
///
/// 使用临时文件可以完整保留多行内容，避免 shell 引号转义问题。子进程继承应用
/// 当前用户身份和提权上下文。
pub fn run_script(kind: ScriptKind, content: &str) -> Result<String, BackendError> {
    #[cfg(windows)]
    {
        use std::io::Write;

        let (extension, program, args): (&str, &str, &[&str]) = match kind {
            ScriptKind::Cmd => (".cmd", "cmd", &["/C"]),
            ScriptKind::PowerShell => (
                ".ps1",
                "powershell",
                &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"],
            ),
        };
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        path.push(format!(
            "rust-win-tool-{}-{}{}",
            std::process::id(),
            stamp,
            extension
        ));
        {
            let mut file = std::fs::File::create(&path)
                .map_err(|error| BackendError::Command(error.to_string()))?;
            file.write_all(content.as_bytes())
                .map_err(|error| BackendError::Command(error.to_string()))?;
        }
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.arg(&path);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output();
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|error| BackendError::Command(error.to_string()))?;
        let mut text = decode_console(&output.stdout);
        let stderr = decode_console(&output.stderr);
        if !stderr.trim().is_empty() {
            text.push('\n');
            text.push_str(&stderr);
        }
        if !output.status.success() {
            return Err(BackendError::Command(text.trim().to_string()));
        }
        Ok(text.trim().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = kind;
        run_command(content)
    }
}

/// 控制台程序输出使用 OEM 代码页，而不是 UTF-8。
#[cfg(windows)]
fn decode_console(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::MultiByteToWideChar;

    const CP_OEMCP: u32 = 1;
    if bytes.is_empty() {
        return String::new();
    }
    let length = unsafe {
        MultiByteToWideChar(
            CP_OEMCP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    };
    if length <= 0 {
        return String::from_utf8_lossy(bytes).to_string();
    }
    let mut wide = vec![0u16; length as usize];
    let written = unsafe {
        MultiByteToWideChar(
            CP_OEMCP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            length,
        )
    };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).to_string();
    }
    wide.truncate(written as usize);
    String::from_utf16_lossy(&wide)
}

#[cfg(not(windows))]
fn decode_console(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}
