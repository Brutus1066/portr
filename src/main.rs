//! portr - Lightning-fast port inspector and process killer
//!
//! Never google "kill process on port" again.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
#[cfg(feature = "docker")]
use portr::docker;
use portr::{config, display, export, interactive, port, process, services, tui, PortrError};
use std::io;
use std::process::ExitCode;

/// Lightning-fast port inspector and process killer
#[derive(Parser)]
#[command(name = "portr")]
#[command(author = "Kindware.dev <support@kindware.dev>")]
#[command(version)]
#[command(about = "Lightning-fast port inspector and process killer", long_about = None)]
#[command(after_help = "Examples:
  portr                  List all listening ports
  portr -i               Interactive mode with TUI
  portr 3000             Inspect port 3000
  portr 3000 8080        Inspect multiple ports
  portr 3000 --kill      Kill process on port 3000
  portr 3000 -k -f       Force kill without confirmation
  portr 3000 --dry-run   Show what would be killed
  portr 3000-3010        Scan port range
  portr --tcp            Show only TCP ports
  portr --csv            Export as CSV
  portr --md             Export as Markdown
  portr completions bash Generate shell completions

🐸 LazyFrog | kindware.dev")]
struct Cli {
    /// Port numbers, ranges (e.g., 3000-3010), or multiple ports
    #[arg(value_name = "PORTS")]
    ports: Vec<String>,

    /// Launch interactive TUI mode
    #[arg(short, long = "interactive")]
    interactive: bool,

    /// Kill the process using this port
    #[arg(short, long)]
    kill: bool,

    /// Force kill without confirmation
    #[arg(short, long)]
    force: bool,

    /// Dry run - show what would be killed without actually killing
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Show process tree (parent/child relationships)
    #[arg(short = 't', long)]
    tree: bool,

    /// Show only TCP connections
    #[arg(long)]
    tcp: bool,

    /// Show only UDP connections
    #[arg(long)]
    udp: bool,

    /// Output as JSON
    #[arg(long, conflicts_with_all = ["csv", "md"])]
    json: bool,

    /// Output as CSV
    #[arg(long, conflicts_with_all = ["json", "md"])]
    csv: bool,

    /// Output as Markdown
    #[arg(long, conflicts_with_all = ["json", "csv"])]
    md: bool,

    /// Verbose output with extra details
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all listening ports
    List {
        /// Show only TCP connections
        #[arg(long)]
        tcp: bool,

        /// Show only UDP connections
        #[arg(long)]
        udp: bool,
    },

    /// Interactive TUI mode with keyboard navigation
    Interactive,

    /// Full-screen TUI dashboard (beautiful real-time view)
    #[command(alias = "tui")]
    Dashboard,

