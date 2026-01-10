# CLAUDE.md - 開発履歴と次のステップ

このドキュメントは、`rs-process-monitor` の開発過程と今後の拡張案を記録したものです。

## 🎯 プロジェクトの目的

Apache/PHP-FPM などのWebサーバーのメモリ設定を最適化するため、プロセスのメモリ使用状況を詳細に監視できるツールを開発する。

**背景**:
- 実務でApacheのメモリ設定値（`MaxRequestWorkers`）を決定する必要があった
- 既存のツール（`ps`, `top`, `htop`）では統計情報（Min/Avg/Max）が取得しにくい
- システム全体のメモリ状況と合わせて確認したい

## 📝 開発履歴

### Phase 1: 基本機能の実装 (2025-12-29)

#### Step 1-3: プロジェクトセットアップと基本機能
- プロジェクト名: `rs-process-monitor`
- `sysinfo` クレートを使用してプロセス情報を取得
- 自分自身のプロセス情報を表示する最小実装

**学び**:
- `sysinfo` のAPIバージョンによる違い（`ProcessesToUpdate::All`）
- メモリ単位がバイトであることに注意

#### Step 4-5: CLI引数とフォーマット
- `clap` を使ったCLI引数パース
- PID指定、プロセス名検索の実装
- バイト数を見やすい単位（KB/MB/GB）に変換する関数

**課題**:
- 表のズレ問題（後でTUIで解決）

#### Step 6-7: 複数プロセスの表示と統計情報
- プロセス名での部分一致検索
- 複数プロセスの一覧表示
- 合計メモリ・CPU使用率の表示
- Min/Avg/Max 統計情報の追加

**重要な発見**:
- 統計情報があることで、Apache設定の判断材料になる

### Phase 2: リアルタイム監視とソート機能

#### Step 8-9: リアルタイム更新とソート
- `--watch` オプションでリアルタイム監視
- 画面クリアして定期更新
- メモリ・CPU・PID・名前でのソート機能

**学び**:
- ANSIエスケープシーケンスでの画面クリア
- `chrono` クレートでの時刻表示

#### Step 10: コードのモジュール分割
```
src/
├── main.rs       # エントリーポイント
├── process.rs    # プロセス情報取得・表示
├── formatter.rs  # フォーマット関連
└── monitor.rs    # 監視モード
```

**学び**:
- Rustのモジュールシステム
- 関心の分離による保守性向上

### Phase 3: TUI実装

#### Step 11: TUI (Terminal User Interface)
- `ratatui` + `crossterm` を使用
- インタラクティブな表示
- リアルタイム更新
- キーボード操作（`q`で終了）

**学び**:
- TUIフレームワークの使い方
- イベントループの実装
- レイアウト管理

**成果**:
- 表のズレ問題が解決
- 見た目が大幅に改善

### Phase 4: 実用機能の追加

#### Step 12: 統計情報の強化
- Min/Avg/Max メモリ統計
- より詳細な情報表示

#### Step 13: 最小メモリフィルタ
- `--min-memory-mb` オプション追加
- 親プロセスや異常プロセスの除外
- 正常なワーカープロセスのみの統計

**重要な発見**:
- 異常に小さいメモリのプロセス（896 KB）を発見
- 11月から起動したままの古いプロセスを特定

#### Step 14: システムメモリ情報の追加
- システム全体のメモリ使用状況
- スワップ使用状況
- 一発で全体像が把握できるように

**成果**:
```
System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB available)
Swap: 901.81 MB / 5.00 GB (17.6% used)
```

### Phase 5: マルチスレッドプロセス対応（2025-12-29）

#### Step 15: スレッド数表示とプロセスの正確なカウント

**背景**:
Apache event MPM のようなマルチスレッドプロセスでは、`sysinfo` が LWP（Light Weight Process = スレッドID）を個別の「プロセス」として返していた。このため:
- プロセス数が実際より多く表示される（4個 → 148個）
- 同じプロセスが複数回表示される
- スレッド数の計算が異常（148個 × 平均73スレッド = 10,788スレッド）

**解決策**:
1. `/proc/{lwp}/status` から `Tgid:` (Thread Group ID = 本当のPID) を読み取る関数を追加
2. TGID でプロセスをグループ化
3. 各プロセスのスレッド数を表示
4. ユニークなプロセスだけを表示

**実装内容**:
- `formatter.rs` に `get_tgid()` 関数追加（Linux専用）
- `process.rs` でプロセスをTGIDでグループ化
- テーブルに "Threads" 列を追加
- ヘッダーに "X process(es) (Y threads)" 形式で表示
- TUIモードも同様に対応

**デバッグ過程**:
- 当初、コードが正しいのに反映されない問題が発生
- デバッグ出力で `sysinfo` が返す148個のエントリがすべてユニークなPIDであることを発見
- `ps -eLf` の出力と比較し、LWP と PID の違いを理解
- `/proc/{lwp}/status` の `Tgid:` フィールドで解決

**成果**:
```
Before: Total: 148 process(es) (10788 threads)  ← 異常
After:  Total: 4 process(es) (148 threads)      ← 正確

PID      Name     Threads  CPU %    Memory       Status
--------------------------------------------------------
4170992  httpd    65       0.00     13.51 MB     Sleep
4149852  httpd    81       0.00     12.20 MB     Sleep
4104475  httpd    1        0.00     4.31 MB      Sleep  (親)
4104476  httpd    1        0.00     1.59 MB      Sleep
```

**価値**:
- Apache の設定値との比較が容易に
- 各プロセスのスレッド構成が一目瞭然
- マルチスレッドプロセスの監視に最適化

### Phase 6: データの永続化（履歴記録機能）（2026-01-05）

#### 背景と目的

Phase 5 までで監視機能は完成したが、データの蓄積と分析機能が欠けていた。実務では：
- 過去のメモリ使用パターンを分析したい
- 特定時刻のメモリスパイクを調査したい
- 長期的なトレンドを把握したい

そこで、SQLite を使った履歴記録機能を実装することにした。

#### 実装内容（Phase 5-1: 履歴記録のみ）

**新規モジュール `src/history.rs`**:
- `ProcessSnapshot` 構造体: 1つのプロセスの記録単位
  - タイムスタンプ（ISO 8601形式）
  - プロセス名、PID、CPU使用率、メモリ使用量
  - スレッド数、ステータス
- `ProcessHistory` 構造体: SQLite データベース管理
  - データベース初期化とスキーマ作成
  - トランザクションによる一括挿入

