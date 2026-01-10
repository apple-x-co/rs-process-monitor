use crate::formatter::{format_bytes, format_status, format_system_memory, format_system_swap, get_tgid, get_thread_count, truncate_string};
use crate::history::ProcessSnapshot;
use crate::tree::{build_process_tree, create_tree_nodes, generate_tree_prefix};
use chrono::Local;
use std::collections::HashSet;
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

    let matching_processes: Vec<_> = sys.processes()
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

    // ツリーノードに変換（TGIDでグループ化される）
    let tree_nodes = create_tree_nodes(&matching_processes);

    // ソート
    let mut sorted_nodes = tree_nodes;
    match sort_order {
        SortOrder::Memory => {
            sorted_nodes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        }
        SortOrder::Cpu => {
            sorted_nodes.sort_by(|a, b| {
                b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Pid => {
            sorted_nodes.sort_by_key(|n| n.pid);
        }
        SortOrder::Name => {
            sorted_nodes.sort_by(|a, b| a.process_name.cmp(&b.process_name));
        }
    }

    // 統計情報の計算（グループ化後のユニークなプロセスから）
    let total_count = sorted_nodes.len();
    let total_memory: u64 = sorted_nodes.iter().map(|n| n.memory_bytes).sum();
    let total_cpu: f32 = sorted_nodes.iter().map(|n| n.cpu_usage).sum();
    let total_threads: usize = sorted_nodes.iter().map(|n| n.thread_count).sum();

    // メモリの統計値（Min/Avg/Max）
    let (min_memory, avg_memory, max_memory) = if total_count > 0 {
        let memories: Vec<u64> = sorted_nodes.iter().map(|n| n.memory_bytes).collect();
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

    println!("Total: {} process(es) ({} threads)", total_count, total_threads);
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

    // ソート済みのユニークなプロセスを表示
    for node in sorted_nodes {
        println!("{:<8} {:<25} {:<8} {:<8.2} {:<12} {:<15}",
                 node.pid,
                 truncate_string(&node.process_name, 25),
                 node.thread_count,
                 node.cpu_usage,
                 format_bytes(node.memory_bytes),
                 format_status(node.status));
    }
}

/// プロセス名でプロセス情報をツリー表示（複数マッチする可能性あり）
pub fn show_processes_by_name_tree(sys: &System, name: &str, sort_order: &SortOrder, min_memory_mb: Option<u64>) {
    let min_memory_bytes = min_memory_mb.map(|mb| mb * 1024 * 1024);

    let matching_processes: Vec<_> = sys.processes()
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

    // ツリーノードに変換
    let tree_nodes = create_tree_nodes(&matching_processes);

    // ツリー構築
    let flattened_tree = build_process_tree(&tree_nodes, sort_order);

    // 統計情報の計算
    let total_count = tree_nodes.len();
    let total_memory: u64 = tree_nodes.iter().map(|n| n.memory_bytes).sum();
    let total_cpu: f32 = tree_nodes.iter().map(|n| n.cpu_usage).sum();
    let total_threads: usize = tree_nodes.iter().map(|n| n.thread_count).sum();

    // メモリの統計値（Min/Avg/Max）
    let (min_memory, avg_memory, max_memory) = if total_count > 0 {
        let memories: Vec<u64> = tree_nodes.iter().map(|n| n.memory_bytes).collect();
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

    println!("=== Process Information (Tree View) ===");
    print!("Processes matching '{}'", name);
    if let Some(min_mb) = min_memory_mb {
        print!(" (>= {} MB)", min_mb);
    }
    println!(" (sorted by {:?}):", sort_order);

    println!("Total: {} process(es) ({} threads)", total_count, total_threads);
    println!("Memory: {} (Min: {}, Avg: {}, Max: {})",
             format_bytes(total_memory),
             format_bytes(min_memory),
             format_bytes(avg_memory),
             format_bytes(max_memory));
    println!("CPU: {:.2}%\n", total_cpu);

    // 表のヘッダー
    println!("{:<8} {:<35} {:<8} {:<8} {:<12} {:<15}",
             "PID", "Name", "Threads", "CPU %", "Memory", "Status");
    println!("{}", "-".repeat(92));

    // ツリー表示
    let mut prefix_stack: Vec<bool> = Vec::new();
    for node in &flattened_tree {
        // プレフィックス更新
        while prefix_stack.len() > node.depth {
            prefix_stack.pop();
        }
        if node.depth > 0 && prefix_stack.len() < node.depth {
            prefix_stack.push(!node.is_last_child);
        }

        let prefix = generate_tree_prefix(node.depth, node.is_last_child, &prefix_stack);
        let max_name_len = 30usize.saturating_sub(node.depth * 3);
        let name_with_prefix = format!("{}{}", prefix, truncate_string(&node.process_name, max_name_len));

        println!("{:<8} {:<35} {:<8} {:<8.2} {:<12} {:<15}",
                 node.pid,
                 name_with_prefix,
                 node.thread_count,
                 node.cpu_usage,
                 format_bytes(node.memory_bytes),
                 format_status(node.status));
    }
}

/// プロセス情報のリストからスナップショットを生成
///
/// TGID でグループ化された後のユニークなプロセスのみを記録する
pub fn create_snapshots(
    sys: &System,
    name: &str,
    min_memory_mb: Option<u64>
) -> Vec<ProcessSnapshot> {
    let min_memory_bytes = min_memory_mb.map(|mb| mb * 1024 * 1024);
    let timestamp = Local::now();

    // 既存のフィルタリング処理を再利用
    let matching_processes: Vec<_> = sys.processes()
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

    // TGID でグループ化（既存の処理と同じ）
    let mut seen_pids = HashSet::new();
    let mut snapshots = Vec::new();

    for (_, process) in matching_processes {
        let lwp = process.pid().as_u32();
        let tgid = get_tgid(lwp);

        if seen_pids.contains(&tgid) {
            continue;
        }
        seen_pids.insert(tgid);

        snapshots.push(ProcessSnapshot {
            timestamp,
            process_name: process.name().to_string_lossy().to_string(),
            pid: tgid,
            cpu_usage: process.cpu_usage(),
            memory_bytes: process.memory(),
            thread_count: get_thread_count(tgid),
            status: process.status(),
        });
    }

    snapshots
}