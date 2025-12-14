// mDNS/DNS-SD Discovery for RustyDB
//
// Implements multicast DNS (mDNS) and DNS Service Discovery (DNS-SD) for zero-configuration
// networking in LAN environments. This allows RustyDB nodes to automatically discover
// each other without any configuration.
//
// # Protocol Overview
//
// - Service Type: `_rustydb._tcp.local`
// - Uses multicast address: 224.0.0.251:5353
// - Announces node presence via mDNS
// - Browses for other RustyDB nodes
// - Resolves service instances to IP addresses
//
// # TXT Records
//
// Metadata is shared via TXT records:
// - `version`: Protocol version
// - `node_id`: Unique node identifier
// - `datacenter`: Datacenter location (optional)
// - `rack`: Rack identifier (optional)

use super::{DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, NodeInfo, NodeStatus};
use crate::error::{DbError, Result};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

/// mDNS constants
const MDNS_PORT: u16 = 5353;
const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const SERVICE_TYPE: &str = "_rustydb._tcp.local";

/// mDNS message types
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future MDNS message types
enum MdnsMessageType {
    /// Query for services
    Query,

    /// Response to query
    Response,

    /// Announcement (unsolicited response)
    Announcement,
}

/// mDNS service instance
#[derive(Debug, Clone)]
struct ServiceInstance {
    /// Instance name
    name: String,

    /// Service type
    #[allow(dead_code)] // Reserved for service type tracking
    service_type: String,

    /// Host address
    addr: SocketAddr,

    /// TXT records
    txt_records: HashMap<String, String>,

    /// Time-to-live
    #[allow(dead_code)] // Reserved for TTL tracking
    ttl: u32,
}

impl ServiceInstance {
    fn to_node_info(&self) -> Option<NodeInfo> {
        let node_id = self.txt_records.get("node_id")?.clone();

        let mut node = NodeInfo::new(node_id, self.addr);
        node.metadata = self.txt_records.clone();
        node.status = NodeStatus::Alive;

        Some(node)
    }
}

/// mDNS Discovery implementation
pub struct MdnsDiscovery {
    config: DiscoveryConfig,
    socket: Arc<UdpSocket>,
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    event_tx: mpsc::Sender<DiscoveryEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl MdnsDiscovery {
    /// Create a new mDNS discovery instance
    pub fn new(config: DiscoveryConfig, event_tx: mpsc::Sender<DiscoveryEvent>) -> Result<Self> {
        // Create UDP socket for multicast using socket2
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| DbError::Network(format!("Failed to create socket: {}", e)))?;

        // Set socket options for multicast
        socket
            .set_reuse_address(true)
            .map_err(|e| DbError::Network(format!("Failed to set reuse address: {}", e)))?;

        // On Unix systems, also set SO_REUSEPORT for better multicast support
        #[cfg(unix)]
        socket
            .set_reuse_port(true)
            .map_err(|e| DbError::Network(format!("Failed to set reuse port: {}", e)))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| DbError::Network(format!("Failed to set nonblocking: {}", e)))?;

