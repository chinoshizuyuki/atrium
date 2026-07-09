// SPDX-License-Identifier: MIT
//! Atrium TUI 库 — 供 atrium-core 集成的公共接口
//! Atrium TUI Library — Public interface for atrium-core integration.
//!
//! 当 atrium-core 启动时，可以直接调用 `run_tui()` 在前台运行 TUI，
//! 后台运行核心服务（scheduler + HTTP 网关），实现单进程即生命体。
//!
//! When atrium-core starts, it can call `run_tui()` to run TUI in the foreground
//! while the core service (scheduler + HTTP gateway) runs in the background,
//! achieving a single-process digital life.

pub mod app;
pub mod client;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::{anyhow, Result};
use crossterm::cursor::Hide as CursorHide;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use tokio::sync::mpsc;

use app::{App, AppEvent, Message, MessageRole};

/// 启动 TUI 主循环 — 供 atrium-core 集成调用
/// Start the TUI main loop — for atrium-core integration.
///
/// 此函数会：
/// 1. 创建 HTTP 客户端并探测网关健康
/// 2. 初始化终端（alternate screen + raw mode）
/// 3. 进入 TUI 事件循环（SSE 流式对话 + 状态刷新 + 键盘输入）
/// 4. Esc 或 /q 退出后恢复终端
///
/// This function will:
/// 1. Create HTTP client and probe gateway health
/// 2. Initialize terminal (alternate screen + raw mode)
/// 3. Enter TUI event loop (SSE streaming chat + status refresh + keyboard input)
/// 4. Restore terminal on Esc or /q exit
///
/// @param gateway  Atrium HTTP Gateway 地址 / Gateway URL (e.g. "http://127.0.0.1:8080")
/// @param session  会话 ID / Session ID (e.g. "console")
/// @param user     用户 ID / User ID (e.g. "master")
///
/// @return 退出结果 / Exit result
pub async fn run_tui(gateway: String, session: String, user: String) -> Result<()> {
    let gateway = gateway.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // 探测网关健康 — 最多重试 50 次（5 秒），等 core 服务启动
    // Probe gateway health — retry up to 50 times (5s) waiting for core service to start
    let mut probe_ok = false;
    for i in 0..50usize {
        match http.get(format!("{}/health", gateway)).send().await {
            Ok(r) if r.status().is_success() => {
                probe_ok = true;
                break;
            }
            Ok(_) => {}
            Err(_) => {}
        }
        if i == 0 {
            eprintln!("等待 Atrium Gateway 启动: {}", gateway);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if !probe_ok {
        return Err(anyhow!("无法连接到网关 {} — core 服务启动超时", gateway));
    }

    let mut app = App::new(gateway.clone(), session, user, http.clone());

    // 单一事件通道: 所有后台任务 → 主循环
    // Single event channel: all background tasks → main loop
    let (tx, rx) = mpsc::channel::<AppEvent>(128);
    let status_tx = tx.clone();
    let status_gateway = gateway.clone();
    let status_http = http.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = app::refresh_status(&status_http, &status_gateway, &status_tx).await {
                let _ = status_tx.send(AppEvent::StatusError(e.to_string())).await;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // 终端初始化 / Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, CursorHide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app, rx, tx).await;

    // 恢复终端 / Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = &result {
        eprintln!("TUI 错误: {}", e);
    }
    Ok(())
}

/// TUI 主事件循环 / TUI main event loop
async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    mut rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
) -> Result<()> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = Duration::from_millis(80);

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(10));
        tokio::select! {
            Some(ev) = rx.recv() => {
                if app.handle_event(ev) {
                    break;
                }
            }
            _ = tokio::time::sleep(timeout) => {}
        }

        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                last_tick = std::time::Instant::now();
                if handle_key(key, app, &tx).await? {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }
    Ok(())
}

/// 键盘事件处理 / Keyboard event handler
async fn handle_key(key: KeyEvent, app: &mut App, tx: &mpsc::Sender<AppEvent>) -> Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }
    match key.code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Enter => {
            if app.streaming {
                return Ok(false);
            }
            let text = app.input.drain(..).collect::<String>();
            let text = text.trim().to_string();
            if text.is_empty() {
                return Ok(false);
            }
            match text.as_str() {
                "/q" | "/quit" | "/exit" => return Ok(true),
                "/clear" => {
                    app.messages.clear();
                    return Ok(false);
                }
                "/help" => {
                    app.messages.push(Message {
                        role: MessageRole::System,
                        text: "命令: /q 退出 · /clear 清空 · /help 帮助".into(),
                        ts: chrono::Local::now(),
                    });
                    return Ok(false);
                }
                _ => {}
            }
            app.messages.push(Message {
                role: MessageRole::User,
                text: text.clone(),
                ts: chrono::Local::now(),
            });
            app.streaming = true;
            // 为本次对话 spawn 独立的流式任务
            // Spawn an independent streaming task for this conversation
            let tx2 = tx.clone();
            let http = app.http.clone();
            let gateway = app.gateway.clone();
            let session = app.session_id.clone();
            let user = app.user_id.clone();
            tokio::spawn(async move {
                let req = client::ChatRequest {
                    message: text,
                    session_id: session,
                    user_id: user,
                    channel: "tui".into(),
                    model_type: "chat".into(),
                };
                let _ = tx2.send(AppEvent::StreamStart).await;
                match client::chat_stream(&http, &gateway, req).await {
                    Ok(mut srx) => {
                        while let Some(ev) = srx.recv().await {
                            let _ = tx2.send(ev).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx2
                            .send(AppEvent::StreamError(format!("连接失败: {}", e)))
                            .await;
                    }
                }
            });
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }
        _ => {}
    }
    Ok(false)
}

