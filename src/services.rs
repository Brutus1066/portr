//! Known service detection and warnings
//!
//! Identifies common services by port and provides safety warnings.

use colored::Colorize;

/// Known service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Port number
    pub port: u16,
    /// Service name
    pub name: &'static str,
    /// Service description
    pub description: &'static str,
    /// Risk level when killing
    pub risk: RiskLevel,
    /// Common process names
    pub process_hints: &'static [&'static str],
}

/// Risk level for killing a service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Safe to kill - development/test services
    Low,
    /// Caution - may affect local applications
    Medium,
    /// Dangerous - system/database services
    High,
    /// Critical - never kill these automatically
    Critical,
}

impl RiskLevel {
    /// Get colored warning string
    pub fn warning(&self) -> String {
        match self {
            RiskLevel::Low => "●".green().to_string(),
            RiskLevel::Medium => "●".yellow().to_string(),
            RiskLevel::High => "●".red().to_string(),
            RiskLevel::Critical => "⚠".red().bold().to_string(),
        }
    }

    /// Get risk label
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "Low Risk",
            RiskLevel::Medium => "Medium Risk",
            RiskLevel::High => "High Risk",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    /// Get colored label
    pub fn colored_label(&self) -> String {
        match self {
            RiskLevel::Low => "Low Risk".green().to_string(),
            RiskLevel::Medium => "Medium Risk".yellow().to_string(),
            RiskLevel::High => "High Risk".red().to_string(),
            RiskLevel::Critical => "CRITICAL".red().bold().to_string(),
        }
    }
}