    /// Watch ports in real-time
    Watch {
        /// Port to watch (optional, watches all if not specified)
        #[arg(value_name = "PORT")]
        port: Option<u16>,

        /// Refresh interval in seconds
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// Find which process is using a port
    Find {
        /// Port number to find
        #[arg(value_name = "PORT")]
        port: u16,
    },

    /// Kill process on a specific port
    Kill {
        /// Port numbers to kill
        #[arg(value_name = "PORTS", required = true)]
        ports: Vec<u16>,

        /// Force kill without confirmation
        #[arg(short, long)]
        force: bool,

        /// Dry run - show what would be killed
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Use SIGKILL instead of SIGTERM (Unix only)
        #[arg(long)]
        sigkill: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize config file with defaults
    Init,
    /// Show config file path
    Path,
    /// Show current configuration
    Show,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Launch interactive mode if requested
    if cli.interactive {
        return match interactive::run_interactive() {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{} {}", "error:".red().bold(), e);
                ExitCode::FAILURE
            }
        };
    }

    // Print banner for interactive commands (no output format, no subcommand, no ports)
    if !cli.json && !cli.csv && !cli.md && cli.command.is_none() && cli.ports.is_empty() {
        display::print_banner();
    }

    let result = run(cli);

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}

/// Output format helper
#[derive(Clone, Copy)]
enum OutputFormat {
    Pretty,
    Json,
    Csv,
    Markdown,
}

fn get_output_format(cli: &Cli) -> OutputFormat {
    if cli.json {
        OutputFormat::Json
    } else if cli.csv {
        OutputFormat::Csv
    } else if cli.md {
        OutputFormat::Markdown
    } else {
        OutputFormat::Pretty
    }
}

fn run(cli: Cli) -> Result<(), PortrError> {
    let format = get_output_format(&cli);
    let app_config = config::load_config();

    // Handle subcommands first
    if let Some(cmd) = cli.command {
        return match cmd {
            Commands::List { tcp, udp } => cmd_list(tcp, udp, format),
            Commands::Interactive => interactive::run_interactive(),
            Commands::Dashboard => tui::run_dashboard(),
            Commands::Watch { port, interval } => cmd_watch(port, interval),
            Commands::Find { port } => cmd_find(port, format, cli.verbose, false),
            Commands::Kill {
                ports,
                force,
                dry_run,
                sigkill,
            } => {
                // Force = true if --force flag OR confirm=false in config
                let effective_force = force || !app_config.defaults.confirm;
                for port in ports {
                    cmd_kill(port, effective_force, dry_run, sigkill)?;
                }
                Ok(())
            }
            Commands::Completions { shell } => {
                let mut cmd = Cli::command();
                generate(shell, &mut cmd, "portr", &mut io::stdout());
                Ok(())
            }
            Commands::Config { action } => cmd_config(action),
        };
    }

    // Handle direct port arguments
    if !cli.ports.is_empty() {
        // Check if any port is a range
        for port_arg in &cli.ports {
            if port_arg.contains('-') {
                return cmd_range(port_arg, format);
            }
        }

        // Try to resolve aliases first, then parse as port numbers
        let mut ports = Vec::new();
        for port_arg in &cli.ports {
            if let Ok(port) = port_arg.parse::<u16>() {
                ports.push(port);
            } else if let Some(port) = config::resolve_alias(port_arg, &app_config) {
                println!(
                    "{} Resolved alias '{}' → port {}",
                    "→".dimmed(),
                    port_arg.cyan(),
                    port.to_string().yellow()
                );
                ports.push(port);
            } else {
                return Err(PortrError::InvalidPort(port_arg.clone()));
            }
        }

        if cli.kill {
            // Force = true if --force flag OR confirm=false in config
            let effective_force = cli.force || !app_config.defaults.confirm;
            for port in &ports {
                cmd_kill(*port, effective_force, cli.dry_run, false)?;
            }
            return Ok(());
        }

        // Find info for each port
        if ports.len() == 1 {
            return cmd_find(ports[0], format, cli.verbose, cli.tree);
        }
        return cmd_find_multiple(&ports, format, cli.verbose);
    }

    // Default: list all ports
    cmd_list(cli.tcp, cli.udp, format)
}

/// Handle config subcommand
fn cmd_config(action: ConfigAction) -> Result<(), PortrError> {
    match action {
        ConfigAction::Init => match config::init_config() {
            Ok(path) => {
                println!(
                    "{} Created config file at: {}",
                    "✓".green().bold(),
                    path.display().to_string().cyan()
                );
                println!();
                println!("Edit this file to customize portr settings and add port aliases.");
            }
            Err(e) => {
                println!("{} {}", "!".yellow().bold(), e);
            }
        },
        ConfigAction::Path => {
            if let Some(path) = config::config_path() {
                println!("{}", path.display());
                if path.exists() {
                    println!("{}", "(file exists)".green());
                } else {
                    println!("{}", "(file does not exist)".dimmed());
                    println!("Run 'portr config init' to create it.");
                }
            } else {
                println!("{}", "Could not determine config path".red());
            }
        }
        ConfigAction::Show => {
            let cfg = config::load_config();
            println!("{}", "Current Configuration:".cyan().bold());
            println!();
            println!("  {}", "[defaults]".yellow());
            println!("    signal = \"{}\"", cfg.defaults.signal);
            println!("    confirm = {}", cfg.defaults.confirm);
            println!("    color = \"{}\"", cfg.defaults.color);
            println!();
            println!("  {}", "[aliases]".yellow());
            if cfg.aliases.is_empty() {
                println!("    (none defined)");
            } else {
                let mut aliases: Vec<_> = cfg.aliases.iter().collect();
                aliases.sort_by_key(|(k, _)| *k);
                for (name, port) in aliases {
                    println!("    {} = {}", name, port);
                }
            }
            println!();
            println!("  {}", "[theme]".yellow());
            println!("    banner_color = \"{}\"", cfg.theme.banner_color);
            println!("    success_color = \"{}\"", cfg.theme.success_color);
            println!("    warning_color = \"{}\"", cfg.theme.warning_color);
            println!("    error_color = \"{}\"", cfg.theme.error_color);
        }
    }
    Ok(())
}

/// List all listening ports
fn cmd_list(tcp_only: bool, udp_only: bool, format: OutputFormat) -> Result<(), PortrError> {
    let ports = port::get_listening_ports()?;

    let filtered: Vec<_> = ports
        .into_iter()
        .filter(|p| {
            if tcp_only {
                p.protocol == "TCP"
            } else if udp_only {
                p.protocol == "UDP"
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        println!("{}", "No listening ports found.".dimmed());
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", export::to_json(&filtered)?);
        }
        OutputFormat::Csv => {
            print!("{}", export::to_csv(&filtered));
        }
        OutputFormat::Markdown => {
            print!("{}", export::to_markdown(&filtered));
        }
        OutputFormat::Pretty => {
            display::print_port_table(&filtered);
        }
    }

    Ok(())
}

/// Find what's using a specific port
fn cmd_find(
    port: u16,
    format: OutputFormat,
    verbose: bool,
    show_tree: bool,
) -> Result<(), PortrError> {
    let info = port::get_port_info(port)?;

    match info {
        Some(port_info) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", export::to_json(&port_info)?);
                }
                OutputFormat::Csv => {
                    print!("{}", export::port_to_csv(&port_info));
                }
                OutputFormat::Markdown => {
                    print!("{}", export::port_to_markdown(&port_info));
                }
                OutputFormat::Pretty => {
                    display::print_port_details(&port_info, verbose);
                    // Show known service info
                    services::print_service_info(port);
                    // Show Docker container info if available
                    #[cfg(feature = "docker")]
                    docker::print_container_info(port);
                    // Show process tree if requested
                    if show_tree {
                        port::print_process_tree(&port_info);
                    }
                }
            }
        }
        None => {
            println!(
                "{} Port {} is {}",
                "✓".green().bold(),
                port.to_string().cyan(),
                "available".green()
            );
            // Show what service typically runs on this port
            if let Some(service) = services::lookup(port) {
                println!(
                    "  {} This port is typically used by: {} ({})",
                    "ℹ".blue().bold(),
                    service.name.cyan(),
                    service.description.dimmed()
                );
            }
        }
    }

    Ok(())
}

