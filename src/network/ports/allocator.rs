// # Port Allocator
//
// Dynamic port allocation with multiple strategies for distributed database services.
//
// ## Allocation Strategies
//
// - **Sequential**: Allocate ports in sequential order
// - **Random**: Allocate random ports from the available range
// - **HashBased**: Use consistent hashing based on node ID for predictable allocation

use crate::common::NodeId;
use crate::error::{DbError, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Port allocation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// Sequential allocation starting from range_start
    Sequential,

    /// Random allocation within range
    Random,

    /// Hash-based allocation for consistent node assignment
    HashBased(NodeId),
}

/// Port allocator manages dynamic port allocation
pub struct PortAllocator {
    /// Start of port range (inclusive)
    range_start: u16,

    /// End of port range (inclusive)
    range_end: u16,

    /// Currently allocated ports
    allocated: HashSet<u16>,

    /// Allocation strategy
    strategy: AllocationStrategy,

    /// Next sequential port (for sequential strategy)
    next_sequential: u16,
}

impl PortAllocator {
    /// Create a new port allocator
    ///
    /// # Arguments
    ///
    /// * `range_start` - Start of allocatable port range (inclusive)
    /// * `range_end` - End of allocatable port range (inclusive)
    /// * `strategy` - Allocation strategy to use
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rusty_db::network::ports::{PortAllocator, AllocationStrategy};
    ///
    /// let allocator = PortAllocator::new(6000, 7000, AllocationStrategy::Random);
    /// ```
    pub fn new(range_start: u16, range_end: u16, strategy: AllocationStrategy) -> Self {
        assert!(range_start <= range_end, "Invalid port range");

        Self {
            range_start,
            range_end,
            allocated: HashSet::new(),
            strategy,
            next_sequential: range_start,
        }
    }

    /// Allocate a port using the configured strategy
    ///
    /// Returns `None` if no ports are available in the range
    pub fn allocate(&mut self) -> Option<u16> {
        match self.strategy.clone() {
            AllocationStrategy::Sequential => self.allocate_sequential(),
            AllocationStrategy::Random => self.allocate_random(),
            AllocationStrategy::HashBased(node_id) => self.allocate_hash_based(&node_id),
        }
    }

    /// Allocate a specific port if available
    ///
    /// # Arguments
    ///
    /// * `port` - The specific port to allocate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the port was allocated, `Err` if already in use or out of range
    pub fn allocate_specific(&mut self, port: u16) -> Result<()> {
        if port < self.range_start || port > self.range_end {
            return Err(DbError::InvalidInput(format!(
                "Port {} outside allowed range {}-{}",
                port, self.range_start, self.range_end
            )));
        }

        if self.allocated.contains(&port) {
            return Err(DbError::AlreadyExists(format!(
                "Port {} already allocated",
                port
            )));
        }

        self.allocated.insert(port);
        Ok(())
    }

    /// Release a previously allocated port
    ///
    /// # Arguments
    ///
    /// * `port` - The port to release
    pub fn release(&mut self, port: u16) {
        self.allocated.remove(&port);
    }

    /// Check if a port is currently allocated
    pub fn is_allocated(&self, port: u16) -> bool {
        self.allocated.contains(&port)
    }

    /// Get the number of allocated ports
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    /// Get the number of available ports
    pub fn available_count(&self) -> usize {
        let total = (self.range_end - self.range_start + 1) as usize;
        total - self.allocated.len()
    }

    /// Get all allocated ports
    pub fn get_allocated_ports(&self) -> Vec<u16> {
        let mut ports: Vec<u16> = self.allocated.iter().copied().collect();
        ports.sort_unstable();
        ports
    }

    /// Change the allocation strategy
    pub fn set_strategy(&mut self, strategy: AllocationStrategy) {
        self.strategy = strategy;
    }

    /// Reset the allocator, releasing all ports
    pub fn reset(&mut self) {
        self.allocated.clear();
        self.next_sequential = self.range_start;
    }

    /// Sequential allocation implementation
    fn allocate_sequential(&mut self) -> Option<u16> {
        let start = self.next_sequential;
        let mut current = start;

        loop {
            if !self.allocated.contains(&current) {
                self.allocated.insert(current);
                self.next_sequential = if current == self.range_end {
                    self.range_start
                } else {
                    current + 1
                };
                return Some(current);
            }

            current = if current == self.range_end {
                self.range_start
            } else {
                current + 1
            };

            // We've wrapped around and checked all ports
            if current == start {
                return None;
            }
        }
    }

    /// Random allocation implementation
    fn allocate_random(&mut self) -> Option<u16> {
        let available = self.available_count();
        if available == 0 {
            return None;
        }

        let mut rng = rand::rng();
        let range_size = (self.range_end - self.range_start + 1) as usize;

        // Try random selection up to range_size times
        for _ in 0..range_size {
            let offset = rng.random_range(0..=self.range_end - self.range_start);
            let port = self.range_start + offset;

            if !self.allocated.contains(&port) {
                self.allocated.insert(port);
                return Some(port);
            }
        }

        // Fallback to sequential search if random failed
        for port in self.range_start..=self.range_end {
            if !self.allocated.contains(&port) {
                self.allocated.insert(port);
                return Some(port);
            }
        }

        None
    }

