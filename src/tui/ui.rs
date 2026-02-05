//! UI rendering for the TUI dashboard
//!
//! Beautiful, viral-screenshot-worthy interface!

use super::app::{App, MENU_ITEMS};
use crate::services;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
    Frame,
};

// Modern color palette (inspired by Tokyo Night)
const ACCENT: Color = Color::Rgb(122, 162, 247); // Soft blue
const ACCENT2: Color = Color::Rgb(187, 154, 247); // Purple
const SUCCESS: Color = Color::Rgb(158, 206, 106); // Green
const WARNING: Color = Color::Rgb(224, 175, 104); // Orange/yellow
const DANGER: Color = Color::Rgb(247, 118, 142); // Red/pink
const MUTED: Color = Color::Rgb(86, 95, 137); // Muted gray-blue
const BG_DARK: Color = Color::Rgb(26, 27, 38); // Dark background
const TEXT_DIM: Color = Color::Rgb(169, 177, 214); // Dimmed text

/// Main draw function
pub fn draw(f: &mut Frame, app: &App) {
    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/banner
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer/help
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_body(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    // Overlays (only one at a time)
    if app.show_export {
        draw_export_popup(f, app);
    } else if app.show_menu {
        draw_menu_popup(f, app);
    } else if app.show_help {
        draw_help_popup(f);
    }
}

/// Draw the header with animated banner and status
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    // Animated pulse effect based on tick
    let pulse = (app.tick % 20) < 10;
    let accent_color = if pulse { ACCENT } else { ACCENT2 };

    // Stylized compact ASCII logo - fits in header
    let logo = vec![
        Span::styled("╭─", Style::default().fg(MUTED)),
        Span::styled("🐸", Style::default()),
        Span::styled("─╮", Style::default().fg(MUTED)),
        Span::styled(" ░▒▓", Style::default().fg(MUTED)),
        Span::styled(
            " PORTR ",
            Style::default()
                .fg(accent_color)
                .bold()
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled("▓▒░ ", Style::default().fg(MUTED)),
        Span::styled("╭", Style::default().fg(MUTED)),
        Span::styled("──", Style::default().fg(ACCENT)),
        Span::styled("╮", Style::default().fg(MUTED)),
        Span::raw("  "),
    ];

    // Stats section
    let mut stats = vec![
        Span::styled(
            format!("⬢ {} ", app.ports.len()),
            Style::default().fg(SUCCESS).bold(),
        ),
        Span::styled("ports", Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(MUTED)),
        Span::styled(
            format!("◈ {}", app.filter_mode_str()),
            Style::default().fg(WARNING),
        ),
        Span::styled("  │  ", Style::default().fg(MUTED)),
        Span::styled(
            format!("⇅ {}", app.sort_mode_str()),
            Style::default().fg(ACCENT2),
        ),
    ];

    // Add active filter badges with pill-style
    if app.docker_only {
        stats.push(Span::styled("  ", Style::default()));
        stats.push(Span::styled(
            " 🐳 Docker ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .bold(),
        ));
    }
    if app.critical_only {
        stats.push(Span::styled("  ", Style::default()));
        stats.push(Span::styled(
            " ⚠ Critical ",
            Style::default().fg(Color::Black).bg(DANGER).bold(),
        ));
    }
    if !app.filter_text.is_empty() {
        stats.push(Span::styled("  ", Style::default()));
        stats.push(Span::styled(
            format!(" 🔍 {} ", app.filter_text),
            Style::default().fg(Color::Black).bg(ACCENT).bold(),
        ));
    }

    // Add status message if present (with fade effect based on time)
    let status = if let Some(ref msg) = app.status_message {
        vec![
            Span::styled("  │  ", Style::default().fg(MUTED)),
            Span::styled(msg.as_str(), Style::default().fg(Color::White).italic()),
        ]
    } else {
        vec![]
    };

    // Combine all sections
    let content: Vec<Span> = [logo, stats, status].concat();

    let header = Paragraph::new(Line::from(content))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(MUTED)),
        )
        .style(Style::default());

    f.render_widget(header, area);
}