/// Find what's using multiple ports
fn cmd_find_multiple(ports: &[u16], format: OutputFormat, verbose: bool) -> Result<(), PortrError> {
    let all_ports = port::get_listening_ports()?;
    let found: Vec<_> = all_ports
        .into_iter()
        .filter(|p| ports.contains(&p.port))
        .collect();

    if found.is_empty() {
        println!(
            "{} All specified ports are {}",
            "✓".green().bold(),
            "available".green()
        );
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", export::to_json(&found)?);
        }
        OutputFormat::Csv => {
            print!("{}", export::to_csv(&found));
        }
        OutputFormat::Markdown => {
            print!("{}", export::to_markdown(&found));
        }
        OutputFormat::Pretty => {
            for port_info in &found {
                display::print_port_details(port_info, verbose);
                println!();
            }
            // Show which ports are available
            for &port in ports {
                if !found.iter().any(|p| p.port == port) {
                    println!(
                        "{} Port {} is {}",
                        "✓".green().bold(),
                        port.to_string().cyan(),
                        "available".green()
                    );
                }
            }
        }
    }

    Ok(())
}

/// Kill process on a port
fn cmd_kill(port: u16, force: bool, dry_run: bool, sigkill: bool) -> Result<(), PortrError> {
    // Check if this port is used by a Docker container
    #[cfg(feature = "docker")]
    if let Some(container) = docker::get_container_for_port(port) {
        return kill_docker_container(port, &container, force, dry_run);
    }

    let info = port::get_port_info(port)?;

    match info {
        Some(port_info) => {
            // Check for critical services
            let is_critical = services::requires_confirmation(port);

            // Dry run mode - just show what would happen
            if dry_run {
                let warning = if is_critical {
                    format!(" {}", services::get_warning(port).unwrap_or_default())
                } else {
                    String::new()
                };
                println!(
                    "{} Would kill: PID {} ({}) on port {}{}",
                    "⚡".yellow().bold(),
                    port_info.pid.to_string().yellow(),
                    port_info.process_name.cyan(),
                    port.to_string().cyan(),
                    warning
                );
                return Ok(());
            }

            if !force {
                display::print_port_details(&port_info, false);

                // Show service warning for critical services
                if is_critical {
                    services::print_service_info(port);
                }

                println!();

                if !confirm_kill(&port_info, is_critical) {
                    println!("{}", "Cancelled.".dimmed());
                    return Ok(());
                }
            }

            process::kill_process(port_info.pid, sigkill)?;

            println!(
                "{} Killed process {} ({}) on port {}",
                "✓".green().bold(),
                port_info.pid.to_string().yellow(),
                port_info.process_name.cyan(),
                port.to_string().cyan()
            );
        }
        None => {
            println!(
                "{} Port {} is not in use",
                "ℹ".blue().bold(),
                port.to_string().cyan()
            );
        }
    }

    Ok(())
}

