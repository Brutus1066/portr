//! Application state for the TUI dashboard
//!
//! Manages ports, selection, filters, and all UI state.

use crate::port::{self, PortInfo};
use crate::process;
use crate::services;
use crate::export;

/// Filter mode for port display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    TcpOnly,
    UdpOnly,
}

/// Sort mode for port display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Port,
    Process,
    Memory,
    Pid,
}

/// Export format for TUI export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Markdown,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Markdown => "md",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Json => "JSON",
            ExportFormat::Csv => "CSV",
            ExportFormat::Markdown => "Markdown",
        }
    }
    
    pub fn cycle(&self) -> Self {
        match self {
            ExportFormat::Json => ExportFormat::Csv,
            ExportFormat::Csv => ExportFormat::Markdown,
            ExportFormat::Markdown => ExportFormat::Json,
        }
    }
}

/// Main application state
pub struct App {
    /// All ports (unfiltered)
    pub all_ports: Vec<PortInfo>,
    /// Filtered/visible ports
    pub ports: Vec<PortInfo>,
    /// Currently selected index
    pub selected: usize,
    /// Is the app still running?
    pub running: bool,
    /// Current filter mode
    pub filter_mode: FilterMode,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Text filter/search
    pub filter_text: String,
    /// Filter input buffer (while typing)
    pub filter_input: String,
    /// Are we in input mode?
    pub input_mode: bool,
    /// Show details panel?
    pub show_details: bool,
    /// Show help overlay?
    pub show_help: bool,
    /// Show menu overlay?
    pub show_menu: bool,
    /// Menu selected item
    pub menu_selected: usize,
    /// Filter Docker only?
    pub docker_only: bool,
    /// Filter critical only?
    pub critical_only: bool,
    /// Status message
    pub status_message: Option<String>,
    /// Status message timestamp
    pub status_time: std::time::Instant,
    /// Tick counter for animations
    pub tick: u64,
    /// Show export popup?
    pub show_export: bool,
    /// Selected export format
    pub export_format: ExportFormat,
}

/// Menu items - updated with Export option
pub const MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("1", "Dashboard", "Full TUI with details panel"),
    ("2", "Ports Only", "Simple port list view"),
    ("3", "TCP Filter", "Show only TCP connections"),
    ("4", "UDP Filter", "Show only UDP connections"),
    ("5", "Docker", "Filter Docker containers only"),
    ("6", "Critical", "Show critical services only"),
    ("7", "Export", "Export ports to JSON/CSV/Markdown"),
    ("8", "Help", "Show keyboard shortcuts"),
    ("0", "Quit", "Exit portr"),
];

impl App {
    /// Create a new app with default state
    pub fn new() -> Self {
        Self {
            all_ports: Vec::new(),
            ports: Vec::new(),
            selected: 0,
            running: true,
            filter_mode: FilterMode::All,
            sort_mode: SortMode::Port,
            filter_text: String::new(),
            filter_input: String::new(),
            input_mode: false,
            show_details: true,
            show_help: false,
            show_menu: false,
            menu_selected: 0,
            docker_only: false,
            critical_only: false,
            status_message: None,
            status_time: std::time::Instant::now(),
            tick: 0,
            show_export: false,
            export_format: ExportFormat::Json,
        }
    }

    /// Refresh port list from system
    pub fn refresh_ports(&mut self) {
        self.all_ports = port::get_listening_ports().unwrap_or_default();
        self.apply_filters();
        self.set_status("Refreshed");
    }

    /// Apply all filters and sorting
    pub fn apply_filters(&mut self) {
        let mut filtered: Vec<PortInfo> = self
            .all_ports
            .iter()
            .filter(|p| {
                // Protocol filter
                let proto_match = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::TcpOnly => p.protocol.to_uppercase() == "TCP",
                    FilterMode::UdpOnly => p.protocol.to_uppercase() == "UDP",
                };

                // Text filter (search)
                let text_match = if self.filter_text.is_empty() {
                    true
                } else {
                    let search = self.filter_text.to_lowercase();
                    p.port.to_string().contains(&search)
                        || p.process_name.to_lowercase().contains(&search)
                        || p.pid.to_string().contains(&search)
                        || p.local_address.to_lowercase().contains(&search)
                        || p.protocol.to_lowercase().contains(&search)
                };

                // Docker filter - check if process is Docker-related
                let docker_match = if self.docker_only {
                    let name = p.process_name.to_lowercase();
                    name.contains("docker") 
                        || name.contains("containerd")
                        || name.contains("com.docker")
                        || name == "vpnkit.exe"
                        || name == "vpnkit"
                } else {
                    true
                };

                // Critical filter
                let critical_match = if self.critical_only {
                    services::requires_confirmation(p.port)
                } else {
                    true
                };

                proto_match && text_match && docker_match && critical_match
            })
            .cloned()
            .collect();