**データベース設計**:
```sql
CREATE TABLE process_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    process_name TEXT NOT NULL,
    pid INTEGER NOT NULL,
    cpu_usage REAL NOT NULL,
    memory_bytes INTEGER NOT NULL,
    thread_count INTEGER NOT NULL,
    status TEXT NOT NULL
);

-- パフォーマンス最適化のためのインデックス
CREATE INDEX idx_timestamp ON process_snapshots(timestamp);
CREATE INDEX idx_pid ON process_snapshots(pid);
CREATE INDEX idx_process_name ON process_snapshots(process_name);
```

**既存モジュールへの統合**:
1. `src/process.rs`: `create_snapshots()` 関数追加
   - 既存のフィルタリング処理を再利用
   - TGID でグループ化してユニークなプロセスのみ記録
2. `src/monitor.rs`: watch モードに履歴記録統合
   - `MonitorArgs` に `log_path` フィールド追加
   - ループ内でスナップショット記録
3. `src/tui.rs`: TUI モードに履歴記録統合
   - `TuiApp` に `history` フィールド追加
   - 更新タイミングで記録
4. `src/main.rs`: `--log` オプション追加

**依存クレート追加**:
```toml
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
```
- `bundled`: SQLite をバンドル（システム依存を回避）
- `chrono`: `DateTime<Local>` を直接保存可能

**エラーハンドリング戦略**:
- 履歴記録は補助機能なので、失敗してもメイン機能（監視）は継続
- DB初期化失敗時: 警告表示 + 続行
- スナップショット挿入失敗（watch）: 警告表示 + 続行
- スナップショット挿入失敗（TUI）: 無視（`eprintln!` が画面を壊すため）

#### 使用例

```bash
# watch モードで履歴記録
rs-process-monitor --name httpd --watch 2 --log /tmp/httpd_history.db

# TUI モードで履歴記録
rs-process-monitor --name httpd --watch 2 --tui --log /tmp/httpd_history.db

# DB の確認
sqlite3 /tmp/httpd_history.db "SELECT * FROM process_snapshots ORDER BY timestamp DESC LIMIT 10;"

# プロセスごとの平均メモリ
SELECT pid, process_name, AVG(memory_bytes)/1024/1024 as avg_mb
FROM process_snapshots
GROUP BY pid
ORDER BY avg_mb DESC;
```

#### 動作確認結果

- ✅ DB ファイル作成成功
- ✅ タイムスタンプ付きデータの記録（ISO 8601形式）
- ✅ TGID グループ化されたプロセスのみ記録
- ✅ トランザクションによる一括挿入
- ✅ インデックス作成確認
- ✅ watch モードと TUI モード両方で動作
- ✅ コンパイル警告なし

#### 学び

**設計パターン**:
- 既存の `MonitorArgs` パターンを踏襲（構造体による引数グループ化）
- 参照中心の設計を維持（`log_path: Option<&'a str>`）
- エラーハンドリングを既存コードと統一（`Result<T, rusqlite::Error>`）

**Rust の機能**:
- 可変参照の必要性（`&mut self` for `insert_snapshots()`）
- `Option<ProcessHistory>` でオプショナルな機能を実装
- `ref mut` パターンで可変借用

**データベース設計**:
- トランザクションでパフォーマンス最適化
- インデックスでクエリ性能確保
- タイムスタンプを ISO 8601 形式で統一

### Phase 8: グラフ表示（TUI での可視化）（2026-01-08）

#### 背景と目的

Phase 7 までで履歴記録と分析機能が完成したが、リアルタイムでのトレンド可視化機能が欠けていた。実務では：
- メモリ使用量の推移をリアルタイムで確認したい
- CPU使用率のスパイクを視覚的に捉えたい
- 数値だけでなく、グラフで直感的にパターンを把握したい

そこで、TUIモードにSparklineウィジェットを使ったグラフ表示機能を実装することにした。

#### 実装内容

**新規モジュール `src/graph.rs`**:
- `GraphData` 構造体: リングバッファ方式のデータ管理
  - `VecDeque<T>` による効率的な循環バッファ
  - メモリとCPUの2系列のデータを保持
  - デフォルト60ポイント（2-10分の履歴、更新間隔により変動）
- メソッド:
  - `new(capacity: usize)`: 指定容量でバッファ初期化
  - `push_snapshot(&mut self, snapshots: &[ProcessSnapshot])`: データポイント追加
  - `get_memory_sparkline_data()`: Sparkline用メモリデータ取得
  - `get_cpu_sparkline_data()`: Sparkline用CPUデータ取得（u64にスケーリング）
  - `get_max_memory()` / `get_max_cpu()`: 最大値取得（スケーリング用）
  - `len()`: データポイント数取得

**TUIへの統合（`src/tui.rs`）**:
- レイアウトの拡張:
  - 3セクション → 4セクションレイアウトに変更
  - Header (7行) + Graphs (6行) + Table (可変) + Footer (3行)
  - `--graph-points 0` の場合は従来の3セクションレイアウトを維持
- `TuiApp` 構造体の拡張:
  - `graph_data: Option<GraphData>` フィールド追加
  - コンストラクタで `graph_points` に応じて初期化
- イベントループの更新:
  - プロセス情報更新時に `GraphData::push_snapshot()` を呼び出し
  - 履歴記録と同じスナップショットを再利用（効率的）
- `render_graphs()` 関数の追加:
  - 2つのSparklineウィジェットを縦に配置
  - メモリSparkline（シアン色、上部）
  - CPU Sparkline（イエロー色、下部）
  - データ不足時（< 2ポイント）は "Collecting data..." プレースホルダー表示

**CLIオプション追加（`src/main.rs`）**:
- `--graph-points <N>` オプション追加（デフォルト: 60）
  - N = 0: グラフ無効化（従来のレイアウト）
  - N > 0: N個のデータポイントをバッファリング
- `run_tui()` 関数のシグネチャ更新

**デザイン特性**:
- **コンパクト**: 6行のみ追加（テーブル領域を大きく維持）
- **軽量**: ~2KB のメモリオーバーヘッド（60ポイント × 2系列）
- **高速**: VecDeque の O(1) 操作、< 1ms/フレーム
- **依存なし**: 既存の ratatui 0.30.0 に含まれる Sparkline ウィジェットを使用

#### 使用例