/// TUI 主界面渲染 / TUI main UI rendering
fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(size);

    // 标题栏 / Title bar
    let title = Paragraph::new(format!(
        "  ◈ Atrium  —  {}  ·  会话: {}  ·  用户: {}",
        app.gateway, app.session_id, app.user_id
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(title, chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);

    draw_chat(f, app, body[0]);
    draw_status(f, app, body[1]);

    // 输入框 / Input box
    let input_style = if app.streaming {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let input_prompt = if app.streaming {
        "(atrium 思考中…) ".to_string()
    } else {
        "> ".to_string()
    };
    let input_para = Paragraph::new(format!("{}{}", input_prompt, app.input))
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("输入 (Enter 发送 · Esc 退出 · /help 命令)")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(input_para, chunks[2]);

    if !app.streaming {
        let cursor_x = chunks[2].x
            + 2
            + input_prompt.chars().count() as u16
            + app.input.chars().count() as u16;
        let cursor_y = chunks[2].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // 底部状态栏 / Bottom status bar
    let status_text = if app.streaming {
        format!(
            "流式中… 当前情绪: {} · 模块: {}",
            app.emotion_label, app.module_count
        )
    } else {
        format!(
            "就绪 · 情绪: {} (P={:.2} A={:.2} D={:.2}) · 关系: {} · 成长: {} · 模块: {}",
            app.emotion_label,
            app.pleasure,
            app.arousal,
            app.dominance,
            app.relationship_stage,
            app.maturity_stage,
            app.module_count
        )
    };
    let status = Paragraph::new(status_text).style(Style::default().fg(Color::Green));
    f.render_widget(status, chunks[3]);
}

/// 对话面板渲染 / Chat panel rendering
fn draw_chat(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .messages
        .iter()
        .flat_map(|m| {
            let (prefix, color) = match m.role {
                MessageRole::User => ("你", Color::Cyan),
                MessageRole::Atrium => ("Atrium", Color::Magenta),
                MessageRole::System => ("系统", Color::Yellow),
            };
            let ts = m.ts.format("%H:%M:%S").to_string();
            let header = Line::from(vec![
                Span::styled(format!("[{}] ", ts), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}: ", prefix),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ]);
            let body_lines = wrap_text(&m.text, area.width.saturating_sub(2) as usize);
            let mut out = vec![ListItem::new(header)];
            for bl in body_lines {
                out.push(ListItem::new(Line::from(Span::styled(
                    bl,
                    Style::default().fg(Color::White),
                ))));
            }
            out.push(ListItem::new(Line::from("")));
            out
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("对话")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_stateful_widget(list, area, &mut app.chat_state.list_state);
}

/// 数字生命状态面板渲染 / Digital life status panel rendering
fn draw_status(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    let card = Block::default()
        .borders(Borders::ALL)
        .title("数字生命状态")
        .border_style(Style::default().fg(Color::DarkGray));
    let pad_bar = |label: &str, v: f32| -> String {
        let n = ((v + 1.0) * 10.0).round() as i32;
        let n = n.clamp(0, 20);
        let bar: String = "█".repeat(n as usize) + &"░".repeat((20 - n) as usize);
        format!("{} {} {:>+.3}", label, bar, v)
    };
    let status_lines = vec![
        Line::from(vec![
            Span::styled("情绪: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.emotion_label.clone(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(pad_bar("P", app.pleasure)),
        Line::from(pad_bar("A", app.arousal)),
        Line::from(pad_bar("D", app.dominance)),
        Line::from(""),
        Line::from(format!("关系: {}", app.relationship_stage)),
        Line::from(format!("成长: {}", app.maturity_stage)),
    ];
    let para = Paragraph::new(status_lines).block(card);
    f.render_widget(para, chunks[0]);

    let module_items: Vec<ListItem> = app
        .module_states
        .iter()
        .map(|(name, state)| {
            let (color, mark) = if state.contains("ok") || state.contains("OK") {
                (Color::Green, "✓")
            } else if state.contains("warn") || state.contains("degraded") {
                (Color::Yellow, "!")
            } else {
                (Color::DarkGray, "·")
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", mark), Style::default().fg(color)),
                Span::styled(
                    format!("{:<20}", truncate(name, 20)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(state.clone(), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let list = List::new(module_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("模块健康 ({})", app.module_count))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(list, chunks[1]);
}

/// 截断字符串（按 Unicode 宽度）/ Truncate string by Unicode width
fn truncate(s: &str, n: usize) -> String {
    let mut out = String::new();
    let mut w = 0usize;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if w + cw > n {
            out.push('…');
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

/// 文本换行（按 Unicode 宽度）/ Wrap text by Unicode width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut w = 0usize;
        for word in paragraph.split_whitespace() {
            let ww = unicode_width::UnicodeWidthStr::width(word);
            let sep = if line.is_empty() { 0 } else { 1 };
            if w + sep + ww > width && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line.push_str(word);
                w = ww;
            } else {
                if !line.is_empty() {
                    line.push(' ');
                    w += 1;
                }
                line.push_str(word);
                w += ww;
            }
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
