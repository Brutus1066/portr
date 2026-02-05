//! Docker container detection and management
//!
//! Detects when a port is bound by a Docker container and provides
//! container-aware kill functionality.
//!
//! ## Container Identification
//! Containers are identified by name + image, not ID, to handle
//! container ID changes during restarts/recreations.

use crate::error::PortrError;

/// Information about a Docker container using a port
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    /// Container ID (short form) - may change on restart
    pub id: String,
    /// Container name - stable identifier
    pub name: String,
    /// Image name - used with name for stable identification
    pub image: String,
    /// Container status
    pub status: String,
    /// All exposed ports
    pub ports: Vec<PortMapping>,
}

impl ContainerInfo {
    /// Get a stable key for identifying this container across restarts.
    /// Uses name + image since container IDs can change.
    pub fn stable_key(&self) -> String {
        format!("{}:{}", self.name, self.image)
    }

    /// Check if this container matches another by stable key (name + image)
    pub fn matches(&self, other: &ContainerInfo) -> bool {
        self.name == other.name && self.image == other.image
    }

    /// Check if container is bound to localhost only (lower risk)
    pub fn is_localhost_only(&self) -> bool {
        // If all host ports are bound to 127.0.0.1, it's localhost only
        // Note: bollard doesn't provide bind IP directly in port summary,
        // so we assume non-localhost by default for safety
        false
    }
}

/// Port mapping for a container
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: Option<u16>,
    pub container_port: u16,
    pub protocol: String,
}

/// Check if Docker is available on the system
pub fn is_docker_available() -> bool {
    #[cfg(windows)]
    {
        // Check if Docker named pipe exists
        std::path::Path::new(r"\\.\pipe\docker_engine").exists()
    }

    #[cfg(unix)]
    {
        std::path::Path::new("/var/run/docker.sock").exists()
    }
}

/// Get container info for a specific port (blocking wrapper for async)
pub fn get_container_for_port(port: u16) -> Option<ContainerInfo> {
    if !is_docker_available() {
        return None;
    }

    // Use blocking runtime for sync context
    std::panic::catch_unwind(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .and_then(|rt| rt.block_on(get_container_for_port_async(port)))
    })
    .unwrap_or_default()
}

/// Get all running containers with their port mappings
pub fn get_all_containers() -> Result<Vec<ContainerInfo>, PortrError> {
    if !is_docker_available() {
        return Ok(Vec::new());
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PortrError::DockerError(e.to_string()))?
        .block_on(get_all_containers_async())
}

/// Stop a container by ID
pub fn stop_container(container_id: &str) -> Result<(), PortrError> {
    if !is_docker_available() {
        return Err(PortrError::DockerError("Docker not available".to_string()));
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PortrError::DockerError(e.to_string()))?
        .block_on(stop_container_async(container_id))
}

/// Stop a container by name (more stable than ID which can change)
pub fn stop_container_by_name(container_name: &str) -> Result<(), PortrError> {
    if !is_docker_available() {
        return Err(PortrError::DockerError("Docker not available".to_string()));
    }

    // Docker API accepts container name as well as ID
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PortrError::DockerError(e.to_string()))?
        .block_on(stop_container_async(container_name))
}

/// Check if a container is running a critical service that requires confirmation
pub fn is_critical_container(container: &ContainerInfo) -> bool {
    let critical_images = [
        "postgres", "mysql", "mariadb", "mongo", "redis",
        "elasticsearch", "rabbitmq", "kafka", "zookeeper",
        "consul", "vault", "etcd", "minio",
    ];

    let image_lower = container.image.to_lowercase();
    critical_images.iter().any(|&c| image_lower.contains(c))
}