```bash
# デフォルト設定（60ポイント）でグラフ表示
rs-process-monitor --name httpd --watch 2 --tui

# データポイント数をカスタマイズ（120ポイント = 4-20分）
rs-process-monitor --name httpd --watch 2 --tui --graph-points 120

# グラフを無効化（従来のレイアウト）
rs-process-monitor --name httpd --watch 2 --tui --graph-points 0

# 履歴記録と併用
rs-process-monitor --name httpd --watch 2 --tui --log /tmp/httpd.db

# 最小メモリフィルタと併用
rs-process-monitor --name php-fpm --watch 5 --tui --min-memory-mb 5
```

#### 表示例（TUI）

```
┌─ System & Process Info ──────────────────────────────┐
│ System Memory: 397 MB / 769 MB (51.7%)               │
│ Processes: 4 (148 threads) | CPU: 0.00%              │
│ Memory: 41.61 MB (Min: 1.59 MB, Avg: 10.40 MB, ...)  │
└──────────────────────────────────────────────────────┘
┌─ Memory Trend (60 points, Max: 13.51 MB) ────────────┐
│ ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁      │
├─ CPU Trend (60 points, Max: 2.35%) ──────────────────┤
│ ▁▁▂▃▄▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▁▂▃▄▅▄▃▂▁▁▂▃▄▅▆▇█▇       │
└──────────────────────────────────────────────────────┘
┌─ Processes ───────────────────────────────────────────┐
│ PID     Name    Threads  CPU %   Memory      Status   │
│ 4170992 httpd   65       0.00    13.51 MB    Sleep    │
│ 4149852 httpd   81       0.00    12.20 MB    Sleep    │
│ ...                                                    │
└──────────────────────────────────────────────────────┘
┌─ Help ────────────────────────────────────────────────┐
│ Press 'q' or 'Esc' to quit                            │
└──────────────────────────────────────────────────────┘
```

#### 動作確認結果

- ✅ `src/graph.rs` 作成（GraphData + ユニットテスト3件）
- ✅ リングバッファ動作確認（容量超過時の自動削除）
- ✅ デュアルSparkline表示（メモリ + CPU）
- ✅ リアルタイム更新（2秒間隔でスムーズに動作）
- ✅ `--graph-points` オプション動作確認
- ✅ グラフ無効化モード（`--graph-points 0`）動作確認
- ✅ 履歴記録との併用動作確認
- ✅ コンパイル警告なし
- ✅ 全ユニットテスト合格（3/3）

#### 学び

**設計パターン**:
- リングバッファパターンの実装（VecDeque + capacity管理）
- オプショナル機能の実装（`Option<GraphData>`）
- 動的レイアウト切り替え（グラフ有無で制約を変更）
- データ再利用（履歴記録と同じスナップショットを使用）

**Rust の機能**:
- `VecDeque<T>` の効率的な使用（push_back/pop_front）
- `with_capacity()` による事前メモリ確保
- `iter().copied().collect()` によるデータ変換
- `partial_cmp()` を使った f32 の最大値検索

**TUI 実装**:
- Sparkline ウィジェットの使用（`symbols::bar::NINE_LEVELS`）
- Layout の Constraint による柔軟なレイアウト
- 動的なチャンクインデックス計算
- プレースホルダー表示による UX 向上

**パフォーマンス最適化**:
- メモリオーバーヘッド最小化（~2KB）
- O(1) の push/pop 操作
- フレームあたり < 1ms の描画時間
- 既存機能への影響ゼロ

#### 価値

- **視覚的なトレンド把握**: 数値だけでなくパターンが一目瞭然
- **リアルタイム性**: メモリスパイクを即座に発見
- **Apache/PHP-FPM 最適化**: 負荷時の挙動が視覚的に確認可能
- **コンパクト設計**: テーブル表示領域を最大限維持
- **柔軟性**: データポイント数調整、グラフ無効化が可能

### Phase 7: データ分析機能（2026-01-06）

#### 背景と目的

Phase 6 で履歴記録機能が完成したので、次は蓄積されたデータの分析機能を実装した。実務では：
- 過去のメモリ使用パターンを分析したい
- ピーク時のメモリとCPU使用率を特定したい
- JSON形式でエクスポートして他のツールと連携したい

#### 実装内容

**新規サブコマンド `analyze`**:
- 履歴データベースからデータをクエリして統計分析
- オプションのフィルタ: 時間範囲（--from, --to）、プロセス名（--name）
- 出力形式: Table（デフォルト）、JSON

**新規モジュール `src/analyze.rs`**:
- `AnalysisResult` 構造体: 統計結果を保持（Serialize対応）
  - `TimeRange`: 分析対象の時間範囲
  - `MemoryStats`: メモリの Min/Avg/Max
  - `CpuStats`: CPUの Min/Avg/Max
  - `ProcessCountStats`: プロセス数の範囲と平均
  - `PeakDetail`: ピーク値の詳細（タイムスタンプ、PID、プロセス名）
- `run_analyze()`: エントリーポイント
  - データベース存在確認
  - タイムスタンプ検証（ISO 8601形式）
  - データクエリ
  - 統計計算
  - 出力（Table or JSON）
- `AnalysisResult::from_snapshots()`: 統計計算ロジック
  - メモリとCPUの Min/Avg/Max
  - タイムスタンプごとのユニークなPID数をカウント
  - ピーク値とその発生時刻を特定
- `print_table()`: テーブル形式の出力
  - 既存の `formatter::format_bytes()` を再利用
  - 時間範囲、フィルタ、統計、ピーク詳細を表示
- `print_json()`: JSON形式の出力
  - `serde_json` で整形出力

**`src/history.rs` への追加**:
- `query_snapshots()` メソッド
  - オプショナルなフィルタ（from, to, name）でWHERE句を動的構築
  - LIKE パターンでプロセス名検索
  - RFC3339 文字列を `DateTime<Local>` に変換
- `row_to_snapshot()`: SQLの行を `ProcessSnapshot` に変換
- `parse_status()`: ステータス文字列を `ProcessStatus` enum に変換

**CLI構造の変更（`src/main.rs`）**:
- サブコマンドサポートを追加
  - `Commands` enum: `Analyze(AnalyzeArgs)`
  - `Cli` struct: `Option<Commands>` + flatten された監視モード引数
  - **重要**: `Option<Commands>` により後方互換性を維持
    - サブコマンドなし: 既存の監視モード
    - `analyze`: 新しい分析機能
