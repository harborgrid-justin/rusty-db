use serde::{Deserialize, Serialize};

pub type PageId = u32;

/// A page represents a fixed-size block of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub data: Vec<u8>,
    pub is_dirty: bool,
    pub pin_count: usize,
}

impl Page {
    pub fn new(id: PageId, size: usize) -> Self {
        Self {
            id,
            data: vec![0; size],
            is_dirty: false,
            pin_count: 0,
        }
    }
    
    pub fn from_bytes(id: PageId, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            is_dirty: false,
            pin_count: 0,
        }
    }
    
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
    
    pub fn reset(&mut self) {
        self.data.fill(0);
        self.is_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_creation() {
        let page = Page::new(1, 4096);
        assert_eq!(page.id, 1);
        assert_eq!(page.data.len(), 4096);
        assert!(!page.is_dirty);
    }
    
    #[test]
    fn test_mark_dirty() {
        let mut page = Page::new(1, 4096);
        page.mark_dirty();
        assert!(page.is_dirty);
    }
}