// Async implementations using bollard
#[cfg(feature = "docker")]
async fn get_container_for_port_async(port: u16) -> Option<ContainerInfo> {
    use bollard::Docker;
    use bollard::container::ListContainersOptions;
    use std::collections::HashMap;

    let docker = Docker::connect_with_local_defaults().ok()?;
    
    let options = ListContainersOptions::<String> {
        all: false, // Only running containers
        filters: HashMap::new(),
        ..Default::default()
    };

    let containers = docker.list_containers(Some(options)).await.ok()?;

    for container in containers {
        if let Some(ports) = &container.ports {
            for port_binding in ports {
                if let Some(public_port) = port_binding.public_port {
                    if public_port == port {
                        let name = container.names
                            .as_ref()
                            .and_then(|n| n.first())
                            .map(|n| n.trim_start_matches('/').to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        let port_mappings: Vec<PortMapping> = ports
                            .iter()
                            .map(|p| PortMapping {
                                host_port: p.public_port,
                                container_port: p.private_port,
                                protocol: p.typ.map(|t| format!("{:?}", t).to_lowercase()).unwrap_or_else(|| "tcp".to_string()),
                            })
                            .collect();

                        let id_str = container.id.clone().unwrap_or_default();
                        let short_id = if id_str.len() >= 12 {
                            id_str[..12].to_string()
                        } else {
                            id_str
                        };

                        return Some(ContainerInfo {
                            id: short_id,
                            name,
                            image: container.image.clone().unwrap_or_else(|| "unknown".to_string()),
                            status: container.status.clone().unwrap_or_else(|| "unknown".to_string()),
                            ports: port_mappings,
                        });
                    }
                }
            }
        }
    }

    None
}

#[cfg(feature = "docker")]
async fn get_all_containers_async() -> Result<Vec<ContainerInfo>, PortrError> {
    use bollard::Docker;
    use bollard::container::ListContainersOptions;
    use std::collections::HashMap;

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| PortrError::DockerError(e.to_string()))?;

    let options = ListContainersOptions::<String> {
        all: false,
        filters: HashMap::new(),
        ..Default::default()
    };

    let containers = docker
        .list_containers(Some(options))
        .await
        .map_err(|e| PortrError::DockerError(e.to_string()))?;

    let mut result = Vec::new();

    for container in containers {
        let name = container.names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let port_mappings: Vec<PortMapping> = container.ports
            .as_ref()
            .map(|ports| {
                ports.iter().map(|p| PortMapping {
                    host_port: p.public_port,
                    container_port: p.private_port,
                    protocol: p.typ.map(|t| format!("{:?}", t).to_lowercase()).unwrap_or_else(|| "tcp".to_string()),
                }).collect()
            })
            .unwrap_or_default();

        result.push(ContainerInfo {
            id: container.id.clone().unwrap_or_default().chars().take(12).collect(),
            name,
            image: container.image.clone().unwrap_or_else(|| "unknown".to_string()),
            status: container.status.clone().unwrap_or_else(|| "unknown".to_string()),
            ports: port_mappings,
        });
    }

    Ok(result)
}

#[cfg(feature = "docker")]
async fn stop_container_async(container_id: &str) -> Result<(), PortrError> {
    use bollard::Docker;
    use bollard::container::StopContainerOptions;

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| PortrError::DockerError(e.to_string()))?;

    let options = StopContainerOptions {
        t: 10, // 10 second timeout
    };

    docker
        .stop_container(container_id, Some(options))
        .await
        .map_err(|e| PortrError::DockerError(e.to_string()))
}

// Stub implementations when docker feature is not enabled
#[cfg(not(feature = "docker"))]
async fn get_container_for_port_async(_port: u16) -> Option<ContainerInfo> {
    None
}

#[cfg(not(feature = "docker"))]
async fn get_all_containers_async() -> Result<Vec<ContainerInfo>, PortrError> {
    Ok(Vec::new())
}

#[cfg(not(feature = "docker"))]
async fn stop_container_async(_container_id: &str) -> Result<(), PortrError> {
    Err(PortrError::DockerError("Docker feature not enabled. Rebuild with --features docker".to_string()))
}

