// Routing table for cluster communication
//
// This module implements the routing table that maps nodes to addresses,
// shards to nodes, and provides datacenter-aware routing.

use crate::error::Result;
use crate::networking::types::{NodeAddress, NodeId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Shard identifier
pub type ShardId = u32;

/// Datacenter/region identifier
pub type DatacenterId = String;

/// Version number for routing table updates
pub type RouteVersion = u64;

/// Routing table for cluster nodes
pub struct RoutingTable {
    /// Inner state protected by RwLock
    inner: Arc<RwLock<RoutingTableInner>>,
}

struct RoutingTableInner {
    /// Map from node ID to node address
    node_addresses: HashMap<NodeId, NodeAddress>,

    /// Map from node ID to datacenter ID
    node_datacenters: HashMap<NodeId, DatacenterId>,

    /// Map from shard ID to primary node
    shard_primary: HashMap<ShardId, NodeId>,

    /// Map from shard ID to replica nodes
    shard_replicas: HashMap<ShardId, Vec<NodeId>>,

    /// Map from datacenter to nodes in that datacenter
    datacenter_nodes: HashMap<DatacenterId, Vec<NodeId>>,

    /// Current version of the routing table
    version: RouteVersion,
}

impl RoutingTable {
    /// Create a new empty routing table
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoutingTableInner {
                node_addresses: HashMap::new(),
                node_datacenters: HashMap::new(),
                shard_primary: HashMap::new(),
                shard_replicas: HashMap::new(),
                datacenter_nodes: HashMap::new(),
                version: 0,
            })),
        }
    }

    /// Add a node to the routing table
    pub fn add_node(
        &self,
        node_id: NodeId,
        address: NodeAddress,
        datacenter: Option<DatacenterId>,
    ) {
        let mut inner = self.inner.write();

        inner.node_addresses.insert(node_id.clone(), address);

        if let Some(dc) = datacenter {
            inner.node_datacenters.insert(node_id.clone(), dc.clone());
            inner
                .datacenter_nodes
                .entry(dc)
                .or_insert_with(Vec::new)
                .push(node_id);
        }

        inner.version += 1;
    }

    /// Remove a node from the routing table
    pub fn remove_node(&self, node_id: &NodeId) -> Result<()> {
        let mut inner = self.inner.write();

        inner.node_addresses.remove(node_id);

        if let Some(dc) = inner.node_datacenters.remove(node_id) {
            if let Some(nodes) = inner.datacenter_nodes.get_mut(&dc) {
                nodes.retain(|n| n != node_id);
            }
        }

        // Remove from shard mappings
        inner.shard_primary.retain(|_, primary| primary != node_id);

        for replicas in inner.shard_replicas.values_mut() {
            replicas.retain(|n| n != node_id);
        }

        inner.version += 1;

        Ok(())
    }

    /// Get the address for a node
    pub fn get_node_address(&self, node_id: &NodeId) -> Option<NodeAddress> {
        let inner = self.inner.read();
        inner.node_addresses.get(node_id).cloned()
    }

    /// Get all known nodes
    pub fn get_all_nodes(&self) -> Vec<NodeId> {
        let inner = self.inner.read();
        inner.node_addresses.keys().cloned().collect()
    }

    /// Set the primary node for a shard
    pub fn set_shard_primary(&self, shard_id: ShardId, node_id: NodeId) {
        let mut inner = self.inner.write();
        inner.shard_primary.insert(shard_id, node_id);
        inner.version += 1;
    }

    /// Get the primary node for a shard
    pub fn get_shard_primary(&self, shard_id: ShardId) -> Option<NodeId> {
        let inner = self.inner.read();
        inner.shard_primary.get(&shard_id).cloned()
    }

    /// Set the replica nodes for a shard
    pub fn set_shard_replicas(&self, shard_id: ShardId, replicas: Vec<NodeId>) {
        let mut inner = self.inner.write();
        inner.shard_replicas.insert(shard_id, replicas);
        inner.version += 1;
    }

    /// Get the replica nodes for a shard
    pub fn get_shard_replicas(&self, shard_id: ShardId) -> Vec<NodeId> {
        let inner = self.inner.read();
        inner
            .shard_replicas
            .get(&shard_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all nodes for a shard (primary + replicas)
    pub fn get_shard_nodes(&self, shard_id: ShardId) -> Vec<NodeId> {
        let inner = self.inner.read();
        let mut nodes = Vec::new();

        if let Some(primary) = inner.shard_primary.get(&shard_id) {
            nodes.push(primary.clone());
        }

        if let Some(replicas) = inner.shard_replicas.get(&shard_id) {
            nodes.extend(replicas.iter().cloned());
        }

        nodes
    }

    /// Get nodes in a specific datacenter
    pub fn get_datacenter_nodes(&self, datacenter: &DatacenterId) -> Vec<NodeId> {
        let inner = self.inner.read();
        inner
            .datacenter_nodes
            .get(datacenter)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the datacenter for a node
    pub fn get_node_datacenter(&self, node_id: &NodeId) -> Option<DatacenterId> {
        let inner = self.inner.read();
        inner.node_datacenters.get(node_id).cloned()
    }

    /// Find the closest node to a given datacenter
    pub fn find_closest_node(
        &self,
        preferred_datacenter: &DatacenterId,
        exclude: &[NodeId],
    ) -> Option<NodeId> {
        let inner = self.inner.read();

        // First try nodes in the preferred datacenter
        if let Some(nodes) = inner.datacenter_nodes.get(preferred_datacenter) {
            for node in nodes {
                if !exclude.contains(node) {
                    return Some(node.clone());
                }
            }
        }

        // Fall back to any available node
        for node in inner.node_addresses.keys() {
            if !exclude.contains(node) {
                return Some(node.clone());
            }
        }

        None
    }

    /// Get the current version of the routing table
    pub fn get_version(&self) -> RouteVersion {
        let inner = self.inner.read();
        inner.version
    }

    /// Get the number of nodes in the routing table
    pub fn node_count(&self) -> usize {
        let inner = self.inner.read();
        inner.node_addresses.len()
    }

    /// Check if a node exists in the routing table
    pub fn has_node(&self, node_id: &NodeId) -> bool {
        let inner = self.inner.read();
        inner.node_addresses.contains_key(node_id)
    }

    /// Clear all entries from the routing table
    pub fn clear(&self) {
        let mut inner = self.inner.write();
        inner.node_addresses.clear();
        inner.node_datacenters.clear();
        inner.shard_primary.clear();
        inner.shard_replicas.clear();
        inner.datacenter_nodes.clear();
        inner.version += 1;
    }

    /// Export the routing table as a snapshot
    pub fn export(&self) -> RoutingTableSnapshot {
        let inner = self.inner.read();
        RoutingTableSnapshot {
            node_addresses: inner.node_addresses.clone(),
            node_datacenters: inner.node_datacenters.clone(),
            shard_primary: inner.shard_primary.clone(),
            shard_replicas: inner.shard_replicas.clone(),
            datacenter_nodes: inner.datacenter_nodes.clone(),
            version: inner.version,
        }
    }

    /// Import a routing table snapshot
    pub fn import(&self, snapshot: RoutingTableSnapshot) {
        let mut inner = self.inner.write();
        *inner = RoutingTableInner {
            node_addresses: snapshot.node_addresses,
            node_datacenters: snapshot.node_datacenters,
            shard_primary: snapshot.shard_primary,
            shard_replicas: snapshot.shard_replicas,
            datacenter_nodes: snapshot.datacenter_nodes,
            version: snapshot.version,
        };
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RoutingTable {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Snapshot of the routing table for serialization
#[derive(Debug, Clone)]
pub struct RoutingTableSnapshot {
    pub node_addresses: HashMap<NodeId, NodeAddress>,
    pub node_datacenters: HashMap<NodeId, DatacenterId>,
    pub shard_primary: HashMap<ShardId, NodeId>,
    pub shard_replicas: HashMap<ShardId, Vec<NodeId>>,
    pub datacenter_nodes: HashMap<DatacenterId, Vec<NodeId>>,
    pub version: RouteVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_node() {
        let table = RoutingTable::new();
        let node_id = NodeId::new("node1");
        let address = NodeAddress::new("localhost", 8000);

        table.add_node(node_id.clone(), address.clone(), None);

        assert_eq!(table.get_node_address(&node_id), Some(address));
        assert_eq!(table.node_count(), 1);
    }

    #[test]
    fn test_remove_node() {
        let table = RoutingTable::new();
        let node_id = NodeId::new("node1");
        let address = NodeAddress::new("localhost", 8000);

        table.add_node(node_id.clone(), address, None);
        assert_eq!(table.node_count(), 1);

        table.remove_node(&node_id).unwrap();
        assert_eq!(table.node_count(), 0);
    }

    #[test]
    fn test_shard_routing() {
        let table = RoutingTable::new();
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");

        table.set_shard_primary(0, node1.clone());
        table.set_shard_replicas(0, vec![node2.clone()]);

        assert_eq!(table.get_shard_primary(0), Some(node1.clone()));
        assert_eq!(table.get_shard_replicas(0), vec![node2]);
    }

    #[test]
    fn test_datacenter_routing() {
        let table = RoutingTable::new();
        let node1 = NodeId::new("node1");
        let address1 = NodeAddress::new("localhost", 8000);
        let dc = "dc1".to_string();

        table.add_node(node1.clone(), address1, Some(dc.clone()));

        assert_eq!(table.get_datacenter_nodes(&dc), vec![node1.clone()]);
        assert_eq!(table.get_node_datacenter(&node1), Some(dc));
    }
}
