//! Interactive mode with keyboard navigation
//!
//! Provides a beautiful interactive TUI for port inspection and management.

use std::io::{self, Write};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::{display, port::{self, PortInfo}, process, services, PortrError};

/// Interactive mode state
pub struct InteractiveApp {
    /// All ports being displayed
    ports: Vec<PortInfo>,
    /// Currently selected index
    selected: usize,
    /// Filter string
    filter: String,
    /// Filtered port indices
    filtered_indices: Vec<usize>,
    /// Show help overlay
    show_help: bool,
    /// Status message
    status: Option<String>,
    /// Terminal size
    term_size: (u16, u16),
}

impl InteractiveApp {
    /// Create a new interactive app
    pub fn new() -> Result<Self, PortrError> {
        let ports = port::get_listening_ports()?;
        let filtered_indices: Vec<usize> = (0..ports.len()).collect();
        let term_size = terminal::size().unwrap_or((80, 24));
        
        Ok(Self {
            ports,
            selected: 0,
            filter: String::new(),
            filtered_indices,
            show_help: false,
            status: None,
            term_size,
        })
    }

    /// Refresh port list
    pub fn refresh(&mut self) -> Result<(), PortrError> {
        self.ports = port::get_listening_ports()?;
        self.apply_filter();
        if self.selected >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
        self.status = Some("Refreshed".to_string());
        Ok(())
    }