/// Kill a Docker container that's using a port
#[cfg(feature = "docker")]
fn kill_docker_container(
    port: u16,
    container: &docker::ContainerInfo,
    force: bool,
    dry_run: bool,
) -> Result<(), PortrError> {
    use std::io::Write;

    let is_critical = docker::is_critical_container(container);

    // Dry run mode
    if dry_run {
        let warning = if is_critical {
            format!(" {}", "⚠ CRITICAL DATABASE CONTAINER".red().bold())
        } else {
            String::new()
        };
        println!(
            "{} Would stop Docker container: {} ({}) on port {}{}",
            "🐳".blue().bold(),
            container.name.cyan().bold(),
            container.id.dimmed(),
            port.to_string().cyan(),
            warning
        );
        return Ok(());
    }

    // Show container info
    println!(
        "\n  {} Docker container detected on port {}:",
        "🐳".blue().bold(),
        port.to_string().cyan()
    );
    println!("     Name: {}", container.name.cyan().bold());
    println!("     ID: {}", container.id.dimmed());
    println!("     Image: {}", container.image);
    println!("     Status: {}", container.status.green());

    // Show all port mappings
    if !container.ports.is_empty() {
        let port_strs: Vec<String> = container
            .ports
            .iter()
            .filter_map(|p| {
                p.host_port
                    .map(|hp| format!("{}:{}/{}", hp, p.container_port, p.protocol))
            })
            .collect();
        if !port_strs.is_empty() {
            println!("     Ports: {}", port_strs.join(", ").yellow());
        }
    }

    // Show critical warning
    if is_critical {
        println!();
        println!(
            "  {} This is a {} container!",
            "⚠".red().bold(),
            "CRITICAL DATABASE".red().bold()
        );
        println!("    Stopping may cause {}", "DATA LOSS".red().bold());
    }
    println!();

    if !force {
        // Critical containers require typing "yes"
        if is_critical {
            print!(
                "  {} Type '{}' to stop this container: ",
                "?".red().bold(),
                "yes".yellow()
            );
            std::io::stdout().flush().ok();

            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                return Ok(());
            }

            if input.trim() != "yes" {
                println!("{}", "Cancelled. (Must type 'yes' exactly)".dimmed());
                return Ok(());
            }
        } else {
            print!("  {} Stop this container? [y/N]: ", "?".yellow().bold());
            std::io::stdout().flush().ok();

            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                return Ok(());
            }

            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("{}", "Cancelled.".dimmed());
                return Ok(());
            }
        }
    }

    // Stop the container by NAME (more stable than ID which can change)
    docker::stop_container_by_name(&container.name)?;

    println!(
        "{} Stopped container {} on port {}",
        "✓".green().bold(),
        container.name.cyan(),
        port.to_string().cyan()
    );

    Ok(())
}

