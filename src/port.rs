//! Port detection and information gathering

use crate::error::PortrError;
use serde::Serialize;
use sysinfo::{Pid, System};

/// Information about a port and its associated process
#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub pid: u32,
    pub process_name: String,
    pub process_path: Option<String>,
    pub local_address: String,
    pub remote_address: Option<String>,
    pub state: String,
    pub user: Option<String>,
    pub memory_mb: f64,
    pub cpu_percent: f32,
    pub uptime_secs: u64,
    /// Parent process ID (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_pid: Option<u32>,
    /// Parent process name (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
}

/// Process tree node for display
#[derive(Debug, Clone)]
pub struct ProcessTreeNode {
    pub pid: u32,
    pub name: String,
    pub is_target: bool,
    pub children: Vec<ProcessTreeNode>,
}

impl PortInfo {
    /// Format uptime as human-readable string
    pub fn uptime_display(&self) -> String {
        let secs = self.uptime_secs;
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
}

/// Get all listening ports on the system
pub fn get_listening_ports() -> Result<Vec<PortInfo>, PortrError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let connections = get_network_connections()?;
    let mut results = Vec::new();

    for conn in connections {
        if let Some(pid) = conn.pid {
            let process_info = get_process_info(&sys, pid);

            results.push(PortInfo {
                port: conn.local_port,
                protocol: conn.protocol.clone(),
                pid,
                process_name: process_info.name,
                process_path: process_info.path,
                local_address: format!("{}:{}", conn.local_addr, conn.local_port),
                remote_address: conn
                    .remote_addr
                    .map(|a| format!("{}:{}", a, conn.remote_port.unwrap_or(0))),
                state: conn.state.clone(),
                user: process_info.user,
                memory_mb: process_info.memory_mb,
                cpu_percent: process_info.cpu_percent,
                uptime_secs: process_info.uptime_secs,
                parent_pid: process_info.parent_pid,
                parent_name: process_info.parent_name,
            });
        }
    }

    // Sort by port number
    results.sort_by_key(|p| p.port);

    // Deduplicate by port (keep first occurrence)
    let mut seen = std::collections::HashSet::new();
    results.retain(|p| seen.insert(p.port));

    Ok(results)
}

/// Get information about a specific port
pub fn get_port_info(port: u16) -> Result<Option<PortInfo>, PortrError> {
    let ports = get_listening_ports()?;
    Ok(ports.into_iter().find(|p| p.port == port))
}

/// Internal: Process information
struct ProcessInfo {
    name: String,
    path: Option<String>,
    user: Option<String>,
    memory_mb: f64,
    cpu_percent: f32,
    uptime_secs: u64,
    parent_pid: Option<u32>,
    parent_name: Option<String>,
}

/// Get process information by PID
fn get_process_info(sys: &System, pid: u32) -> ProcessInfo {
    let pid = Pid::from_u32(pid);

    if let Some(process) = sys.process(pid) {
        // Get parent process info
        let (parent_pid, parent_name) = process
            .parent()
            .and_then(|ppid| sys.process(ppid).map(|p| (ppid, p)))
            .map(|(ppid, parent)| {
                (
                    Some(ppid.as_u32()),
                    Some(parent.name().to_string_lossy().to_string()),
                )
            })
            .unwrap_or((None, None));

        ProcessInfo {
            name: process.name().to_string_lossy().to_string(),
            path: process.exe().map(|p| p.to_string_lossy().to_string()),
            user: process.user_id().map(|u| format!("{:?}", u)),
            memory_mb: process.memory() as f64 / 1024.0 / 1024.0,
            cpu_percent: process.cpu_usage(),
            uptime_secs: process.run_time(),
            parent_pid,
            parent_name,
        }
    } else {
        ProcessInfo {
            name: "<unknown>".to_string(),
            path: None,
            user: None,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            uptime_secs: 0,
            parent_pid: None,
            parent_name: None,
        }
    }
}

/// Internal: Network connection information
struct NetConnection {
    protocol: String,
    local_addr: String,
    local_port: u16,
    remote_addr: Option<String>,
    remote_port: Option<u16>,
    state: String,
    pid: Option<u32>,
}