    /// Apply current filter
    fn apply_filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.ports.len()).collect();
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.filtered_indices = self.ports
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    p.port.to_string().contains(&filter_lower)
                        || p.process_name.to_lowercase().contains(&filter_lower)
                        || p.protocol.to_lowercase().contains(&filter_lower)
                        || services::short_name(p.port)
                            .map(|s| s.to_lowercase().contains(&filter_lower))
                            .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }
        
        // Reset selection if needed
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }

    /// Get currently selected port
    fn selected_port(&self) -> Option<&PortInfo> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.ports.get(i))
    }

    /// Run the interactive loop
    pub fn run(&mut self) -> Result<(), PortrError> {
        // Enter raw mode and alternate screen
        terminal::enable_raw_mode().map_err(|e| PortrError::IoError(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)
            .map_err(|e| PortrError::IoError(e.to_string()))?;

        let result = self.main_loop(&mut stdout);

        // Cleanup
        execute!(stdout, Show, LeaveAlternateScreen)
            .map_err(|e| PortrError::IoError(e.to_string()))?;
        terminal::disable_raw_mode().map_err(|e| PortrError::IoError(e.to_string()))?;

        result
    }

    fn main_loop(&mut self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        loop {
            self.term_size = terminal::size().unwrap_or((80, 24));
            self.draw(stdout)?;

            if event::poll(std::time::Duration::from_millis(100))
                .map_err(|e| PortrError::IoError(e.to_string()))?
            {
                if let Event::Key(key) = event::read().map_err(|e| PortrError::IoError(e.to_string()))? {
                    match self.handle_key(key, stdout)? {
                        Action::Continue => {}
                        Action::Quit => break,
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent, stdout: &mut io::Stdout) -> Result<Action, PortrError> {
        // Clear status on any key
        self.status = None;

        // Handle help overlay
        if self.show_help {
            self.show_help = false;
            return Ok(Action::Continue);
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Action::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Action::Quit)
            }

            // Navigation - check modifiers first for 'k'
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Char('k') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered_indices.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.selected = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                if !self.filtered_indices.is_empty() {
                    self.selected = self.filtered_indices.len() - 1;
                }
            }
            KeyCode::PageUp => {
                let page = (self.term_size.1 as usize).saturating_sub(10);
                self.selected = self.selected.saturating_sub(page);
            }
            KeyCode::PageDown => {
                let page = (self.term_size.1 as usize).saturating_sub(10);
                self.selected = (self.selected + page).min(
                    self.filtered_indices.len().saturating_sub(1)
                );
            }

            // Actions
            KeyCode::Enter | KeyCode::Char('i') => {
                // Show detailed info
                if let Some(port_info) = self.selected_port().cloned() {
                    self.show_details(stdout, &port_info)?;
                }
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Kill selected process (Ctrl+K)
                if let Some(port_info) = self.selected_port().cloned() {
                    self.kill_process(stdout, &port_info)?;
                }
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                // Kill selected process with confirmation
                if let Some(port_info) = self.selected_port().cloned() {
                    self.kill_process(stdout, &port_info)?;
                }
            }
            KeyCode::Char('r') | KeyCode::F(5) => {
                self.refresh()?;
            }
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.show_help = true;
            }
            KeyCode::Char('/') => {
                self.enter_filter_mode(stdout)?;
            }
            KeyCode::Char('c') => {
                // Clear filter
                self.filter.clear();
                self.apply_filter();
                self.status = Some("Filter cleared".to_string());
            }
            KeyCode::Char('t') => {
                // Toggle TCP/UDP filter (cycle: all -> TCP -> UDP -> all)
                if self.filter.is_empty() {
                    self.filter = "TCP".to_string();
                } else if self.filter == "TCP" {
                    self.filter = "UDP".to_string();
                } else {
                    self.filter.clear();
                }
                self.apply_filter();
            }

            _ => {}
        }

        Ok(Action::Continue)
    }

    fn enter_filter_mode(&mut self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        let (_width, height) = self.term_size;
        
        // Draw filter prompt
        execute!(
            stdout,
            MoveTo(0, height - 1),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Yellow),
            Print("Filter: "),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;
        stdout.flush().map_err(|e| PortrError::IoError(e.to_string()))?;

        self.filter.clear();
        
        loop {
            if let Event::Key(key) = event::read().map_err(|e| PortrError::IoError(e.to_string()))? {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Esc => {
                        self.filter.clear();
                        self.apply_filter();
                        break;
                    }
                    KeyCode::Backspace => {
                        self.filter.pop();
                    }
                    KeyCode::Char(c) => {
                        self.filter.push(c);
                    }
                    _ => {}
                }
                
                // Update filter display
                self.apply_filter();
                execute!(
                    stdout,
                    MoveTo(8, height - 1),
                    Clear(ClearType::UntilNewLine),
                    Print(&self.filter),
                    Print(format!(" ({} matches)", self.filtered_indices.len()))
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
                stdout.flush().map_err(|e| PortrError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }

    fn show_details(&mut self, stdout: &mut io::Stdout, port_info: &PortInfo) -> Result<(), PortrError> {
        let (width, height) = self.term_size;
        
        // Draw detail overlay
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
            .map_err(|e| PortrError::IoError(e.to_string()))?;

        // Header
        let title = format!(" Port {} Details ", port_info.port);
        let padding = "═".repeat(((width as usize).saturating_sub(title.len())) / 2);
        execute!(
            stdout,
            SetForegroundColor(Color::Cyan),
            Print(format!("{}{}{}\n\n", padding, title, padding)),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        // Details
        let details = [
            ("Port", port_info.port.to_string()),
            ("Protocol", port_info.protocol.clone()),
            ("PID", port_info.pid.to_string()),
            ("Process", port_info.process_name.clone()),
            ("Path", port_info.process_path.clone().unwrap_or_else(|| "N/A".to_string())),
            ("Local Address", port_info.local_address.clone()),
            ("State", port_info.state.clone()),
            ("Memory", format!("{:.1} MB", port_info.memory_mb)),
            ("CPU", format!("{:.1}%", port_info.cpu_percent)),
            ("Uptime", display::format_uptime(port_info.uptime_secs)),
        ];

        for (label, value) in details {
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("  {:>14}: ", label)),
                SetForegroundColor(Color::White),
                Print(format!("{}\n", value)),
                ResetColor
            ).map_err(|e| PortrError::IoError(e.to_string()))?;
        }

        // Service info
        if let Some(service) = services::lookup(port_info.port) {
            execute!(
                stdout,
                Print("\n"),
                SetForegroundColor(Color::Blue),
                Print(format!("  ℹ Known Service: {}\n", service.name)),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("    {}\n", service.description)),
                Print("    Risk Level: "),
                SetForegroundColor(match service.risk {
                    services::RiskLevel::Low => Color::Green,
                    services::RiskLevel::Medium => Color::Yellow,
                    services::RiskLevel::High => Color::Red,
                    services::RiskLevel::Critical => Color::DarkRed,
                }),
                Print(format!("{}\n", service.risk.label())),
                ResetColor
            ).map_err(|e| PortrError::IoError(e.to_string()))?;
        }

        // Footer
        execute!(
            stdout,
            MoveTo(0, height - 2),
            SetForegroundColor(Color::DarkGrey),
            Print("─".repeat(width as usize)),
            MoveTo(0, height - 1),
            SetForegroundColor(Color::Yellow),
            Print(" Press any key to return "),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;
        stdout.flush().map_err(|e| PortrError::IoError(e.to_string()))?;

        // Wait for key
        loop {
            if let Event::Key(_) = event::read().map_err(|e| PortrError::IoError(e.to_string()))? {
                break;
            }
        }

        Ok(())
    }

    fn kill_process(&mut self, stdout: &mut io::Stdout, port_info: &PortInfo) -> Result<(), PortrError> {
        let (_width, height) = self.term_size;

        // Check for critical services
        let is_critical = services::requires_confirmation(port_info.port);
        
        // Draw confirmation dialog
        execute!(
            stdout,
            MoveTo(0, height - 3),
            Clear(ClearType::CurrentLine)
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        if is_critical {
            if let Some(service) = services::lookup(port_info.port) {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" ⚠ WARNING: {} is a {} service!\n", service.name, service.risk.label())),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            }
        }

        execute!(
            stdout,
            MoveTo(0, height - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Yellow),
            Print(format!(
                " Kill PID {} ({}) on port {}? [y/N] ",
                port_info.pid, port_info.process_name, port_info.port
            )),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;
        stdout.flush().map_err(|e| PortrError::IoError(e.to_string()))?;

        // Wait for confirmation
        loop {
            if let Event::Key(key) = event::read().map_err(|e| PortrError::IoError(e.to_string()))? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        // Kill the process
                        if let Err(e) = process::kill_process(port_info.pid, false) {
                            self.status = Some(format!("Error: {}", e));
                        } else {
                            self.status = Some(format!(
                                "Killed PID {} ({})",
                                port_info.pid, port_info.process_name
                            ));
                            // Refresh after kill
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            self.refresh()?;
                        }
                        break;
                    }
                    _ => {
                        self.status = Some("Cancelled".to_string());
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        let (width, height) = self.term_size;
        
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
            .map_err(|e| PortrError::IoError(e.to_string()))?;

        // Draw banner (compact version)
        self.draw_header(stdout)?;

        // Calculate visible area
        let header_height = 4;
        let footer_height = 3;
        let list_height = (height as usize).saturating_sub(header_height + footer_height);
        
        // Calculate scroll offset
        let scroll_offset = if self.selected >= list_height {
            self.selected - list_height + 1
        } else {
            0
        };

        // Draw column headers
        execute!(
            stdout,
            MoveTo(0, header_height as u16 - 1),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(
                " {:>5} │ {:>5} │ {:>7} │ {:<20} │ {:<10} │ {:>9} │ {}\n",
                "PORT", "PROTO", "PID", "PROCESS", "SERVICE", "MEMORY", "STATE"
            )),
            Print("─".repeat(width as usize)),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        // Draw port list
        for (display_idx, &port_idx) in self.filtered_indices
            .iter()
            .skip(scroll_offset)
            .take(list_height)
            .enumerate()
        {
            let port = &self.ports[port_idx];
            let is_selected = scroll_offset + display_idx == self.selected;
            let y = header_height as u16 + 1 + display_idx as u16;
            
            execute!(stdout, MoveTo(0, y))
                .map_err(|e| PortrError::IoError(e.to_string()))?;

            // Selection indicator and background
            if is_selected {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Black),
                    SetAttribute(Attribute::Reverse),
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            }

            // Service name
            let service_name = services::short_name(port.port).unwrap_or("-");
            
            // Risk indicator
            let risk_indicator = services::lookup(port.port)
                .map(|s| match s.risk {
                    services::RiskLevel::Low => " ",
                    services::RiskLevel::Medium => "●",
                    services::RiskLevel::High => "●",
                    services::RiskLevel::Critical => "⚠",
                })
                .unwrap_or(" ");

            let line = format!(
                "{} {:>5} │ {:>5} │ {:>7} │ {:<20} │ {:<10} │ {:>8.1} MB │ {}",
                risk_indicator,
                port.port,
                port.protocol,
                port.pid,
                truncate(&port.process_name, 20),
                truncate(service_name, 10),
                port.memory_mb,
                truncate(&port.state, 12)
            );
            
            // Color based on protocol and risk
            if !is_selected {
                let color = if services::lookup(port.port)
                    .map(|s| matches!(s.risk, services::RiskLevel::Critical))
                    .unwrap_or(false)
                {
                    Color::Red
                } else if port.protocol == "TCP" {
                    Color::Cyan
                } else {
                    Color::Green
                };
                execute!(stdout, SetForegroundColor(color))
                    .map_err(|e| PortrError::IoError(e.to_string()))?;
            }

            execute!(
                stdout,
                Print(truncate(&line, width as usize)),
                SetAttribute(Attribute::Reset),
                ResetColor
            ).map_err(|e| PortrError::IoError(e.to_string()))?;
        }

        // Draw footer
        self.draw_footer(stdout)?;

        // Draw help overlay if active
        if self.show_help {
            self.draw_help_overlay(stdout)?;
        }

        stdout.flush().map_err(|e| PortrError::IoError(e.to_string()))?;
        Ok(())
    }

    fn draw_header(&self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        let (_width, _) = self.term_size;
        
        // Compact header
        execute!(
            stdout,
            SetForegroundColor(Color::Cyan),
            SetAttribute(Attribute::Bold),
            Print(" ██████╗  ██████╗ ██████╗ ████████╗██████╗  "),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkGrey),
            Print("  Lightning-fast port inspector\n"),
            SetForegroundColor(Color::Cyan),
            Print(" ██╔══██╗██╔═══██╗██╔══██╗╚══██╔══╝██╔══██╗ "),
            ResetColor,
            SetForegroundColor(Color::Yellow),
            Print(format!("  {} ports", self.ports.len())),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(" │ {} shown", self.filtered_indices.len())),
            ResetColor,
            Print("\n"),
            SetForegroundColor(Color::Cyan),
            Print(" ██████╔╝╚██████╔╝██║  ██║   ██║   ██║  ██║ "),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        // Filter indicator
        if !self.filter.is_empty() {
            execute!(
                stdout,
                SetForegroundColor(Color::Yellow),
                Print(format!("  Filter: {}", self.filter)),
                ResetColor
            ).map_err(|e| PortrError::IoError(e.to_string()))?;
        }

        execute!(stdout, Print("\n")).map_err(|e| PortrError::IoError(e.to_string()))?;

        Ok(())
    }

    fn draw_footer(&self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        let (width, height) = self.term_size;

        // Status line
        execute!(stdout, MoveTo(0, height - 3))
            .map_err(|e| PortrError::IoError(e.to_string()))?;
        
        if let Some(ref status) = self.status {
            execute!(
                stdout,
                SetForegroundColor(Color::Green),
                Print(format!(" ✓ {}", status)),
                ResetColor
            ).map_err(|e| PortrError::IoError(e.to_string()))?;
        }

        // Separator
        execute!(
            stdout,
            MoveTo(0, height - 2),
            SetForegroundColor(Color::DarkGrey),
            Print("─".repeat(width as usize)),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        // Help bar
        execute!(
            stdout,
            MoveTo(0, height - 1),
            SetForegroundColor(Color::DarkGrey),
            Print(" "),
            SetForegroundColor(Color::Yellow),
            Print("↑↓"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Navigate  "),
            SetForegroundColor(Color::Yellow),
            Print("Enter"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Details  "),
            SetForegroundColor(Color::Yellow),
            Print("x"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Kill  "),
            SetForegroundColor(Color::Yellow),
            Print("/"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Filter  "),
            SetForegroundColor(Color::Yellow),
            Print("r"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Refresh  "),
            SetForegroundColor(Color::Yellow),
            Print("?"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Help  "),
            SetForegroundColor(Color::Yellow),
            Print("q"),
            SetForegroundColor(Color::DarkGrey),
            Print(" Quit"),
            ResetColor
        ).map_err(|e| PortrError::IoError(e.to_string()))?;

        Ok(())
    }

    fn draw_help_overlay(&self, stdout: &mut io::Stdout) -> Result<(), PortrError> {
        let (width, height) = self.term_size;
        let box_width = 50;
        let box_height = 18;
        let start_x = (width.saturating_sub(box_width)) / 2;
        let start_y = (height.saturating_sub(box_height)) / 2;

        let help_items = [
            ("Navigation", ""),
            ("  ↑/k, ↓/j", "Move up/down"),
            ("  PgUp/PgDn", "Page up/down"),
            ("  Home/g, End/G", "Go to start/end"),
            ("", ""),
            ("Actions", ""),
            ("  Enter/i", "View port details"),
            ("  x/Delete", "Kill process"),
            ("  r/F5", "Refresh list"),
            ("", ""),
            ("Filtering", ""),
            ("  /", "Enter filter mode"),
            ("  c", "Clear filter"),
            ("  t", "Toggle TCP/UDP"),
            ("", ""),
            ("  q/Esc", "Quit"),
        ];

        // Draw box
        for y in 0..box_height {
            execute!(stdout, MoveTo(start_x, start_y + y))
                .map_err(|e| PortrError::IoError(e.to_string()))?;
            
            if y == 0 {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("╭"),
                    Print("─".repeat((box_width - 2) as usize)),
                    Print("╮"),
                    ResetColor
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            } else if y == box_height - 1 {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("╰"),
                    Print("─".repeat((box_width - 2) as usize)),
                    Print("╯"),
                    ResetColor
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            } else if y == 1 {
                let title = " Keyboard Shortcuts ";
                let padding = ((box_width - 2) as usize - title.len()) / 2;
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("│"),
                    SetForegroundColor(Color::Yellow),
                    SetAttribute(Attribute::Bold),
                    Print(" ".repeat(padding)),
                    Print(title),
                    Print(" ".repeat((box_width - 2) as usize - padding - title.len())),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::Cyan),
                    Print("│"),
                    ResetColor
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            } else {
                let idx = (y - 2) as usize;
                let (key, desc) = if idx < help_items.len() {
                    help_items[idx]
                } else {
                    ("", "")
                };
                
                let content = if desc.is_empty() {
                    if key.is_empty() {
                        " ".repeat((box_width - 2) as usize)
                    } else {
                        format!("{:<width$}", key, width = (box_width - 2) as usize)
                    }
                } else {
                    format!("{:<15} {:<width$}", key, desc, width = (box_width - 17) as usize)
                };

                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("│"),
                    SetForegroundColor(if desc.is_empty() { Color::White } else { Color::Yellow }),
                    Print(&content[..content.len().min((box_width - 2) as usize)]),
                    SetForegroundColor(Color::Cyan),
                    Print("│"),
                    ResetColor
                ).map_err(|e| PortrError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }
}

enum Action {
    Continue,
    Quit,
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Run interactive mode
pub fn run_interactive() -> Result<(), PortrError> {
    let mut app = InteractiveApp::new()?;
    app.run()
}