        // Sort
        match self.sort_mode {
            SortMode::Port => filtered.sort_by_key(|p| p.port),
            SortMode::Process => filtered.sort_by(|a, b| a.process_name.cmp(&b.process_name)),
            SortMode::Memory => filtered.sort_by(|a, b| {
                b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortMode::Pid => filtered.sort_by_key(|p| p.pid),
        }

        self.ports = filtered;

        // Adjust selection
        if self.ports.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.ports.len() {
            self.selected = self.ports.len() - 1;
        }
    }

    /// Apply filter from input
    pub fn apply_filter(&mut self) {
        self.filter_text = self.filter_input.clone();
        self.apply_filters();
        if self.filter_text.is_empty() {
            self.set_status(&format!("Search cleared ({} ports)", self.ports.len()));
        } else if self.ports.is_empty() {
            self.set_status(&format!("No matches for '{}'", self.filter_text));
        } else {
            self.set_status(&format!("Found {} for '{}'", self.ports.len(), self.filter_text));
        }
    }

    /// Clear search filter
    pub fn clear_filter(&mut self) {
        self.filter_text.clear();
        self.filter_input.clear();
        self.apply_filters();
        self.set_status(&format!("Filter cleared ({} ports)", self.ports.len()));
    }

    /// Move selection down
    pub fn next(&mut self) {
        if !self.ports.is_empty() {
            self.selected = (self.selected + 1) % self.ports.len();
        }
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if !self.ports.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.ports.len() - 1);
        }
    }

    /// Page down (move 10 items)
    pub fn page_down(&mut self) {
        if !self.ports.is_empty() {
            self.selected = (self.selected + 10).min(self.ports.len() - 1);
        }
    }

    /// Page up (move 10 items)
    pub fn page_up(&mut self) {
        if !self.ports.is_empty() {
            self.selected = self.selected.saturating_sub(10);
        }
    }

    /// Jump to first
    pub fn first(&mut self) {
        self.selected = 0;
    }

    /// Jump to last
    pub fn last(&mut self) {
        if !self.ports.is_empty() {
            self.selected = self.ports.len() - 1;
        }
    }

    /// Cycle through filter modes
    pub fn cycle_filter(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::All => FilterMode::TcpOnly,
            FilterMode::TcpOnly => FilterMode::UdpOnly,
            FilterMode::UdpOnly => FilterMode::All,
        };
        self.apply_filters();
        let msg = match self.filter_mode {
            FilterMode::All => format!("Filter: ALL ({} ports)", self.ports.len()),
            FilterMode::TcpOnly => format!("Filter: TCP ({} ports)", self.ports.len()),
            FilterMode::UdpOnly => format!("Filter: UDP ({} ports)", self.ports.len()),
        };
        self.set_status(&msg);
    }

    /// Cycle through sort modes
    pub fn cycle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Port => SortMode::Process,
            SortMode::Process => SortMode::Memory,
            SortMode::Memory => SortMode::Pid,
            SortMode::Pid => SortMode::Port,
        };
        self.apply_filters();
        let msg = match self.sort_mode {
            SortMode::Port => "Sort: PORT",
            SortMode::Process => "Sort: PROCESS",
            SortMode::Memory => "Sort: MEMORY",
            SortMode::Pid => "Sort: PID",
        };
        self.set_status(msg);
    }

    /// Toggle Docker only filter
    pub fn toggle_docker_filter(&mut self) {
        self.docker_only = !self.docker_only;
        self.critical_only = false; // Clear other filter
        self.apply_filters();
        if self.docker_only {
            if self.ports.is_empty() {
                self.set_status("Docker: No containers found");
            } else {
                self.set_status(&format!("Docker: {} containers", self.ports.len()));
            }
        } else {
            self.set_status(&format!("Docker filter OFF ({} ports)", self.ports.len()));
        }
    }

    /// Toggle critical only filter
    pub fn toggle_critical_filter(&mut self) {
        self.critical_only = !self.critical_only;
        self.docker_only = false; // Clear other filter
        self.apply_filters();
        if self.critical_only {
            if self.ports.is_empty() {
                self.set_status("Critical: No critical services found");
            } else {
                self.set_status(&format!("Critical: {} services", self.ports.len()));
            }
        } else {
            self.set_status(&format!("Critical filter OFF ({} ports)", self.ports.len()));
        }
    }

    /// Toggle details panel
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Toggle menu overlay
    pub fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        if self.show_menu {
            self.menu_selected = 0;
        }
    }

    /// Navigate menu up
    pub fn menu_up(&mut self) {
        if self.menu_selected > 0 {
            self.menu_selected -= 1;
        }
    }

    /// Navigate menu down
    pub fn menu_down(&mut self) {
        if self.menu_selected < MENU_ITEMS.len() - 1 {
            self.menu_selected += 1;
        }
    }

    /// Execute selected menu item
    pub fn menu_select(&mut self) {
        let selected = self.menu_selected;
        self.show_menu = false;
        
        match selected {
            0 => {
                // Dashboard - full view with details
                self.filter_mode = FilterMode::All;
                self.docker_only = false;
                self.critical_only = false;
                self.show_details = true;
                self.apply_filters();
                self.set_status("View: Dashboard");
            }
            1 => {
                // Ports Only - no details panel
                self.filter_mode = FilterMode::All;
                self.docker_only = false;
                self.critical_only = false;
                self.show_details = false;
                self.apply_filters();
                self.set_status("View: Ports Only");
            }
            2 => {
                // TCP Only
                self.filter_mode = FilterMode::TcpOnly;
                self.docker_only = false;
                self.critical_only = false;
                self.apply_filters();
                self.set_status("Filter: TCP Only");
            }
            3 => {
                // UDP Only
                self.filter_mode = FilterMode::UdpOnly;
                self.docker_only = false;
                self.critical_only = false;
                self.apply_filters();
                self.set_status("Filter: UDP Only");
            }
            4 => {
                // Docker only
                self.filter_mode = FilterMode::All;
                self.docker_only = true;
                self.critical_only = false;
                self.apply_filters();
                if self.ports.is_empty() {
                    self.set_status("No Docker containers found");
                } else {
                    self.set_status("Filter: Docker Only");
                }
            }
            5 => {
                // Critical only
                self.filter_mode = FilterMode::All;
                self.docker_only = false;
                self.critical_only = true;
                self.apply_filters();
                self.set_status("Filter: Critical Services");
            }
            6 => {
                // Export
                self.show_export = true;
            }
            7 => {
                // Help
                self.show_help = true;
            }
            8 => {
                // Quit
                self.running = false;
            }
            _ => {}
        }
    }

    /// Kill selected port's process
    pub fn kill_selected(&mut self) {
        if let Some(port_info) = self.get_selected() {
            let pid = port_info.pid;
            let port = port_info.port;
            let name = port_info.process_name.clone();

            // Check if critical
            if services::requires_confirmation(port) {
                self.set_status(&format!(
                    "⚠ Port {} is critical! Use CLI: portr {} --kill",
                    port, port
                ));
                return;
            }

            match process::kill_process(pid, false) {
                Ok(_) => {
                    self.set_status(&format!("✓ Killed PID {} ({}) on port {}", pid, name, port));
                    self.refresh_ports();
                }
                Err(e) => {
                    self.set_status(&format!("✗ Failed to kill: {}", e));
                }
            }
        }
    }

    /// Get currently selected port info
    pub fn get_selected(&self) -> Option<&PortInfo> {
        self.ports.get(self.selected)
    }

    /// Set status message
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_time = std::time::Instant::now();
    }

    /// Called on each tick
    pub fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        // Clear old status messages
        if self.status_message.is_some() && self.status_time.elapsed().as_secs() > 3 {
            self.status_message = None;
        }
    }

    /// Get filter mode display string
    pub fn filter_mode_str(&self) -> &'static str {
        match self.filter_mode {
            FilterMode::All => "ALL",
            FilterMode::TcpOnly => "TCP",
            FilterMode::UdpOnly => "UDP",
        }
    }

    /// Get sort mode display string
    pub fn sort_mode_str(&self) -> &'static str {
        match self.sort_mode {
            SortMode::Port => "PORT",
            SortMode::Process => "PROC",
            SortMode::Memory => "MEM",
            SortMode::Pid => "PID",
        }
    }

    /// Toggle export popup
    pub fn toggle_export(&mut self) {
        self.show_export = !self.show_export;
        if self.show_export {
            self.show_menu = false;
            self.show_help = false;
        }
    }

    /// Cycle export format
    pub fn cycle_export_format(&mut self) {
        self.export_format = self.export_format.cycle();
    }

    /// Export current ports to file
    pub fn do_export(&mut self) {
        use std::fs;
        
        if self.ports.is_empty() {
            self.set_status("No ports to export");
            self.show_export = false;
            return;
        }

        // Generate filename with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("portr_export_{}.{}", timestamp, self.export_format.extension());

        let content = match self.export_format {
            ExportFormat::Json => {
                export::to_json(&self.ports).unwrap_or_else(|e| format!("Error: {}", e))
            }
            ExportFormat::Csv => export::to_csv(&self.ports),
            ExportFormat::Markdown => export::to_markdown(&self.ports),
        };

        match fs::write(&filename, &content) {
            Ok(_) => {
                self.set_status(&format!(
                    "✓ Exported {} ports to {}", 
                    self.ports.len(), 
                    filename
                ));
            }
            Err(e) => {
                self.set_status(&format!("✗ Export failed: {}", e));
            }
        }

        self.show_export = false;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new();
        assert!(app.running);
        assert!(app.ports.is_empty());
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_filter_cycle() {
        let mut app = App::new();
        assert_eq!(app.filter_mode, FilterMode::All);
        app.cycle_filter();
        assert_eq!(app.filter_mode, FilterMode::TcpOnly);
        app.cycle_filter();
        assert_eq!(app.filter_mode, FilterMode::UdpOnly);
        app.cycle_filter();
        assert_eq!(app.filter_mode, FilterMode::All);
    }

    #[test]
    fn test_sort_cycle() {
        let mut app = App::new();
        assert_eq!(app.sort_mode, SortMode::Port);
        app.cycle_sort();
        assert_eq!(app.sort_mode, SortMode::Process);
    }
}
