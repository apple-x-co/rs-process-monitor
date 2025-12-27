use clap::Parser;
use sysinfo::{System, Pid, ProcessesToUpdate};

#[derive(Parser, Debug)]
#[command(name = "rs-process-monitor")]
#[command(about = "A simple process monitoring tool", long_about = None)]
struct Args {
    /// 監視するプロセスのPID
    #[arg(short, long)]
    pid: Option<u32>,
}

fn main() {
    let args = Args::parse();

    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    // PIDが指定されていなければ自分自身のPIDを使う
    let target_pid = args.pid.unwrap_or_else(|| std::process::id());
    let pid = Pid::from_u32(target_pid);

    if let Some(process) = sys.process(pid) {
        println!("Process Information:");
        println!("  PID:     {}", process.pid());
        println!("  Name:    {}", process.name().to_string_lossy());
        println!("  CPU:     {:.2}%", process.cpu_usage());
        println!("  Memory:  {}", format_bytes(process.memory()));
        println!("  Status:  {:?}", process.status());
    } else {
        eprintln!("Error: Process not found (PID: {})", target_pid);
        std::process::exit(1);
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