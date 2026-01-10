use crate::formatter::{
    format_bytes, format_status, format_system_memory, format_system_swap, get_tgid,
    get_thread_count, truncate_string,
};
use crate::graph::GraphData;
use crate::history::ProcessHistory;
use crate::process::{SortOrder, create_snapshots};
use crate::tree::{build_process_tree, create_tree_nodes, generate_tree_prefix};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table},
};
use std::io;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};

pub struct TuiApp {
    should_quit: bool,
    last_update: Instant,
    update_interval: Duration,
    history: Option<ProcessHistory>,
    graph_data: Option<GraphData>,
    tree_mode: bool,
}

impl TuiApp {
    pub fn new(interval_secs: u64, log_path: Option<&str>, graph_points: usize, tree_mode: bool) -> Self {
        let history = if let Some(path) = log_path {
            match ProcessHistory::new(path) {
                Ok(h) => Some(h),
                Err(e) => {
                    eprintln!("Warning: Failed to initialize history database: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let graph_data = if graph_points > 0 {
            Some(GraphData::new(graph_points))
        } else {
            None
        };

        Self {
            should_quit: false,
            // 起動直後に即座に更新されるように、過去の時刻で初期化
            last_update: Instant::now() - Duration::from_secs(interval_secs),
            update_interval: Duration::from_secs(interval_secs),
            history,
            graph_data,
            tree_mode,
        }
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() >= self.update_interval
    }

    pub fn mark_updated(&mut self) {
        self.last_update = Instant::now();
    }
}

/// TUIモードでプロセス監視を実行
pub fn run_tui(
    name: &str,
    sort_order: &SortOrder,
    interval_secs: u64,
    min_memory_mb: Option<u64>,
    log_path: Option<&str>,
    graph_points: usize,
    tree_mode: bool,
) -> Result<(), io::Error> {
    // ターミナルの初期化
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // アプリの実行
    let mut app = TuiApp::new(interval_secs, log_path, graph_points, tree_mode);
    let mut sys = System::new_all();

    let res = run_app(
        &mut terminal,
        &mut app,
        &mut sys,
        name,
        sort_order,
        min_memory_mb,
    );

    // ターミナルの復元
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
    sys: &mut System,
    name: &str,
    sort_order: &SortOrder,
    min_memory_mb: Option<u64>,
) -> Result<(), io::Error> {
    loop {
        // プロセス情報の更新
        if app.should_update() {
            sys.refresh_processes(ProcessesToUpdate::All, true);

            // スナップショットを作成（履歴とグラフで共有）
            let snapshots = create_snapshots(sys, name, min_memory_mb);

            // グラフデータの更新
            if let Some(ref mut graph) = app.graph_data {
                graph.push_snapshot(&snapshots);
            }

            app.mark_updated();

            // 履歴記録
            if let Some(ref mut hist) = app.history {
                if let Err(_e) = hist.insert_snapshots(&snapshots) {
                    // TUI では eprintln! が画面を壊すので無視
                }
            }
        }

        // 画面描画
        terminal.draw(|f| {
            ui(f, app, sys, name, sort_order, min_memory_mb);
        })?;

        // イベント処理（100msタイムアウト）
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(
    f: &mut Frame,
    app: &TuiApp,
    sys: &System,
    name: &str,
    sort_order: &SortOrder,
    min_memory_mb: Option<u64>,
) {
    // レイアウトの作成（グラフの有無で動的に変更）
    let constraints = if app.graph_data.is_some() {
        vec![
            Constraint::Length(7), // ヘッダー
            Constraint::Length(6), // グラフ（NEW）
            Constraint::Min(10),   // プロセステーブル
            Constraint::Length(3), // フッター
        ]
    } else {
        vec![
            Constraint::Length(7), // ヘッダー
            Constraint::Min(10),   // プロセステーブル
            Constraint::Length(3), // フッター
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // プロセスの抽出とソート
    let min_memory_bytes = min_memory_mb.map(|mb| mb * 1024 * 1024);

    let mut matching_processes: Vec<_> = sys
        .processes()
        .iter()
        .filter(|(_, p)| {
            let matches_name = p.name().to_string_lossy().contains(name);
            let meets_min_memory = if let Some(min_bytes) = min_memory_bytes {
                p.memory() >= min_bytes
            } else {
                true
            };
            matches_name && meets_min_memory
        })
        .collect();

    // ソート
    match sort_order {
        SortOrder::Memory => {
            matching_processes.sort_by(|a, b| b.1.memory().cmp(&a.1.memory()));
        }
        SortOrder::Cpu => {
            matching_processes.sort_by(|a, b| {
                b.1.cpu_usage()
                    .partial_cmp(&a.1.cpu_usage())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Pid => {
            matching_processes.sort_by_key(|(_, p)| p.pid());
        }
        SortOrder::Name => {
            matching_processes.sort_by(|a, b| {
                a.1.name()
                    .to_string_lossy()
                    .cmp(&b.1.name().to_string_lossy())
            });
        }
    }

    // 統計情報
    let total_count = matching_processes.len();
    let total_memory: u64 = matching_processes.iter().map(|(_, p)| p.memory()).sum();
    let total_cpu: f32 = matching_processes.iter().map(|(_, p)| p.cpu_usage()).sum();

    // メモリの統計値（Min/Avg/Max）
    let (min_memory, avg_memory, max_memory) = if total_count > 0 {
        let memories: Vec<u64> = matching_processes.iter().map(|(_, p)| p.memory()).collect();
        let min = *memories.iter().min().unwrap_or(&0);
        let max = *memories.iter().max().unwrap_or(&0);
        let avg = total_memory / total_count as u64;
        (min, avg, max)
    } else {
        (0, 0, 0)
    };

    // スレッド数の集計（TGID でグループ化）
    let mut pid_threads: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (_, process) in &matching_processes {
        let lwp = process.pid().as_u32();
        let tgid = get_tgid(lwp);

        if !pid_threads.contains_key(&tgid) {
            pid_threads.insert(tgid, get_thread_count(tgid));
        }
    }
    let total_threads: usize = pid_threads.values().sum();

    // 実際のプロセス数（ユニークなPID）
    let actual_process_count = pid_threads.len();

    // ===== ヘッダー（システム情報追加） =====
    let title = if let Some(min_mb) = min_memory_mb {
        format!(
            "Process Monitor: '{}' (>= {} MB) | Sort: {:?}",
            name, min_mb, sort_order
        )
    } else {
        format!("Process Monitor: '{}' | Sort: {:?}", name, sort_order)
    };

    let header_lines = vec![
        Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            format_system_memory(sys),
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![Span::styled(
            format_system_swap(sys),
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "Processes: {} ({} threads) | CPU: {:.2}%",
                actual_process_count, total_threads, total_cpu
            ),
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "Memory: {} (Min: {}, Avg: {}, Max: {})",
                format_bytes(total_memory),
                format_bytes(min_memory),
                format_bytes(avg_memory),
                format_bytes(max_memory)
            ),
            Style::default().fg(Color::Green),
        )]),
    ];

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("System & Process Info"),
    );
    f.render_widget(header, chunks[0]);

    // グラフセクション（有効な場合）
    let table_chunk_index = if let Some(ref graph) = app.graph_data {
        render_graphs(f, graph, chunks[1]);
        2 // テーブルは chunks[2] に移動
    } else {
        1 // テーブルは chunks[1] のまま
    };

    // プロセステーブル
    let header_cells = ["PID", "Name", "Threads", "CPU %", "Memory", "Status"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header_row = Row::new(header_cells).height(1).bottom_margin(1);

    // ツリーモードの場合
    let rows: Vec<Row> = if app.tree_mode {
        let tree_nodes = create_tree_nodes(&matching_processes);
        let flattened_tree = build_process_tree(&tree_nodes, sort_order);

        let mut prefix_stack: Vec<bool> = Vec::new();
        flattened_tree.iter().map(|node| {
            // プレフィックス更新
            while prefix_stack.len() > node.depth {
                prefix_stack.pop();
            }
            if node.depth > 0 && prefix_stack.len() < node.depth {
                prefix_stack.push(!node.is_last_child);
            }

            let prefix = generate_tree_prefix(node.depth, node.is_last_child, &prefix_stack);
            let max_name_len = 17usize.saturating_sub(node.depth * 3);
            let name_display = format!("{}{}", prefix, truncate_string(&node.process_name, max_name_len));

            let cells = vec![
                Cell::from(format!("{}", node.pid)),
                Cell::from(name_display),
                Cell::from(format!("{}", node.thread_count)),
                Cell::from(format!("{:.2}", node.cpu_usage)),
                Cell::from(format_bytes(node.memory_bytes)),
                Cell::from(format_status(node.status)),
            ];
            Row::new(cells).height(1)
        }).collect()
    } else {
        // 通常モード: ユニークなPIDだけを表示
        let mut seen_pids = std::collections::HashSet::new();
        matching_processes.iter().filter_map(|(_, process)| {
            let lwp = process.pid().as_u32();
            let tgid = get_tgid(lwp);

            // 既に表示したPIDはスキップ
            if seen_pids.contains(&tgid) {
                return None;
            }
            seen_pids.insert(tgid);

            let thread_count = get_thread_count(tgid);
            let cells = vec![
                Cell::from(format!("{}", tgid)), // LWPではなくTGIDを表示
                Cell::from(truncate_string(&process.name().to_string_lossy(), 20)),
                Cell::from(format!("{}", thread_count)),
                Cell::from(format!("{:.2}", process.cpu_usage())),
                Cell::from(format_bytes(process.memory())),
                Cell::from(format_status(process.status())),
            ];
            Some(Row::new(cells).height(1))
        }).collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),  // PID
            Constraint::Length(20), // Name
            Constraint::Length(8),  // Threads
            Constraint::Length(10), // CPU %
            Constraint::Length(12), // Memory
            Constraint::Length(15), // Status
        ],
    )
    .header(header_row)
    .block(Block::default().borders(Borders::ALL).title("Processes"))
    .style(Style::default().fg(Color::White));

    f.render_widget(table, chunks[table_chunk_index]);

    // フッター
    let footer = Paragraph::new("Press 'q' or 'Esc' to quit")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(footer, chunks[table_chunk_index + 1]);
}

/// グラフセクションをレンダリング
fn render_graphs(f: &mut Frame, graph: &GraphData, area: Rect) {
    // データポイントが不足している場合
    if graph.len() < 2 {
        let placeholder = Paragraph::new("Collecting data for graphs...")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Trends"));
        f.render_widget(placeholder, area);
        return;
    }

    // 2つのSparklineに分割
    let sparkline_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Memory sparkline
            Constraint::Length(3), // CPU sparkline
        ])
        .split(area);

    // メモリSparkline
    let memory_data = graph.get_memory_sparkline_data();
    let max_memory = graph.get_max_memory();

    // データの最大値の1.5倍を上限とすることで、適度な余白を持たせる
    // これにより、トレンド（増減）が視覚的に分かりやすくなる
    let sparkline_max = if max_memory > 0 {
        max_memory * 3 / 2  // 1.5倍
    } else {
        1024 * 1024  // 1MB (最小値)
    };

    let memory_sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Memory Trend ({} points, Max: {})",
            graph.len(),
            format_bytes(max_memory)
        )))
        .data(&memory_data)
        .max(sparkline_max)
        .style(Style::default().fg(Color::Cyan))
        .bar_set(symbols::bar::NINE_LEVELS);

    f.render_widget(memory_sparkline, sparkline_chunks[0]);

    // CPU Sparkline
    let cpu_data = graph.get_cpu_sparkline_data();
    let max_cpu = graph.get_max_cpu();

    let cpu_sparkline = Sparkline::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            "CPU Trend ({} points, Max: {:.2}%)",
            graph.len(),
            max_cpu
        )))
        .data(&cpu_data)
        .max(100) // CPU は 0-100%
        .style(Style::default().fg(Color::Yellow))
        .bar_set(symbols::bar::NINE_LEVELS);

    f.render_widget(cpu_sparkline, sparkline_chunks[1]);
}