- `AnalyzeArgs` struct:
  - `--log` (必須): データベースパス
  - `--name` (オプション): プロセス名フィルタ
  - `--from` / `--to` (オプション): ISO 8601形式の時間範囲
  - `--format` (デフォルト: table): 出力形式
- `OutputFormatArg` enum: CLI引数用の enum（ValueEnum derive）

**依存クレート追加**:
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### 使用例

```bash
# 全データの分析
rs-process-monitor analyze --log /tmp/httpd_history.db

# プロセス名でフィルタ
rs-process-monitor analyze --log /tmp/httpd_history.db --name httpd

# JSON形式で出力
rs-process-monitor analyze --log /tmp/httpd_history.db --format json

# 時間範囲指定（ISO 8601形式）
rs-process-monitor analyze --log /tmp/httpd_history.db \
  --from "2026-01-05T14:00:00+09:00" \
  --to "2026-01-05T16:00:00+09:00"
```

#### 出力例（Table形式）

```
======================================================================
Analysis Report
======================================================================

Time Range:
  From: 2026-01-06T17:10:11.980145+09:00
  To:   2026-01-06T17:10:21.164810+09:00

Memory Statistics:
  Min:  8.95 MB
  Avg:  9.33 MB
  Max:  9.41 MB

CPU Statistics:
  Min:  0.15%
  Avg:  1.39%
  Max:  1.83%

Process Count:
  Range: 1-1
  Avg:   1.0

Peak Details:
  Memory Peak: 9.41 MB at 2026-01-06T17:10:21.164810+09:00 (PID: 82886, rs-process-monitor)
  CPU Peak: 1.83% at 2026-01-06T17:10:16.064066+09:00 (PID: 82886, rs-process-monitor)

Total Records: 10
======================================================================
```

#### 動作確認結果

- ✅ サブコマンド `analyze` の追加
- ✅ データベースクエリ（オプショナルフィルタ対応）
- ✅ 統計計算（Min/Avg/Max for メモリ・CPU・プロセス数）
- ✅ ピーク値の特定（タイムスタンプ、PID、プロセス名）
- ✅ Table形式出力（既存のformatter再利用）
- ✅ JSON形式出力（serde_json）
- ✅ エラーハンドリング（DB不存在、無効なタイムスタンプ）
- ✅ 後方互換性の維持（既存コマンドが動作）
- ✅ コンパイル警告なし

#### 学び

**設計パターン**:
- サブコマンドの追加と後方互換性の両立
  - `Option<Commands>` により、サブコマンドなしの場合は既存動作
  - `#[command(flatten)]` で既存の引数を維持
- CLI引数用とロジック用でenumを分ける
  - `OutputFormatArg` (clap::ValueEnum) → `OutputFormat` (analyze.rs)
  - From trait で変換

**Rust の機能**:
- `#[derive(Serialize)]` で構造体を簡単にJSON化
- `match params.len()` でSQLパラメータ数に応じた分岐
- `DateTime::parse_from_rfc3339()` でRFC3339文字列をパース
- `with_timezone(&chrono::Local)` でタイムゾーン変換

**SQL と Rust の役割分担**:
- SQL: フィルタリング（WHERE句）とソート（ORDER BY）
- Rust: 統計計算とピーク検出
  - より柔軟な処理が可能
  - テストしやすい
  - コンテキスト情報（タイムスタンプ、PID）を保持

**エラーハンドリングのベストプラクティス**:
- ユーザーフレンドリーなエラーメッセージ
  - "Database file not found: /path/to/db"
  - "Invalid timestamp format: 'xxx'. Expected ISO 8601 (e.g., ...)"
- トラブルシューティングのヒントを提供
  - "Try: - Widening the time range - Checking the process name filter"

**タイムスタンプの扱い**:
- データベース: RFC3339形式のTEXT
- ProcessSnapshot: `DateTime<Local>`
- ユーザー入力: ISO 8601形式の文字列
- クエリ時にパース・変換が必要

### Phase 9: プロセスツリー表示機能（2026-01-09）

#### 背景と目的

Phase 8 までで履歴記録・分析・グラフ表示機能が完成したが、プロセスの親子関係を視覚的に把握する機能が欠けていた。実務では：
- Apache の親プロセスと子プロセスの関係を確認したい
- プロセスツリー構造を一目で理解したい
- 階層構造を維持しながらソートしたい

そこで、`--tree` オプションを追加してプロセスの親子関係をツリー形式で表示する機能を実装することにした。

#### 実装内容

**新規モジュール `src/tree.rs`**:
- `ProcessTreeNode` 構造体: ツリー表示用のノード
  - pid, parent_pid, process_name, cpu_usage, memory_bytes, thread_count, status
  - depth (ツリーの深さ)、is_last_child (描画用フラグ)
- ツリー描画用の定数:
  - `TREE_BRANCH` ("├─ "), `TREE_LAST` ("└─ ")
  - `TREE_VERTICAL` ("│  "), `TREE_SPACE` ("   ")
- 関数:
  - `create_tree_node()`: sysinfo::Process から ProcessTreeNode を作成
  - `create_tree_nodes()`: プロセスリストを TGID でグループ化してノード化
  - `build_process_tree()`: ツリー構造を構築してフラット化
  - `generate_tree_prefix()`: ASCII プレフィックスを生成
- ユニットテスト: 3件（プレフィックス生成のテスト）

**ツリー構築アルゴリズム**:
1. フィルタリング済みプロセスを `HashMap<pid, Node>` に格納
2. 各ノードの `parent_pid` が HashMap に存在するか確認
   - 存在: 子として登録
   - 不存在: ルートとして扱う
3. 兄弟間でソート適用（Memory/CPU/PID/Name）
4. 深さ優先探索でフラット化（表示順序を決定）

**親PID取得**:
- `sysinfo::Process::parent()` で親PIDを取得（`Option<Pid>`）
- 親PIDも `get_tgid()` でグループ化（Linux）

**既存モジュールへの統合**:
1. `src/process.rs`: `show_processes_by_name_tree()` 関数追加
   - 既存のフィルタリングロジックを再利用
   - ツリー構築後にプレフィックス付きで表示
2. `src/main.rs`: `--tree` オプション追加
   - `single_shot_mode()` で分岐処理
3. `src/monitor.rs`: watch モードへの統合
   - `MonitorArgs` に `tree: bool` フィールド追加
