# rs-process-monitor

高速で使いやすいプロセス監視ツール - Rust製

## 概要

`rs-process-monitor` は、サーバーのプロセスとメモリ使用状況をリアルタイムで監視するためのコマンドラインツールです。Apache/PHP-FPMなどのWebサーバーのメモリ設定を最適化する際に特に有用です。

## 特徴

### 📊 システム情報表示
- システム全体のメモリ使用状況（総メモリ・使用率・空き容量）
- スワップ使用状況
- プロセス統計（プロセス数・合計CPU・合計メモリ）

### 📈 詳細な統計情報
- メモリ使用量の Min/Avg/Max 表示
- プロセスごとの CPU使用率・メモリ・ステータス表示
- 複数のソート方法（Memory/CPU/PID/Name）

### 🎨 2つの表示モード
- **通常モード**: テーブル形式の見やすい出力
- **TUIモード**: リアルタイム更新のインタラクティブ表示

### 🔧 実用的な機能
- プロセス名での検索（部分一致）
- PID指定での詳細表示
- 最小メモリフィルタ（小さいプロセスを除外）
- リアルタイム監視（任意の更新間隔）
- **履歴記録機能（SQLite）**: プロセス情報をデータベースに記録

## インストール

### ビルド要件
- Rust 1.70以上

### ビルド手順

```bash
git clone https://github.com/apple-x-co/rs-process-monitor.git
cd rs-process-monitor
cargo build --release
```

バイナリは `target/release/rs-process-monitor` に生成されます。

## 使い方

### 基本的な使用例

```bash
# 自分自身のプロセス情報を表示
rs-process-monitor

# プロセス名で検索（例: Apache）
rs-process-monitor --name httpd

# PIDを指定して表示
rs-process-monitor --pid 1234

# メモリ使用量でソート（デフォルト）
rs-process-monitor --name httpd --sort memory

# CPU使用率でソート
rs-process-monitor --name httpd --sort cpu
```

### フィルタリング

```bash
# 10MB以上のプロセスのみ表示（親プロセスや小さいプロセスを除外）
rs-process-monitor --name httpd --min-memory-mb 10

# 正常なワーカープロセスのみを抽出
rs-process-monitor --name php-fpm --min-memory-mb 5
```

### リアルタイム監視

```bash
# 2秒ごとに更新（通常モード）
rs-process-monitor --name httpd --watch 2

# 1秒ごとに更新（TUIモード）
rs-process-monitor --name httpd --watch 1 --tui

# CPU使用率でソート + TUI
rs-process-monitor --name httpd --watch 2 --tui --sort cpu
```

### TUIモードの操作

- `q` または `Esc`: 終了

### 履歴記録機能（SQLite）

```bash
# watch モードで履歴記録
rs-process-monitor --name httpd --watch 2 --log /tmp/httpd_history.db

# TUI モードで履歴記録
rs-process-monitor --name httpd --watch 2 --tui --log /tmp/httpd_history.db

# 最小メモリフィルタと併用
rs-process-monitor --name httpd --watch 5 --min-memory-mb 10 --log /tmp/httpd_history.db
```

### データベースの確認

```bash
# SQLite CLI でデータを確認
sqlite3 /tmp/httpd_history.db

# 最新10件を表示
SELECT * FROM process_snapshots ORDER BY timestamp DESC LIMIT 10;

# プロセスごとの平均メモリ使用量
SELECT pid, process_name, AVG(memory_bytes)/1024/1024 as avg_mb
FROM process_snapshots
GROUP BY pid
ORDER BY avg_mb DESC;

# 時間範囲指定でのデータ抽出
SELECT * FROM process_snapshots
WHERE timestamp >= '2026-01-05T00:00:00'
ORDER BY timestamp;
```

## 出力例

### 通常モード

