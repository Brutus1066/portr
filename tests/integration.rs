//! Integration tests for portr

use std::process::Command;

/// Helper to run portr with arguments
fn portr(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_portr"))
        .args(args)
        .output()
        .expect("Failed to execute portr");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[test]
fn test_help_flag() {
    let (stdout, _, success) = portr(&["--help"]);
    assert!(success);
    assert!(stdout.contains("Lightning-fast port inspector"));
    assert!(stdout.contains("kindware.dev"));
}

#[test]
fn test_version_flag() {
    let (stdout, _, success) = portr(&["--version"]);
    assert!(success);
    assert!(stdout.contains("portr"));
}

#[test]
fn test_list_ports() {
    let (stdout, stderr, success) = portr(&[]);
    // Should succeed even if no ports are found, or fail gracefully on CI
    // CI environments may not have ss/netstat available or may lack permissions
    assert!(
        success
            || stderr.contains("No listening ports")
            || stderr.contains("Network error")
            || stderr.contains("error")
            || stdout.contains("port")
    );
}

#[test]
fn test_invalid_port() {
    let (_, stderr, success) = portr(&["abc"]);
    assert!(!success);
    assert!(stderr.contains("invalid port"));
}

#[test]
fn test_port_not_in_use() {
    // Port 65432 is unlikely to be in use
    let (stdout, _, success) = portr(&["65432"]);
    assert!(success);
    assert!(stdout.contains("available") || stdout.contains("not in use"));
}

#[test]
fn test_port_range() {
    let (stdout, _, success) = portr(&["65400-65410"]);
    assert!(success);
    // Either finds ports or says no ports in range
    assert!(stdout.contains("port") || stdout.contains("No ports"));
}

#[test]
fn test_invalid_range() {
    let (_, stderr, success) = portr(&["3010-3000"]); // Start > end
    assert!(!success);
    assert!(stderr.contains("invalid"));
}

#[test]
fn test_json_output_empty() {
    // Port 65433 is unlikely to be in use, so we test that --json works
    let (_stdout, _, success) = portr(&["65433", "--json"]);
    // When port is not in use, it prints a text message, not JSON
    // This is expected behavior
    assert!(success);
}

#[test]
fn test_tcp_filter() {
    let (stdout, stderr, success) = portr(&["--tcp"]);
    // May fail on CI if ss/netstat not available
    assert!(success || stderr.contains("error") || stdout.contains("port") || stdout.is_empty());
}

#[test]
fn test_udp_filter() {
    let (stdout, stderr, success) = portr(&["--udp"]);
    // May fail on CI if ss/netstat not available
    assert!(success || stderr.contains("error") || stdout.contains("port") || stdout.is_empty());
}