/// Platform-specific network connection retrieval
#[cfg(target_os = "windows")]
fn get_network_connections() -> Result<Vec<NetConnection>, PortrError> {
    use std::process::Command;

    let output = Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut connections = Vec::new();

    for line in stdout.lines().skip(4) {
        // Skip header lines
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let protocol = parts[0].to_uppercase();
            if protocol != "TCP" && protocol != "UDP" {
                continue;
            }

            // Parse local address
            if let Some((local_addr, local_port)) = parse_address(parts[1]) {
                let (remote_addr, remote_port) = if parts.len() > 2 && protocol == "TCP" {
                    parse_address(parts[2])
                        .map(|(a, p)| (Some(a), Some(p)))
                        .unwrap_or((None, None))
                } else {
                    (None, None)
                };

                let state = if protocol == "TCP" && parts.len() > 3 {
                    parts[3].to_string()
                } else {
                    "LISTENING".to_string()
                };

                let pid_str = parts.last().unwrap_or(&"0");
                let pid: u32 = pid_str.parse().unwrap_or(0);

                // Only include listening sockets
                if state == "LISTENING" || protocol == "UDP" {
                    connections.push(NetConnection {
                        protocol,
                        local_addr,
                        local_port,
                        remote_addr,
                        remote_port,
                        state,
                        pid: if pid > 0 { Some(pid) } else { None },
                    });
                }
            }
        }
    }

    // Also get UDP
    let output_udp = Command::new("netstat")
        .args(["-ano", "-p", "UDP"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout_udp = String::from_utf8_lossy(&output_udp.stdout);

    for line in stdout_udp.lines().skip(4) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Some((local_addr, local_port)) = parse_address(parts[1]) {
                let pid_str = parts.last().unwrap_or(&"0");
                let pid: u32 = pid_str.parse().unwrap_or(0);

                connections.push(NetConnection {
                    protocol: "UDP".to_string(),
                    local_addr,
                    local_port,
                    remote_addr: None,
                    remote_port: None,
                    state: "*".to_string(),
                    pid: if pid > 0 { Some(pid) } else { None },
                });
            }
        }
    }

    Ok(connections)
}

#[cfg(target_os = "linux")]
fn get_network_connections() -> Result<Vec<NetConnection>, PortrError> {
    use std::process::Command;

    let output = Command::new("ss")
        .args(["-tlnp"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut connections = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            // Parse local address (format: addr:port or [::]:port)
            let local = parts[3];
            if let Some((local_addr, local_port)) = parse_linux_address(local) {
                // Extract PID from users:(("name",pid=1234,fd=5))
                let pid = extract_pid_from_ss(&parts[parts.len() - 1..].join(" "));

                connections.push(NetConnection {
                    protocol: "TCP".to_string(),
                    local_addr,
                    local_port,
                    remote_addr: None,
                    remote_port: None,
                    state: parts[0].to_string(),
                    pid,
                });
            }
        }
    }

    // Also get UDP
    let output_udp = Command::new("ss")
        .args(["-ulnp"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout_udp = String::from_utf8_lossy(&output_udp.stdout);

    for line in stdout_udp.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let local = parts[4];
            if let Some((local_addr, local_port)) = parse_linux_address(local) {
                let pid = extract_pid_from_ss(&parts[parts.len() - 1..].join(" "));

                connections.push(NetConnection {
                    protocol: "UDP".to_string(),
                    local_addr,
                    local_port,
                    remote_addr: None,
                    remote_port: None,
                    state: "*".to_string(),
                    pid,
                });
            }
        }
    }

    Ok(connections)
}

#[cfg(target_os = "macos")]
fn get_network_connections() -> Result<Vec<NetConnection>, PortrError> {
    use std::process::Command;

    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut connections = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            let pid: u32 = parts[1].parse().unwrap_or(0);
            let name_part = parts[8]; // e.g., "TCP *:3000 (LISTEN)"

            if let Some(port) = extract_port_from_lsof(name_part) {
                connections.push(NetConnection {
                    protocol: "TCP".to_string(),
                    local_addr: "*".to_string(),
                    local_port: port,
                    remote_addr: None,
                    remote_port: None,
                    state: "LISTEN".to_string(),
                    pid: if pid > 0 { Some(pid) } else { None },
                });
            }
        }
    }

    // Also get UDP
    let output_udp = Command::new("lsof")
        .args(["-iUDP", "-n", "-P"])
        .output()
        .map_err(|e| PortrError::NetworkError(e.to_string()))?;

    let stdout_udp = String::from_utf8_lossy(&output_udp.stdout);

    for line in stdout_udp.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            let pid: u32 = parts[1].parse().unwrap_or(0);
            let name_part = parts[8];

            if let Some(port) = extract_port_from_lsof(name_part) {
                connections.push(NetConnection {
                    protocol: "UDP".to_string(),
                    local_addr: "*".to_string(),
                    local_port: port,
                    remote_addr: None,
                    remote_port: None,
                    state: "*".to_string(),
                    pid: if pid > 0 { Some(pid) } else { None },
                });
            }
        }
    }

    Ok(connections)
}

