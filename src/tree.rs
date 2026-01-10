use crate::formatter::{get_tgid, get_thread_count};
use crate::process::SortOrder;
use std::collections::HashMap;
use sysinfo::{Process, ProcessStatus};

/// ツリー表示用のプロセスノード
#[derive(Debug, Clone)]
pub struct ProcessTreeNode {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub process_name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub thread_count: usize,
    pub status: ProcessStatus,
    pub depth: usize,
    pub is_last_child: bool,
}

/// ツリー描画用の文字定数
pub const TREE_BRANCH: &str = "├─ ";
pub const TREE_LAST: &str = "└─ ";
pub const TREE_VERTICAL: &str = "│  ";
pub const TREE_SPACE: &str = "   ";

/// sysinfo::Process から ProcessTreeNode を作成
pub fn create_tree_node(process: &Process) -> ProcessTreeNode {
    let lwp = process.pid().as_u32();
    let tgid = get_tgid(lwp);

    // 親PID も TGID でグループ化する
    let parent_pid = process.parent().map(|p| {
        let parent_lwp = p.as_u32();
        get_tgid(parent_lwp)
    });

    ProcessTreeNode {
        pid: tgid,
        parent_pid,
        process_name: process.name().to_string_lossy().to_string(),
        cpu_usage: process.cpu_usage(),
        memory_bytes: process.memory(),
        thread_count: get_thread_count(tgid),
        status: process.status(),
        depth: 0,
        is_last_child: false,
    }
}

/// プロセスのリストから ProcessTreeNode のリストを作成（TGID でグループ化）
pub fn create_tree_nodes(processes: &[(&sysinfo::Pid, &Process)]) -> Vec<ProcessTreeNode> {
    let mut tgid_to_process: HashMap<u32, &Process> = HashMap::new();

    for (_, process) in processes {
        let lwp = process.pid().as_u32();
        let tgid = get_tgid(lwp);

        // メインスレッド（LWP == TGID）を優先的に選択
        // これにより、正しい親PID情報を取得できる
        if lwp == tgid {
            tgid_to_process.insert(tgid, process);
        } else if !tgid_to_process.contains_key(&tgid) {
            // まだエントリがなければ追加（メインスレッドが見つからない場合のフォールバック）
            tgid_to_process.insert(tgid, process);
        }
    }

    tgid_to_process
        .values()
        .map(|process| create_tree_node(process))
        .collect()
}

/// プロセスリストからツリー構造を構築してフラット化
pub fn build_process_tree(
    nodes: &[ProcessTreeNode],
    sort_order: &SortOrder,
) -> Vec<ProcessTreeNode> {
    if nodes.is_empty() {
        return Vec::new();
    }

    // Step 1: PID をキーとした HashMap を作成
    let mut nodes_map: HashMap<u32, ProcessTreeNode> = HashMap::new();
    for node in nodes {
        nodes_map.insert(node.pid, node.clone());
    }

    // Step 2: 親子関係を構築
    let pids: Vec<u32> = nodes_map.keys().copied().collect();
    let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut root_pids: Vec<u32> = Vec::new();

    for pid in &pids {
        if let Some(node) = nodes_map.get(pid) {
            if let Some(parent_pid) = node.parent_pid {
                // 自己参照チェック（スレッドの親PIDが自分自身のTGIDを指す場合）
                if parent_pid == *pid {
                    // 自己参照 -> ルートとして扱う
                    root_pids.push(*pid);
                } else if nodes_map.contains_key(&parent_pid) {
                    // 親が検索結果内に存在 -> 子として登録
                    children_map
                        .entry(parent_pid)
                        .or_default()
                        .push(*pid);
                } else {
                    // 親が検索結果外 -> ルートとして扱う
                    root_pids.push(*pid);
                }
            } else {
                // 親なし -> ルート
                root_pids.push(*pid);
            }
        }
    }

    // Step 3: 兄弟間でソート
    sort_siblings(&mut root_pids, &nodes_map, sort_order);
    for children in children_map.values_mut() {
        sort_siblings(children, &nodes_map, sort_order);
    }

    // Step 4: 深さ優先探索でフラット化
    let mut result = Vec::new();
    let mut prefix_stack: Vec<bool> = Vec::new();

    for (i, &pid) in root_pids.iter().enumerate() {
        let is_last = i == root_pids.len() - 1;
        flatten_tree_dfs(
            pid,
            &nodes_map,
            &children_map,
            &mut result,
            &mut prefix_stack,
            0,
            is_last,
        );
    }

    result
}

