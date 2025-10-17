use anyhow::Result;
use crossterm::{event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{Terminal, prelude::*, widgets::*};
use std::{io, time::Duration};
use crate::processor::ProcessorState;

pub async fn run_ui(state: ProcessorState) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.size());
            let title = Paragraph::new("FreshRSS Filter — Press q to quit")
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(title, chunks[0]);

            let status = state.last_run_status.lock().map(|s| s.clone()).unwrap_or_else(|_| "n/a".into());
            let body = Paragraph::new(format!("Last run: {}", status)).wrap(Wrap { trim: true });
            f.render_widget(body, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
