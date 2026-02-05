//! Build script for portr - generates man pages
//!
//! Run `cargo build` and find man pages in `target/man/`

use clap::{CommandFactory, Parser, Subcommand};
use clap_mangen::Man;
use std::env;
use std::fs;
use std::path::PathBuf;

// Mirror the CLI struct from main.rs for man page generation
#[derive(Parser)]
#[command(name = "portr")]
#[command(author = "Kindware.dev <support@kindware.dev>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
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
    #[arg(long)]
    json: bool,

    /// Output as CSV
    #[arg(long)]
    csv: bool,

    /// Output as Markdown
    #[arg(long)]
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
        shell: String,
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

fn main() {
    // Get the output directory
    let out_dir = match env::var_os("OUT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => return, // Skip if not in cargo build context
    };

    // Create man page directory in target/man
    let man_dir = out_dir
        .ancestors()
        .nth(3) // Go up from OUT_DIR to target/profile
        .map(|p| p.join("man"))
        .unwrap_or_else(|| out_dir.join("man"));

    if fs::create_dir_all(&man_dir).is_err() {
        return; // Skip silently if we can't create the directory
    }

    // Generate man page
    let cmd = Cli::command();
    let man = Man::new(cmd);

    let man_path = man_dir.join("portr.1");
    if let Ok(mut file) = fs::File::create(&man_path) {
        let _ = man.render(&mut file);
        println!("cargo:warning=Man page generated at: {}", man_path.display());
    }

    // Also generate man pages for subcommands
    let cmd = Cli::command();
    for subcommand in cmd.get_subcommands() {
        let name = subcommand.get_name();
        let sub_man = Man::new(subcommand.clone());
        let sub_path = man_dir.join(format!("portr-{}.1", name));
        if let Ok(mut file) = fs::File::create(&sub_path) {
            let _ = sub_man.render(&mut file);
        }
    }

    // Tell cargo to rerun if main.rs changes
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