/// Scan a range of ports
fn cmd_range(range: &str, format: OutputFormat) -> Result<(), PortrError> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(PortrError::InvalidPortRange(range.to_string()));
    }

    let start: u16 = parts[0]
        .parse()
        .map_err(|_| PortrError::InvalidPortRange(range.to_string()))?;
    let end: u16 = parts[1]
        .parse()
        .map_err(|_| PortrError::InvalidPortRange(range.to_string()))?;

    if start > end {
        return Err(PortrError::InvalidPortRange(range.to_string()));
    }

    let all_ports = port::get_listening_ports()?;
    let in_range: Vec<_> = all_ports
        .into_iter()
        .filter(|p| p.port >= start && p.port <= end)
        .collect();

    if in_range.is_empty() {
        println!(
            "{} No ports in use in range {}-{}",
            "✓".green().bold(),
            start.to_string().cyan(),
            end.to_string().cyan()
        );
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", export::to_json(&in_range)?);
        }
        OutputFormat::Csv => {
            print!("{}", export::to_csv(&in_range));
        }
        OutputFormat::Markdown => {
            print!("{}", export::to_markdown(&in_range));
        }
        OutputFormat::Pretty => {
            println!(
                "{} Found {} port(s) in range {}-{}:\n",
                "●".blue().bold(),
                in_range.len().to_string().yellow(),
                start.to_string().cyan(),
                end.to_string().cyan()
            );
            display::print_port_table(&in_range);
        }
    }

    Ok(())
}

/// Watch ports in real-time
fn cmd_watch(port: Option<u16>, interval: u64) -> Result<(), PortrError> {
    use std::{thread, time::Duration};

    println!(
        "{} Watching ports (refresh every {}s, Ctrl+C to stop)\n",
        "👁".bold(),
        interval
    );

    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        display::print_banner();

        if let Some(p) = port {
            let info = port::get_port_info(p)?;
            match info {
                Some(port_info) => display::print_port_details(&port_info, true),
                None => println!(
                    "{} Port {} is {}",
                    "✓".green().bold(),
                    p.to_string().cyan(),
                    "available".green()
                ),
            }
        } else {
            let ports = port::get_listening_ports()?;
            if ports.is_empty() {
                println!("{}", "No listening ports found.".dimmed());
            } else {
                display::print_port_table(&ports);
            }
        }

        println!(
            "\n{}",
            format!("Last updated: {} | Press Ctrl+C to stop", chrono_now()).dimmed()
        );

        thread::sleep(Duration::from_secs(interval));
    }
}

/// Get current time as string (simple implementation without chrono)
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
}

/// Prompt user for confirmation
fn confirm_kill(info: &port::PortInfo, is_critical: bool) -> bool {
    use std::io::{self, Write};

    if is_critical {
        print!(
            "{} Kill {} process {} ({})? Type '{}' to confirm: ",
            "⚠".red().bold(),
            "CRITICAL".red().bold(),
            info.pid.to_string().yellow(),
            info.process_name.cyan(),
            "yes".red().bold()
        );
    } else {
        print!(
            "Kill process {} ({})? [y/N] ",
            info.pid.to_string().yellow(),
            info.process_name.cyan()
        );
    }
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if is_critical {
        // Require full "yes" for critical services
        input.trim().to_lowercase() == "yes"
    } else {
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }
}
