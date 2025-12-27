use std::thread;
use std::time::Duration;
use clap::Parser;
use sysinfo::{System, Pid, ProcessesToUpdate};

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

/// ソート順の指定
#[derive(Debug, Clone, clap::ValueEnum)]
enum SortOrder {
    Memory,  // メモリ使用量順（降順）
    Cpu,     // CPU使用率順（降順）
    Pid,     // PID順（昇順）
    Name,    // プロセス名順（昇順）
}

fn main() {
    let args = Args::parse();

    // リアルタイム監視モードの場合
    if let Some(interval) = args.watch {
        watch_mode(&args, interval);
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

/// リアルタイム監視モード
fn watch_mode(args: &Args, interval_secs: u64) {
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
        if let Some(name) = &args.name {
            show_processes_by_name(&sys, name, &args.sort);
        } else {
            let target_pid = args.pid.unwrap_or_else(|| std::process::id());
            show_process_by_pid(&sys, target_pid);
        }

        // 指定秒数待機
        thread::sleep(Duration::from_secs(interval_secs));
    }
}

/// PIDでプロセス情報を表示
fn show_process_by_pid(sys: &System, target_pid: u32) {
    let pid = Pid::from_u32(target_pid);

    if let Some(process) = sys.process(pid) {
        print_single_process(process);
    } else {
        eprintln!("Error: Process not found (PID: {})", target_pid);
        std::process::exit(1);
    }
}

/// プロセス名でプロセス情報を表示（複数マッチする可能性あり）
fn show_processes_by_name(sys: &System, name: &str, sort_order: &SortOrder) {
    let mut matching_processes: Vec<_> = sys.processes()
        .iter()
        .filter(|(_, p)| p.name().to_string_lossy().contains(name))
        .collect();

    if matching_processes.is_empty() {
        eprintln!("Error: No processes found matching '{}'", name);
        std::process::exit(1);
    }

    // ソート
    match sort_order {
        SortOrder::Memory => {
            matching_processes.sort_by(|a, b| b.1.memory().cmp(&a.1.memory()));
        }
        SortOrder::Cpu => {
            matching_processes.sort_by(|a, b| {
                b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Pid => {
            matching_processes.sort_by_key(|(_, p)| p.pid());
        }
        SortOrder::Name => {
            matching_processes.sort_by(|a, b| {
                a.1.name().to_string_lossy().cmp(&b.1.name().to_string_lossy())
            });
        }
    }

    // 統計情報の計算
    let total_count = matching_processes.len();
    let total_memory: u64 = matching_processes.iter()
        .map(|(_, p)| p.memory())
        .sum();
    let total_cpu: f32 = matching_processes.iter()
        .map(|(_, p)| p.cpu_usage())
        .sum();

    // ヘッダー表示
    println!("Processes matching '{}':", name);
    println!("Total: {} process(es) | Memory: {} | CPU: {:.2}%\n",
             total_count, format_bytes(total_memory), total_cpu);

    // 表のヘッダー（数値は右寄せ >、文字列は左寄せ <）
    println!("{:>8} {:<25} {:>8} {:>12} {:<15}",
             "PID", "Name", "CPU %", "Memory", "Status");
    println!("{}", "-".repeat(75));

    // 各プロセスの情報を表示
    for (_, process) in matching_processes {
        println!("{:>8} {:<25} {:>8.2} {:>12} {:<15}",
                 process.pid(),
                 truncate_string(&process.name().to_string_lossy(), 25),
                 process.cpu_usage(),
                 format_bytes(process.memory()),
                 format_status(process.status()));
    }
}

/// ステータスを短く整形
fn format_status(status: sysinfo::ProcessStatus) -> String {
    match status {
        sysinfo::ProcessStatus::Run => "Run".to_string(),
        sysinfo::ProcessStatus::Sleep => "Sleep".to_string(),
        sysinfo::ProcessStatus::Idle => "Idle".to_string(),
        sysinfo::ProcessStatus::Zombie => "Zombie".to_string(),
        _ => format!("{:?}", status).chars().take(15).collect(),
    }
}

/// 単一プロセスの詳細情報を表示
fn print_single_process(process: &sysinfo::Process) {
    println!("Process Information:");
    println!("  PID:     {}", process.pid());
    println!("  Name:    {}", process.name().to_string_lossy());
    println!("  CPU:     {:.2}%", process.cpu_usage());
    println!("  Memory:  {}", format_bytes(process.memory()));
    println!("  Status:  {:?}", process.status());
}

/// 文字列を指定長で切り詰める
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
}

fn format_bytes(bytes: u64) -> String {
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