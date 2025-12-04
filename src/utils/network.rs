use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Network utility functions
pub struct NetworkUtils;

impl NetworkUtils {
    /// Check if a port is available
    pub async fn is_port_available(port: u16) -> bool {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        match timeout(Duration::from_millis(100), TcpStream::connect(addr)).await {
            Ok(Ok(_)) => false, // Port is in use
            Ok(Err(_)) => true,  // Port is available
            Err(_) => true,       // Timeout, assume port is available
        }
    }
    
    /// Find an available port
    pub async fn find_available_port(start_port: u16, end_port: u16) -> Option<u16> {
        for port in start_port..=end_port {
            if Self::is_port_available(port).await {
                return Some(port);
            }
        }
        None
    }
    
    /// Check network connectivity
    pub async fn check_network_connectivity(url: &str) -> bool {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        
        match client.get(url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
    
    /// Get local IP address
    pub fn get_local_ip() -> Option<IpAddr> {
        use std::net::UdpSocket;
        
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;
        socket.local_addr().ok().map(|addr| addr.ip())
    }
    
    /// Get public IP address
    pub async fn get_public_ip() -> Option<IpAddr> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        
        // Try multiple IP lookup services
        let services = vec![
            "https://api.ipify.org",
            "https://icanhazip.com",
            "https://ifconfig.me/ip",
        ];
        
        for service in services {
            if let Ok(response) = client.get(service).send().await {
                if let Ok(ip_str) = response.text().await {
                    let ip_str = ip_str.trim();
                    if let Ok(ip) = ip_str.parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
        
        None
    }
    
    /// Check if an IP address is private
    pub fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                ipv4.is_private()
                    || ipv4.is_loopback()
                    || ipv4.is_link_local()
                    || ipv4.is_multicast()
            }
            IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
                    || ipv6.is_unspecified()
                    || ipv6.is_unique_local()
                    || ipv6.is_multicast()
            }
        }
    }
    
    /// Validate IP address format
    pub fn is_valid_ip(ip_str: &str) -> bool {
        ip_str.parse::<IpAddr>().is_ok()
    }
    
    /// Validate port number
    pub fn is_valid_port(port: u16) -> bool {
        port > 0 && port <= 65535
    }
    
    /// Parse socket address
    pub fn parse_socket_addr(addr_str: &str) -> Option<SocketAddr> {
        addr_str.parse::<SocketAddr>().ok()
    }
    
    /// Format socket address
    pub fn format_socket_addr(addr: SocketAddr) -> String {
        format!("{}:{}", addr.ip(), addr.port())
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub bind_port: u16,
    pub external_address: Option<String>,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub keep_alive: bool,
    pub tcp_nodelay: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            bind_port: 8080,
            external_address: None,
            max_connections: 1000,
            connection_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
            keep_alive: true,
            tcp_nodelay: true,
        }
    }
}

/// Network monitoring utilities
pub struct NetworkMonitor;

impl NetworkMonitor {
    /// Measure network latency
    pub async fn measure_latency(host: &str, port: u16) -> Option<Duration> {
        let addr = format!("{}:{}", host, port);
        let start = std::time::Instant::now();
        
        match timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => Some(start.elapsed()),
            _ => None,
        }
    }
    
    /// Batch test network latency
    pub async fn batch_latency_test(
        hosts: &[String],
        port: u16,
    ) -> Vec<(String, Option<Duration>)> {
        let mut results = Vec::new();
        
        for host in hosts {
            let latency = Self::measure_latency(host, port).await;
            results.push((host.clone(), latency));
        }
        
        results
    }
    
    /// Check network quality
    pub async fn check_network_quality(host: &str, port: u16) -> NetworkQuality {
        let mut latencies = Vec::new();
        
        // Perform multiple tests
        for _ in 0..5 {
            if let Some(latency) = Self::measure_latency(host, port).await {
                latencies.push(latency);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        if latencies.is_empty() {
            return NetworkQuality::Poor;
        }
        
        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let min_latency = latencies.iter().min().unwrap();
        let max_latency = latencies.iter().max().unwrap();
        let jitter = max_latency.saturating_sub(*min_latency);
        
        // Evaluate network quality based on latency and jitter
        if avg_latency < Duration::from_millis(50) && jitter < Duration::from_millis(20) {
            NetworkQuality::Excellent
        } else if avg_latency < Duration::from_millis(100) && jitter < Duration::from_millis(50) {
            NetworkQuality::Good
        } else if avg_latency < Duration::from_millis(200) && jitter < Duration::from_millis(100) {
            NetworkQuality::Fair
        } else {
            NetworkQuality::Poor
        }
    }
}

/// Network quality levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

impl std::fmt::Display for NetworkQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkQuality::Excellent => write!(f, "Excellent"),
            NetworkQuality::Good => write!(f, "Good"),
            NetworkQuality::Fair => write!(f, "Fair"),
            NetworkQuality::Poor => write!(f, "Poor"),
        }
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub failed_connections: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub average_latency: Duration,
    pub connection_success_rate: f64,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            failed_connections: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            average_latency: Duration::from_millis(0),
            connection_success_rate: 1.0,
        }
    }
}

