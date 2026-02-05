# portr Command Reference

> 🐸 Lightning-fast port inspector & process killer

---

## Quick Start

```bash
# Install from crates.io
cargo install portr

# Or build from source
git clone https://github.com/Brutus1066/portr.git
cd portr
cargo build --release
```

---

## Basic Commands

### List All Ports
```bash
portr                    # List all listening ports
portr --tcp              # TCP only
portr --udp              # UDP only
```

### Inspect Specific Port
```bash
portr 3000               # Inspect port 3000
portr 3000 8080          # Inspect multiple ports
portr 3000-3010          # Scan port range
```

### Kill Process on Port
```bash
portr 3000 --kill        # Kill with confirmation
portr 3000 -k -f         # Force kill (no confirmation)
portr 3000 -k -n         # Dry run (show what would be killed)
```

---

## TUI Dashboard

```bash
portr dashboard          # Launch full-screen TUI
portr tui                # Alias for dashboard
portr -i                 # Interactive mode
```

### TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `m` | Open quick menu |
| `j/↓` | Move down |
| `k/↑` | Move up |
| `PgDn/PgUp` | Page scroll |
| `g/G` | First/Last |
| `/` | Search/filter |
| `f` | Cycle filter (All/TCP/UDP) |
| `d` | Docker only filter |
| `c` | Critical services only |
| `e` | Export (JSON/CSV/MD) |
| `Tab` | Cycle sort mode |
| `K` | Kill selected process |
| `r` | Refresh ports |
| `?` | Show help |
| `Esc` | Clear filters / Exit |
| `q` | Quit |

---

## Export Options

```bash
# CLI Export
portr --json             # JSON output
portr --csv              # CSV output
portr --md               # Markdown output

# Redirect to file
portr --json > ports.json
portr --csv > ports.csv
portr --md > ports.md
```

### TUI Export
Press `e` in the TUI to open export dialog:
- `J` - JSON format
- `C` - CSV format
- `M` - Markdown format
- `Enter` - Export to file
- `Esc` - Cancel

Files are saved as: `portr_export_YYYYMMDD_HHMMSS.{json,csv,md}`

---

## Watch Mode

```bash
portr watch              # Watch all ports
portr watch 3000         # Watch specific port
portr watch --interval 5 # Custom refresh (seconds)
```

---

## Configuration

```bash
portr config init        # Create config file
portr config path        # Show config location
portr config show        # Display current settings
```

### Config Locations
- **Windows:** `%APPDATA%\portr\config.toml`
- **Linux/macOS:** `~/.config/portr/config.toml`

### Using Aliases
```bash
portr react              # Resolves to port 3000
portr postgres           # Resolves to port 5432
portr ollama             # Resolves to port 11434
```

---

## Docker Support (Optional)

```bash
# Install with Docker support
cargo install portr --features docker

# Inspect Docker containers
portr 5432               # Shows container info
portr 5432 --kill        # Stops container safely
```

---

## Development

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run TUI in development
cargo run -- dashboard
```

---

## Examples

```bash
# Find what's using port 8080
portr 8080

# Kill Node.js dev server
portr 3000 -k -f

# Export all ports to JSON
portr --json > ports.json

# Monitor ports in real-time
portr dashboard

# Check if port is available
portr 4000
# Output: ✓ Port 4000 is available
```

---

**🐸 LazyFrog | [kindware.dev](https://kindware.dev)**
