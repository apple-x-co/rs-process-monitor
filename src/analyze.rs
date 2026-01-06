use crate::formatter;
use crate::history::{ProcessHistory, ProcessSnapshot};
use chrono::DateTime;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// 分析結果を表す構造体
#[derive(Serialize)]
pub struct AnalysisResult {
    pub time_range: TimeRange,
    pub memory_stats: MemoryStats,
    pub cpu_stats: CpuStats,
    pub process_count: ProcessCountStats,
    pub total_records: usize,
    pub peak_details: Vec<PeakDetail>,
}

/// 分析対象の時間範囲
#[derive(Serialize)]
pub struct TimeRange {
    pub from: String,  // ISO 8601 format
    pub to: String,    // ISO 8601 format
}

/// メモリ統計情報
#[derive(Serialize)]
pub struct MemoryStats {
    pub min_bytes: u64,
    pub avg_bytes: f64,
    pub max_bytes: u64,
}

/// CPU統計情報
#[derive(Serialize)]
pub struct CpuStats {
    pub min_percent: f32,
    pub avg_percent: f64,
    pub max_percent: f32,
}

/// プロセス数統計情報
#[derive(Serialize)]
pub struct ProcessCountStats {
    pub min: usize,
    pub max: usize,
    pub avg: f64,
}

/// ピーク値の詳細情報
#[derive(Serialize)]
pub struct PeakDetail {
    pub metric: String,      // "Memory" or "CPU"
    pub value: String,        // Formatted value
    pub timestamp: String,    // ISO 8601
    pub pid: u32,
    pub process_name: String,
}

/// analyze サブコマンドのエントリーポイント
pub fn run_analyze(
    db_path: &str,
    name: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    format: &OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. データベースファイルの存在確認
    if !std::path::Path::new(db_path).exists() {
        return Err(format!("Database file not found: {}", db_path).into());
    }

    // 2. タイムスタンプの検証
    if let Some(from_time) = from {
        validate_timestamp(from_time)?;
    }
    if let Some(to_time) = to {
        validate_timestamp(to_time)?;
    }

    // 3. データベースを開く
    let history = ProcessHistory::new(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // 4. データをクエリ
    let snapshots = history
        .query_snapshots(from, to, name)
        .map_err(|e| format!("Database query failed: {}", e))?;

    // 5. データが空でないことを確認
    if snapshots.is_empty() {
        return Err(
            "No records found matching the criteria. Try:\n  \
             - Widening the time range\n  \
             - Checking the process name filter\n  \
             - Verifying data exists in the database"
                .into(),
        );
    }

    // 6. 統計を計算
    let analysis = AnalysisResult::from_snapshots(&snapshots)?;

    // 7. 出力
    match format {
        OutputFormat::Table => print_table(&analysis, name),
        OutputFormat::Json => print_json(&analysis)?,
    }

    Ok(())
}

/// タイムスタンプの形式を検証（ISO 8601 / RFC3339）
fn validate_timestamp(ts: &str) -> Result<(), Box<dyn std::error::Error>> {
    DateTime::parse_from_rfc3339(ts).map_err(|_| {
        format!(
            "Invalid timestamp format: '{}'. Expected ISO 8601 (e.g., 2026-01-05T14:00:00+09:00)",
            ts
        )
    })?;
    Ok(())
}

/// 出力フォーマットの種類
#[derive(Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
}

