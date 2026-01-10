mod formatter;
mod process;
mod monitor;
mod tui;
mod history;
mod analyze;
mod graph;
mod tree;

use analyze::OutputFormat;
use clap::{Parser, Subcommand};
use monitor::{watch_mode, MonitorArgs};
use process::{show_process_by_pid, show_processes_by_name, show_processes_by_name_tree, SortOrder};
use sysinfo::{ProcessesToUpdate, System};

/// プロセス監視ツール
#[derive(Parser, Debug)]
#[command(name = "rs-process-monitor")]
#[command(about = "A process monitoring and analysis tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    monitor_args: Args,
}

/// サブコマンド
#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze historical process data
    Analyze(AnalyzeArgs),
}

/// analyze サブコマンドの引数
#[derive(Parser, Debug)]
struct AnalyzeArgs {
    /// Path to history database
    #[arg(long, required = true)]
    log: String,

    /// Filter by process name
    #[arg(long)]
    name: Option<String>,

    /// Start time (ISO 8601: 2026-01-05T14:00:00+09:00)
    #[arg(long)]
    from: Option<String>,

    /// End time (ISO 8601: 2026-01-05T16:00:00+09:00)
    #[arg(long)]
    to: Option<String>,

    /// Output format
    #[arg(long, default_value = "table", value_enum)]
    format: OutputFormatArg,
}

/// 出力フォーマット（CLI引数用）
#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormatArg {
    Table,
    Json,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Table => OutputFormat::Table,
            OutputFormatArg::Json => OutputFormat::Json,
        }
    }
}

/// 監視モードの引数
#[derive(Parser, Debug)]
struct Args {
    /// 監視するプロセスのPID
    #[arg(short, long, conflicts_with = "name")]
    pid: Option<u32>,

    /// 監視するプロセス名（部分一致）
    #[arg(short, long, conflicts_with = "pid")]
    name: Option<String>,

    /// リアルタイム監視モード（指定した間隔で更新、単位: 秒）
    #[arg(short, long)]
    watch: Option<u64>,

    /// ソート順: memory (デフォルト), cpu, pid, name
    #[arg(short, long, default_value = "memory")]
    sort: SortOrder,

    /// TUIモードを使用（--watchと併用時のみ有効）
    #[arg(short = 't', long)]
    tui: bool,

    /// 最小メモリ使用量でフィルタ（MB単位、指定値未満のプロセスを除外）
    #[arg(long)]
    min_memory_mb: Option<u64>,

    /// 履歴をSQLiteに記録（watch/tuiモードのみ）
    #[arg(short, long)]
    log: Option<String>,

    /// グラフ表示のデータポイント数（0で無効化）
    #[arg(long, default_value = "60")]
    graph_points: usize,

    /// プロセスをツリー形式で表示
    #[arg(long)]
    tree: bool,
}

fn main() {
    let cli = Cli::parse();

    // サブコマンドのルーティング
    match cli.command {
        Some(Commands::Analyze(analyze_args)) => {
            // analyze サブコマンド
            let format: OutputFormat = analyze_args.format.into();
            if let Err(e) = analyze::run_analyze(
                &analyze_args.log,
                analyze_args.name.as_deref(),
                analyze_args.from.as_deref(),
                analyze_args.to.as_deref(),
                &format,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // サブコマンドなし: 既存の監視モード
            let args = &cli.monitor_args;

            // リアルタイム監視モードの場合
            if let Some(interval) = args.watch {
                if args.tui {
                    // TUIモード
                    if let Some(name) = &args.name {
                        if let Err(e) = tui::run_tui(name, &args.sort, interval, args.min_memory_mb, args.log.as_deref(), args.graph_points, args.tree) {
                            eprintln!("Error running TUI: {}", e);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!("Error: TUI mode requires --name option");
                        std::process::exit(1);
                    }
                } else {
                    // 通常の監視モード
                    let monitor_args = MonitorArgs {
                        pid: args.pid,
                        name: args.name.as_deref(),
                        sort: &args.sort,
                        min_memory_mb: args.min_memory_mb,
                        log_path: args.log.as_deref(),
                        tree: args.tree,
                    };
                    watch_mode(monitor_args, interval);
                }
            } else {
                // 通常モード（1回だけ表示）
                single_shot_mode(args);
            }
        }
    }
}

/// 1回だけ表示するモード
fn single_shot_mode(args: &Args) {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(name) = &args.name {
        if args.tree {
            show_processes_by_name_tree(&sys, name, &args.sort, args.min_memory_mb);
        } else {
            show_processes_by_name(&sys, name, &args.sort, args.min_memory_mb);
        }
    } else {
        let target_pid = args.pid.unwrap_or_else(|| std::process::id());
        show_process_by_pid(&sys, target_pid);
    }
}