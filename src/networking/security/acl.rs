//! Network Access Control Lists (ACLs)
//!
//! This module provides IP-based ACLs, node-based ACLs, action-based permissions,
//! and audit logging.

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use std::path::Path;
use std::time::SystemTime;

/// IP network range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpNetwork {
    /// Base IP address
    pub addr: IpAddr,

    /// Prefix length (CIDR notation)
    pub prefix_len: u8,
}

impl IpNetwork {
    /// Create a new IP network
    pub fn new(addr: IpAddr, prefix_len: u8) -> Result<Self> {
        // Validate prefix length
        let max_prefix = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };

        if prefix_len > max_prefix {
            return Err(DbError::InvalidInput(format!(
                "Invalid prefix length: {} > {}",
                prefix_len, max_prefix
            )));
        }

        Ok(Self { addr, prefix_len })
    }

    /// Parse from CIDR notation (e.g., "192.168.1.0/24")
    pub fn from_cidr(cidr: &str) -> Result<Self> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err(DbError::ParseError(format!("Invalid CIDR notation: {}", cidr)));
        }

        let addr: IpAddr = parts[0]
            .parse()
            .map_err(|e| DbError::ParseError(format!("Invalid IP address: {}", e)))?;

        let prefix_len: u8 = parts[1]
            .parse()
            .map_err(|e| DbError::ParseError(format!("Invalid prefix length: {}", e)))?;

        Self::new(addr, prefix_len)
    }

    /// Convert to CIDR notation
    pub fn to_cidr(&self) -> String {
        format!("{}/{}", self.addr, self.prefix_len)
    }

    /// Check if IP address is in this network
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(addr)) => {
                let net_u32 = u32::from(net);
                let addr_u32 = u32::from(addr);
                let mask = if self.prefix_len == 0 {
                    0
                } else {
                    !0u32 << (32 - self.prefix_len)
                };
                (net_u32 & mask) == (addr_u32 & mask)
            }
            (IpAddr::V6(net), IpAddr::V6(addr)) => {
                let net_u128 = u128::from(net);
                let addr_u128 = u128::from(addr);
                let mask = if self.prefix_len == 0 {
                    0
                } else {
                    !0u128 << (128 - self.prefix_len)
                };
                (net_u128 & mask) == (addr_u128 & mask)
            }
            _ => false,
        }
    }
}

/// Port range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortRange {
    /// Start port (inclusive)
    pub start: u16,

    /// End port (inclusive)
    pub end: u16,
}

impl PortRange {
    /// Create a new port range
    pub fn new(start: u16, end: u16) -> Result<Self> {
        if start > end {
            return Err(DbError::InvalidInput(format!(
                "Invalid port range: {} > {}",
                start, end
            )));
        }
        Ok(Self { start, end })
    }

    /// Create a single port range
    pub fn single(port: u16) -> Self {
        Self {
            start: port,
            end: port,
        }
    }

    /// Create any port range (0-65535)
    pub fn any() -> Self {
        Self {
            start: 0,
            end: 65535,
        }
    }

    /// Check if port is in range
    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }

    /// Parse from string (e.g., "8080" or "8000-9000")
    pub fn from_string(s: &str) -> Result<Self> {
        if s == "*" || s == "any" {
            return Ok(Self::any());
        }

        if let Some(pos) = s.find('-') {
            let start: u16 = s[..pos]
                .parse()
                .map_err(|e| DbError::ParseError(format!("Invalid start port: {}", e)))?;
            let end: u16 = s[pos + 1..]
                .parse()
                .map_err(|e| DbError::ParseError(format!("Invalid end port: {}", e)))?;
            Self::new(start, end)
        } else {
            let port: u16 = s
                .parse()
                .map_err(|e| DbError::ParseError(format!("Invalid port: {}", e)))?;
            Ok(Self::single(port))
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        if self.start == 0 && self.end == 65535 {
            "*".to_string()
        } else if self.start == self.end {
            self.start.to_string()
        } else {
            format!("{}-{}", self.start, self.end)
        }
    }
}

/// ACL action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Allow the connection
    Allow,
    /// Deny the connection
    Deny,
    /// Reject the connection (send RST)
    Reject,
}

impl Action {
    /// Parse action from string
    pub fn from_string(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "allow" | "accept" => Ok(Action::Allow),
            "deny" | "drop" => Ok(Action::Deny),
            "reject" => Ok(Action::Reject),
            _ => Err(DbError::ParseError(format!("Invalid action: {}", s))),
        }
    }

    /// Convert action to string
    pub fn to_string(&self) -> &'static str {
        match self {
            Action::Allow => "allow",
            Action::Deny => "deny",
            Action::Reject => "reject",
        }
    }
}

/// ACL rule
#[derive(Debug, Clone)]
pub struct AclRule {
    /// Rule ID
    pub id: String,

