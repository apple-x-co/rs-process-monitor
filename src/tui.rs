use crate::formatter::{format_bytes, format_status, truncate_string};
use crate::process::SortOrder;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};

pub struct TuiApp {
    should_quit: bool,
    last_update: Instant,
    update_interval: Duration,
}

impl TuiApp {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            should_quit: false,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(interval_secs),
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
) -> Result<(), io::Error> {
    // ターミナルの初期化
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // アプリの実行
    let mut app = TuiApp::new(interval_secs);
    let mut sys = System::new_all();

    let res = run_app(&mut terminal, &mut app, &mut sys, name, sort_order, min_memory_mb);

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
            app.mark_updated();
        }

        // 画面描画
        terminal.draw(|f| {
            ui(f, sys, name, sort_order, min_memory_mb);
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

fn ui(f: &mut Frame, sys: &System, name: &str, sort_order: &SortOrder, min_memory_mb: Option<u64>) {
    // レイアウトの作成
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // ヘッダー（3行→5行に変更）
            Constraint::Min(10),    // プロセステーブル
            Constraint::Length(3),  // フッター
        ])
        .split(f.area());

    // プロセスの抽出とソート（フィルタ追加）
    let min_memory_bytes = min_memory_mb.map(|mb| mb * 1024 * 1024);

    // プロセスの抽出とソート
    let mut matching_processes: Vec<_> = sys.processes()
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
                b.1.cpu_usage().partial_cmp(&a.1.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortOrder::Pid => {
            matching_processes.sort_by_key(|(_, p)| p.pid());
        }
        SortOrder::Name => {
            matching_processes.sort_by(|a, b| {
                a.1.name().to_string_lossy().cmp(&b.1.name().to_string_lossy())
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

    // ヘッダー表示時にフィルタ情報を追加
    let title = if let Some(min_mb) = min_memory_mb {
        format!("Process Monitor: '{}' (>= {} MB) | Sort: {:?}", name, min_mb, sort_order)
    } else {
        format!("Process Monitor: '{}' | Sort: {:?}", name, sort_order)
    };

    // ヘッダー（複数行に変更）
    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                title,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(vec![
            Span::styled(
                format!("Process Monitor: '{}' | Sort: {:?}", name, sort_order),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(vec![
            Span::styled(
                format!("Processes: {} | CPU: {:.2}%", total_count, total_cpu),
                Style::default().fg(Color::White)
            )
        ]),
        Line::from(vec![
            Span::styled(
                format!("Memory: {} (Min: {}, Avg: {}, Max: {})",
                        format_bytes(total_memory),
                        format_bytes(min_memory),
                        format_bytes(avg_memory),
                        format_bytes(max_memory)),
                Style::default().fg(Color::Green)
            )
        ]),
    ];

    let header = Paragraph::new(header_lines)
        .block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(header, chunks[0]);

    // プロセステーブル
    let header_cells = ["PID", "Name", "CPU %", "Memory", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header_row = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = matching_processes.iter().map(|(_, process)| {
        let cells = vec![
            Cell::from(format!("{}", process.pid())),
            Cell::from(truncate_string(&process.name().to_string_lossy(), 20)),
            Cell::from(format!("{:.2}", process.cpu_usage())),
            Cell::from(format_bytes(process.memory())),
            Cell::from(format_status(process.status())),
        ];
        Row::new(cells).height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(22),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(15),
        ],
    )
        .header(header_row)
        .block(Block::default().borders(Borders::ALL).title("Processes"))
        .style(Style::default().fg(Color::White));

    f.render_widget(table, chunks[1]);

    // フッター
    let footer = Paragraph::new("Press 'q' or 'Esc' to quit")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(footer, chunks[2]);
}