    /// Hash-based allocation implementation
    fn allocate_hash_based(&mut self, node_id: &NodeId) -> Option<u16> {
        // Use hash of node_id to determine preferred port
        let mut hasher = DefaultHasher::new();
        node_id.hash(&mut hasher);
        let hash = hasher.finish();

        let range_size = (self.range_end - self.range_start + 1) as u64;
        let preferred_offset = (hash % range_size) as u16;
        let preferred_port = self.range_start + preferred_offset;

        // Try preferred port first
        if !self.allocated.contains(&preferred_port) {
            self.allocated.insert(preferred_port);
            return Some(preferred_port);
        }

        // Search forward from preferred port
        for offset in 1..range_size {
            let port = self.range_start + ((preferred_offset + offset as u16) % range_size as u16);
            if !self.allocated.contains(&port) {
                self.allocated.insert(port);
                return Some(port);
            }
        }

        None
    }

    /// Get port range information
    pub fn get_range(&self) -> (u16, u16) {
        (self.range_start, self.range_end)
    }

    /// Check if the allocator is at capacity
    pub fn is_full(&self) -> bool {
        self.available_count() == 0
    }

    /// Get utilization percentage (0-100)
    pub fn utilization_percentage(&self) -> f64 {
        let total = (self.range_end - self.range_start + 1) as f64;
        (self.allocated.len() as f64 / total) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_allocation() {
        let mut allocator = PortAllocator::new(6000, 6010, AllocationStrategy::Sequential);

        // Allocate first port
        let port1 = allocator.allocate().unwrap();
        assert_eq!(port1, 6000);

        // Allocate second port
        let port2 = allocator.allocate().unwrap();
        assert_eq!(port2, 6001);

        // Release first port
        allocator.release(port1);

        // Allocate again
        let port3 = allocator.allocate().unwrap();
        assert_eq!(port3, 6002);
    }

    #[test]
    fn test_random_allocation() {
        let mut allocator = PortAllocator::new(6000, 6100, AllocationStrategy::Random);

        let port1 = allocator.allocate().unwrap();
        let port2 = allocator.allocate().unwrap();

        assert!(port1 >= 6000 && port1 <= 6100);
        assert!(port2 >= 6000 && port2 <= 6100);
        assert_ne!(port1, port2);
    }

    #[test]
    fn test_hash_based_allocation() {
        let node_id = "node-1".to_string();
        let mut allocator =
            PortAllocator::new(6000, 6100, AllocationStrategy::HashBased(node_id.clone()));

        let port1 = allocator.allocate().unwrap();
        let port2 = allocator.allocate().unwrap();

        assert!(port1 >= 6000 && port1 <= 6100);
        assert!(port2 >= 6000 && port2 <= 6100);
        assert_ne!(port1, port2);
    }

    #[test]
    fn test_allocate_specific() {
        let mut allocator = PortAllocator::new(6000, 6100, AllocationStrategy::Sequential);

        // Allocate specific port
        allocator.allocate_specific(6050).unwrap();
        assert!(allocator.is_allocated(6050));

        // Try to allocate same port again
        assert!(allocator.allocate_specific(6050).is_err());

        // Try to allocate out of range
        assert!(allocator.allocate_specific(5000).is_err());
        assert!(allocator.allocate_specific(7000).is_err());
    }

    #[test]
    fn test_port_exhaustion() {
        let mut allocator = PortAllocator::new(6000, 6002, AllocationStrategy::Sequential);

        assert_eq!(allocator.available_count(), 3);

        let _port1 = allocator.allocate().unwrap();
        let _port2 = allocator.allocate().unwrap();
        let _port3 = allocator.allocate().unwrap();

        assert_eq!(allocator.available_count(), 0);
        assert!(allocator.is_full());

        // Should fail to allocate when full
        assert!(allocator.allocate().is_none());
    }

    #[test]
    fn test_utilization() {
        let mut allocator = PortAllocator::new(6000, 6099, AllocationStrategy::Sequential);

        assert_eq!(allocator.utilization_percentage(), 0.0);

        for _ in 0..50 {
            allocator.allocate();
        }

        assert_eq!(allocator.utilization_percentage(), 50.0);
    }

    #[test]
    fn test_reset() {
        let mut allocator = PortAllocator::new(6000, 6100, AllocationStrategy::Sequential);

        allocator.allocate();
        allocator.allocate();
        allocator.allocate();

        assert_eq!(allocator.allocated_count(), 3);

        allocator.reset();

        assert_eq!(allocator.allocated_count(), 0);
        assert_eq!(allocator.available_count(), 101);
    }
}