    /// Source network
    pub source: IpNetwork,

    /// Destination network
    pub destination: IpNetwork,

    /// Port range
    pub ports: PortRange,

    /// Action
    pub action: Action,

    /// Priority (lower number = higher priority)
    pub priority: u32,

    /// Description
    pub description: String,

    /// Enabled
    pub enabled: bool,

    /// Hit count
    pub hit_count: u64,

    /// Last hit timestamp
    pub last_hit: Option<SystemTime>,
}

impl AclRule {
    /// Create a new ACL rule
    pub fn new(
        id: String,
        source: IpNetwork,
        destination: IpNetwork,
        ports: PortRange,
        action: Action,
    ) -> Self {
        Self {
            id,
            source,
            destination,
            ports,
            action,
            priority: 1000,
            description: String::new(),
            enabled: true,
            hit_count: 0,
            last_hit: None,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Check if rule matches
    pub fn matches(&self, source: IpAddr, destination: IpAddr, port: u16) -> bool {
        if !self.enabled {
            return false;
        }

        self.source.contains(source)
            && self.destination.contains(destination)
            && self.ports.contains(port)
    }

    /// Record hit
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
        self.last_hit = Some(SystemTime::now());
    }

    /// Parse from line (format: "id|source|dest|ports|action|priority|description")
    pub fn from_line(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            return Err(DbError::ParseError(format!("Invalid ACL rule: {}", line)));
        }

        let id = parts[0].to_string();
        let source = IpNetwork::from_cidr(parts[1])?;
        let destination = IpNetwork::from_cidr(parts[2])?;
        let ports = PortRange::from_string(parts[3])?;
        let action = Action::from_string(parts[4])?;

        let mut rule = Self::new(id, source, destination, ports, action);

        if parts.len() > 5 {
            rule.priority = parts[5]
                .parse()
                .map_err(|e| DbError::ParseError(format!("Invalid priority: {}", e)))?;
        }

        if parts.len() > 6 {
            rule.description = parts[6].to_string();
        }

        Ok(rule)
    }

    /// Convert to line
    pub fn to_line(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.id,
            self.source.to_cidr(),
            self.destination.to_cidr(),
            self.ports.to_string(),
            self.action.to_string(),
            self.priority,
            self.description
        )
    }
}

/// Network ACL manager
pub struct NetworkAcl {
    /// ACL rules
    rules: Vec<AclRule>,

    /// Default action
    default_action: Action,

    /// Audit log
    audit_log: Option<File>,

    /// Node-based rules (node_id -> rules)
    node_rules: HashMap<String, Vec<AclRule>>,
}

