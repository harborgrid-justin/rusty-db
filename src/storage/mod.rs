pub mod disk;
pub mod buffer;
pub mod page;
pub mod partitioning;
pub mod json;
pub mod tiered;
pub mod lsm;
pub mod columnar;

pub use disk::{DiskManager, IoPriority, DirectIoConfig};
pub use buffer::{BufferPoolManager, BufferPoolStats};
pub use page::{Page, PageId, SlottedPage, PageSplitter, PageMerger};
pub use partitioning::{PartitionManager, PartitionStrategy, PartitionPruner};
pub use json::{JsonData, JsonPath, JsonOperators};
pub use tiered::{TieredStorageManager, StorageTier, TierStats};
pub use lsm::{LsmTree, LsmStats};
pub use columnar::{ColumnarTable, ColumnDef, ColumnType, ColumnValue};

use crate::Result;

/// Storage engine that manages data persistence
pub struct StorageEngine {
    disk_manager: DiskManager,
    buffer_pool: BufferPoolManager,
}

impl StorageEngine {
    pub fn new(data_dir: &str, page_size: usize, pool_size: usize) -> Result<Self> {
        let disk_manager = DiskManager::new(data_dir, page_size)?;
        let buffer_pool = BufferPoolManager::new(pool_size, disk_manager.clone());
        
        Ok(Self {
            disk_manager,
            buffer_pool,
        })
    }
    
    pub fn get_page(&mut self, page_id: PageId) -> Result<Page> {
        self.buffer_pool.fetch_page(page_id)
    }
    
    pub fn new_page(&mut self) -> Result<Page> {
        self.buffer_pool.new_page()
    }
    
    pub fn flush_page(&mut self, page_id: PageId) -> Result<()> {
        self.buffer_pool.flush_page(page_id)
    }
    
    pub fn flush_all(&mut self) -> Result<()> {
        self.buffer_pool.flush_all()
    }
}