```
=== System Information ===
System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB available)
Swap: 901.81 MB / 5.00 GB (17.6% used)

=== Process Information ===
Processes matching 'httpd' (>= 11 MB) (sorted by Memory):
Total: 4 process(es) (148 threads)
Memory: 1.61 GB (Min: 11.02 MB, Avg: 11.27 MB, Max: 11.57 MB)
CPU: 0.00%

PID      Name                      Threads  CPU %    Memory       Status
----------------------------------------------------------------------------------
4170992  httpd                     65       0.00     13.51 MB     Sleep
4149852  httpd                     81       0.00     12.20 MB     Sleep
4104475  httpd                     1        0.00     4.31 MB      Sleep
4104476  httpd                     1        0.00     1.59 MB      Sleep
```

### TUIモード

```
┌─ System & Process Info ─────────────────────────────────────────┐
│ Process Monitor: 'httpd' (>= 11 MB) | Sort: Memory              │
│ System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB ... │
│ Swap: 901.81 MB / 5.00 GB (17.6% used)                          │
│ Processes: 4 (148 threads) | CPU: 0.00%                          │
│ Memory: 1.61 GB (Min: 11.02 MB, Avg: 11.27 MB, Max: 11.57 MB)   │
└──────────────────────────────────────────────────────────────────┘
┌─ Processes ──────────────────────────────────────────────────────┐
│ PID      Name     Threads  CPU %    Memory       Status          │
│ ──────────────────────────────────────────────────────────────── │
│ 4170992  httpd    65       0.00     13.51 MB     Sleep           │
│ 4149852  httpd    81       0.00     12.20 MB     Sleep           │
│ 4104475  httpd    1        0.00     4.31 MB      Sleep           │
│ 4104476  httpd    1        0.00     1.59 MB      Sleep           │
└──────────────────────────────────────────────────────────────────┘
```

## コマンドラインオプション

```
Options:
  -p, --pid <PID>
          監視するプロセスのPID

  -n, --name <NAME>
          監視するプロセス名（部分一致）

  -w, --watch <WATCH>
          リアルタイム監視モード（指定した間隔で更新、単位: 秒）

  -s, --sort <SORT>
          ソート順: memory (デフォルト), cpu, pid, name
          [default: memory]

  -t, --tui
          TUIモードを使用（--watchと併用時のみ有効）

      --min-memory-mb <MIN_MEMORY_MB>
          最小メモリ使用量でフィルタ（MB単位、指定値未満のプロセスを除外）

  -l, --log <LOG>
          履歴をSQLiteに記録（watch/tuiモードのみ）

  -h, --help
          ヘルプを表示

  -V, --version
          バージョンを表示
```

## 実用例: Apache のメモリ設定最適化

### 1. 現在のメモリ使用状況を確認

```bash
rs-process-monitor --name httpd --min-memory-mb 10
```

出力例:
```
System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB available)
Memory: 1.61 GB (Min: 11.02 MB, Avg: 11.27 MB, Max: 11.57 MB)
```

### 2. MaxRequestWorkers の計算

```
利用可能メモリ: 769 MB
安全マージン(20%): 769 MB × 0.8 = 615 MB

MaxRequestWorkers = 615 MB / 11.57 MB ≈ 53
→ 推奨値: 50
```

### 3. リアルタイム監視で負荷時の挙動を確認

```bash
rs-process-monitor --name httpd --watch 2 --tui --min-memory-mb 10 --sort memory
```

負荷をかけた時にメモリの Max 値がどこまで上がるかを確認。

## 技術スタック

- **言語**: Rust
- **依存クレート**:
  - `sysinfo` - システム情報取得
  - `clap` - CLI引数パース
  - `ratatui` - TUI構築
  - `crossterm` - ターミナル制御
  - `chrono` - 時刻処理
  - `rusqlite` - SQLite データベース（履歴記録）

## 対応プラットフォーム

- ✅ Linux (CentOS Stream 9, Ubuntu, Debian, etc.)
- ✅ macOS (開発環境)
- ⚠️ Windows (未テスト)

## ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。

## 貢献

バグ報告や機能リクエストは、GitHub の Issues でお気軽にお知らせください。
プルリクエストも歓迎します！