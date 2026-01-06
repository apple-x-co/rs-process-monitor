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

#### 次のステップ

**実装優先順位**:
- ✅ Phase 5-1: 履歴記録（完了: 2026-01-05）
- ✅ Phase 5-2: 分析機能（完了: 2026-01-06）
- 🔜 Phase 6: アラート機能（閾値監視とWebhook通知）
- 🔜 Phase 7: グラフ表示（TUI での可視化）
- 🔜 Phase 8: 設定ファイル対応（プロファイル管理）

**今後の拡張案**:
1. **CSV エクスポート** - スプレッドシートでの分析
2. **相対時間指定** - "--since '1 day ago'" のようなパース
3. **トレンド分析** - 時系列での変化を検出
4. **グラフ化** - ASCII art または TUI でのグラフ表示

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
- [ ] グラフ表示（TUI での可視化）- 将来実装予定

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
1. ✅ **履歴記録機能**（Phase 5-1）- **完了（2026-01-05）**
   - SQLite によるデータ蓄積機能を実装
   - watch/TUI モードでリアルタイム記録
   - トランザクションとインデックスでパフォーマンス最適化

2. 🔜 **分析機能**（Phase 5-2）- **次回実装**
   - `analyze` サブコマンドの追加
   - 時間範囲指定でのデータ抽出と統計分析
   - CSV/JSON エクスポート機能

3. **設定ファイル対応**（Phase 8）
   - よく使うオプションの組み合わせを保存
   - UX向上につながる

余裕があれば:
3. **アラート機能**（Phase 6）
   - 自動監視に便利
   - Webhook対応で運用が楽になる

後回しでOK:
4. グラフ表示、プロセスツリー、その他の拡張
   - あると便利だが、必須ではない

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
- ✅ Phase 5-1（履歴記録機能）完了
- 🔜 Phase 5-2（分析機能）が次の目標
- 設定ファイル対応でUX向上
- Apache設定問題は別途調査

---

**開発期間**:
- Phase 1-5: 2025年12月29日
- Phase 6: 2026年1月5日

**作成者**: [@apple-x-co](https://github.com/apple-x-co)
**Claude対話**: Claude Sonnet 4.5