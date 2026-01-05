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

## 🚀 次のステップ（優先順位順）

### Phase 5: データの永続化と分析（推奨度: ★★★）

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

### Phase 6: アラート機能（推奨度: ★★☆）

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
- クレートの選定と使い方（`sysinfo`, `clap`, `ratatui`等）
- モジュール分割とコード設計
- エラーハンドリングのベストプラクティス
- 所有権・借用の実践的な活用
- イテレータとクロージャの使いこなし
- 条件付きコンパイル（`#[cfg(target_os = "linux")]`）

### システムプログラミング
- プロセス情報の取得方法
- `/proc` ファイルシステムの理解
- スレッドとプロセスの違い
- メモリ管理（RSS, VSZ, 共有メモリ）
- LWP（Light Weight Process）と TGID（Thread Group ID）の違い
- Linux のスレッド実装とスレッドモデル

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
1. ✅ **履歴記録機能**（Phase 5-1, 5-2）
   - データ蓄積と分析は実務で最も価値がある
   - SQLite なら軽量で依存が少ない

2. ✅ **設定ファイル対応**（Phase 8）
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

**Rustの学習**:
- 7つ目のRustプロジェクトとして、クレートの使い方やTUI実装など、新しい技術を習得

**今後**:
- 履歴記録機能の追加が最も価値が高い
- 設定ファイル対応でUX向上
- Apache設定問題は別途調査

---

**開発期間**: 2025年12月29日  
**作成者**: [@apple-x-co](https://github.com/apple-x-co)  
**Claude対話**: Claude Sonnet 4.5