/// Parse Windows address format (e.g., "0.0.0.0:3000" or "[::]:3000")
#[cfg(target_os = "windows")]
fn parse_address(addr: &str) -> Option<(String, u16)> {
    // Handle IPv6 with brackets
    if addr.starts_with('[') {
        if let Some(bracket_end) = addr.find(']') {
            let ip = &addr[1..bracket_end];
            let port_part = &addr[bracket_end + 1..];
            if let Some(port_str) = port_part.strip_prefix(':') {
                if let Ok(port) = port_str.parse() {
                    return Some((ip.to_string(), port));
                }
            }
        }
    } else {
        // IPv4
        if let Some(colon_pos) = addr.rfind(':') {
            let ip = &addr[..colon_pos];
            if let Ok(port) = addr[colon_pos + 1..].parse() {
                return Some((ip.to_string(), port));
            }
        }
    }
    None
}

/// Parse Linux ss address format
#[cfg(target_os = "linux")]
fn parse_linux_address(addr: &str) -> Option<(String, u16)> {
    // Format: *:port, 0.0.0.0:port, [::]:port, :::port
    if let Some(colon_pos) = addr.rfind(':') {
        let port_str = &addr[colon_pos + 1..];
        if let Ok(port) = port_str.parse() {
            let ip = &addr[..colon_pos];
            let ip = ip.trim_start_matches('[').trim_end_matches(']');
            return Some((ip.to_string(), port));
        }
    }
    None
}

/// Extract PID from ss output
#[cfg(target_os = "linux")]
fn extract_pid_from_ss(users_str: &str) -> Option<u32> {
    // Format: users:(("node",pid=12345,fd=21))
    if let Some(pid_start) = users_str.find("pid=") {
        let rest = &users_str[pid_start + 4..];
        let pid_end = rest.find(|c: char| !c.is_numeric()).unwrap_or(rest.len());
        rest[..pid_end].parse().ok()
    } else {
        None
    }
}

/// Extract port from lsof output
#[cfg(target_os = "macos")]
fn extract_port_from_lsof(name_part: &str) -> Option<u16> {
    // Format: "*:3000" or "localhost:3000"
    if let Some(colon_pos) = name_part.rfind(':') {
        let port_str = &name_part[colon_pos + 1..];
        // Remove any trailing stuff like "(LISTEN)"
        let port_end = port_str
            .find(|c: char| !c.is_numeric())
            .unwrap_or(port_str.len());
        port_str[..port_end].parse().ok()
    } else {
        None
    }
}

/// Get process tree for a given PID (parent chain)
pub fn get_process_tree(pid: u32) -> Vec<(u32, String)> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut tree = Vec::new();
    let mut current_pid = Some(Pid::from_u32(pid));

    // Walk up the parent chain
    while let Some(cpid) = current_pid {
        if let Some(process) = sys.process(cpid) {
            tree.push((cpid.as_u32(), process.name().to_string_lossy().to_string()));
            current_pid = process.parent();
        } else {
            break;
        }

        // Safety limit to prevent infinite loops
        if tree.len() > 20 {
            break;
        }
    }

    tree
}

/// Get child processes for a given PID
pub fn get_child_processes(pid: u32) -> Vec<(u32, String)> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let target_pid = Pid::from_u32(pid);
    let mut children = Vec::new();

    for (child_pid, process) in sys.processes() {
        if process.parent() == Some(target_pid) {
            children.push((
                child_pid.as_u32(),
                process.name().to_string_lossy().to_string(),
            ));
        }
    }

    children
}