4. `src/tui.rs`: TUI モードへの統合
   - `TuiApp` に `tree_mode: bool` フィールド追加
   - テーブル描画時にツリー表示ロジック分岐

**デザイン特性**:
- **表示範囲**: 検索結果のプロセス同士の親子関係のみ
- **ソート連携**: ツリー構造を優先し、兄弟プロセス間でのみソート適用
- **全モード対応**: 通常モード、watch モード、TUI モード全てで動作

#### 使用例

```bash
# 通常モードでツリー表示
rs-process-monitor --name httpd --tree

# ソート指定（兄弟間でソート）
rs-process-monitor --name httpd --tree --sort memory

# メモリフィルタと組み合わせ
rs-process-monitor --name httpd --tree --min-memory-mb 10

# watch モードでツリー表示
rs-process-monitor --name httpd --watch 2 --tree

# TUI モードでツリー表示
rs-process-monitor --name httpd --watch 2 --tui --tree

# 全オプション組み合わせ
rs-process-monitor --name httpd --watch 2 --tui --tree --min-memory-mb 10 --log /tmp/httpd.db
```

#### 出力例

```
=== Process Information (Tree View) ===
Processes matching 'httpd' (sorted by Memory):
Total: 4 process(es) (148 threads)
Memory: 41.61 MB (Min: 1.59 MB, Avg: 10.40 MB, Max: 13.51 MB)
CPU: 0.00%

PID      Name                                Threads  CPU %    Memory       Status
--------------------------------------------------------------------------------------------
4104475  httpd                               1        0.00     4.31 MB      Sleep
├─ 4170992  httpd                            65       0.00     13.51 MB     Sleep
├─ 4149852  httpd                            81       0.00     12.20 MB     Sleep
└─ 4104476  httpd                            1        0.00     1.59 MB      Sleep
```

#### 動作確認結果

- ✅ `src/tree.rs` 作成（ProcessTreeNode + ツリー構築アルゴリズム）
- ✅ ユニットテスト合格（3/3）
- ✅ 通常モードでのツリー表示動作確認
- ✅ watch モードでのツリー表示動作確認
- ✅ TUI モードでのツリー表示動作確認
- ✅ ソート機能との連携動作確認
- ✅ メモリフィルタとの併用動作確認
- ✅ ビルド成功、コンパイル警告なし

#### 学び

**設計パターン**:
- 新規モジュール分割（関心の分離）
- 既存パターンの踏襲（`Option<bool>` によるオプショナル機能）
- データ変換の層化（Process → TreeNode → Flattened List）

**Rust の機能**:
- `HashMap` による効率的なツリー構築
- 深さ優先探索の実装
- `Vec<bool>` によるスタック管理（プレフィックス生成）

**アルゴリズム設計**:
- 親子関係の解析（検索結果内のみ）
- 兄弟間ソートの実装
- 深さ優先探索による表示順序の決定

**TUI 実装**:
- 既存の描画ロジックとの統合
- 動的なレイアウト切り替え（ツリーモード ON/OFF）

#### 価値

- **視覚的な理解**: プロセスの親子関係が一目瞭然
- **Apache 最適化**: 親プロセスと子プロセスの構成が明確
- **柔軟性**: 既存の全機能（ソート、フィルタ、watch、TUI）と組み合わせ可能

### Phase 9.1: ツリー表示のバグ修正（2026-01-10）

#### 発見された問題

リモートサーバー（CentOS）での実行時に2つの重大なバグを発見：

**問題1: プロセスが消える**
- ツリーモードで PID 86856 が表示されない
- 通常モード: 4プロセス表示 ✅
- ツリーモード: 3プロセスのみ表示 ❌（PID 86856 が消失）

**問題2: メモリ合計が異常**
- 通常モード: Memory: **2.32 GB** ❌（実際の約62倍）
- ツリーモード: Memory: **37.34 MB** ✅（正確）

#### 原因分析

**問題1の原因**: マルチスレッドプロセスの親PID処理

Linux では、各スレッド（LWP）が個別のプロセスとして扱われる。スレッドの `parent()` は、そのプロセスのメインスレッド（TGID）を指すことがあり、これが**自己参照**を引き起こしていた。

```
例: PID 86856 の場合
- メインスレッド: LWP 86856 (TGID: 86856)
- 子スレッド: LWP 86857, 86858, ... (TGID: 86856)
- sysinfo が返す parent(): いずれかのスレッドを基準に取得
- 問題: LWP 86857 の parent() → 86856 → get_tgid() → 86856（自己参照！）
```

この場合、PID 86856 は：
- ルートとして登録されない（parent_pid が存在するため）
- 誰の子としても登録されない（自分自身を親としているため）
- **結果**: ツリー構築時に訪問されず、表示されない

**問題2の原因**: 統計計算のタイミング

通常モードでは、TGID グループ化の**前**に統計計算を行っていたため、全スレッド（148個）のメモリを合計していた。

```
148 threads × 約16 MB ≈ 2,368 MB ≈ 2.32 GB ❌
```

#### 修正内容

**修正1: メインスレッド優先選択（`src/tree.rs`）**

`create_tree_nodes()` で、各 TGID に対して**メインスレッド（LWP == TGID）を優先的に使用**するように変更。

```rust
// 修正前: 最初に見つけたスレッドを使用（順序依存）
for (_, process) in processes {
    let tgid = get_tgid(lwp);
    if !seen_pids.contains(&tgid) {
        nodes.push(create_tree_node(process));  // 任意のスレッドを使用
    }
}

// 修正後: メインスレッドを優先的に選択
let mut tgid_to_process: HashMap<u32, &Process> = HashMap::new();
for (_, process) in processes {
    let lwp = process.pid().as_u32();
    let tgid = get_tgid(lwp);

    if lwp == tgid {
        // メインスレッド優先
        tgid_to_process.insert(tgid, process);
    } else if !tgid_to_process.contains_key(&tgid) {
        // フォールバック
        tgid_to_process.insert(tgid, process);
    }
}
```

これにより、正しい親PID情報を取得できるようになった。

**修正2: 自己参照の検出（`src/tree.rs`）**

`build_process_tree()` で、親PIDが自分自身を指している場合、**ルートとして扱う**ように修正。

```rust
if let Some(parent_pid) = node.parent_pid {
    if parent_pid == *pid {
        // 自己参照 -> ルートとして扱う
        root_pids.push(*pid);
    } else if nodes_map.contains_key(&parent_pid) {
        // 通常の親子関係
        children_map.entry(parent_pid).or_default().push(*pid);
    }
}
```

