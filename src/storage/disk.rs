use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::Result;
use crate::storage::page::{Page, PageId};
use crate::error::DbError;

/// Manages reading and writing pages to disk
#[derive(Clone)]
pub struct DiskManager {
    data_file: Arc<Mutex<File>>,
    pub page_size: usize,
    num_pages: Arc<Mutex<u32>>,
}

impl DiskManager {
    pub fn new(data_dir: &str, page_size: usize) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        
        let mut path = PathBuf::from(data_dir);
        path.push("data.db");
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        
        let metadata = file.metadata()?;
        let num_pages = (metadata.len() / page_size as u64) as u32;
        
        Ok(Self {
            data_file: Arc::new(Mutex::new(file)),
            page_size,
            num_pages: Arc::new(Mutex::new(num_pages)),
        })
    }
    
    pub fn read_page(&self, page_id: PageId) -> Result<Page> {
        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let offset = page_id as u64 * self.page_size as u64;
        
        file.seek(SeekFrom::Start(offset))?;
        
        let mut data = vec![0u8; self.page_size];
        file.read_exact(&mut data)?;
        
        Ok(Page::from_bytes(page_id, data))
    }
    
    pub fn write_page(&self, page: &Page) -> Result<()> {
        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let offset = page.id as u64 * self.page_size as u64;
        
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&page.data)?;
        file.sync_all()?;
        
        Ok(())
    }
    
    pub fn allocate_page(&self) -> Result<PageId> {
        let mut num_pages = self.num_pages.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let page_id = *num_pages;
        *num_pages += 1;
        
        // Write empty page
        let page = Page::new(page_id, self.page_size);
        self.write_page(&page)?;
        
        Ok(page_id)
    }
    
    pub fn get_num_pages(&self) -> u32 {
        *self.num_pages.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_disk_manager() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;
        
        let page_id = dm.allocate_page()?;
        assert_eq!(page_id, 0);
        
        let mut page = dm.read_page(page_id)?;
        page.data[0] = 42;
        dm.write_page(&page)?;
        
        let loaded = dm.read_page(page_id)?;
        assert_eq!(loaded.data[0], 42);
        
        Ok(())
    }
}
