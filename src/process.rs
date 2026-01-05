use crate::formatter::{format_bytes, format_status, format_system_memory, format_system_swap, get_tgid, get_thread_count, truncate_string};
use std::collections::HashMap;
use sysinfo::{Pid, System};

/// ソート順の指定
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SortOrder {
    Memory,  // メモリ使用量順（降順）
    Cpu,     // CPU使用率順（降順）
    Pid,     // PID順（昇順）
    Name,    // プロセス名順（昇順）
}

/// PIDでプロセス情報を表示
pub fn show_process_by_pid(sys: &System, target_pid: u32) {
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

/// プロセス名でプロセス情報を表示（複数マッチする可能性あり）
pub fn show_processes_by_name(sys: &System, name: &str, sort_order: &SortOrder, min_memory_mb: Option<u64>) {
    let min_memory_bytes = min_memory_mb.map(|mb| mb * 1024 * 1024);

    let mut matching_processes: Vec<_> = sys.processes()
        .iter()
        .filter(|(_, p)| {
            let matches_name = p.name().to_string_lossy().contains(name);
            let meets_min_memory = if let Some(min_bytes) = min_memory_bytes {
                p.memory() >= min_bytes
            } else {
                true
            };
            matches_name && meets_min_memory
        })
        .collect();

    if matching_processes.is_empty() {
        eprintln!("Error: No processes found matching '{}'", name);
        if let Some(min_mb) = min_memory_mb {
            eprintln!("(with minimum memory filter: {} MB)", min_mb);
        }
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
    let total_memory: u64 = matching_processes.iter().map(|(_, p)| p.memory()).sum();
    let total_cpu: f32 = matching_processes.iter().map(|(_, p)| p.cpu_usage()).sum();

    // スレッド数の集計
    let mut pid_threads: HashMap<u32, usize> = HashMap::new();
    for (_, process) in &matching_processes {
        let lwp = process.pid().as_u32();
        let tgid = get_tgid(lwp);  // 本当のPIDを取得

        if !pid_threads.contains_key(&tgid) {
            pid_threads.insert(tgid, get_thread_count(tgid));
        }
    }
    let total_threads: usize = pid_threads.values().sum();

    // 実際のプロセス数（ユニークなPID）
    let actual_process_count = pid_threads.len();

    // メモリの統計値（Min/Avg/Max）
    let (min_memory, avg_memory, max_memory) = if total_count > 0 {
        let memories: Vec<u64> = matching_processes.iter().map(|(_, p)| p.memory()).collect();
        let min = *memories.iter().min().unwrap_or(&0);
        let max = *memories.iter().max().unwrap_or(&0);
        let avg = total_memory / total_count as u64;
        (min, avg, max)
    } else {
        (0, 0, 0)
    };

    // ===== ヘッダー表示（システム情報追加） =====
    println!("=== System Information ===");
    println!("{}", format_system_memory(sys));
    println!("{}", format_system_swap(sys));
    println!();

    println!("=== Process Information ===");
    print!("Processes matching '{}'", name);
    if let Some(min_mb) = min_memory_mb {
        print!(" (>= {} MB)", min_mb);
    }
    println!(" (sorted by {:?}):", sort_order);

    println!("Total: {} process(es) ({} threads)", actual_process_count, total_threads);
    println!("Memory: {} (Min: {}, Avg: {}, Max: {})",
             format_bytes(total_memory),
             format_bytes(min_memory),
             format_bytes(avg_memory),
             format_bytes(max_memory));
    println!("CPU: {:.2}%\n", total_cpu);

    // 表のヘッダー
    println!("{:<8} {:<25} {:<8} {:<8} {:<12} {:<15}",
             "PID", "Name", "Threads", "CPU %", "Memory", "Status");
    println!("{}", "-".repeat(82));

    // ユニークなPIDだけを表示
    let mut seen_pids = std::collections::HashSet::new();
    for (_, process) in matching_processes {
        let lwp = process.pid().as_u32();
        let tgid = get_tgid(lwp);  // 本当のPIDを取得

        if seen_pids.contains(&tgid) {
            continue;
        }
        seen_pids.insert(tgid);

        let thread_count = get_thread_count(tgid);
        println!("{:<8} {:<25} {:<8} {:<8.2} {:<12} {:<15}",
                 tgid,  // LWPではなくTGIDを表示
                 truncate_string(&process.name().to_string_lossy(), 25),
                 thread_count,
                 process.cpu_usage(),
                 format_bytes(process.memory()),
                 format_status(process.status()));
    }
}