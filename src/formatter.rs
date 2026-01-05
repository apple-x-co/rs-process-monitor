use sysinfo::{ProcessStatus, System};

/// バイト数を見やすい単位に変換
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 文字列を指定長で切り詰める
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
}

/// ステータスを短く整形
pub fn format_status(status: ProcessStatus) -> String {
    match status {
        ProcessStatus::Run => "Run".to_string(),
        ProcessStatus::Sleep => "Sleep".to_string(),
        ProcessStatus::Idle => "Idle".to_string(),
        ProcessStatus::Zombie => "Zombie".to_string(),
        _ => format!("{:?}", status).chars().take(15).collect(),
    }
}

/// システムメモリ情報を整形して返す
pub fn format_system_memory(sys: &System) -> String {
    let total = sys.total_memory();
    let used = sys.used_memory();
    let available = sys.available_memory();
    let usage_percent = (used as f64 / total as f64) * 100.0;

    format!(
        "System Memory: {} / {} ({:.1}% used, {} available)",
        format_bytes(used),
        format_bytes(total),
        usage_percent,
        format_bytes(available)
    )
}

/// スワップ情報を整形して返す
pub fn format_system_swap(sys: &System) -> String {
    let total = sys.total_swap();
    let used = sys.used_swap();

    if total == 0 {
        "Swap: N/A".to_string()
    } else {
        let usage_percent = (used as f64 / total as f64) * 100.0;
        format!(
            "Swap: {} / {} ({:.1}% used)",
            format_bytes(used),
            format_bytes(total),
            usage_percent
        )
    }
}

/// プロセスのスレッド数を取得
/// Linux: /proc/{pid}/status から Threads: の行を読む
/// macOS: sysinfo ではスレッド数が取れないので 1 を返す
#[cfg(target_os = "linux")]
pub fn get_thread_count(pid: u32) -> usize {
    use std::fs;

    let status_path = format!("/proc/{}/status", pid);

    if let Ok(content) = fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("Threads:") {
                if let Some(count_str) = line.split_whitespace().nth(1) {
                    if let Ok(count) = count_str.parse::<usize>() {
                        return count;
                    }
                }
            }
        }
    }

    // 取得できなかった場合は 1 を返す
    1
}

#[cfg(not(target_os = "linux"))]
pub fn get_thread_count(_pid: u32) -> usize {
    // Linux以外ではスレッド数を取得できないので 1 を返す
    1
}

/// プロセスの実際のPID（TGID）を取得
#[cfg(target_os = "linux")]
pub fn get_tgid(lwp: u32) -> u32 {
    use std::fs;

    let status_path = format!("/proc/{}/status", lwp);

    if let Ok(content) = fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("Tgid:") {
                if let Some(tgid_str) = line.split_whitespace().nth(1) {
                    if let Ok(tgid) = tgid_str.parse::<u32>() {
                        return tgid;
                    }
                }
            }
        }
    }

    // 取得できなかった場合はlwpをそのまま返す
    lwp
}

#[cfg(not(target_os = "linux"))]
pub fn get_tgid(lwp: u32) -> u32 {
    lwp
}