use chrono::{DateTime, Local};
use rusqlite::{Connection, Result, params};
use sysinfo::ProcessStatus;

/// プロセス情報のスナップショット（1つのプロセスの記録単位）
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub timestamp: DateTime<Local>,
    pub process_name: String,
    pub pid: u32,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub thread_count: usize,
    pub status: ProcessStatus,
}

/// 履歴データベース管理
pub struct ProcessHistory {
    conn: Connection,
}

impl ProcessHistory {
    /// 新規データベース接続を作成（ファイルが存在しなければ作成）
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let history = Self { conn };
        history.init_schema()?;
        Ok(history)
    }

    /// テーブルとインデックスを作成
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS process_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                process_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                cpu_usage REAL NOT NULL,
                memory_bytes INTEGER NOT NULL,
                thread_count INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            [],
        )?;

        // インデックス作成
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON process_snapshots(timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pid ON process_snapshots(pid)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_process_name ON process_snapshots(process_name)",
            [],
        )?;

        Ok(())
    }

    /// 複数のスナップショットを一括挿入（トランザクション使用）
    pub fn insert_snapshots(&mut self, snapshots: &[ProcessSnapshot]) -> Result<()> {
        if snapshots.is_empty() {
            return Ok(());
        }

        let tx = self.conn.transaction()?;

        for snapshot in snapshots {
            // ProcessStatus を文字列に変換
            let status_str = format!("{:?}", snapshot.status);

            tx.execute(
                "INSERT INTO process_snapshots
                 (timestamp, process_name, pid, cpu_usage, memory_bytes, thread_count, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    snapshot.timestamp.to_rfc3339(),
                    snapshot.process_name,
                    snapshot.pid,
                    snapshot.cpu_usage,
                    snapshot.memory_bytes as i64,
                    snapshot.thread_count as i64,
                    status_str,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// スナップショットをクエリ（オプションのフィルタ付き）
    pub fn query_snapshots(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        name: Option<&str>,
    ) -> Result<Vec<ProcessSnapshot>> {
        // SQLクエリを構築
        let mut sql = String::from(
            "SELECT timestamp, process_name, pid, cpu_usage, memory_bytes, thread_count, status \
             FROM process_snapshots WHERE 1=1"
        );

        let mut params: Vec<String> = vec![];

        if let Some(from_time) = from {
            sql.push_str(" AND timestamp >= ?");
            params.push(from_time.to_string());
        }

        if let Some(to_time) = to {
            sql.push_str(" AND timestamp <= ?");
            params.push(to_time.to_string());
        }

        if let Some(process_name) = name {
            sql.push_str(" AND process_name LIKE ?");
            params.push(format!("%{}%", process_name));
        }

        sql.push_str(" ORDER BY timestamp ASC");

        // クエリを実行
        let mut stmt = self.conn.prepare(&sql)?;

        let snapshots = match params.len() {
            0 => stmt.query_map([], Self::row_to_snapshot)?,
            1 => stmt.query_map([&params[0]], Self::row_to_snapshot)?,
            2 => stmt.query_map([&params[0], &params[1]], Self::row_to_snapshot)?,
            3 => stmt.query_map([&params[0], &params[1], &params[2]], Self::row_to_snapshot)?,
            _ => unreachable!(),
        }
        .collect::<Result<Vec<_>>>()?;

        Ok(snapshots)
    }

    /// データベースの行をProcessSnapshotに変換
    fn row_to_snapshot(row: &rusqlite::Row) -> Result<ProcessSnapshot> {
        let timestamp_str: String = row.get(0)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?
            .with_timezone(&chrono::Local);

        let status_str: String = row.get(6)?;
        let status = Self::parse_status(&status_str);

        Ok(ProcessSnapshot {
            timestamp,
            process_name: row.get(1)?,
            pid: row.get(2)?,
            cpu_usage: row.get(3)?,
            memory_bytes: row.get::<_, i64>(4)? as u64,
            thread_count: row.get::<_, i64>(5)? as usize,
            status,
        })
    }

    /// ステータス文字列をProcessStatusに変換
    fn parse_status(status_str: &str) -> ProcessStatus {
        match status_str {
            "Run" => ProcessStatus::Run,
            "Sleep" => ProcessStatus::Sleep,
            "Idle" => ProcessStatus::Idle,
            "Zombie" => ProcessStatus::Zombie,
            _ => ProcessStatus::Unknown(0),
        }
    }
}
