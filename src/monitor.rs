use sysinfo::{System, ProcessesToUpdate};
use std::thread;
use std::time::Duration;
use crate::process::{show_process_by_pid, show_processes_by_name, create_snapshots, SortOrder};
use crate::history::ProcessHistory;

pub struct MonitorArgs<'a> {
    pub pid: Option<u32>,
    pub name: Option<&'a str>,
    pub sort: &'a SortOrder,
    pub min_memory_mb: Option<u64>,
    pub log_path: Option<&'a str>,
}

/// リアルタイム監視モード
pub fn watch_mode(args: MonitorArgs, interval_secs: u64) {
    let mut sys = System::new_all();

    // 履歴記録の初期化
    let mut history = if let Some(log_path) = args.log_path {
        match ProcessHistory::new(log_path) {
            Ok(h) => {
                println!("Logging to: {}", log_path);
                Some(h)
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize history database: {}", e);
                eprintln!("Continuing without logging...");
                None
            }
        }
    } else {
        None
    };

    loop {
        // 画面をクリア（ANSIエスケープシーケンス）
        print!("\x1B[2J\x1B[1;1H");

        // プロセス情報を更新
        sys.refresh_processes(ProcessesToUpdate::All, true);

        // 履歴記録（name モードのみ）
        if let Some(ref mut hist) = history {
            if let Some(name) = args.name {
                let snapshots = create_snapshots(&sys, name, args.min_memory_mb);
                if let Err(e) = hist.insert_snapshots(&snapshots) {
                    eprintln!("Warning: Failed to log snapshots: {}", e);
                }
            }
        }

        // 現在時刻を表示
        println!("Last updated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        if history.is_some() {
            println!("Logging: enabled");
        }
        println!("Press Ctrl+C to exit\n");

        // プロセス情報を表示
        if let Some(name) = args.name {
            show_processes_by_name(&sys, name, args.sort, args.min_memory_mb);
        } else {
            let target_pid = args.pid.unwrap_or_else(|| std::process::id());
            show_process_by_pid(&sys, target_pid);
        }

        // 指定秒数待機
        thread::sleep(Duration::from_secs(interval_secs));
    }
}