pub mod buffer;
pub mod checksum;
pub mod columnar;
pub mod disk;
pub mod json;
pub mod lsm;
pub mod page;
pub mod partitioning;
pub mod tiered;

pub use buffer::{BufferPoolManager, BufferPoolStats};
pub use checksum::hardware_crc32c;
pub use columnar::{ColumnDef, ColumnType, ColumnValue, ColumnarTable};
pub use disk::{DirectIoConfig, DiskManager, IoPriority};
pub use json::{JsonData, JsonOperators, JsonPath};
pub use lsm::{LsmStats, LsmTree};
pub use page::{Page, PageMerger, PageSplitter, SlottedPage};
pub use partitioning::{PartitionManager, PartitionPruner, PartitionStrategy};
pub use tiered::{StorageTier, TierStats, TieredStorageManager};
// Re-export PageId from common for convenience
pub use crate::common::PageId;

use crate::error::Result;

// Storage engine that manages data persistence
pub struct StorageEngine {
    #[allow(dead_code)]
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
