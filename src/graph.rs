use crate::history::ProcessSnapshot;
use chrono::{DateTime, Local};
use std::collections::VecDeque;

/// グラフ表示用のデータバッファ（リングバッファ）
pub struct GraphData {
    timestamps: VecDeque<DateTime<Local>>,
    memory_data: VecDeque<u64>,     // Total memory in bytes
    cpu_data: VecDeque<f32>,        // Total CPU percentage
    max_capacity: usize,             // Maximum number of data points
}

impl GraphData {
    /// 新しいGraphDataを作成
    pub fn new(capacity: usize) -> Self {
        Self {
            timestamps: VecDeque::with_capacity(capacity),
            memory_data: VecDeque::with_capacity(capacity),
            cpu_data: VecDeque::with_capacity(capacity),
            max_capacity: capacity,
        }
    }

    /// スナップショットを追加（リングバッファとして動作）
    pub fn push_snapshot(&mut self, snapshots: &[ProcessSnapshot]) {
        if snapshots.is_empty() {
            return;
        }

        // 合計メモリとCPUを計算
        let total_memory: u64 = snapshots.iter().map(|s| s.memory_bytes).sum();
        let total_cpu: f32 = snapshots.iter().map(|s| s.cpu_usage).sum();

        // 最新のタイムスタンプを使用（すべて同じと仮定）
        let timestamp = snapshots[0].timestamp;

        // データを追加
        self.timestamps.push_back(timestamp);
        self.memory_data.push_back(total_memory);
        self.cpu_data.push_back(total_cpu);

        // 容量を超えたら古いデータを削除
        if self.timestamps.len() > self.max_capacity {
            self.timestamps.pop_front();
            self.memory_data.pop_front();
            self.cpu_data.pop_front();
        }
    }

    /// Sparkline用のメモリデータを取得
    pub fn get_memory_sparkline_data(&self) -> Vec<u64> {
        self.memory_data.iter().copied().collect()
    }

    /// Sparkline用のCPUデータを取得（u64にスケーリング）
    pub fn get_cpu_sparkline_data(&self) -> Vec<u64> {
        self.cpu_data.iter().map(|&cpu| cpu as u64).collect()
    }

    /// メモリの最大値を取得
    pub fn get_max_memory(&self) -> u64 {
        self.memory_data.iter().copied().max().unwrap_or(0)
    }

    /// CPUの最大値を取得
    pub fn get_max_cpu(&self) -> f32 {
        self.cpu_data.iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// データポイント数を取得
    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    /// データが空かどうか
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo::ProcessStatus;

    #[test]
    fn test_new_graph_data() {
        let graph = GraphData::new(60);
        assert_eq!(graph.len(), 0);
        assert_eq!(graph.get_max_memory(), 0);
        assert_eq!(graph.get_max_cpu(), 0.0);
    }

    #[test]
    fn test_push_snapshot() {
        let mut graph = GraphData::new(3);

        let snapshot1 = ProcessSnapshot {
            timestamp: Local::now(),
            process_name: "test".to_string(),
            pid: 1234,
            cpu_usage: 10.5,
            memory_bytes: 1024 * 1024,
            thread_count: 1,
            status: ProcessStatus::Run,
        };

        let snapshot2 = ProcessSnapshot {
            timestamp: Local::now(),
            process_name: "test2".to_string(),
            pid: 5678,
            cpu_usage: 5.5,
            memory_bytes: 2 * 1024 * 1024,
            thread_count: 1,
            status: ProcessStatus::Run,
        };

        graph.push_snapshot(&[snapshot1.clone(), snapshot2.clone()]);

        assert_eq!(graph.len(), 1);
        assert_eq!(graph.get_max_memory(), 3 * 1024 * 1024); // 1MB + 2MB
        assert_eq!(graph.get_max_cpu(), 16.0); // 10.5 + 5.5
    }

    #[test]
    fn test_ring_buffer_behavior() {
        let mut graph = GraphData::new(2);

        let timestamp1 = Local::now();
        let snapshot1 = ProcessSnapshot {
            timestamp: timestamp1,
            process_name: "test1".to_string(),
            pid: 1,
            cpu_usage: 10.0,
            memory_bytes: 1024,
            thread_count: 1,
            status: ProcessStatus::Run,
        };

        let timestamp2 = timestamp1 + chrono::Duration::seconds(1);
        let snapshot2 = ProcessSnapshot {
            timestamp: timestamp2,
            process_name: "test2".to_string(),
            pid: 2,
            cpu_usage: 20.0,
            memory_bytes: 2048,
            thread_count: 1,
            status: ProcessStatus::Run,
        };

        let timestamp3 = timestamp2 + chrono::Duration::seconds(1);
        let snapshot3 = ProcessSnapshot {
            timestamp: timestamp3,
            process_name: "test3".to_string(),
            pid: 3,
            cpu_usage: 30.0,
            memory_bytes: 3072,
            thread_count: 1,
            status: ProcessStatus::Run,
        };

        // Add the first snapshot
        graph.push_snapshot(&[snapshot1]);
        assert_eq!(graph.len(), 1);

        // Add a second snapshot
        graph.push_snapshot(&[snapshot2]);
        assert_eq!(graph.len(), 2);

        // Add a third snapshot - should evict the first
        graph.push_snapshot(&[snapshot3]);
        assert_eq!(graph.len(), 2); // Still 2, not 3

        // Check that the oldest was removed
        let memory_data = graph.get_memory_sparkline_data();
        assert_eq!(memory_data, vec![2048, 3072]);
    }
}
