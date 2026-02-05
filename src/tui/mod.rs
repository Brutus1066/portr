//! TUI Dashboard module for portr
//!
//! Beautiful real-time port monitoring with keyboard navigation.
//! The "Wow Factor" - designed for viral screenshots!

pub mod app;
pub mod events;
pub mod ui;

use crate::PortrError;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::{Duration, Instant};

pub use app::App;

/// Run the TUI dashboard
pub fn run_dashboard() -> Result<(), PortrError> {
    // Setup terminal
    enable_raw_mode().map_err(|e| PortrError::SystemError(e.to_string()))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| PortrError::SystemError(e.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| PortrError::SystemError(e.to_string()))?;

    // Create app and run
    let mut app = App::new();
    app.refresh_ports();
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().map_err(|e| PortrError::SystemError(e.to_string()))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| PortrError::SystemError(e.to_string()))?;
    terminal
        .show_cursor()
        .map_err(|e| PortrError::SystemError(e.to_string()))?;

    result
}

/// Main event loop
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), PortrError> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let refresh_rate = Duration::from_secs(2);
    let mut last_refresh = Instant::now();

    loop {
        // Draw UI
        terminal
            .draw(|f| ui::draw(f, app))
            .map_err(|e| PortrError::SystemError(e.to_string()))?;

        // Handle events with timeout
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout).map_err(|e| PortrError::SystemError(e.to_string()))? {
            if let Event::Key(key) = event::read().map_err(|e| PortrError::SystemError(e.to_string()))? {
                if key.kind == KeyEventKind::Press {
                    // Handle input mode first
                    if app.input_mode {
                        match key.code {
                            KeyCode::Enter => {
                                app.apply_filter();
                                app.input_mode = false;
                            }
                            KeyCode::Esc => {
                                app.filter_input.clear();
                                app.input_mode = false;
                            }
                            KeyCode::Char(c) => {
                                app.filter_input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.filter_input.pop();
                            }
                            _ => {}
                        }
                    } else if app.show_export {
                        // Export popup mode
                        match key.code {
                            KeyCode::Esc => {
                                app.show_export = false;
                            }
                            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                                app.cycle_export_format();
                            }
                            KeyCode::Enter => {
                                app.do_export();
                            }
                            KeyCode::Char('j') | KeyCode::Char('c') => {
                                app.export_format = app::ExportFormat::Json;
                            }
                            KeyCode::Char('s') | KeyCode::Char('v') => {
                                app.export_format = app::ExportFormat::Csv;
                            }
                            KeyCode::Char('d') | KeyCode::Char('m') => {
                                app.export_format = app::ExportFormat::Markdown;
                            }
                            _ => {}
                        }
                    } else if app.show_menu {
                        // Menu mode navigation - q always exits app from menu
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(()); // Quit app entirely
                            }
                            KeyCode::Esc | KeyCode::Char('m') => {
                                app.show_menu = false; // Just close menu
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.menu_down();
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.menu_up();
                            }
                            KeyCode::Enter => {
                                app.menu_select();
                                if !app.running {
                                    return Ok(());
                                }
                            }
                            KeyCode::Char('1') => {
                                app.menu_selected = 0;
                                app.menu_select();
                            }
                            KeyCode::Char('2') => {
                                app.menu_selected = 1;
                                app.menu_select();
                            }
                            KeyCode::Char('3') => {
                                app.menu_selected = 2;
                                app.menu_select();
                            }
                            KeyCode::Char('4') => {
                                app.menu_selected = 3;
                                app.menu_select();
                            }
                            KeyCode::Char('5') => {
                                app.menu_selected = 4;
                                app.menu_select();
                            }
                            KeyCode::Char('6') => {
                                app.menu_selected = 5;
                                app.menu_select();
                            }
                            KeyCode::Char('7') => {
                                app.menu_selected = 6;
                                app.menu_select();
                            }
                            KeyCode::Char('8') => {
                                app.menu_selected = 7;
                                app.menu_select();
                            }
                            KeyCode::Char('0') => {
                                app.menu_selected = 8;
                                app.menu_select();
                                if !app.running {
                                    return Ok(());
                                }
                            }
                            _ => {}
                        }
                    } else if app.show_help {
                        // Any key closes help
                        app.show_help = false;
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(());
                            }
                            KeyCode::Esc => {
                                // Esc clears filters first, then quits
                                if !app.filter_text.is_empty() || app.docker_only || app.critical_only {
                                    app.clear_filter();
                                    app.docker_only = false;
                                    app.critical_only = false;
                                    app.apply_filters();
                                    app.set_status(&format!("Filters cleared ({} ports)", app.ports.len()));
                                } else {
                                    return Ok(());
                                }
                            }
                            KeyCode::Char('m') => {
                                app.toggle_menu();
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.next();
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.previous();
                            }
                            KeyCode::PageDown => {
                                app.page_down();
                            }
                            KeyCode::PageUp => {
                                app.page_up();
                            }
                            KeyCode::Char('g') => {
                                app.first();
                            }
                            KeyCode::Char('G') => {
                                app.last();
                            }
                            KeyCode::Char('/') => {
                                app.input_mode = true;
                                app.filter_input.clear();
                            }
                            KeyCode::Char('f') => {
                                app.cycle_filter();
                            }
                            KeyCode::Char('K') => {
                                app.kill_selected();
                            }
                            KeyCode::Char('r') | KeyCode::F(5) => {
                                app.refresh_ports();
                            }
                            KeyCode::Char('d') => {
                                app.toggle_docker_filter();
                            }
                            KeyCode::Char('c') => {
                                app.toggle_critical_filter();
                            }
                            KeyCode::Char('e') => {
                                app.toggle_export();
                            }
                            KeyCode::Char('?') => {
                                app.show_help = !app.show_help;
                            }
                            KeyCode::Enter => {
                                app.toggle_details();
                            }
                            KeyCode::Tab => {
                                app.cycle_sort();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Tick
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        // Auto-refresh
        if last_refresh.elapsed() >= refresh_rate {
            app.refresh_ports();
            last_refresh = Instant::now();
        }

        if !app.running {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert!(app.running);
        assert_eq!(app.selected, 0);
    }
}