/// Draw the main body with table and details
fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    if app.show_details && !app.ports.is_empty() {
        // Split: table (left) + details (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);

        draw_table(f, app, chunks[0]);
        draw_details(f, app, chunks[1]);
    } else {
        // Full width table
        draw_table(f, app, area);
    }
}

/// Draw the port table
fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = [
        "", "PORT", "PROTO", "PID", "PROCESS", "MEMORY", "UPTIME", "STATE",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().fg(ACCENT).bold()));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.ports.iter().enumerate().map(|(i, port)| {
        let is_selected = i == app.selected;
        let is_critical = services::requires_confirmation(port.port);
        let is_docker = port.process_name.to_lowercase().contains("docker");

        // Selection indicator with animation
        let selector = if is_selected { "▶" } else { " " };
        let selector_style = if is_selected {
            Style::default().fg(SUCCESS).bold()
        } else {
            Style::default()
        };

        // Format memory with color based on usage
        let memory = format_mb(port.memory_mb);
        let mem_style = if port.memory_mb > 500.0 {
            Style::default().fg(DANGER)
        } else if port.memory_mb > 100.0 {
            Style::default().fg(WARNING)
        } else {
            Style::default().fg(SUCCESS)
        };

        // Format uptime
        let uptime = format_uptime(port.uptime_secs);

        // State with color
        let state = &port.state;

        // Row style based on type
        let row_style = if is_critical {
            Style::default().fg(WARNING)
        } else if is_docker {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default().fg(TEXT_DIM)
        };

        // Port number style
        let port_style = if is_critical {
            Style::default().fg(DANGER).bold()
        } else {
            Style::default().fg(ACCENT).bold()
        };

        let cells = vec![
            Cell::from(selector).style(selector_style),
            Cell::from(port.port.to_string()).style(port_style),
            Cell::from(port.protocol.as_str()).style(Style::default().fg(ACCENT2)),
            Cell::from(port.pid.to_string()).style(Style::default().fg(MUTED)),
            Cell::from(truncate(&port.process_name, 20)).style(Style::default().fg(Color::White)),
            Cell::from(memory).style(mem_style),
            Cell::from(uptime).style(Style::default().fg(TEXT_DIM)),
            Cell::from(state.as_str()).style(state_style(state)),
        ];

        Row::new(cells).style(row_style)
    });

    let widths = [
        Constraint::Length(2),  // selector
        Constraint::Length(7),  // port
        Constraint::Length(6),  // proto
        Constraint::Length(8),  // pid
        Constraint::Length(20), // process
        Constraint::Length(10), // memory
        Constraint::Length(10), // uptime
        Constraint::Min(8),     // state
    ];

    // Dynamic title with count
    let title = format!(" ⬡ Listening Ports ({}) ", app.ports.len());

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(title, Style::default().fg(ACCENT).bold())),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(45, 50, 80))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);

    // Scrollbar with modern style
    if app.ports.len() > (area.height as usize - 4) {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        let mut scrollbar_state = ScrollbarState::new(app.ports.len()).position(app.selected);
        f.render_stateful_widget(
            scrollbar.style(Style::default().fg(MUTED)),
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the details panel
fn draw_details(f: &mut Frame, app: &App, area: Rect) {
    let port = match app.get_selected() {
        Some(p) => p,
        None => {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No port selected",
                    Style::default().fg(MUTED),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Use ↑↓ to navigate",
                    Style::default().fg(TEXT_DIM),
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(" ◈ Details ", Style::default().fg(ACCENT))),
            )
            .style(Style::default().fg(TEXT_DIM));
            f.render_widget(empty, area);
            return;
        }
    };

    let is_critical = services::requires_confirmation(port.port);
    let is_docker = port.process_name.to_lowercase().contains("docker");
    let service = services::lookup(port.port);

    // Memory usage for mini-sparkline visual
    let mem_bar = create_mem_bar(port.memory_mb);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⬢ Port: ", Style::default().fg(MUTED)),
            Span::styled(port.port.to_string(), Style::default().fg(ACCENT).bold()),
            if is_critical {
                Span::styled(" ⚠", Style::default().fg(DANGER))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("  ◉ Process: ", Style::default().fg(MUTED)),
            Span::styled(
                port.process_name.as_str(),
                Style::default().fg(SUCCESS).bold(),
            ),
            if is_docker {
                Span::styled(" 🐳", Style::default())
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("  ⊙ PID: ", Style::default().fg(MUTED)),
            Span::styled(port.pid.to_string(), Style::default().fg(WARNING)),
        ]),
        Line::from(vec![
            Span::styled("  ◈ Protocol: ", Style::default().fg(MUTED)),
            Span::styled(&port.protocol, Style::default().fg(ACCENT2)),
        ]),
        Line::from(vec![
            Span::styled("  ⊕ Local: ", Style::default().fg(MUTED)),
            Span::styled(&port.local_address, Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ────────────────────────",
            Style::default().fg(MUTED),
        )),
        Line::from(""),
    ];

    // Memory with visual bar
    lines.push(Line::from(vec![
        Span::styled("  ▤ Memory: ", Style::default().fg(MUTED)),
        Span::styled(
            format_mb(port.memory_mb),
            Style::default().fg(if port.memory_mb > 100.0 {
                WARNING
            } else {
                SUCCESS
            }),
        ),
        Span::styled(
            format!(" {}", mem_bar),
            Style::default().fg(if port.memory_mb > 500.0 {
                DANGER
            } else if port.memory_mb > 100.0 {
                WARNING
            } else {
                SUCCESS
            }),
        ),
    ]));

    // CPU
    lines.push(Line::from(vec![
        Span::styled("  ◐ CPU: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{:.1}%", port.cpu_percent),
            Style::default().fg(ACCENT),
        ),
    ]));

    // Uptime
    lines.push(Line::from(vec![
        Span::styled("  ◷ Uptime: ", Style::default().fg(MUTED)),
        Span::styled(
            format_uptime(port.uptime_secs),
            Style::default().fg(TEXT_DIM),
        ),
    ]));

    lines.push(Line::from(""));

    // Service info
    if let Some(svc) = service {
        lines.push(Line::from(vec![
            Span::styled("  ★ Service: ", Style::default().fg(MUTED)),
            Span::styled(svc.name, Style::default().fg(ACCENT).bold()),
        ]));

        if !svc.description.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(svc.description, Style::default().fg(TEXT_DIM).italic()),
            ]));
        }
    }

    // Critical warning box
    if is_critical {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ┌─────────────────────────┐",
            Style::default().fg(DANGER),
        )));
        lines.push(Line::from(Span::styled(
            "  │  ⚠ CRITICAL SERVICE    │",
            Style::default().fg(DANGER).bold(),
        )));
        lines.push(Line::from(Span::styled(
            "  │  System may be affected │",
            Style::default().fg(WARNING),
        )));
        lines.push(Line::from(Span::styled(
            "  └─────────────────────────┘",
            Style::default().fg(DANGER),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Status: ", Style::default().fg(MUTED)),
            Span::styled("PROTECTED", Style::default().fg(DANGER).bold()),
        ]));
        lines.push(Line::from(Span::styled(
            "  Use: portr kill <port> -f",
            Style::default().fg(TEXT_DIM).italic(),
        )));
    }

    let border_style = if is_critical {
        Style::default().fg(DANGER)
    } else if is_docker {
        Style::default().fg(Color::LightBlue)
    } else {
        Style::default().fg(MUTED)
    };

    let title_icon = if is_critical {
        "⚠"
    } else if is_docker {
        "🐳"
    } else {
        "◈"
    };
    let title = format!(" {} Details ", title_icon);

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, Style::default().fg(ACCENT).bold())),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(details, area);
}

