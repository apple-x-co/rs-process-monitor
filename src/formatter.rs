use sysinfo::ProcessStatus;
use sysinfo::System;

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