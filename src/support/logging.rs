//! 简单的追加式文件日志记录器。
//!
//! 日志保存在配置目录旁的 `config/logs/rust-win-tool.log` 中，并以同步方式写入。
//! 写入失败时会忽略错误，避免日志功能影响界面运行。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOGGER: OnceLock<Mutex<File>> = OnceLock::new();

pub fn log_dir() -> PathBuf {
    base_dir().join("logs")
}

fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config")
}

pub fn init() {
    let dir = log_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("rust-win-tool.log"))
    {
        let _ = LOGGER.set(Mutex::new(file));
    }
    log(&format!("application started (pid {})", std::process::id()));
}

pub fn log(message: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    if let Ok(mut file) = logger.lock() {
        let _ = writeln!(file, "[{}] {}", timestamp(), message);
    }
}

/// 在文件资源管理器中打开日志目录。
pub fn open_dir() {
    let _ = std::process::Command::new("explorer")
        .arg(log_dir())
        .spawn();
}

fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}Z")
}

/// 使用 Howard Hinnant 算法，将距 Unix 纪元的天数转换为公历日期。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn converts_epoch_days_to_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }
}