/// Create a visual memory bar
fn create_mem_bar(memory_mb: f64) -> String {
    let filled = ((memory_mb / 100.0).min(10.0) as usize).min(10);
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Draw the footer with help
fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let help = if app.input_mode {
        Line::from(vec![
            Span::styled(" 🔍 ", Style::default().fg(ACCENT)),
            Span::styled(&app.filter_input, Style::default().fg(Color::White).bold()),
            Span::styled(
                "▋",
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::SLOW_BLINK),
            ), // Cursor
            Span::styled("  ", Style::default()),
            Span::styled("⏎ apply", Style::default().fg(SUCCESS)),
            Span::styled("  ", Style::default()),
            Span::styled("⎋ cancel", Style::default().fg(DANGER)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled("m", Style::default().fg(Color::Black).bg(ACCENT).bold()),
            Span::styled(" menu ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" ↑↓ ", Style::default().fg(WARNING)),
            Span::styled("nav ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" K ", Style::default().fg(Color::Black).bg(DANGER).bold()),
            Span::styled(" kill ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" / ", Style::default().fg(ACCENT)),
            Span::styled("search ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" e ", Style::default().fg(Color::Black).bg(SUCCESS).bold()),
            Span::styled(" export ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" r ", Style::default().fg(SUCCESS)),
            Span::styled("refresh ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" ? ", Style::default().fg(ACCENT2)),
            Span::styled("help ", Style::default().fg(TEXT_DIM)),
            Span::styled("│", Style::default().fg(MUTED)),
            Span::styled(" q ", Style::default().fg(DANGER)),
            Span::styled("quit", Style::default().fg(TEXT_DIM)),
        ])
    };

    let footer = Paragraph::new(help)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(MUTED)),
        )
        .style(Style::default());

    f.render_widget(footer, area);
}

/// Draw menu popup overlay
fn draw_menu_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(55, 65, f.area());

    // Clear background
    f.render_widget(Clear, area);

    let pulse = (app.tick % 16) < 8;
    let title_color = if pulse { ACCENT } else { ACCENT2 };

    let mut menu_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "     ╭────────────────────────────────────╮",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("     │   ", Style::default().fg(MUTED)),
            Span::styled(
                "🐸 PORTR QUICK MENU",
                Style::default().fg(title_color).bold(),
            ),
            Span::styled("      │", Style::default().fg(MUTED)),
        ]),
        Line::from(Span::styled(
            "     ╰────────────────────────────────────╯",
            Style::default().fg(MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "     ┌─ Views ────────────────────────────┐",
            Style::default().fg(MUTED),
        )),
    ];

    // Icons for each menu item (9 items now)
    let icons = ["📊", "📋", "🔌", "📡", "🐳", "⚠️", "📁", "❓", "🚪"];
    let item_colors = [
        ACCENT,
        SUCCESS,
        ACCENT2,
        WARNING,
        Color::LightBlue,
        DANGER,
        SUCCESS,
        ACCENT,
        DANGER,
    ];

    for (i, (key, name, desc)) in MENU_ITEMS.iter().enumerate() {
        let is_selected = i == app.menu_selected;

        // Add section dividers
        if i == 4 {
            menu_lines.push(Line::from(Span::styled(
                "     ├─ Filters ──────────────────────────┤",
                Style::default().fg(MUTED),
            )));
        }
        if i == 6 {
            menu_lines.push(Line::from(Span::styled(
                "     ├─ Actions ──────────────────────────┤",
                Style::default().fg(MUTED),
            )));
        }
        if i == 8 {
            menu_lines.push(Line::from(Span::styled(
                "     ├────────────────────────────────────┤",
                Style::default().fg(MUTED),
            )));
        }

        let selector = if is_selected { " ▶" } else { "  " };
        let bg_style = if is_selected {
            Style::default().bg(Color::Rgb(45, 50, 80))
        } else {
            Style::default()
        };
        let name_style = if is_selected {
            Style::default().fg(item_colors[i]).bold()
        } else {
            Style::default().fg(TEXT_DIM)
        };
        let desc_style = Style::default().fg(MUTED).italic();

        menu_lines.push(
            Line::from(vec![
                Span::styled("     │", Style::default().fg(MUTED)),
                Span::styled(selector, Style::default().fg(SUCCESS)),
                Span::styled(
                    format!(" {} ", key),
                    Style::default().fg(Color::Black).bg(item_colors[i]).bold(),
                ),
                Span::raw(format!(" {} ", icons[i])),
                Span::styled(format!("{:<12}", name), name_style),
                Span::styled(desc.to_string(), desc_style),
            ])
            .style(bg_style),
        );
    }

    menu_lines.push(Line::from(Span::styled(
        "     └────────────────────────────────────┘",
        Style::default().fg(MUTED),
    )));
    menu_lines.push(Line::from(""));
    menu_lines.push(Line::from(vec![
        Span::styled("     ", Style::default()),
        Span::styled(" ↑↓ ", Style::default().fg(Color::Black).bg(WARNING)),
        Span::styled(" navigate  ", Style::default().fg(TEXT_DIM)),
        Span::styled(" ⏎ ", Style::default().fg(Color::Black).bg(SUCCESS)),
        Span::styled(" select  ", Style::default().fg(TEXT_DIM)),
        Span::styled(" 1-8,0 ", Style::default().fg(Color::Black).bg(ACCENT)),
        Span::styled(" jump", Style::default().fg(TEXT_DIM)),
    ]));
    menu_lines.push(Line::from(vec![
        Span::styled("     ", Style::default()),
        Span::styled(" m ", Style::default().fg(Color::Black).bg(ACCENT2)),
        Span::styled(" close  ", Style::default().fg(TEXT_DIM)),
        Span::styled(" q ", Style::default().fg(Color::Black).bg(DANGER)),
        Span::styled(" quit app", Style::default().fg(TEXT_DIM)),
    ]));

    let menu = Paragraph::new(menu_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(BG_DARK)),
        )
        .style(Style::default().bg(BG_DARK));

    f.render_widget(menu, area);
}