impl NetworkStats {
    /// Update connection statistics
    pub fn update_connection_stats(&mut self, success: bool) {
        self.total_connections += 1;
        if success {
            self.active_connections += 1;
        } else {
            self.failed_connections += 1;
        }
        
        self.connection_success_rate = self.active_connections as f64 / self.total_connections as f64;
    }
    
    /// Update byte statistics
    pub fn update_byte_stats(&mut self, bytes_sent: u64, bytes_received: u64) {
        self.total_bytes_sent += bytes_sent;
        self.total_bytes_received += bytes_received;
    }
    
    /// Update latency statistics
    pub fn update_latency_stats(&mut self, new_latency: Duration) {
        let current_total_ms = self.average_latency.as_millis() as u64 * self.total_connections;
        let new_total_ms = current_total_ms + new_latency.as_millis() as u64;
        self.average_latency = Duration::from_millis(new_total_ms / (self.total_connections + 1));
        self.total_connections += 1;
    }
    
    /// Get network utilization
    pub fn get_network_utilization(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            self.active_connections as f64 / self.total_connections as f64
        }
    }
    
    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Network connection pool
pub struct ConnectionPool {
    config: NetworkConfig,
    stats: NetworkStats,
    connections: Vec<TcpStream>,
}

impl ConnectionPool {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            stats: NetworkStats::default(),
            connections: Vec::new(),
        }
    }
    
    /// Get a connection
    pub async fn get_connection(&mut self, host: &str, port: u16) -> Option<TcpStream> {
        // Try to reuse existing connection
        if let Some(connection) = self.connections.pop() {
            self.stats.update_connection_stats(true);
            return Some(connection);
        }
        
        // Create a new connection
        let addr = format!("{}:{}", host, port);
        match timeout(self.config.connection_timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(stream)) => {
                self.stats.update_connection_stats(true);
                Some(stream)
            }
            _ => {
                self.stats.update_connection_stats(false);
                None
            }
        }
    }
    
    /// Return a connection
    pub fn return_connection(&mut self, connection: TcpStream) {
        if self.connections.len() < self.config.max_connections {
            self.connections.push(connection);
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> &NetworkStats {
        &self.stats
    }
    
    /// Cleanup the connection pool
    pub fn cleanup(&mut self) {
        self.connections.clear();
    }
}

/// Network tools
pub struct NetworkTools;

impl NetworkTools {
    /// Resolve domain name
    pub async fn resolve_domain(domain: &str) -> Option<IpAddr> {
        use tokio::net::lookup_host;
        
        let addr = format!("{}:80", domain);
        let lookup_result = lookup_host(addr).await;
        match lookup_result {
            Ok(mut addresses) => addresses.next().map(|addr| addr.ip()),
            Err(_) => None,
        }
    }
    
    /// Scan port range
    pub async fn scan_port_range(
        host: &str,
        start_port: u16,
        end_port: u16,
    ) -> Vec<u16> {
        let mut open_ports = Vec::new();
        
        for port in start_port..=end_port {
            if NetworkUtils::is_port_available(port).await {
                open_ports.push(port);
            }
        }
        
        open_ports
    }
    
    /// Network connectivity test
    pub async fn connectivity_test(hosts: &[String]) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        
        for host in hosts {
            let is_reachable = NetworkUtils::check_network_connectivity(&format!("http://{}", host)).await;
            results.push((host.clone(), is_reachable));
        }
        
        results
    }
    
    /// Get network interface information
    pub fn get_network_interfaces() -> Vec<NetworkInterface> {
        // NetworkInterface::show() does not exist; using a placeholder implementation
        vec![NetworkInterface { 
            name: "eth0".to_string(), 
            addresses: vec!["192.168.1.1".to_string()],
            is_up: true,
            is_loopback: false,
        }]
    }
}

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub addresses: Vec<String>,
    pub is_up: bool,
    pub is_loopback: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_port_availability() {
        // Test a port that is unlikely to be in use
        let port = 49152; // Dynamic port range
        let is_available = NetworkUtils::is_port_available(port).await;
        assert!(is_available);
    }
    
    #[test]
    fn test_ip_validation() {
        assert!(NetworkUtils::is_valid_ip("127.0.0.1"));
        assert!(NetworkUtils::is_valid_ip("::1"));
        assert!(!NetworkUtils::is_valid_ip("invalid"));
    }
    
    #[test]
    fn test_port_validation() {
        assert!(NetworkUtils::is_valid_port(80));
        assert!(NetworkUtils::is_valid_port(8080));
        assert!(!NetworkUtils::is_valid_port(0));
    }
    
    #[test]
    fn test_private_ip_detection() {
        assert!(NetworkUtils::is_private_ip(&"127.0.0.1".parse().unwrap()));
        assert!(NetworkUtils::is_private_ip(&"192.168.1.1".parse().unwrap()));
        assert!(!NetworkUtils::is_private_ip(&"8.8.8.8".parse().unwrap()));
    }
    
    #[tokio::test]
    async fn test_network_stats() {
        let mut stats = NetworkStats::default();
        
        stats.update_connection_stats(true);
        stats.update_connection_stats(false);
        stats.update_connection_stats(true);
        
        assert_eq!(stats.total_connections, 3);
        assert_eq!(stats.active_connections, 2);
        assert_eq!(stats.failed_connections, 1);
        assert!((stats.connection_success_rate - 2.0/3.0).abs() < f64::EPSILON);
    }
}