/// Print Docker container info for a port
pub fn print_container_info(port: u16) {
    use colored::Colorize;

    if !is_docker_available() {
        return;
    }

    if let Some(container) = get_container_for_port(port) {
        println!();
        println!(
            "  {} Docker Container: {}",
            "🐳".blue().bold(),
            container.name.cyan().bold()
        );
        println!("    ID: {}", container.id.dimmed());
        println!("    Image: {}", container.image);
        println!("    Status: {}", container.status.green());
        
        if !container.ports.is_empty() {
            print!("    Ports: ");
            let port_strs: Vec<String> = container.ports
                .iter()
                .filter_map(|p| {
                    p.host_port.map(|hp| format!("{}:{}/{}", hp, p.container_port, p.protocol))
                })
                .collect();
            println!("{}", port_strs.join(", ").yellow());
        }
        
        println!(
            "\n  {} Stop container: {}",
            "→".dimmed(),
            format!("docker stop {}", container.name).yellow()
        );
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_docker_available() {
        // Just ensure it doesn't panic
        let _ = is_docker_available();
    }

    #[test]
    fn test_container_stable_key() {
        let container = ContainerInfo {
            id: "abc123def456".to_string(),
            name: "my-postgres".to_string(),
            image: "postgres:15".to_string(),
            status: "Up 2 hours".to_string(),
            ports: vec![],
        };
        
        // Stable key should be name:image
        assert_eq!(container.stable_key(), "my-postgres:postgres:15");
    }

    #[test]
    fn test_container_matches_same_container_different_id() {
        // Simulates container restart where ID changes but name/image stay same
        let container1 = ContainerInfo {
            id: "abc123def456".to_string(),
            name: "my-postgres".to_string(),
            image: "postgres:15".to_string(),
            status: "Up 2 hours".to_string(),
            ports: vec![],
        };
        
        let container2 = ContainerInfo {
            id: "xyz789abc012".to_string(), // Different ID after restart
            name: "my-postgres".to_string(),
            image: "postgres:15".to_string(),
            status: "Up 1 minute".to_string(),
            ports: vec![],
        };
        
        // Should match because name + image are the same
        assert!(container1.matches(&container2));
        assert_eq!(container1.stable_key(), container2.stable_key());
    }

    #[test]
    fn test_container_does_not_match_different_container() {
        let container1 = ContainerInfo {
            id: "abc123def456".to_string(),
            name: "my-postgres".to_string(),
            image: "postgres:15".to_string(),
            status: "Up 2 hours".to_string(),
            ports: vec![],
        };
        
        let container2 = ContainerInfo {
            id: "xyz789abc012".to_string(),
            name: "my-redis".to_string(),
            image: "redis:7".to_string(),
            status: "Up 1 hour".to_string(),
            ports: vec![],
        };
        
        // Should NOT match - different containers
        assert!(!container1.matches(&container2));
        assert_ne!(container1.stable_key(), container2.stable_key());
    }

    #[test]
    fn test_critical_container_detection_postgres() {
        let container = ContainerInfo {
            id: "abc123".to_string(),
            name: "my-db".to_string(),
            image: "postgres:15-alpine".to_string(),
            status: "Up".to_string(),
            ports: vec![],
        };
        
        assert!(is_critical_container(&container));
    }

    #[test]
    fn test_critical_container_detection_mysql() {
        let container = ContainerInfo {
            id: "abc123".to_string(),
            name: "my-db".to_string(),
            image: "mysql:8.0".to_string(),
            status: "Up".to_string(),
            ports: vec![],
        };
        
        assert!(is_critical_container(&container));
    }

    #[test]
    fn test_critical_container_detection_redis() {
        let container = ContainerInfo {
            id: "abc123".to_string(),
            name: "cache".to_string(),
            image: "redis:7-alpine".to_string(),
            status: "Up".to_string(),
            ports: vec![],
        };
        
        assert!(is_critical_container(&container));
    }

    #[test]
    fn test_non_critical_container_detection() {
        let container = ContainerInfo {
            id: "abc123".to_string(),
            name: "my-app".to_string(),
            image: "node:20-alpine".to_string(),
            status: "Up".to_string(),
            ports: vec![],
        };
        
        // Node app is not critical
        assert!(!is_critical_container(&container));
    }

    #[test]
    fn test_non_critical_nginx_container() {
        let container = ContainerInfo {
            id: "abc123".to_string(),
            name: "web".to_string(),
            image: "nginx:latest".to_string(),
            status: "Up".to_string(),
            ports: vec![],
        };
        
        // Nginx is not in critical list (stateless)
        assert!(!is_critical_container(&container));
    }
}