/// Draw help popup overlay
fn draw_help_popup(f: &mut Frame) {
    let area = centered_rect(65, 75, f.area());

    // Clear background
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("⌨️ KEYBOARD SHORTCUTS", Style::default().fg(ACCENT).bold()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "   ┌─ Navigation ─────────────────────────────┐",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" j/↓ ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Move down          ", Style::default().fg(TEXT_DIM)),
            Span::styled(" PgDn ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Page down │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" k/↑ ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Move up            ", Style::default().fg(TEXT_DIM)),
            Span::styled(" PgUp ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Page up   │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" g ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Go to first        ", Style::default().fg(TEXT_DIM)),
            Span::styled(" G ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(" Go to last      │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(Span::styled(
            "   ├─ Actions ────────────────────────────────┤",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" K ", Style::default().fg(Color::Black).bg(DANGER).bold()),
            Span::styled(
                " Kill selected process (Shift+K)      │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" r ", Style::default().fg(Color::Black).bg(SUCCESS)),
            Span::styled(
                " Refresh port list                    │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" e ", Style::default().fg(Color::Black).bg(SUCCESS)),
            Span::styled(
                " Export ports (JSON/CSV/MD)           │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(Span::styled(
            "   ├─ Filters & Views ────────────────────────┤",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" m ", Style::default().fg(Color::Black).bg(ACCENT)),
            Span::styled(
                " Open quick menu                      │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" / ", Style::default().fg(Color::Black).bg(ACCENT)),
            Span::styled(
                " Search/filter by text                │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" f ", Style::default().fg(Color::Black).bg(ACCENT2)),
            Span::styled(
                " Cycle filter (All/TCP/UDP)           │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(
                " d ",
                Style::default().fg(Color::Black).bg(Color::LightBlue),
            ),
            Span::styled(" Docker only      ", Style::default().fg(TEXT_DIM)),
            Span::styled(" c ", Style::default().fg(Color::Black).bg(DANGER)),
            Span::styled(" Critical only │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" Tab ", Style::default().fg(Color::Black).bg(ACCENT2)),
            Span::styled(" Cycle sort mode ", Style::default().fg(TEXT_DIM)),
            Span::styled(" ⏎ ", Style::default().fg(Color::Black).bg(SUCCESS)),
            Span::styled(" Toggle details │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(Span::styled(
            "   ├─ General ────────────────────────────────┤",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" ? ", Style::default().fg(Color::Black).bg(ACCENT2)),
            Span::styled(" Toggle this help   ", Style::default().fg(TEXT_DIM)),
            Span::styled(" q ", Style::default().fg(Color::Black).bg(DANGER)),
            Span::styled(" Quit portr    │", Style::default().fg(TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("   │ ", Style::default().fg(MUTED)),
            Span::styled(" Esc ", Style::default().fg(Color::Black).bg(WARNING)),
            Span::styled(
                " Clear filters / Exit                │",
                Style::default().fg(TEXT_DIM),
            ),
        ]),
        Line::from(Span::styled(
            "   └──────────────────────────────────────────┘",
            Style::default().fg(MUTED),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "          Press any key to close",
            Style::default().fg(TEXT_DIM).italic(),
        )]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(Span::styled(
                    " 🐸 Help ",
                    Style::default().fg(ACCENT).bold(),
                ))
                .style(Style::default().bg(BG_DARK)),
        )
        .style(Style::default().bg(BG_DARK));

    f.render_widget(help, area);
}

/// Create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Format MB to human readable
fn format_mb(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else if mb >= 1.0 {
        format!("{:.1} MB", mb)
    } else {
        format!("{:.0} KB", mb * 1024.0)
    }
}

/// Format uptime to human readable
fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        format!("{}s", secs)
    }
}

/// Truncate string with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

/// Get style for connection state
fn state_style(state: &str) -> Style {
    match state.to_uppercase().as_str() {
        "LISTEN" | "LISTENING" => Style::default().fg(SUCCESS).bold(),
        "ESTABLISHED" => Style::default().fg(ACCENT),
        "TIME_WAIT" | "CLOSE_WAIT" => Style::default().fg(WARNING),
        "CLOSED" => Style::default().fg(DANGER),
        _ => Style::default().fg(MUTED),
    }
}

/// Draw the export popup
fn draw_export_popup(f: &mut Frame, app: &App) {
    use super::app::ExportFormat;

    let area = centered_rect(50, 35, f.area());

    // Clear background
    f.render_widget(Clear, area);

    // Build format selector
    let json_style = if app.export_format == ExportFormat::Json {
        Style::default().fg(Color::Black).bg(SUCCESS).bold()
    } else {
        Style::default().fg(TEXT_DIM)
    };
    let csv_style = if app.export_format == ExportFormat::Csv {
        Style::default().fg(Color::Black).bg(SUCCESS).bold()
    } else {
        Style::default().fg(TEXT_DIM)
    };
    let md_style = if app.export_format == ExportFormat::Markdown {
        Style::default().fg(Color::Black).bg(SUCCESS).bold()
    } else {
        Style::default().fg(TEXT_DIM)
    };

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  📁 Export ", Style::default().fg(ACCENT).bold()),
            Span::styled(
                format!("{} ports", app.ports.len()),
                Style::default().fg(SUCCESS).bold(),
            ),
            Span::styled(" to file", Style::default().fg(ACCENT).bold()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Select format: ",
            Style::default().fg(TEXT_DIM),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(" [J]SON ", json_style),
            Span::styled("   ", Style::default()),
            Span::styled(" [C]SV ", csv_style),
            Span::styled("   ", Style::default()),
            Span::styled(" [M]arkdown ", md_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ─────────────────────────────────",
            Style::default().fg(MUTED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(" Enter ", Style::default().fg(Color::Black).bg(SUCCESS)),
            Span::styled(" Export   ", Style::default().fg(TEXT_DIM)),
            Span::styled(" Tab ", Style::default().fg(Color::Black).bg(ACCENT)),
            Span::styled(" Cycle   ", Style::default().fg(TEXT_DIM)),
            Span::styled(" Esc ", Style::default().fg(Color::Black).bg(MUTED)),
            Span::styled(" Cancel", Style::default().fg(TEXT_DIM)),
        ]),
    ];

    let popup = Paragraph::new(content).block(
        Block::default()
            .title(vec![
                Span::styled(" 📤 ", Style::default()),
                Span::styled("Export Ports ", Style::default().fg(ACCENT).bold()),
            ])
            .title_alignment(ratatui::layout::Alignment::Center)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(BG_DARK)),
    );

    f.render_widget(popup, area);
}