**修正3: 通常モードの統計計算修正（`src/process.rs`）**

TGID グループ化**後**に統計計算を行うように変更。

```rust
// 修正前: グループ化前に統計計算
let total_memory: u64 = matching_processes.iter().map(|(_, p)| p.memory()).sum();  // 148スレッド分

// 修正後: グループ化後に統計計算
let tree_nodes = create_tree_nodes(&matching_processes);  // TGIDでグループ化
let total_memory: u64 = tree_nodes.iter().map(|n| n.memory_bytes).sum();  // 4プロセス分
```

**修正4: TUI モードの統計計算修正（`src/tui.rs`）**

TUI モードでも同じバグがあったため、通常モードと同様に修正。

```rust
// 修正前: グループ化前に統計計算（ui()関数 248-262行目）
let total_memory: u64 = matching_processes.iter().map(|(_, p)| p.memory()).sum();  // 148スレッド分

// 修正後: グループ化後に統計計算
let tree_nodes = create_tree_nodes(&matching_processes);  // TGIDでグループ化
let total_memory: u64 = tree_nodes.iter().map(|n| n.memory_bytes).sum();  // 4プロセス分
```

さらに、ツリーモード時に `tree_nodes` を再利用するように最適化（340-342行目）:

```rust
// 修正前: tree_nodes を2回作成（統計計算時とツリー描画時）
let rows: Vec<Row> = if app.tree_mode {
    let tree_nodes = create_tree_nodes(&matching_processes);  // 重複呼び出し
    let flattened_tree = build_process_tree(&tree_nodes, sort_order);

// 修正後: 統計計算で作成した tree_nodes を再利用
let rows: Vec<Row> = if app.tree_mode {
    // 統計計算で既に作成した tree_nodes を再利用
    let flattened_tree = build_process_tree(&tree_nodes, sort_order);
```

#### 検証結果

**修正後の出力（リモートサーバー）**:

通常モード:
```
Total: 4 process(es) (148 threads)
Memory: 37.34 MB (Min: 1.36 MB, Avg: 9.33 MB, Max: 16.39 MB)  ✅
```

ツリーモード:
```
Total: 4 process(es) (148 threads)
Memory: 37.34 MB (Min: 1.36 MB, Avg: 9.33 MB, Max: 16.39 MB)  ✅

13640    httpd      1   0.00  3.57 MB   Sleep
├─ 86774    httpd   81  0.00  16.39 MB  Sleep
├─ 86856    httpd   65  0.00  16.02 MB  Sleep  ✅ 表示されるようになった！
└─ 13641    httpd   1   0.00  1.36 MB   Sleep
```

TUI モード:
```
System & Process Info
────────────────────────────────────────
System Memory: 397 MB / 769 MB (51.7%)
Processes: 4 (148 threads) | CPU: 0.00%
Memory: 37.34 MB (Min: 1.36 MB, Avg: 9.33 MB, Max: 16.39 MB)  ✅ 正確になった！
```

**`ps auxfww` との比較**:

```bash
$ ps auxfww | grep httpd
root   13640  0.0  0.4  22520  3656 ?  Ss  1月05  0:23  /usr/sbin/httpd
apache 13641  0.0  0.1  24520  1396 ?  S   1月05  0:00   \_ /usr/sbin/httpd
apache 86774  0.0  2.1 1672044 16780 ? Sl  1月09  0:19   \_ /usr/sbin/httpd
apache 86856  0.0  2.0 1540908 16404 ? Sl  1月09  0:16   \_ /usr/sbin/httpd
```

**完全一致！** ✅

| PID | ps (RSS) | rs-process-monitor | 親子関係 |
|-----|----------|-------------------|---------|
| 13640 | 3656 KB | 3.57 MB | 親 |
| 13641 | 1396 KB | 1.36 MB | 子 |
| 86774 | 16780 KB | 16.39 MB | 子 |
| 86856 | 16404 KB | 16.02 MB | 子 ✅ |

#### 学び

**マルチスレッドプロセスの親PID取得**:
- スレッドの `parent()` は信頼できない場合がある
- メインスレッド（LWP == TGID）を優先的に使用することが重要
- 自己参照のチェックが必須

**統計計算のタイミング**:
- データの正規化（TGID グループ化）後に統計を計算
- 同じロジックを複数箇所で使う場合、共通化が重要
- 通常モード、ツリーモード、TUI モードすべてで `create_tree_nodes()` を共有することで整合性を保証
- **重要**: 新機能追加時、既存の全モードに同じバグがないか確認する必要がある

**デバッグの重要性**:
- `ps` コマンドとの比較が有効
- 実際の本番環境（リモートサーバー）でのテストが不可欠
- ローカル（macOS）では再現しない問題もある

**データ構造の設計**:
- HashMap を使ったメインスレッド優先選択
- 明示的な優先順位制御（if lwp == tgid）
- フォールバック処理の実装

#### 価値

- **正確性の向上**: 全プロセスが確実に表示される
- **統計の信頼性**: 通常モード、ツリーモード、TUI モードすべてで統計が一致
- **実用性**: 本番環境（Apache/PHP-FPM）での正確な監視が可能
- **検証可能性**: `ps` コマンドとの完全一致により、正確性を検証できる
- **パフォーマンス最適化**: TUI モードで `tree_nodes` の再利用により、重複計算を削減

#### 次のステップ

**実装優先順位**:
- ✅ Phase 5-1: 履歴記録（完了: 2026-01-05）
- ✅ Phase 5-2: 分析機能（完了: 2026-01-06）
- ✅ Phase 5-3: グラフ表示（完了: 2026-01-08）
- ✅ Phase 5-4: プロセスツリー表示（完了: 2026-01-09）
- 🔜 Phase 6: アラート機能（閾値監視とWebhook通知）
- 🔜 Phase 7: 設定ファイル対応（プロファイル管理）

**今後の拡張案**:
1. **CSV エクスポート** - スプレッドシートでの分析
2. **相対時間指定** - "--since '1 day ago'" のようなパース
3. **トレンド分析** - 時系列での変化を検出
4. **グラフの拡張** - グラフの切り替え機能（'g'キー）、プロセスごとのグラフ

## 🔍 実務での発見

このツールを使って、以下の重大な問題を発見:

### 問題1: メモリ不足
```
総メモリ: 769 MB
Apache使用メモリ: 1.61 GB  ← システムメモリの2倍以上!
```

