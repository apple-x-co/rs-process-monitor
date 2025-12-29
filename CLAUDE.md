# CLAUDE.md - 開発履歴と次のステップ

このドキュメントは、`rs-process-monitor` の開発過程と今後の拡張案を記録したものです。

## 🎯 プロジェクトの目的

Apache/PHP-FPM などのWebサーバーのメモリ設定を最適化するため、プロセスのメモリ使用状況を詳細に監視できるツールを開発する。

**背景**:
- 実務でApacheのメモリ設定値（`MaxRequestWorkers`）を決定する必要があった
- 既存のツール（`ps`, `top`, `htop`）では統計情報（Min/Avg/Max）が取得しにくい
- システム全体のメモリ状況と合わせて確認したい

## 📊 機能一覧

### ✅ 機能

#### 基本機能
- [x] PID指定でプロセス情報表示
- [x] プロセス名検索（部分一致）
- [x] CPU使用率・メモリ使用量の表示

#### 統計機能
- [x] Min/Avg/Max メモリ統計
- [x] プロセス数・合計メモリ・合計CPU表示
- [x] システムメモリ情報（総メモリ・使用率・空き容量）
- [x] スワップ情報

#### 表示機能
- [x] 通常モード（テキスト表形式）
- [x] TUIモード（リアルタイム更新）
- [x] ソート機能（Memory/CPU/PID/Name）

#### 実用機能
- [x] メモリフィルタ（最小メモリ指定）
- [x] リアルタイム監視（任意の間隔）
- [x] クロスプラットフォーム対応（macOS/Linux）

## 🚀 実装予定機能一覧

### データの永続化と分析（推奨度: ★★★）

#### 機能1: 履歴記録

```rust
// SQLite にデータを保存
rs-process-monitor --name httpd --log history.db --watch 60
```

**実装内容**:
- SQLiteでタイムスタンプ付きデータを保存
- テーブル設計:
  ```sql
  CREATE TABLE process_history (
    id INTEGER PRIMARY KEY,
    timestamp DATETIME,
    process_name TEXT,
    pid INTEGER,
    cpu_usage REAL,
    memory_bytes INTEGER,
    status TEXT
  );
  ```

**メリット**:
- 過去のメモリ使用パターンを分析
- 特定時刻のメモリスパイクを調査
- グラフ化のためのデータ蓄積

**依存クレート**:
- `rusqlite` または `sqlx`

#### 機能2: 履歴分析コマンド

```bash
# 過去1日のデータを分析
rs-process-monitor analyze --log history.db --since "1 day ago"

# 特定時間帯のピーク値
rs-process-monitor analyze --log history.db --from "2025-12-20 14:00" --to "2025-12-20 16:00"
```

**出力例**:

```
Analysis from 2025-12-20 14:00 to 16:00:
  Peak Memory: 2.1 GB at 14:32
  Avg Memory: 1.6 GB
  Peak CPU: 45% at 15:15
  Process Count Range: 140-160
```

### アラート機能（推奨度: ★★☆）

#### 機能3: 閾値アラート

```rust
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

### 高度な表示機能（推奨度: ★☆☆）

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

### 設定管理（推奨度: ★★☆）

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

### その他の拡張（推奨度: ★☆☆）

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

## 🎯 実装の優先順位

すぐに実装すべき（次回開発時）:
1. ✅ **履歴記録機能**
    - データ蓄積と分析は実務で最も価値がある
    - SQLite なら軽量で依存が少ない

2. ✅ **設定ファイル対応**
    - よく使うオプションの組み合わせを保存
    - UX向上につながる

余裕があれば:
3. **アラート機能**
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

### 類似ツール
- `htop` - インタラクティブなプロセスビューア
- `btop` - リソースモニター
- `bottom` - Rust製システムモニター

---

**Claude対話**: Claude Sonnet 4.5