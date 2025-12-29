mod formatter;
mod process;
mod monitor;

use clap::Parser;
use sysinfo::{System, ProcessesToUpdate};
use process::{SortOrder, show_process_by_pid, show_processes_by_name};
use monitor::{watch_mode, MonitorArgs};

/// プロセス監視ツール
#[derive(Parser, Debug)]
#[command(name = "rs-process-monitor")]
#[command(about = "A simple process monitoring tool", long_about = None)]
struct Args {
    /// 監視するプロセスのPID
    #[arg(short, long, conflicts_with = "name")]
    pid: Option<u32>,

    /// 監視するプロセス名（部分一致）
    #[arg(short, long, conflicts_with = "pid")]
    name: Option<String>,

    /// リアルタイム監視モード（指定した間隔で更新、単位: 秒）
    #[arg(short, long)]
    watch: Option<u64>,

    /// ソート順: memory (デフォルト), cpu, pid, name
    #[arg(short, long, default_value = "memory")]
    sort: SortOrder,
}

fn main() {
    let args = Args::parse();

    // リアルタイム監視モードの場合
    if let Some(interval) = args.watch {
        let monitor_args = MonitorArgs {
            pid: args.pid,
            name: args.name.as_deref(),
            sort: &args.sort,
        };
        watch_mode(monitor_args, interval);
    } else {
        // 通常モード（1回だけ表示）
        single_shot_mode(&args);
    }
}

/// 1回だけ表示するモード
fn single_shot_mode(args: &Args) {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(name) = &args.name {
        show_processes_by_name(&sys, name, &args.sort);
    } else {
        let target_pid = args.pid.unwrap_or_else(|| std::process::id());
        show_process_by_pid(&sys, target_pid);
    }
}