/// Database of known services
static KNOWN_SERVICES: &[ServiceInfo] = &[
    // Web servers
    ServiceInfo {
        port: 80,
        name: "HTTP",
        description: "Web server (Apache, Nginx, IIS)",
        risk: RiskLevel::Medium,
        process_hints: &["nginx", "apache", "httpd", "iis"],
    },
    ServiceInfo {
        port: 443,
        name: "HTTPS",
        description: "Secure web server",
        risk: RiskLevel::Medium,
        process_hints: &["nginx", "apache", "httpd", "iis"],
    },
    ServiceInfo {
        port: 8080,
        name: "HTTP Alt",
        description: "Alternative HTTP / Development server",
        risk: RiskLevel::Low,
        process_hints: &["java", "node", "python"],
    },
    ServiceInfo {
        port: 8443,
        name: "HTTPS Alt",
        description: "Alternative HTTPS",
        risk: RiskLevel::Low,
        process_hints: &["java", "node"],
    },
    // Databases
    ServiceInfo {
        port: 3306,
        name: "MySQL",
        description: "MySQL/MariaDB database server",
        risk: RiskLevel::Critical,
        process_hints: &["mysqld", "mariadbd", "mysql"],
    },
    ServiceInfo {
        port: 5432,
        name: "PostgreSQL",
        description: "PostgreSQL database server",
        risk: RiskLevel::Critical,
        process_hints: &["postgres", "postgresql"],
    },
    ServiceInfo {
        port: 27017,
        name: "MongoDB",
        description: "MongoDB database server",
        risk: RiskLevel::Critical,
        process_hints: &["mongod", "mongodb"],
    },
    ServiceInfo {
        port: 6379,
        name: "Redis",
        description: "Redis in-memory data store",
        risk: RiskLevel::High,
        process_hints: &["redis-server", "redis"],
    },
    ServiceInfo {
        port: 9200,
        name: "Elasticsearch",
        description: "Elasticsearch search engine",
        risk: RiskLevel::High,
        process_hints: &["elasticsearch", "java"],
    },
    ServiceInfo {
        port: 1433,
        name: "MSSQL",
        description: "Microsoft SQL Server",
        risk: RiskLevel::Critical,
        process_hints: &["sqlservr", "mssql"],
    },
    ServiceInfo {
        port: 1521,
        name: "Oracle",
        description: "Oracle Database",
        risk: RiskLevel::Critical,
        process_hints: &["oracle", "tnslsnr"],
    },
    ServiceInfo {
        port: 5984,
        name: "CouchDB",
        description: "Apache CouchDB",
        risk: RiskLevel::High,
        process_hints: &["couchdb", "beam"],
    },
    ServiceInfo {
        port: 7474,
        name: "Neo4j",
        description: "Neo4j Graph Database",
        risk: RiskLevel::High,
        process_hints: &["neo4j", "java"],
    },
    // Message queues
    ServiceInfo {
        port: 5672,
        name: "RabbitMQ",
        description: "RabbitMQ message broker",
        risk: RiskLevel::High,
        process_hints: &["rabbitmq", "beam", "erlang"],
    },
    ServiceInfo {
        port: 9092,
        name: "Kafka",
        description: "Apache Kafka message broker",
        risk: RiskLevel::High,
        process_hints: &["kafka", "java"],
    },
    ServiceInfo {
        port: 4222,
        name: "NATS",
        description: "NATS message broker",
        risk: RiskLevel::Medium,
        process_hints: &["nats-server", "nats"],
    },
    // Development tools
    ServiceInfo {
        port: 3000,
        name: "Dev Server",
        description: "Node.js / React / Rails dev server",
        risk: RiskLevel::Low,
        process_hints: &["node", "ruby", "rails"],
    },
    ServiceInfo {
        port: 4200,
        name: "Angular",
        description: "Angular development server",
        risk: RiskLevel::Low,
        process_hints: &["node", "ng"],
    },
    ServiceInfo {
        port: 5000,
        name: "Flask/ASP.NET",
        description: "Flask or ASP.NET development server",
        risk: RiskLevel::Low,
        process_hints: &["python", "flask", "dotnet"],
    },
    ServiceInfo {
        port: 5173,
        name: "Vite",
        description: "Vite development server",
        risk: RiskLevel::Low,
        process_hints: &["node", "vite"],
    },
    ServiceInfo {
        port: 8000,
        name: "Django/PHP",
        description: "Django or PHP development server",
        risk: RiskLevel::Low,
        process_hints: &["python", "django", "php"],
    },
    ServiceInfo {
        port: 9000,
        name: "PHP-FPM",
        description: "PHP FastCGI Process Manager",
        risk: RiskLevel::Medium,
        process_hints: &["php-fpm", "php"],
    },
    // Container & orchestration
    ServiceInfo {
        port: 2375,
        name: "Docker",
        description: "Docker daemon (unencrypted)",
        risk: RiskLevel::Critical,
        process_hints: &["dockerd", "docker"],
    },
    ServiceInfo {
        port: 2376,
        name: "Docker TLS",
        description: "Docker daemon (TLS)",
        risk: RiskLevel::Critical,
        process_hints: &["dockerd", "docker"],
    },
    ServiceInfo {
        port: 6443,
        name: "Kubernetes",
        description: "Kubernetes API server",
        risk: RiskLevel::Critical,
        process_hints: &["kube-apiserver", "k8s"],
    },
    ServiceInfo {
        port: 10250,
        name: "Kubelet",
        description: "Kubernetes Kubelet",
        risk: RiskLevel::Critical,
        process_hints: &["kubelet"],
    },
    // System services
    ServiceInfo {
        port: 22,
        name: "SSH",
        description: "Secure Shell server",
        risk: RiskLevel::Critical,
        process_hints: &["sshd", "ssh"],
    },
    ServiceInfo {
        port: 21,
        name: "FTP",
        description: "FTP server",
        risk: RiskLevel::Medium,
        process_hints: &["vsftpd", "proftpd", "ftpd"],
    },
    ServiceInfo {
        port: 23,
        name: "Telnet",
        description: "Telnet server (insecure)",
        risk: RiskLevel::Medium,
        process_hints: &["telnetd"],
    },
    ServiceInfo {
        port: 25,
        name: "SMTP",
        description: "Email server (SMTP)",
        risk: RiskLevel::High,
        process_hints: &["postfix", "sendmail", "exim"],
    },
    ServiceInfo {
        port: 53,
        name: "DNS",
        description: "Domain Name System",
        risk: RiskLevel::Critical,
        process_hints: &["named", "bind", "dnsmasq"],
    },
    ServiceInfo {
        port: 67,
        name: "DHCP",
        description: "DHCP server",
        risk: RiskLevel::Critical,
        process_hints: &["dhcpd", "dnsmasq"],
    },
    ServiceInfo {
        port: 123,
        name: "NTP",
        description: "Network Time Protocol",
        risk: RiskLevel::High,
        process_hints: &["ntpd", "chronyd"],
    },
    ServiceInfo {
        port: 135,
        name: "RPC",
        description: "Windows RPC Endpoint Mapper",
        risk: RiskLevel::Critical,
        process_hints: &["svchost"],
    },
    ServiceInfo {
        port: 139,
        name: "NetBIOS",
        description: "Windows NetBIOS Session",
        risk: RiskLevel::High,
        process_hints: &["smbd", "svchost"],
    },
    ServiceInfo {
        port: 445,
        name: "SMB",
        description: "Windows File Sharing (SMB)",
        risk: RiskLevel::Critical,
        process_hints: &["smbd", "svchost", "System"],
    },
    ServiceInfo {
        port: 3389,
        name: "RDP",
        description: "Windows Remote Desktop",
        risk: RiskLevel::Critical,
        process_hints: &["svchost", "TermService"],
    },
    // Monitoring & observability
    ServiceInfo {
        port: 9090,
        name: "Prometheus",
        description: "Prometheus monitoring",
        risk: RiskLevel::Medium,
        process_hints: &["prometheus"],
    },
    ServiceInfo {
        port: 3100,
        name: "Loki",
        description: "Grafana Loki log aggregation",
        risk: RiskLevel::Medium,
        process_hints: &["loki"],
    },
    ServiceInfo {
        port: 3001,
        name: "Grafana",
        description: "Grafana dashboard (alt port)",
        risk: RiskLevel::Medium,
        process_hints: &["grafana"],
    },
    ServiceInfo {
        port: 9093,
        name: "Alertmanager",
        description: "Prometheus Alertmanager",
        risk: RiskLevel::Medium,
        process_hints: &["alertmanager"],
    },
    ServiceInfo {
        port: 16686,
        name: "Jaeger",
        description: "Jaeger tracing UI",
        risk: RiskLevel::Low,
        process_hints: &["jaeger"],
    },
    // AI/ML
    ServiceInfo {
        port: 11434,
        name: "Ollama",
        description: "Ollama LLM server",
        risk: RiskLevel::Low,
        process_hints: &["ollama"],
    },
    ServiceInfo {
        port: 1234,
        name: "LM Studio",
        description: "LM Studio local LLM",
        risk: RiskLevel::Low,
        process_hints: &["lm studio", "lmstudio"],
    },
    ServiceInfo {
        port: 8888,
        name: "Jupyter",
        description: "Jupyter Notebook server",
        risk: RiskLevel::Low,
        process_hints: &["jupyter", "python"],
    },
    // Caching
    ServiceInfo {
        port: 11211,
        name: "Memcached",
        description: "Memcached cache server",
        risk: RiskLevel::High,
        process_hints: &["memcached"],
    },
    // Version control
    ServiceInfo {
        port: 9418,
        name: "Git",
        description: "Git protocol daemon",
        risk: RiskLevel::Medium,
        process_hints: &["git-daemon"],
    },
    // Proxy
    ServiceInfo {
        port: 8888,
        name: "Proxy",
        description: "HTTP Proxy server",
        risk: RiskLevel::Medium,
        process_hints: &["squid", "privoxy"],
    },
    ServiceInfo {
        port: 1080,
        name: "SOCKS",
        description: "SOCKS proxy",
        risk: RiskLevel::Medium,
        process_hints: &["socks", "dante"],
    },
];

