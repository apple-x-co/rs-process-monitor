# rs-process-monitor

高速で使いやすいプロセス監視ツール - Rust製

![](SCREENSHOT.png "TUI")

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
  - **グラフ可視化**: メモリ・CPU使用率のトレンドをSparklineで表示
- **ツリー表示**: プロセスの親子関係を視覚的に表示

### 🔧 実用的な機能
- プロセス名での検索（部分一致）
- PID指定での詳細表示
- 最小メモリフィルタ（小さいプロセスを除外）
- リアルタイム監視（任意の更新間隔）
- **履歴記録機能（SQLite）**: プロセス情報をデータベースに記録

### 📊 データ分析機能
- **analyze サブコマンド**: 履歴データの統計分析
- 時間範囲フィルタ（ISO 8601形式）
- メモリ・CPU・プロセス数の統計（Min/Avg/Max）
- ピーク値の特定（タイムスタンプ、PID、プロセス名付き）
- 複数の出力形式（Table、JSON）

### 📈 グラフ表示機能
- **Sparkline グラフ**: TUI モードでメモリ・CPU使用率のトレンドを可視化
- リアルタイム更新（デフォルト60データポイント）
- カスタマイズ可能なデータポイント数（`--graph-points`）
- コンパクト設計（6行のみ追加、テーブル領域を維持）

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

### ツリー表示

```bash
# プロセスの親子関係をツリー形式で表示
rs-process-monitor --name httpd --tree

# ソートと組み合わせ（兄弟プロセス間でソート）
rs-process-monitor --name httpd --tree --sort memory

# メモリフィルタと組み合わせ
rs-process-monitor --name httpd --tree --min-memory-mb 10

# watch モードでツリー表示
rs-process-monitor --name httpd --watch 2 --tree

# TUI モードでツリー表示
rs-process-monitor --name httpd --watch 2 --tui --tree
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

TUIモードではメモリとCPUのトレンドがリアルタイムでグラフ表示されます。

### グラフ表示機能

```bash
# デフォルト（60データポイント = 2-10分の履歴）
rs-process-monitor --name httpd --watch 2 --tui

# データポイント数をカスタマイズ（120ポイント = 4-20分）
rs-process-monitor --name httpd --watch 2 --tui --graph-points 120

# グラフを無効化（従来のレイアウト）
rs-process-monitor --name httpd --watch 2 --tui --graph-points 0
```

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

### データ分析（analyze サブコマンド）

履歴データを統計分析し、ピーク値や平均値を簡単に確認できます。

```bash
# 全データの分析
rs-process-monitor analyze --log /tmp/httpd_history.db

# プロセス名でフィルタ
rs-process-monitor analyze --log /tmp/httpd_history.db --name httpd

# JSON形式で出力（他のツールとの連携）
rs-process-monitor analyze --log /tmp/httpd_history.db --format json

# 時間範囲を指定して分析
rs-process-monitor analyze --log /tmp/httpd_history.db \
  --from "2026-01-05T14:00:00+09:00" \
  --to "2026-01-05T16:00:00+09:00"

# 特定時間帯のプロセスのピーク値を確認
rs-process-monitor analyze --log /tmp/httpd_history.db \
  --name httpd \
  --from "2026-01-05T00:00:00+09:00"
```

出力例:
```
======================================================================
Analysis Report
======================================================================

Time Range:
  From: 2026-01-05T14:00:00+09:00
  To:   2026-01-05T16:00:00+09:00
  Filter: process name contains 'httpd'

Memory Statistics:
  Min:  11.02 MB
  Avg:  11.27 MB
  Max:  13.51 MB

CPU Statistics:
  Min:  0.00%
  Avg:  2.35%
  Max:  15.20%

Process Count:
  Range: 140-160
  Avg:   148.5

Peak Details:
  Memory Peak: 13.51 MB at 2026-01-05T14:32:15+09:00 (PID: 4170992, httpd)
  CPU Peak: 15.20% at 2026-01-05T15:15:42+09:00 (PID: 4149852, httpd)

Total Records: 7200
======================================================================
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

### ツリー表示モード

