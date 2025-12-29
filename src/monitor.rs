use sysinfo::{System, ProcessesToUpdate};
use std::thread;
use std::time::Duration;
use crate::process::{show_process_by_pid, show_processes_by_name, SortOrder};

pub struct MonitorArgs<'a> {
    pub pid: Option<u32>,
    pub name: Option<&'a str>,
    pub sort: &'a SortOrder,
}

/// リアルタイム監視モード
pub fn watch_mode(args: MonitorArgs, interval_secs: u64) {
    let mut sys = System::new_all();

    loop {
        // 画面をクリア（ANSIエスケープシーケンス）
        print!("\x1B[2J\x1B[1;1H");

        // プロセス情報を更新
        sys.refresh_processes(ProcessesToUpdate::All, true);

        // 現在時刻を表示
        println!("Last updated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
        println!("Press Ctrl+C to exit\n");

        // プロセス情報を表示
        if let Some(name) = args.name {
            show_processes_by_name(&sys, name, args.sort);
        } else {
            let target_pid = args.pid.unwrap_or_else(|| std::process::id());
            show_process_by_pid(&sys, target_pid);
        }

        // 指定秒数待機
        thread::sleep(Duration::from_secs(interval_secs));
    }
}