/// Look up a known service by port
pub fn lookup(port: u16) -> Option<&'static ServiceInfo> {
    KNOWN_SERVICES.iter().find(|s| s.port == port)
}

/// Get all known services
pub fn all() -> &'static [ServiceInfo] {
    KNOWN_SERVICES
}

/// Check if a port is a known service and return a warning message if applicable
pub fn get_warning(port: u16) -> Option<String> {
    lookup(port).map(|service| {
        format!(
            "{} {} - {} ({})",
            service.risk.warning(),
            service.name.cyan().bold(),
            service.description,
            service.risk.colored_label()
        )
    })
}

/// Check if killing this port should require extra confirmation
pub fn requires_confirmation(port: u16) -> bool {
    lookup(port)
        .map(|s| matches!(s.risk, RiskLevel::High | RiskLevel::Critical))
        .unwrap_or(false)
}

/// Get a short service name for display
pub fn short_name(port: u16) -> Option<&'static str> {
    lookup(port).map(|s| s.name)
}

/// Print detailed service info
pub fn print_service_info(port: u16) {
    if let Some(service) = lookup(port) {
        println!();
        println!(
            "  {} Known Service: {} (port {})",
            "ℹ".blue().bold(),
            service.name.cyan().bold(),
            port.to_string().yellow()
        );
        println!("    {}", service.description.dimmed());
        println!("    Risk Level: {}", service.risk.colored_label());

        if matches!(service.risk, RiskLevel::High | RiskLevel::Critical) {
            println!(
                "    {} Killing this service may cause system instability!",
                "⚠".red().bold()
            );
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_mysql() {
        let service = lookup(3306).unwrap();
        assert_eq!(service.name, "MySQL");
        assert_eq!(service.risk, RiskLevel::Critical);
    }

    #[test]
    fn test_lookup_unknown() {
        assert!(lookup(54321).is_none());
    }

    #[test]
    fn test_requires_confirmation() {
        assert!(requires_confirmation(3306)); // MySQL - Critical
        assert!(requires_confirmation(22)); // SSH - Critical
        assert!(requires_confirmation(6379)); // Redis - High
        assert!(!requires_confirmation(3000)); // Dev server - Low
        assert!(!requires_confirmation(65432)); // Unknown port
    }

    #[test]
    fn test_short_name() {
        assert_eq!(short_name(5432), Some("PostgreSQL"));
        assert_eq!(short_name(11434), Some("Ollama"));
        assert_eq!(short_name(65432), None);
    }
}
