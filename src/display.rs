//! Display formatting and output

use crate::error::PortrError;
use crate::port::PortInfo;
use colored::Colorize;
use tabled::{settings::Style, Table, Tabled};

/// Print the portr ASCII banner
pub fn print_banner() {
    let banner = r#"
╔══════════════════════════════════════════════════════════════════════════╗
║                                                                          ║
║            ██████╗  ██████╗ ██████╗ ████████╗██████╗                     ║
║            ██╔══██╗██╔═══██╗██╔══██╗╚══██╔══╝██╔══██╗                    ║
║            ██████╔╝██║   ██║██████╔╝   ██║   ██████╔╝                    ║
║            ██╔═══╝ ██║   ██║██╔══██╗   ██║   ██╔══██╗                    ║
║            ██║     ╚██████╔╝██║  ██║   ██║   ██║  ██║                    ║
║            ╚═╝      ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝                    ║
║                                                                          ║
║           Lightning-fast port inspector & process killer                 ║
║                                                                          ║
║                    🐸 LazyFrog | kindware.dev                            ║
║                                                                          ║
╚══════════════════════════════════════════════════════════════════════════╝
"#;
    println!("{}", banner.cyan());
}

/// Table row for port display
#[derive(Tabled)]
struct PortRow {
    #[tabled(rename = "PORT")]
    port: String,
    #[tabled(rename = "PROTO")]
    protocol: String,
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "PROCESS")]
    process: String,
    #[tabled(rename = "MEMORY")]
    memory: String,
    #[tabled(rename = "UPTIME")]
    uptime: String,
}

/// Print a table of ports
pub fn print_port_table(ports: &[PortInfo]) {
    if ports.is_empty() {
        println!("{}", "No listening ports found.".dimmed());
        return;
    }

    let rows: Vec<PortRow> = ports
        .iter()
        .map(|p| PortRow {
            port: p.port.to_string(),
            protocol: p.protocol.clone(),
            pid: p.pid.to_string(),
            process: truncate(&p.process_name, 25),
            memory: format!("{:.1} MB", p.memory_mb),
            uptime: p.uptime_display(),
        })
        .collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();

    println!("{}", table);
    println!(
        "\n{} {} port(s) in use",
        "●".blue().bold(),
        ports.len().to_string().yellow()
    );

    // Hint for discoverability
    println!(
        "\n{} {} {} {} {} {} {}",
        "Tip:".dimmed(),
        "portr -i".cyan(),
        "→ interactive".dimmed(),
        "|".dimmed(),
        "portr tui".cyan(),
        "→ dashboard".dimmed(),
        format!("| {} → all options", "portr --help".cyan()).dimmed(),
    );
}

/// Print detailed information about a single port
pub fn print_port_details(info: &PortInfo, verbose: bool) {
    let box_width = 60;
    let title = format!(" Port {} ", info.port);

    // Top border with title
    println!(
        "{}{}{}",
        "╭─".cyan(),
        title.cyan().bold(),
        "─".repeat(box_width - title.len() - 3).cyan()
    );
    println!("{}", "│".cyan());

    // Main info
    print_detail_line("PID", &info.pid.to_string(), "yellow");
    print_detail_line("Process", &info.process_name, "green");

    // Show parent process if available
    if let (Some(ppid), Some(ref pname)) = (info.parent_pid, &info.parent_name) {
        print_detail_line("Parent", &format!("{} (PID {})", pname, ppid), "dimmed");
    }

    print_detail_line("Protocol", &info.protocol, "white");
    print_detail_line("State", &info.state, "white");
    print_detail_line("Local", &info.local_address, "white");

    if let Some(ref remote) = info.remote_address {
        print_detail_line("Remote", remote, "white");
    }

    println!("{}", "│".cyan());
    print_detail_line("Memory", &format!("{:.1} MB", info.memory_mb), "magenta");
    print_detail_line("CPU", &format!("{:.1}%", info.cpu_percent), "magenta");
    print_detail_line("Uptime", &info.uptime_display(), "white");

    if verbose {
        println!("{}", "│".cyan());
        if let Some(ref path) = info.process_path {
            print_detail_line("Path", path, "dimmed");
        }
        if let Some(ref user) = info.user {
            print_detail_line("User", user, "dimmed");
        }
    }

    // Bottom border
    println!("{}", "│".cyan());
    println!(
        "{}{}{}",
        "╰".cyan(),
        "─".repeat(box_width - 2).cyan(),
        "╯".cyan()
    );

    // Action hints
    println!(
        "\n  {} Kill: {}",
        "→".dimmed(),
        format!("portr {} --kill", info.port).yellow()
    );
}

/// Print a detail line in the box
fn print_detail_line(label: &str, value: &str, color: &str) {
    let colored_value = match color {
        "yellow" => value.yellow().to_string(),
        "green" => value.green().to_string(),
        "magenta" => value.magenta().to_string(),
        "dimmed" => value.dimmed().to_string(),
        "cyan" => value.cyan().to_string(),
        _ => value.white().to_string(),
    };

    println!("{}  {:>10}: {}", "│".cyan(), label.dimmed(), colored_value);
}

/// Print JSON output
pub fn print_json<T: serde::Serialize>(data: &T) -> Result<(), PortrError> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}

/// Format a port status indicator
pub fn port_status_icon(state: &str) -> &'static str {
    match state.to_uppercase().as_str() {
        "LISTEN" | "LISTENING" => "●",
        "ESTABLISHED" => "◉",
        "TIME_WAIT" => "◌",
        "CLOSE_WAIT" => "◎",
        _ => "○",
    }
}

/// Format uptime in seconds to human-readable string
pub fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_port_status_icon_listen() {
        assert_eq!(port_status_icon("LISTEN"), "●");
        assert_eq!(port_status_icon("LISTENING"), "●");
    }

    #[test]
    fn test_port_status_icon_established() {
        assert_eq!(port_status_icon("ESTABLISHED"), "◉");
    }

    #[test]
    fn test_port_status_icon_unknown() {
        assert_eq!(port_status_icon("UNKNOWN"), "○");
    }
}