impl AnalysisResult {
    /// スナップショットから統計を計算
    pub fn from_snapshots(
        snapshots: &[ProcessSnapshot],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if snapshots.is_empty() {
            return Err("No snapshots provided for analysis".into());
        }

        // 時間範囲を取得
        let time_range = TimeRange {
            from: snapshots.first().unwrap().timestamp.to_rfc3339(),
            to: snapshots.last().unwrap().timestamp.to_rfc3339(),
        };

        // メモリ統計
        let memory_values: Vec<u64> = snapshots.iter().map(|s| s.memory_bytes).collect();
        let memory_stats = MemoryStats {
            min_bytes: *memory_values.iter().min().unwrap(),
            avg_bytes: memory_values.iter().sum::<u64>() as f64 / memory_values.len() as f64,
            max_bytes: *memory_values.iter().max().unwrap(),
        };

        // CPU統計
        let cpu_values: Vec<f32> = snapshots.iter().map(|s| s.cpu_usage).collect();
        let cpu_stats = CpuStats {
            min_percent: *cpu_values
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
            avg_percent: cpu_values.iter().sum::<f32>() as f64 / cpu_values.len() as f64,
            max_percent: *cpu_values
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        };

        // プロセス数統計（タイムスタンプごとのユニークなPID数）
        let mut counts_by_time: HashMap<String, HashSet<u32>> = HashMap::new();

        for snapshot in snapshots {
            counts_by_time
                .entry(snapshot.timestamp.to_rfc3339())
                .or_insert_with(HashSet::new)
                .insert(snapshot.pid);
        }

        let process_counts: Vec<usize> = counts_by_time.values().map(|set| set.len()).collect();
        let process_count = ProcessCountStats {
            min: *process_counts.iter().min().unwrap(),
            max: *process_counts.iter().max().unwrap(),
            avg: process_counts.iter().sum::<usize>() as f64 / process_counts.len() as f64,
        };

        // ピーク値を見つける
        let peak_memory = snapshots.iter().max_by_key(|s| s.memory_bytes).unwrap();
        let peak_cpu = snapshots
            .iter()
            .max_by(|a, b| a.cpu_usage.partial_cmp(&b.cpu_usage).unwrap())
            .unwrap();

        let peak_details = vec![
            PeakDetail {
                metric: "Memory".to_string(),
                value: formatter::format_bytes(peak_memory.memory_bytes),
                timestamp: peak_memory.timestamp.to_rfc3339(),
                pid: peak_memory.pid,
                process_name: peak_memory.process_name.clone(),
            },
            PeakDetail {
                metric: "CPU".to_string(),
                value: format!("{:.2}%", peak_cpu.cpu_usage),
                timestamp: peak_cpu.timestamp.to_rfc3339(),
                pid: peak_cpu.pid,
                process_name: peak_cpu.process_name.clone(),
            },
        ];

        Ok(AnalysisResult {
            time_range,
            memory_stats,
            cpu_stats,
            process_count,
            total_records: snapshots.len(),
            peak_details,
        })
    }
}

/// テーブル形式で結果を出力
fn print_table(analysis: &AnalysisResult, process_name_filter: Option<&str>) {
    println!("{}", "=".repeat(70));
    println!("Analysis Report");
    println!("{}", "=".repeat(70));

    // 時間範囲
    println!("\nTime Range:");
    println!("  From: {}", analysis.time_range.from);
    println!("  To:   {}", analysis.time_range.to);

    // フィルタ情報
    if let Some(name) = process_name_filter {
        println!("  Filter: process name contains '{}'", name);
    }

    // メモリ統計
    println!("\nMemory Statistics:");
    println!(
        "  Min:  {}",
        formatter::format_bytes(analysis.memory_stats.min_bytes)
    );
    println!(
        "  Avg:  {}",
        formatter::format_bytes(analysis.memory_stats.avg_bytes as u64)
    );
    println!(
        "  Max:  {}",
        formatter::format_bytes(analysis.memory_stats.max_bytes)
    );

    // CPU統計
    println!("\nCPU Statistics:");
    println!("  Min:  {:.2}%", analysis.cpu_stats.min_percent);
    println!("  Avg:  {:.2}%", analysis.cpu_stats.avg_percent);
    println!("  Max:  {:.2}%", analysis.cpu_stats.max_percent);

    // プロセス数
    println!("\nProcess Count:");
    println!(
        "  Range: {}-{}",
        analysis.process_count.min, analysis.process_count.max
    );
    println!("  Avg:   {:.1}", analysis.process_count.avg);

    // ピーク詳細
    println!("\nPeak Details:");
    for peak in &analysis.peak_details {
        println!(
            "  {} Peak: {} at {} (PID: {}, {})",
            peak.metric, peak.value, peak.timestamp, peak.pid, peak.process_name
        );
    }

    // サマリー
    println!("\nTotal Records: {}", analysis.total_records);
    println!("{}", "=".repeat(70));
}

/// JSON形式で結果を出力
fn print_json(analysis: &AnalysisResult) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(analysis)?;
    println!("{}", json);
    Ok(())
}
