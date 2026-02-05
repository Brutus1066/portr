//! Process management - killing processes

use crate::error::PortrError;

/// Kill a process by PID
pub fn kill_process(pid: u32, force: bool) -> Result<(), PortrError> {
    #[cfg(unix)]
    {
        kill_unix(pid, force)
    }

    #[cfg(windows)]
    {
        kill_windows(pid, force)
    }
}

/// Unix implementation using signals
#[cfg(unix)]
fn kill_unix(pid: u32, force: bool) -> Result<(), PortrError> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    let signal = if force {
        Signal::SIGKILL
    } else {
        Signal::SIGTERM
    };

    let pid = Pid::from_raw(pid as i32);

    kill(pid, signal).map_err(|e| match e {
        nix::errno::Errno::EPERM => PortrError::PermissionDenied(format!(
            "Cannot kill process {}. Try running with sudo.",
            pid
        )),
        nix::errno::Errno::ESRCH => PortrError::ProcessNotFound(pid.as_raw() as u32),
        _ => PortrError::KillError(pid.as_raw() as u32, e.to_string()),
    })
}

/// Windows implementation using TerminateProcess
#[cfg(windows)]
fn kill_windows(pid: u32, _force: bool) -> Result<(), PortrError> {
    use std::process::Command;

    // Use taskkill command for simplicity and reliability
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| PortrError::KillError(pid, e.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Access is denied") {
            Err(PortrError::PermissionDenied(format!(
                "Cannot kill process {}. Try running as Administrator.",
                pid
            )))
        } else if stderr.contains("not found") {
            Err(PortrError::ProcessNotFound(pid))
        } else {
            Err(PortrError::KillError(pid, stderr.to_string()))
        }
    }
}

/// Check if the current user has permission to kill a process
pub fn can_kill(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        // Signal 0 checks if we can send signals without actually sending one
        let pid = Pid::from_raw(pid as i32);
        kill(pid, None).is_ok()
    }

    #[cfg(windows)]
    {
        // On Windows, we'd need to open the process with PROCESS_TERMINATE
        // For simplicity, assume we can (taskkill will tell us if not)
        true
    }
}

/// Get information about whether elevated privileges are needed
pub fn needs_elevation() -> bool {
    #[cfg(unix)]
    {
        // Check if running as root
        nix::unistd::geteuid().is_root()
    }

    #[cfg(windows)]
    {
        // Check if running as administrator
        // Simplified check - in production you'd use Windows API
        std::env::var("USERNAME")
            .map(|u| u.to_lowercase() == "administrator")
            .unwrap_or(false)
    }
}