### 問題2: スワップ発生
```
Swap: 901.81 MB / 5.00 GB (17.6% used)
```

スワップが発生しているため、パフォーマンスが大幅に低下していた。

### 問題3: プロセス数
```
設定前: 278プロセス（スレッド含む）
正常ワーカー: 146プロセス
異常プロセス: 132プロセス（メモリ896KB〜10MB）
```

### 推奨設定の算出
```
利用可能メモリ: 769 MB
安全マージン: 600 MB
1プロセスあたり: 11.57 MB (Max値)

推奨 MaxRequestWorkers = 600 / 11.57 ≈ 50
```

## 🛠️ Apache設定の試行錯誤（未解決）

Apache event MPM の設定変更を試みたが、反映されず。

**詳細は別ファイルに記録**: [`CLAUDE_APACHE_MPM.md`](./CLAUDE_APACHE_MPM.md)

### 概要
- 目標: 148スレッド → 50スレッド
- 結果: 設定がまったく反映されない
- 試行: 5種類以上の設定方法を試したが、すべて失敗

### 次のステップ
Apache設定の調査は別途継続。優先度の高い調査項目:
1. コンパイル時のデフォルト値確認
2. RPMパッケージの詳細調査
3. SELinux の確認
4. worker/prefork MPM への変更

## 📊 完成した機能一覧

### ✅ 実装済み機能

#### 基本機能
- [x] PID指定でプロセス情報表示
- [x] プロセス名検索（部分一致）
- [x] CPU使用率・メモリ使用量の表示

#### 統計機能
- [x] Min/Avg/Max メモリ統計
- [x] プロセス数・合計メモリ・合計CPU表示
- [x] システムメモリ情報（総メモリ・使用率・空き容量）
- [x] スワップ情報
- [x] スレッド数の表示とカウント（マルチスレッドプロセス対応）

#### 表示機能
- [x] 通常モード（テキスト表形式）
- [x] TUIモード（リアルタイム更新）
- [x] ソート機能（Memory/CPU/PID/Name）
- [x] ツリー表示（プロセスの親子関係を可視化）

#### 実用機能
- [x] メモリフィルタ（最小メモリ指定）
- [x] リアルタイム監視（任意の間隔）
- [x] クロスプラットフォーム対応（macOS/Linux）

#### データ永続化機能（Phase 6）
- [x] 履歴記録（SQLite）- watch/TUI モードでプロセス情報を記録
- [x] タイムスタンプ付きスナップショット（ISO 8601形式）
- [x] トランザクションによる一括挿入
- [x] インデックスによるクエリ最適化

#### データ分析機能（Phase 7）
- [x] analyze サブコマンド - 履歴データの統計分析
- [x] 時間範囲フィルタ（--from, --to）- ISO 8601形式
- [x] プロセス名フィルタ（--name）
- [x] メモリ・CPU統計（Min/Avg/Max）
- [x] プロセス数統計（Range/Avg）
- [x] ピーク値の特定（タイムスタンプ、PID、プロセス名）
- [x] Table形式出力（既存formatter再利用）
- [x] JSON形式出力（他ツールとの連携）
- [x] エラーハンドリング（ユーザーフレンドリーなメッセージ）
- [x] 後方互換性の維持（既存コマンド動作保証）
- [ ] CSV形式出力 - 将来実装予定

#### グラフ表示機能（Phase 8）
- [x] TUI モードでのグラフ可視化（Sparkline ウィジェット）
- [x] メモリ使用量トレンド表示
- [x] CPU使用率トレンド表示
- [x] リングバッファ方式のデータ管理（VecDeque）
- [x] データポイント数のカスタマイズ（--graph-points）
- [x] グラフ無効化オプション（--graph-points 0）
- [x] コンパクト設計（6行のみ追加）
- [x] リアルタイム更新対応
- [ ] グラフ切り替え機能（'g'キー） - 将来実装予定
- [ ] プロセスごとのグラフ表示 - 将来実装予定
- [ ] Chart ウィジェットでの詳細表示 - 将来実装予定

## 🚀 次のステップ（優先順位順）

### ~~Phase 5-1: データの永続化（履歴記録）~~ ✅ 完了（2026-01-05）

履歴記録機能は実装完了。詳細は「Phase 6: データの永続化（履歴記録機能）」セクション参照。

### ~~Phase 5-2: データ分析機能~~ ✅ 完了（2026-01-06）

分析機能は実装完了。詳細は「Phase 7: データ分析機能」セクション参照。

**実装した機能**:
- ✅ analyze サブコマンド
- ✅ 時間範囲フィルタ（--from, --to）
- ✅ プロセス名フィルタ（--name）
- ✅ 統計情報（メモリ・CPU・プロセス数の Min/Avg/Max）
- ✅ ピーク値の特定（タイムスタンプ、PID、プロセス名）
- ✅ Table / JSON 形式出力
- ✅ 後方互換性の維持

**今後の拡張案**:
- CSV エクスポート機能
- 相対時間指定（"1 day ago"）
- トレンド分析（時系列での変化検出）

### Phase 6: アラート機能（推奨度: ★★☆）

#### 機能3: 閾値アラート
```bash
// メモリ使用率が80%超えたら通知
rs-process-monitor --name httpd --watch 10 --alert-memory 80
```

**実装内容**:
- 閾値超過時にログ出力
- デスクトップ通知（Linux: libnotify, macOS: osascript）
- オプション: Webhook通知（Slack等）

**依存クレート**:
- `notify-rust` (デスクトップ通知)
- `reqwest` (Webhook)

**設定例** (`config.toml`):
```toml
[alerts]
memory_threshold = 80  # %
cpu_threshold = 90     # %
process_count_max = 200

[webhooks]
slack_url = "https://hooks.slack.com/..."
```

### Phase 7: 高度な表示機能（推奨度: ★☆☆）

#### 機能4: グラフ表示（ASCII art）
```
Memory Usage (Last 60 seconds):
2.0GB │     ╭╮
      │    ╭╯╰╮
1.5GB │   ╭╯  ╰╮  ╭╮
      │  ╭╯    ╰╮╭╯╰╮
1.0GB │──╯──────╰╯──╰────
      └────────────────────
      0s   15s  30s  45s 60s
```

**依存クレート**:
- `tui-rs` の Sparkline/Chart ウィジェット