/// Build and print a process tree view (ASCII art)
pub fn print_process_tree(port_info: &PortInfo) {
    use colored::Colorize;

    let parent_chain = get_process_tree(port_info.pid);
    let children = get_child_processes(port_info.pid);

    println!("\n{}", " Process Tree ".cyan().bold());
    println!("{}", "─".repeat(40).dimmed());

    // Print parent chain (reversed to show root first)
    let depth = parent_chain.len();
    for (i, (pid, name)) in parent_chain.iter().rev().enumerate() {
        let indent = "  ".repeat(i);
        let connector = if i == 0 { "●" } else { "├─" };

        if *pid == port_info.pid {
            // Highlight the target process
            println!(
                "{}{} {} {} ← {}",
                indent,
                connector.cyan(),
                name.green().bold(),
                format!("(PID {})", pid).yellow(),
                "Target".magenta().bold()
            );
        } else {
            println!(
                "{}{} {} {}",
                indent,
                connector.dimmed(),
                name,
                format!("(PID {})", pid).dimmed()
            );
        }
    }

    // Print children
    if !children.is_empty() {
        let child_indent = "  ".repeat(depth);
        for (i, (pid, name)) in children.iter().enumerate() {
            let connector = if i == children.len() - 1 {
                "└─"
            } else {
                "├─"
            };
            println!(
                "{}{}─{} {} {}",
                child_indent,
                connector.dimmed(),
                "►".blue(),
                name.cyan(),
                format!("(PID {})", pid).dimmed()
            );
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_ipv4() {
        let result = parse_address("0.0.0.0:3000");
        assert_eq!(result, Some(("0.0.0.0".to_string(), 3000)));
    }

    #[test]
    fn test_parse_address_ipv4_localhost() {
        let result = parse_address("127.0.0.1:8080");
        assert_eq!(result, Some(("127.0.0.1".to_string(), 8080)));
    }

    #[test]
    fn test_parse_address_ipv6() {
        let result = parse_address("[::]:3000");
        assert_eq!(result, Some(("::".to_string(), 3000)));
    }

    #[test]
    fn test_parse_address_ipv6_full() {
        let result = parse_address("[::1]:8080");
        assert_eq!(result, Some(("::1".to_string(), 8080)));
    }

    #[test]
    fn test_parse_address_invalid() {
        let result = parse_address("invalid");
        assert_eq!(result, None);
    }

    #[test]
    fn test_uptime_display_seconds() {
        let info = PortInfo {
            port: 3000,
            protocol: "TCP".to_string(),
            pid: 1234,
            process_name: "test".to_string(),
            process_path: None,
            local_address: "0.0.0.0:3000".to_string(),
            remote_address: None,
            state: "LISTENING".to_string(),
            user: None,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            uptime_secs: 45,
            parent_pid: None,
            parent_name: None,
        };
        assert_eq!(info.uptime_display(), "45s");
    }

    #[test]
    fn test_uptime_display_minutes() {
        let info = PortInfo {
            port: 3000,
            protocol: "TCP".to_string(),
            pid: 1234,
            process_name: "test".to_string(),
            process_path: None,
            local_address: "0.0.0.0:3000".to_string(),
            remote_address: None,
            state: "LISTENING".to_string(),
            user: None,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            uptime_secs: 125,
            parent_pid: None,
            parent_name: None,
        };
        assert_eq!(info.uptime_display(), "2m 5s");
    }

    #[test]
    fn test_uptime_display_hours() {
        let info = PortInfo {
            port: 3000,
            protocol: "TCP".to_string(),
            pid: 1234,
            process_name: "test".to_string(),
            process_path: None,
            local_address: "0.0.0.0:3000".to_string(),
            remote_address: None,
            state: "LISTENING".to_string(),
            user: None,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            uptime_secs: 7384,
            parent_pid: None,
            parent_name: None,
        };
        assert_eq!(info.uptime_display(), "2h 3m");
    }

    #[test]
    fn test_uptime_display_days() {
        let info = PortInfo {
            port: 3000,
            protocol: "TCP".to_string(),
            pid: 1234,
            process_name: "test".to_string(),
            process_path: None,
            local_address: "0.0.0.0:3000".to_string(),
            remote_address: None,
            state: "LISTENING".to_string(),
            user: None,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            uptime_secs: 180000,
            parent_pid: None,
            parent_name: None,
        };
        assert_eq!(info.uptime_display(), "2d 2h");
    }
}