```
=== System Information ===
System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB available)
Swap: 901.81 MB / 5.00 GB (17.6% used)

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

### TUIモード（グラフ表示付き）

```
┌─ System & Process Info ──────────────────────────────────────────┐
│ Process Monitor: 'httpd' (>= 11 MB) | Sort: Memory               │
│ System Memory: 397.27 MB / 769.15 MB (51.7% used, 371.88 MB ... │
│ Swap: 901.81 MB / 5.00 GB (17.6% used)                           │
│ Processes: 4 (148 threads) | CPU: 0.00%                           │
│ Memory: 1.61 GB (Min: 11.02 MB, Avg: 11.27 MB, Max: 11.57 MB)    │
└───────────────────────────────────────────────────────────────────┘
┌─ Memory Trend (60 points, Max: 13.51 MB) ────────────────────────┐
│ ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁                  │
├─ CPU Trend (60 points, Max: 2.35%) ──────────────────────────────┤
│ ▁▁▂▃▄▅▄▃▂▁▁▂▃▄▅▆▇█▇▆▅▄▃▂▁▁▁▂▃▄▅▄▃▂▁▁▂▃▄▅▆▇█▇                   │
└───────────────────────────────────────────────────────────────────┘
┌─ Processes ───────────────────────────────────────────────────────┐
│ PID      Name     Threads  CPU %    Memory       Status           │
│ ───────────────────────────────────────────────────────────────── │
│ 4170992  httpd    65       0.00     13.51 MB     Sleep            │
│ 4149852  httpd    81       0.00     12.20 MB     Sleep            │
│ 4104475  httpd    1        0.00     4.31 MB      Sleep            │
│ 4104476  httpd    1        0.00     1.59 MB      Sleep            │
└───────────────────────────────────────────────────────────────────┘
┌─ Help ────────────────────────────────────────────────────────────┐
│ Press 'q' or 'Esc' to quit                                        │
└───────────────────────────────────────────────────────────────────┘
```

## コマンドラインオプション

### 監視モード

```
Usage: rs-process-monitor [OPTIONS]

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

      --graph-points <GRAPH_POINTS>
          グラフ表示のデータポイント数（0で無効化）
          [default: 60]

      --tree
          プロセスをツリー形式で表示（親子関係を可視化）

  -h, --help
          ヘルプを表示

  -V, --version
          バージョンを表示
```

### analyze サブコマンド

```
Usage: rs-process-monitor analyze [OPTIONS] --log <LOG>

Options:
      --log <LOG>
          履歴データベースのパス（必須）

      --name <NAME>
          プロセス名でフィルタ（部分一致）

      --from <FROM>
          開始時刻（ISO 8601形式: 2026-01-05T14:00:00+09:00）

      --to <TO>
          終了時刻（ISO 8601形式: 2026-01-05T16:00:00+09:00）

      --format <FORMAT>
          出力形式: table (デフォルト), json
          [default: table]

  -h, --help
          ヘルプを表示
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
# 履歴記録を有効にして監視（グラフ表示付き）
rs-process-monitor --name httpd --watch 2 --tui --min-memory-mb 10 --sort memory --log /tmp/httpd.db
```

負荷をかけた時にメモリの Max 値がどこまで上がるかを確認。
**グラフ表示により、メモリスパイクのタイミングが視覚的に把握できます。**

### 4. 履歴データを分析してピーク値を確認

```bash
# 記録した履歴データを分析
rs-process-monitor analyze --log /tmp/httpd.db --name httpd

# 特定時間帯（高負荷時）のデータを分析
rs-process-monitor analyze --log /tmp/httpd.db \
  --name httpd \
  --from "2026-01-05T12:00:00+09:00" \
  --to "2026-01-05T14:00:00+09:00"
```

ピーク時のメモリ使用量と発生時刻を特定し、より正確な MaxRequestWorkers を算出。

## 技術スタック

- **言語**: Rust
- **依存クレート**:
  - `sysinfo` - システム情報取得
  - `clap` - CLI引数パース
  - `ratatui` - TUI構築
  - `crossterm` - ターミナル制御
  - `chrono` - 時刻処理
  - `rusqlite` - SQLite データベース（履歴記録）
  - `serde` / `serde_json` - JSON シリアライズ（分析データのエクスポート）

## 対応プラットフォーム

- ✅ Linux (CentOS Stream 9, Ubuntu, Debian, etc.)
- ✅ macOS (開発環境)
- ⚠️ Windows (未テスト)

## ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。

## 貢献

バグ報告や機能リクエストは、GitHub の Issues でお気軽にお知らせください。
プルリクエストも歓迎します！