/// 兄弟プロセス間でソート
fn sort_siblings(
    pids: &mut Vec<u32>,
    nodes_map: &HashMap<u32, ProcessTreeNode>,
    sort_order: &SortOrder,
) {
    match sort_order {
        SortOrder::Memory => {
            pids.sort_by(|a, b| {
                let mem_a = nodes_map.get(a).map(|n| n.memory_bytes).unwrap_or(0);
                let mem_b = nodes_map.get(b).map(|n| n.memory_bytes).unwrap_or(0);
                mem_b.cmp(&mem_a) // 降順
            });
        }
        SortOrder::Cpu => {
            pids.sort_by(|a, b| {
                let cpu_a = nodes_map.get(a).map(|n| n.cpu_usage).unwrap_or(0.0);
                let cpu_b = nodes_map.get(b).map(|n| n.cpu_usage).unwrap_or(0.0);
                cpu_b.partial_cmp(&cpu_a).unwrap_or(std::cmp::Ordering::Equal) // 降順
            });
        }
        SortOrder::Pid => {
            pids.sort(); // 昇順
        }
        SortOrder::Name => {
            pids.sort_by(|a, b| {
                let name_a = nodes_map.get(a).map(|n| n.process_name.as_str()).unwrap_or("");
                let name_b = nodes_map.get(b).map(|n| n.process_name.as_str()).unwrap_or("");
                name_a.cmp(name_b) // 昇順
            });
        }
    }
}

/// 深さ優先探索でツリーをフラット化
fn flatten_tree_dfs(
    pid: u32,
    nodes_map: &HashMap<u32, ProcessTreeNode>,
    children_map: &HashMap<u32, Vec<u32>>,
    result: &mut Vec<ProcessTreeNode>,
    prefix_stack: &mut Vec<bool>,
    depth: usize,
    is_last_child: bool,
) {
    if let Some(node) = nodes_map.get(&pid) {
        let mut flattened_node = node.clone();
        flattened_node.depth = depth;
        flattened_node.is_last_child = is_last_child;
        result.push(flattened_node);

        if let Some(children) = children_map.get(&pid) {
            prefix_stack.push(!is_last_child);
            for (i, &child_pid) in children.iter().enumerate() {
                let child_is_last = i == children.len() - 1;
                flatten_tree_dfs(
                    child_pid,
                    nodes_map,
                    children_map,
                    result,
                    prefix_stack,
                    depth + 1,
                    child_is_last,
                );
            }
            prefix_stack.pop();
        }
    }
}

/// ツリー表示用のプレフィックスを生成
pub fn generate_tree_prefix(
    depth: usize,
    is_last_child: bool,
    prefix_stack: &[bool],
) -> String {
    let mut prefix = String::new();

    // 深さ 0 はプレフィックスなし
    if depth == 0 {
        return prefix;
    }

    // 親レベルのプレフィックス
    for &has_sibling in prefix_stack.iter().take(depth - 1) {
        if has_sibling {
            prefix.push_str(TREE_VERTICAL);
        } else {
            prefix.push_str(TREE_SPACE);
        }
    }

    // 現在のノードのブランチ
    if is_last_child {
        prefix.push_str(TREE_LAST);
    } else {
        prefix.push_str(TREE_BRANCH);
    }

    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tree_prefix_root() {
        // ルートノード
        assert_eq!(generate_tree_prefix(0, true, &[]), "");
        assert_eq!(generate_tree_prefix(0, false, &[]), "");
    }

    #[test]
    fn test_generate_tree_prefix_first_level() {
        // 最初の子（兄弟が続く）
        assert_eq!(generate_tree_prefix(1, false, &[true]), "├─ ");

        // 最後の子
        assert_eq!(generate_tree_prefix(1, true, &[false]), "└─ ");
    }

    #[test]
    fn test_generate_tree_prefix_second_level() {
        // 孫（深さ2）
        assert_eq!(generate_tree_prefix(2, true, &[true, false]), "│  └─ ");
        assert_eq!(generate_tree_prefix(2, false, &[false, true]), "   ├─ ");
    }
}