impl NetworkAcl {
    /// Create a new network ACL
    pub fn new(default_action: Action) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
            audit_log: None,
            node_rules: HashMap::new(),
        }
    }

    /// Enable audit logging
    pub fn with_audit_log(mut self, path: &Path) -> Result<Self> {
        let file = File::create(path)
            .map_err(|e| DbError::Configuration(format!("Failed to create audit log: {}", e)))?;
        self.audit_log = Some(file);
        Ok(self)
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: AclRule) {
        self.rules.push(rule);
        self.sort_rules();
    }

    /// Remove rule
    pub fn remove_rule(&mut self, id: &str) -> bool {
        if let Some(pos) = self.rules.iter().position(|r| r.id == id) {
            self.rules.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get rule by ID
    pub fn get_rule(&self, id: &str) -> Option<&AclRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Get mutable rule by ID
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut AclRule> {
        self.rules.iter_mut().find(|r| r.id == id)
    }

    /// Sort rules by priority
    fn sort_rules(&mut self) {
        self.rules.sort_by_key(|r| r.priority);
    }

    /// Check if connection is allowed
    pub fn is_allowed(&mut self, source: IpAddr) -> Result<bool> {
        // For simple IP check, use 0.0.0.0 as destination and port 0
        let destination = "0.0.0.0".parse::<IpAddr>().map_err(|e| {
            DbError::Internal(format!("Failed to parse default IP: {}", e))
        })?;

        self.check(source, destination, 0)
    }

    /// Check connection against ACL rules
    pub fn check(&mut self, source: IpAddr, destination: IpAddr, port: u16) -> Result<bool> {
        // Find matching rule and record hit
        let mut matched_rule: Option<(String, Action)> = None;

        for rule in &mut self.rules {
            if rule.matches(source, destination, port) {
                rule.record_hit();
                matched_rule = Some((rule.id.clone(), rule.action));
                break;
            }
        }

        // Log decision after releasing borrow on self.rules
        if let Some((rule_id, action)) = matched_rule {
            self.log_decision(&rule_id, source, destination, port, action)?;
            Ok(action == Action::Allow)
        } else {
            // No matching rule, use default action
            self.log_decision("default", source, destination, port, self.default_action)?;
            Ok(self.default_action == Action::Allow)
        }
    }

    /// Log ACL decision
    fn log_decision(
        &mut self,
        rule_id: &str,
        source: IpAddr,
        destination: IpAddr,
        port: u16,
        action: Action,
    ) -> Result<()> {
        if let Some(ref mut log) = self.audit_log {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| DbError::Internal(format!("Time error: {}", e)))?
                .as_secs();

            writeln!(
                log,
                "{}|{}|{}|{}|{}|{}",
                timestamp,
                rule_id,
                source,
                destination,
                port,
                action.to_string()
            )
            .map_err(|e| DbError::Internal(format!("Failed to write audit log: {}", e)))?;

            log.flush()
                .map_err(|e| DbError::Internal(format!("Failed to flush audit log: {}", e)))?;
        }

        Ok(())
    }

    /// Load rules from file
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| DbError::Configuration(format!("Failed to open ACL file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut acl = NetworkAcl::new(Action::Deny);

        for line in reader.lines() {
            let line = line
                .map_err(|e| DbError::Configuration(format!("Failed to read line: {}", e)))?;

            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for default action
            if line.starts_with("default:") {
                let action_str = line[8..].trim();
                acl.default_action = Action::from_string(action_str)?;
                continue;
            }

            let rule = AclRule::from_line(line)?;
            acl.add_rule(rule);
        }

        Ok(acl)
    }

    /// Save rules to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)
            .map_err(|e| DbError::Configuration(format!("Failed to create ACL file: {}", e)))?;

        writeln!(file, "# Network ACL Rules")
            .map_err(|e| DbError::Configuration(format!("Failed to write header: {}", e)))?;

        writeln!(file, "default:{}", self.default_action.to_string())
            .map_err(|e| DbError::Configuration(format!("Failed to write default: {}", e)))?;

        writeln!(file).map_err(|e| DbError::Configuration(format!("Failed to write newline: {}", e)))?;

        for rule in &self.rules {
            writeln!(file, "{}", rule.to_line())
                .map_err(|e| DbError::Configuration(format!("Failed to write rule: {}", e)))?;
        }

        Ok(())
    }

    /// List all rules
    pub fn list_rules(&self) -> &[AclRule] {
        &self.rules
    }

    /// Add node-specific rule
    pub fn add_node_rule(&mut self, node_id: String, rule: AclRule) {
        self.node_rules
            .entry(node_id)
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Get node rules
    pub fn get_node_rules(&self, node_id: &str) -> Option<&[AclRule]> {
        self.node_rules.get(node_id).map(|v| v.as_slice())
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
        self.node_rules.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_network() {
        let network = IpNetwork::from_cidr("192.168.1.0/24").unwrap();
        assert_eq!(network.to_cidr(), "192.168.1.0/24");

        let ip1: IpAddr = "192.168.1.100".parse().unwrap();
        let ip2: IpAddr = "192.168.2.100".parse().unwrap();

        assert!(network.contains(ip1));
        assert!(!network.contains(ip2));
    }

    #[test]
    fn test_port_range() {
        let range = PortRange::new(8000, 9000).unwrap();
        assert!(range.contains(8080));
        assert!(!range.contains(7000));

        let single = PortRange::single(8080);
        assert!(single.contains(8080));
        assert!(!single.contains(8081));
    }

    #[test]
    fn test_acl_rule() {
        let source = IpNetwork::from_cidr("192.168.1.0/24").unwrap();
        let dest = IpNetwork::from_cidr("0.0.0.0/0").unwrap();
        let ports = PortRange::new(8000, 9000).unwrap();

        let rule = AclRule::new(
            "rule1".to_string(),
            source,
            dest,
            ports,
            Action::Allow,
        );

        let src_ip: IpAddr = "192.168.1.100".parse().unwrap();
        let dst_ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(rule.matches(src_ip, dst_ip, 8080));
        assert!(!rule.matches(src_ip, dst_ip, 7000));
    }

    #[test]
    fn test_network_acl() {
        let mut acl = NetworkAcl::new(Action::Deny);

        let source = IpNetwork::from_cidr("192.168.1.0/24").unwrap();
        let dest = IpNetwork::from_cidr("0.0.0.0/0").unwrap();
        let ports = PortRange::any();

        let rule = AclRule::new(
            "allow-192".to_string(),
            source,
            dest,
            ports,
            Action::Allow,
        );

        acl.add_rule(rule);

        let src_ip: IpAddr = "192.168.1.100".parse().unwrap();
        let dst_ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(acl.check(src_ip, dst_ip, 8080).unwrap());

        let blocked_ip: IpAddr = "10.0.0.100".parse().unwrap();
        assert!(!acl.check(blocked_ip, dst_ip, 8080).unwrap());
    }
}
