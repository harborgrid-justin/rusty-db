use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::storage::page::{Page, PageId};
use crate::storage::disk::DiskManager;
use crate::error::DbError;

/// LRU replacement policy for buffer pool
struct LruReplacer {
    capacity: usize,
    frames: Vec<PageId>,
}

impl LruReplacer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            frames: Vec::new(),
        }
    }
    
    fn victim(&mut self) -> Option<PageId> {
        // Remove from the end (least recently used)
        if !self.frames.is_empty() {
            Some(self.frames.pop().unwrap())
        } else {
            None
        }
    }
    
    fn pin(&mut self, page_id: PageId) {
        self.frames.retain(|&id| id != page_id);
    }
    
    fn unpin(&mut self, page_id: PageId) {
        if !self.frames.contains(&page_id) {
            self.frames.insert(0, page_id);
        }
    }
}

/// Buffer pool manager with LRU replacement
pub struct BufferPoolManager {
    pool: Arc<RwLock<Vec<Page>>>,
    page_table: Arc<RwLock<HashMap<PageId, usize>>>,
    free_list: Arc<RwLock<Vec<usize>>>,
    replacer: Arc<RwLock<LruReplacer>>,
    disk_manager: DiskManager,
}

impl BufferPoolManager {
    pub fn new(pool_size: usize, disk_manager: DiskManager) -> Self {
        let pool = vec![Page::new(0, disk_manager.page_size); pool_size];
        let free_list: Vec<usize> = (0..pool_size).collect();
        
        Self {
            pool: Arc::new(RwLock::new(pool)),
            page_table: Arc::new(RwLock::new(HashMap::new())),
            free_list: Arc::new(RwLock::new(free_list)),
            replacer: Arc::new(RwLock::new(LruReplacer::new(pool_size))),
            disk_manager,
        }
    }
    
    pub fn fetch_page(&self, page_id: PageId) -> Result<Page> {
        // Check if page is already in buffer pool (read lock only)
        {
            let page_table = self.page_table.read();
            if let Some(&frame_id) = page_table.get(&page_id) {
                let mut pool = self.pool.write();
                pool[frame_id].pin_count += 1;
                self.replacer.write().pin(page_id);
                return Ok(pool[frame_id].clone());
            }
        }
        
        // Find a frame to use
        let frame_id = self.get_frame_id()?;
        
        // Flush dirty page if needed before evicting
        {
            let pool = self.pool.read();
            if pool[frame_id].is_dirty {
                self.disk_manager.write_page(&pool[frame_id])?;
            }
        }
        
        // Load page from disk
        let page = self.disk_manager.read_page(page_id)?;
        
        // Update pool and page table
        let mut pool = self.pool.write();
        let old_page_id = pool[frame_id].id;
        pool[frame_id] = page.clone();
        pool[frame_id].pin_count = 1;
        
        let mut page_table = self.page_table.write();
        page_table.remove(&old_page_id);
        page_table.insert(page_id, frame_id);
        
        self.replacer.write().pin(page_id);
        
        Ok(page)
    }
    
    pub fn new_page(&self) -> Result<Page> {
        let page_id = self.disk_manager.allocate_page()?;
        let frame_id = self.get_frame_id()?;
        
        let mut pool = self.pool.write();
        pool[frame_id] = Page::new(page_id, self.disk_manager.page_size);
        pool[frame_id].pin_count = 1;
        
        self.page_table.write().insert(page_id, frame_id);
        self.replacer.write().pin(page_id);
        
        Ok(pool[frame_id].clone())
    }
    
    pub fn flush_page(&self, page_id: PageId) -> Result<()> {
        let page_table = self.page_table.read();
        
        if let Some(&frame_id) = page_table.get(&page_id) {
            let pool = self.pool.read();
            if pool[frame_id].is_dirty {
                self.disk_manager.write_page(&pool[frame_id])?;
            }
        }
        
        Ok(())
    }
    
    pub fn flush_all(&self) -> Result<()> {
        let page_table = self.page_table.read();
        let pool = self.pool.read();
        
        for &frame_id in page_table.values() {
            if pool[frame_id].is_dirty {
                self.disk_manager.write_page(&pool[frame_id])?;
            }
        }
        
        Ok(())
    }
    
    fn get_frame_id(&self) -> Result<usize> {
        // Try free list first
        if let Some(frame_id) = self.free_list.write().pop() {
            return Ok(frame_id);
        }
        
        // Use replacer to find victim
        let victim = self.replacer.write().victim()
            .ok_or_else(|| DbError::Storage("No available frames".to_string()))?;
        
        // Find the frame for the victim page
        let page_table = self.page_table.read();
        if let Some(&frame_id) = page_table.get(&victim) {
            Ok(frame_id)
        } else {
            Err(DbError::Storage("Invalid victim page".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_buffer_pool() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;
        let bp = BufferPoolManager::new(10, dm);
        
        let page = bp.new_page()?;
        assert_eq!(page.id, 0);
        
        Ok(())
    }
}
