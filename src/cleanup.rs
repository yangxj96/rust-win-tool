//! 扫描并清理系统垃圾文件。
//!
//! 操作范围仅限固定白名单中的已知位置；不会跟随重解析点，也不会强制处理被占用
//! 或需要额外权限的文件。

use std::path::{Path, PathBuf};

/// 可清理的已知位置类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 固定清理位置之一；实际路径由程序内部推导。
pub enum CleanupCategory {
    UserTemp,
    WindowsTemp,
    ThumbnailCache,
    WindowsUpdate,
}

impl CleanupCategory {
    pub fn all() -> [Self; 4] {
        [
            Self::UserTemp,
            Self::WindowsTemp,
            Self::ThumbnailCache,
            Self::WindowsUpdate,
        ]
    }

    /// 系统位置仅允许已提权的进程写入。
    pub fn requires_admin(self) -> bool {
        matches!(self, Self::WindowsTemp | Self::WindowsUpdate)
    }

    /// 扫描和清理操作涉及的顶层文件与目录。
    fn entries(self) -> Vec<PathBuf> {
        match self {
            Self::UserTemp => children(std::env::temp_dir()),
            Self::WindowsTemp => children(windows_dir().join("Temp")),
            Self::ThumbnailCache => thumbnail_cache_files(),
            Self::WindowsUpdate => {
                children(windows_dir().join("SoftwareDistribution").join("Download"))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// 扫描汇总结果；`bytes` 为普通文件大小之和。
pub struct Scan {
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    /// 将文件移入回收站，用户之后仍可还原。
    Recycle,
    /// 永久删除文件。
    Permanent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// 所选删除模式执行后的统计结果。
pub struct CleanReport {
    pub freed_bytes: u64,
    pub removed_files: u64,
    pub skipped_files: u64,
    /// Shell 报告操作部分完成或中止时为 `false`。
    pub completed: bool,
}

/// 扫描一个白名单类别，且不跟随重解析点。
pub fn scan(category: CleanupCategory) -> Scan {
    scan_paths(&category.entries())
}

/// 使用回收站或永久删除模式清理指定的白名单类别。
///
/// 被占用或无法访问的文件会跳过；返回结果记录清理前后的字节数和文件数差值。
pub fn clean(categories: &[CleanupCategory], mode: DeleteMode) -> CleanReport {
    let paths: Vec<PathBuf> = categories.iter().flat_map(|c| c.entries()).collect();
    if paths.is_empty() {
        return CleanReport {
            completed: true,
            ..CleanReport::default()
        };
    }

    let before = scan_paths(&paths);
    let completed = match mode {
        DeleteMode::Permanent => {
            for path in &paths {
                delete_permanent(path);
            }
            true
        }
        DeleteMode::Recycle => recycle(&paths),
    };
    let after = scan_paths(&paths);

    CleanReport {
        freed_bytes: before.bytes.saturating_sub(after.bytes),
        removed_files: before.files.saturating_sub(after.files),
        skipped_files: after.files,
        completed,
    }
}

/// 查询回收站当前占用大小（字节）和项目数量。
#[cfg(windows)]
pub fn recycle_bin_usage() -> Option<(u64, u64)> {
    use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};

    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    let result = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if result < 0 {
        return None;
    }
    Some((info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
}

#[cfg(not(windows))]
pub fn recycle_bin_usage() -> Option<(u64, u64)> {
    None
}

/// 清空回收站。此操作无法撤销。
#[cfg(windows)]
pub fn empty_recycle_bin() -> bool {
    use windows_sys::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    };

    let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
    unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) >= 0 }
}

#[cfg(not(windows))]
pub fn empty_recycle_bin() -> bool {
    false
}

/// 使用界面现有的二进制单位标签格式化字节数。
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value < KB {
        format!("{bytes} B")
    } else if value < KB * KB {
        format!("{:.1} KB", value / KB)
    } else if value < KB * KB * KB {
        format!("{:.1} MB", value / (KB * KB))
    } else {
        format!("{:.2} GB", value / (KB * KB * KB))
    }
}

fn scan_paths(paths: &[PathBuf]) -> Scan {
    let mut total = Scan::default();
    for path in paths {
        let scan = path_size(path);
        total.bytes += scan.bytes;
        total.files += scan.files;
    }
    total
}

fn path_size(path: &Path) -> Scan {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return Scan::default();
    };
    let file_type = meta.file_type();
    if file_type.is_symlink() {
        return Scan::default();
    }
    if file_type.is_file() {
        return Scan {
            bytes: meta.len(),
            files: 1,
        };
    }

    let mut total = Scan::default();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = path_size(&entry.path());
            total.bytes += child.bytes;
            total.files += child.files;
        }
    }
    total
}

fn delete_permanent(path: &Path) {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    let file_type = meta.file_type();
    if file_type.is_dir() && !file_type.is_symlink() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                delete_permanent(&entry.path());
            }
        }
        let _ = std::fs::remove_dir(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(windows)]
fn recycle(paths: &[PathBuf]) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    let mut from: Vec<u16> = Vec::new();
    for path in paths {
        from.extend(path.as_os_str().encode_wide());
        from.push(0);
    }
    from.push(0);

    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16,
        ..SHFILEOPSTRUCTW::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };
    result == 0 && operation.fAnyOperationsAborted == 0
}

#[cfg(not(windows))]
fn recycle(_paths: &[PathBuf]) -> bool {
    false
}

fn windows_dir() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn children(dir: PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

fn thumbnail_cache_files() -> Vec<PathBuf> {
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        return Vec::new();
    };
    let dir = PathBuf::from(local_app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Explorer");
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("thumbcache_") && name.ends_with(".db"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, path_size};
    use std::io::Write;

    #[test]
    fn formats_sizes_readably() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn measures_a_directory_recursively() {
        let dir = std::env::temp_dir().join("rwt-cleanup-test");
        let _ = std::fs::create_dir_all(&dir);
        let mut file = std::fs::File::create(dir.join("a.bin")).expect("create file");
        file.write_all(&[0u8; 100]).expect("write");
        drop(file);

        let scan = path_size(&dir);
        assert!(scan.bytes >= 100);
        assert!(scan.files >= 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deletes_files_and_directories_permanently() {
        let dir = std::env::temp_dir().join("rwt-cleanup-delete-test");
        let nested = dir.join("nested");
        let _ = std::fs::create_dir_all(&nested);
        std::fs::write(dir.join("a.bin"), [0u8; 10]).expect("write a");
        std::fs::write(nested.join("b.bin"), [0u8; 10]).expect("write b");

        super::delete_permanent(&dir);

        assert!(!dir.exists());
    }
}
