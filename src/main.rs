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
}

fn main() {
    let args = Args::parse();

    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(name) = args.name {
        // プロセス名で検索
        show_processes_by_name(&sys, &name);
    } else {
        // PIDで検索（指定なしなら自分自身）
        let target_pid = args.pid.unwrap_or_else(|| std::process::id());
        show_process_by_pid(&sys, target_pid);
    }
}

/// PIDでプロセス情報を表示
fn show_process_by_pid(sys: &System, target_pid: u32) {
    let pid = Pid::from_u32(target_pid);

    if let Some(process) = sys.process(pid) {
        print_process_info(process);
    } else {
        eprintln!("Error: Process not found (PID: {})", target_pid);
        std::process::exit(1);
    }
}

/// プロセス名でプロセス情報を表示（複数マッチする可能性あり）
fn show_processes_by_name(sys: &System, name: &str) {
    let mut found = false;

    for (_pid, process) in sys.processes() {
        let process_name = process.name().to_string_lossy();

        // 部分一致で検索
        if process_name.contains(name) {
            if !found {
                println!("Found {} process(es) matching '{}':\n",
                         count_matching_processes(sys, name), name);
                found = true;
            }
            print_process_info(process);
            println!("---");
        }
    }

    if !found {
        eprintln!("Error: No processes found matching '{}'", name);
        std::process::exit(1);
    }
}

/// マッチするプロセスの数をカウント
fn count_matching_processes(sys: &System, name: &str) -> usize {
    sys.processes()
        .values()
        .filter(|p| p.name().to_string_lossy().contains(name))
        .count()
}

/// プロセス情報を整形して表示
fn print_process_info(process: &sysinfo::Process) {
    println!("Process Information:");
    println!("  PID:     {}", process.pid());
    println!("  Name:    {}", process.name().to_string_lossy());
    println!("  CPU:     {:.2}%", process.cpu_usage());
    println!("  Memory:  {}", format_bytes(process.memory()));
    println!("  Status:  {:?}", process.status());
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