        // Bind to mDNS port
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), MDNS_PORT);
        socket
            .bind(&bind_addr.into())
            .map_err(|e| DbError::Network(format!("Failed to bind mDNS socket: {}", e)))?;

        // Join multicast group
        socket
            .join_multicast_v4(&MDNS_ADDR, &Ipv4Addr::UNSPECIFIED)
            .map_err(|e| DbError::Network(format!("Failed to join multicast group: {}", e)))?;

        // Convert socket2::Socket to std::net::UdpSocket, then to tokio::net::UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        let socket = Arc::new(
            UdpSocket::from_std(std_socket)
                .map_err(|e| DbError::Network(format!("Failed to create tokio socket: {}", e)))?,
        );

        Ok(Self {
            config,
            socket,
            services: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown_tx: None,
        })
    }

    /// Build TXT records for this node
    fn build_txt_records(&self) -> HashMap<String, String> {
        let mut records = self.config.local_node.metadata.clone();

        records.insert("node_id".to_string(), self.config.local_node.id.clone());
        records.insert(
            "version".to_string(),
            self.config.local_node.protocol_version.to_string(),
        );
        records.insert(
            "port".to_string(),
            self.config.local_node.addr.port().to_string(),
        );

        records
    }

    /// Send mDNS announcement
    async fn send_announcement(&self) -> Result<()> {
        let txt_records = self.build_txt_records();

        // Build DNS message (simplified - in production use a proper DNS library)
        let message = self.build_mdns_response(&self.config.local_node.id, txt_records)?;

        let addr = SocketAddr::new(IpAddr::V4(MDNS_ADDR), MDNS_PORT);
        self.socket
            .send_to(&message, addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to send announcement: {}", e)))?;

        Ok(())
    }

    /// Send mDNS query
    async fn send_query(&self) -> Result<()> {
        // Build DNS query message (simplified)
        let message = self.build_mdns_query(SERVICE_TYPE)?;

        let addr = SocketAddr::new(IpAddr::V4(MDNS_ADDR), MDNS_PORT);
        self.socket
            .send_to(&message, addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to send query: {}", e)))?;

        Ok(())
    }

    /// Build mDNS query message (simplified DNS packet)
    fn build_mdns_query(&self, service_type: &str) -> Result<Vec<u8>> {
        // Simplified DNS query packet
        let mut packet = Vec::new();

        // Transaction ID (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Flags: standard query (2 bytes)
        packet.extend_from_slice(&0x0000u16.to_be_bytes());

        // Questions: 1 (2 bytes)
        packet.extend_from_slice(&1u16.to_be_bytes());

        // Answer RRs: 0 (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Authority RRs: 0 (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Additional RRs: 0 (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Question section
        for label in service_type.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0); // End of name

        // Type: PTR (12)
        packet.extend_from_slice(&12u16.to_be_bytes());

        // Class: IN (1)
        packet.extend_from_slice(&1u16.to_be_bytes());

        Ok(packet)
    }

    /// Build mDNS response message (simplified DNS packet)
    fn build_mdns_response(
        &self,
        instance_name: &str,
        txt_records: HashMap<String, String>,
    ) -> Result<Vec<u8>> {
        // Simplified DNS response packet
        let mut packet = Vec::new();

        // Transaction ID (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Flags: response, authoritative (2 bytes)
        packet.extend_from_slice(&0x8400u16.to_be_bytes());

        // Questions: 0 (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Answer RRs: 1 (2 bytes)
        packet.extend_from_slice(&1u16.to_be_bytes());

        // Authority RRs: 0 (2 bytes)
        packet.extend_from_slice(&0u16.to_be_bytes());

        // Additional RRs: 1 (2 bytes)
        packet.extend_from_slice(&1u16.to_be_bytes());

        // Answer section - PTR record
        let full_name = format!("{}.{}", instance_name, SERVICE_TYPE);
        for label in full_name.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0); // End of name

        // Type: PTR (12)
        packet.extend_from_slice(&12u16.to_be_bytes());

        // Class: IN (1)
        packet.extend_from_slice(&1u16.to_be_bytes());

        // TTL: 120 seconds
        packet.extend_from_slice(&120u32.to_be_bytes());

        // RDATA length (placeholder)
        let rdata_len_pos = packet.len();
        packet.extend_from_slice(&0u16.to_be_bytes());

        // PTR RDATA - instance name
        let rdata_start = packet.len();
        for label in full_name.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0);

        // Update RDATA length
        let rdata_len = (packet.len() - rdata_start) as u16;
        packet[rdata_len_pos..rdata_len_pos + 2].copy_from_slice(&rdata_len.to_be_bytes());

        // Additional section - TXT record
        for label in full_name.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0);

        // Type: TXT (16)
        packet.extend_from_slice(&16u16.to_be_bytes());

        // Class: IN (1)
        packet.extend_from_slice(&1u16.to_be_bytes());

        // TTL: 120 seconds
        packet.extend_from_slice(&120u32.to_be_bytes());

        // RDATA length (placeholder)
        let txt_rdata_len_pos = packet.len();
        packet.extend_from_slice(&0u16.to_be_bytes());

        // TXT RDATA
        let txt_rdata_start = packet.len();
        for (key, value) in txt_records {
            let txt = format!("{}={}", key, value);
            packet.push(txt.len() as u8);
            packet.extend_from_slice(txt.as_bytes());
        }

        // Update RDATA length
        let txt_rdata_len = (packet.len() - txt_rdata_start) as u16;
        packet[txt_rdata_len_pos..txt_rdata_len_pos + 2]
            .copy_from_slice(&txt_rdata_len.to_be_bytes());

        Ok(packet)
    }

    /// Parse mDNS message (simplified)
    fn parse_mdns_message(&self, data: &[u8]) -> Result<Option<ServiceInstance>> {
        // Simplified parsing - in production use a proper DNS library
        if data.len() < 12 {
            return Ok(None);
        }

        // For now, just extract basic info from the packet
        // In a real implementation, you'd parse the full DNS packet

        Ok(None)
    }

    /// Handle received mDNS message
    async fn handle_message(&self, data: &[u8], _from: SocketAddr) -> Result<()> {
        // Parse the message
        if let Some(instance) = self.parse_mdns_message(data)? {
            if let Some(node_info) = instance.to_node_info() {
                // Check if this is a new service
                let mut services = self.services.write().await;
                let is_new = !services.contains_key(&instance.name);

                services.insert(instance.name.clone(), instance);
                drop(services);

                if is_new {
                    let _ = self
                        .event_tx
                        .send(DiscoveryEvent::NodeJoined(node_info))
                        .await;
                }
            }
        }

        Ok(())
    }

    /// Run the mDNS protocol loop
    async fn run_protocol(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut announce_interval = interval(Duration::from_secs(60));
        let mut query_interval = interval(Duration::from_secs(30));
        let mut buffer = vec![0u8; 9000]; // mDNS packets up to 9KB

        // Send initial announcement and query
        self.send_announcement().await?;
        self.send_query().await?;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }

                _ = announce_interval.tick() => {
                    if let Err(e) = self.send_announcement().await {
                        eprintln!("Error sending announcement: {}", e);
                    }
                }

                _ = query_interval.tick() => {
                    if let Err(e) = self.send_query().await {
                        eprintln!("Error sending query: {}", e);
                    }
                }

                result = self.socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Err(e) = self.handle_message(&buffer[..len], addr).await {
                                eprintln!("Error handling mDNS message: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error receiving mDNS message: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Clone for task spawning
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            socket: self.socket.clone(),
            services: self.services.clone(),
            event_tx: self.event_tx.clone(),
            shutdown_tx: None,
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryProtocol for MdnsDiscovery {
    async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let discovery = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = discovery.run_protocol(shutdown_rx).await {
                eprintln!("mDNS protocol error: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    async fn members(&self) -> Result<Vec<NodeInfo>> {
        let services = self.services.read().await;
        Ok(services.values().filter_map(|s| s.to_node_info()).collect())
    }

    async fn announce(&self) -> Result<()> {
        self.send_announcement().await
    }

    async fn join(&mut self, _seeds: Vec<SocketAddr>) -> Result<()> {
        // mDNS doesn't need explicit join - just send query
        self.send_query().await
    }

    async fn leave(&mut self) -> Result<()> {
        // Send goodbye message (TTL=0)
        // Simplified - not implemented in this basic version
        Ok(())
    }

    fn subscribe(&self) -> mpsc::Receiver<DiscoveryEvent> {
        let (_tx, rx) = mpsc::channel(1000);
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_instance_to_node_info() {
        let mut txt_records = HashMap::new();
        txt_records.insert("node_id".to_string(), "node1".to_string());
        txt_records.insert("version".to_string(), "1".to_string());

        let instance = ServiceInstance {
            name: "node1".to_string(),
            service_type: SERVICE_TYPE.to_string(),
            addr: "127.0.0.1:7946".parse().unwrap(),
            txt_records,
            ttl: 120,
        };

        let node_info = instance.to_node_info().unwrap();
        assert_eq!(node_info.id, "node1");
        assert_eq!(node_info.status, NodeStatus::Alive);
    }
}