#### 機能5: プロセスツリー表示
```
httpd (PID: 1234) - 15 MB
├─ httpd (PID: 1235) - 11 MB [Thread: 25]
├─ httpd (PID: 1236) - 11 MB [Thread: 25]
└─ httpd (PID: 1237) - 11 MB [Thread: 25]
```

### Phase 8: 設定管理（推奨度: ★★☆）

#### 機能6: 設定ファイル対応
```toml
# ~/.config/rs-process-monitor/config.toml
[default]
update_interval = 2
sort_order = "memory"
min_memory_mb = 10

[profiles.apache]
name = "httpd"
min_memory_mb = 11
sort_order = "memory"

[profiles.php-fpm]
name = "php-fpm"
min_memory_mb = 5
```

使い方:
```bash
# プロファイル使用
rs-process-monitor --profile apache --watch 2 --tui
```

**依存クレート**:
- `toml` または `serde_yaml`

### Phase 9: その他の拡張（推奨度: ★☆☆）

#### 機能7: エクスポート機能
```bash
# JSON形式でエクスポート
rs-process-monitor --name httpd --export json > report.json

# CSV形式でエクスポート
rs-process-monitor --name httpd --export csv > report.csv
```

#### 機能8: 比較機能
```bash
# 2つの時点を比較
rs-process-monitor compare --before snapshot1.json --after snapshot2.json
```

#### 機能9: プロセスの詳細情報
```bash
# プロセスの追加情報
rs-process-monitor --name httpd --verbose
```

出力に追加:
- コマンドライン引数
- 起動時刻・実行時間
- 親プロセス情報
- スレッド数
- オープンファイル数

## 🎓 学習成果

このプロジェクトで学んだこと:

### Rustの技術
- クレートの選定と使い方（`sysinfo`, `clap`, `ratatui`, `rusqlite`等）
- モジュール分割とコード設計
- エラーハンドリングのベストプラクティス
- 所有権・借用の実践的な活用（`&mut self`, `ref mut`パターン）
- イテレータとクロージャの使いこなし
- 条件付きコンパイル（`#[cfg(target_os = "linux")]`）
- `Option<T>` によるオプショナルな機能の実装

### システムプログラミング
- プロセス情報の取得方法
- `/proc` ファイルシステムの理解
- スレッドとプロセスの違い
- メモリ管理（RSS, VSZ, 共有メモリ）
- LWP（Light Weight Process）と TGID（Thread Group ID）の違い
- Linux のスレッド実装とスレッドモデル

### データベース設計
- SQLite の基本操作（rusqlite クレート）
- トランザクションによるパフォーマンス最適化
- インデックス設計とクエリ最適化
- タイムスタンプの扱い（ISO 8601形式）
- データの永続化と分析の分離

### 実務スキル
- 要件から設計、実装、テストまでの一連の流れ
- ツールを使った問題発見と分析
- パフォーマンスチューニングの考え方

## 💡 設計の学び

### 良かった点
1. **段階的な機能追加**: 最小機能から始めて徐々に拡張
2. **早期のモジュール分割**: 保守性が高まった
3. **実用性重視**: 実務の課題から出発したため、本当に役立つツールになった

### 改善できる点
1. **エラーハンドリング**: より詳細なエラーメッセージ
2. **テストの追加**: ユニットテスト・統合テストが不足
3. **ドキュメント**: コード内のコメントをもっと充実させる

## 🎯 実装の優先順位

すぐに実装すべき（次回開発時）:
1. ✅ **履歴記録機能**（Phase 6）- **完了（2026-01-05）**
   - SQLite によるデータ蓄積機能を実装
   - watch/TUI モードでリアルタイム記録
   - トランザクションとインデックスでパフォーマンス最適化

2. ✅ **分析機能**（Phase 7）- **完了（2026-01-06）**
   - `analyze` サブコマンドの追加
   - 時間範囲指定でのデータ抽出と統計分析
   - JSON エクスポート機能

3. ✅ **グラフ表示**（Phase 8）- **完了（2026-01-08）**
   - TUI モードでのリアルタイムグラフ可視化
   - Sparkline による メモリ・CPU トレンド表示
   - リングバッファによる効率的なデータ管理

4. ✅ **プロセスツリー表示**（Phase 9）- **完了（2026-01-09）**
   - `--tree` オプションの追加
   - 親子関係の可視化
   - 全モード（通常/watch/TUI）で動作

5. **設定ファイル対応**（Phase 10）
   - よく使うオプションの組み合わせを保存
   - UX向上につながる

余裕があれば:
6. **アラート機能**（Phase 11）
   - 自動監視に便利
   - Webhook対応で運用が楽になる

後回しでOK:
7. その他の拡張
   - CSV エクスポート、相対時間指定など

## 📚 参考リソース

### ドキュメント
- [sysinfo crate](https://docs.rs/sysinfo/)
- [ratatui book](https://ratatui.rs/)
- [clap documentation](https://docs.rs/clap/)
- [rusqlite documentation](https://docs.rs/rusqlite/)

### 類似ツール
- `htop` - インタラクティブなプロセスビューア
- `btop` - リソースモニター
- `bottom` - Rust製システムモニター

## 🎊 まとめ

`rs-process-monitor` は、実務の課題から生まれた実用的なツールです。

**成果**:
- Apache/PHP-FPMのメモリ問題を可視化
- スワップ発生を発見
- メモリ設定の最適化に必要なデータを取得
- **Phase 6で履歴記録機能を実装**（2026-01-05）

**Rustの学習**:
- 8つのフェーズを通じて、クレートの使い方やTUI実装、データベース操作など、実践的な技術を習得
- rusqlite による SQLite 操作
- 可変参照と借用の理解を深める
- オプショナルな機能の実装パターン

**今後**:
- ✅ Phase 6（履歴記録機能）完了
- ✅ Phase 7（分析機能）完了
- ✅ Phase 8（グラフ表示）完了
- ✅ Phase 9（プロセスツリー表示）完了
- 🔜 Phase 10（設定ファイル対応）が次の目標
- Apache設定問題は別途調査

---

**開発期間**:
- Phase 1-5: 2025年12月29日
- Phase 6: 2026年1月5日
- Phase 7: 2026年1月6日
- Phase 8: 2026年1月8日
- Phase 9: 2026年1月9日
- Phase 9.1（バグ修正）: 2026年1月10日

**作成者**: [@apple-x-co](https://github.com/apple-x-co)
**Claude対話**: Claude Sonnet 4.5