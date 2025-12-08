use serde::{Deserialize, Serialize};
use std::mem::size_of;
use crc32fast::Hasher;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// Use PageId from common module for consistency
pub use crate::common::PageId;
pub type SlotId = u16;

const SLOT_SIZE: usize = size_of::<Slot>();
const PAGE_HEADER_SIZE: usize = size_of::<PageHeader>();

/// Hardware-accelerated CRC32C checksum (SSE4.2 on x86_64)
#[inline]
fn hardware_crc32c(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sse4.2") {
            return unsafe { hardware_crc32c_impl(data) };
        }
    }
    // Fallback to software CRC32
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
unsafe fn hardware_crc32c_impl(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    let mut ptr = data.as_ptr();
    let mut remaining = data.len();

    // Process 8 bytes at a time for maximum throughput
    while remaining >= 8 {
        let value = (ptr as *const u64).read_unaligned();
        crc = _mm_crc32_u64(crc as u64, value) as u32;
        ptr = ptr.add(8);
        remaining -= 8;
    }

    // Process remaining bytes
    while remaining > 0 {
        let value = *ptr;
        crc = _mm_crc32_u8(crc, value);
        ptr = ptr.add(1);
        remaining -= 1;
    }

    !crc
}

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
        let mut page = Self {
            id,
            data: vec![0; size],
            is_dirty: false,
            pin_count: 0,
        };

        // Initialize page header
        let header = PageHeader::new(size);
        page.write_header(&header);

        page
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

    fn write_header(&mut self, header: &PageHeader) {
        let bytes = bincode::serialize(header).unwrap();
        self.data[..PAGE_HEADER_SIZE].copy_from_slice(&bytes[..PAGE_HEADER_SIZE]);
    }

    fn read_header(&self) -> PageHeader {
        bincode::deserialize(&self.data[..PAGE_HEADER_SIZE]).unwrap()
    }

    /// Verify page checksum
    pub fn verify_checksum(&self) -> bool {
        let header = self.read_header();
        let computed = self.compute_checksum();
        header.checksum == computed
    }

    fn compute_checksum(&self) -> u32 {
        // Use hardware-accelerated CRC32C
        // Hash everything except the checksum field itself
        hardware_crc32c(&self.data[4..])
    }

    pub fn update_checksum(&mut self) {
        let checksum = self.compute_checksum();
        let mut header = self.read_header();
        header.checksum = checksum;
        self.write_header(&header);
    }
}

/// Page header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageHeader {
    checksum: u32,
    page_type: PageType,
    free_space_offset: u16,
    num_slots: u16,
    free_space: u16,
}

impl PageHeader {
    fn new(page_size: usize) -> Self {
        Self {
            checksum: 0,
            page_type: PageType::Slotted,
            free_space_offset: PAGE_HEADER_SIZE as u16,
            num_slots: 0,
            free_space: (page_size - PAGE_HEADER_SIZE) as u16,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
enum PageType {
    Slotted,
    Overflow,
    Index,
}

/// Slot directory entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Slot {
    offset: u16,
    length: u16,
}

impl Slot {
    fn new(offset: u16, length: u16) -> Self {
        Self { offset, length }
    }

    fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// Slotted page implementation for variable-length records
pub struct SlottedPage {
    page: Page,
}

impl SlottedPage {
    pub fn new(page_id: PageId, page_size: usize) -> Self {
        Self {
            page: Page::new(page_id, page_size),
        }
    }

    pub fn from_page(page: Page) -> Self {
        Self { page }
    }

    /// Insert a record into the slotted page
    pub fn insert_record(&mut self, data: &[u8]) -> Option<SlotId> {
        let record_size = data.len();
        let header = self.page.read_header();

        // Check if we have enough space
        let required_space = record_size + SLOT_SIZE;
        if header.free_space < required_space as u16 {
            return None;
        }

        // Find a free slot or create new one
        let slot_id = self.find_free_slot().unwrap_or(header.num_slots);

        // Calculate offset for new record (grows from end of page backwards)
        let record_offset = if slot_id == 0 {
            self.page.data.len() - record_size
        } else {
            // Find the last used record's offset
            let last_offset = self.get_last_record_offset();
            last_offset - record_size
        };

        // Write record data
        self.page.data[record_offset..record_offset + record_size]
            .copy_from_slice(data);

        // Update slot
        let slot = Slot::new(record_offset as u16, record_size as u16);
        self.write_slot(slot_id, &slot);

        // Update header
        let mut new_header = header;
        if slot_id >= new_header.num_slots {
            new_header.num_slots = slot_id + 1;
        }
        new_header.free_space -= required_space as u16;
        new_header.free_space_offset += SLOT_SIZE as u16;

        self.page.write_header(&new_header);
        self.page.mark_dirty();

        Some(slot_id)
    }

    /// Get a record from the slotted page
    pub fn get_record(&self, slot_id: SlotId) -> Option<Vec<u8>> {
        let header = self.page.read_header();
        if slot_id >= header.num_slots {
            return None;
        }

        let slot = self.read_slot(slot_id);
        if slot.is_empty() {
            return None;
        }

        let start = slot.offset as usize;
        let end = start + slot.length as usize;

        Some(self.page.data[start..end].to_vec())
    }

    /// Delete a record from the slotted page
    pub fn delete_record(&mut self, slot_id: SlotId) -> bool {
        let header = self.page.read_header();
        if slot_id >= header.num_slots {
            return false;
        }

        let slot = self.read_slot(slot_id);
        if slot.is_empty() {
            return false;
        }

        // Mark slot as empty
        let empty_slot = Slot::new(0, 0);
        self.write_slot(slot_id, &empty_slot);

        // Update free space
        let mut new_header = header;
        new_header.free_space += slot.length + SLOT_SIZE as u16;
        self.page.write_header(&new_header);

        self.page.mark_dirty();
        true
    }

    /// Update a record in place
    pub fn update_record(&mut self, slot_id: SlotId, data: &[u8]) -> bool {
        let slot = self.read_slot(slot_id);
        if slot.is_empty() {
            return false;
        }

        // If new data fits in existing slot, update in place
        if data.len() <= slot.length as usize {
            let start = slot.offset as usize;
            self.page.data[start..start + data.len()].copy_from_slice(data);

            // Update slot length if smaller
            if data.len() < slot.length as usize {
                let new_slot = Slot::new(slot.offset, data.len() as u16);
                self.write_slot(slot_id, &new_slot);
            }

            self.page.mark_dirty();
            true
        } else {
            // Delete old record and insert new one
            self.delete_record(slot_id);
            self.insert_record(data).is_some()
        }
    }

    /// Compact the page to reclaim fragmented space
    pub fn compact(&mut self) {
        let header = self.page.read_header();
        let mut records = Vec::new();

        // Collect all valid records
        for slot_id in 0..header.num_slots {
            if let Some(data) = self.get_record(slot_id) {
                records.push((slot_id, data));
            }
        }

        // Reset page
        let page_size = self.page.data.len();
        self.page = Page::new(self.page.id, page_size);

        // Reinsert records
        for (original_slot_id, data) in records {
            let new_slot_id = self.insert_record(&data);
            assert_eq!(Some(original_slot_id), new_slot_id);
        }

        self.page.mark_dirty();
    }

    /// Get free space available in the page
    pub fn free_space(&self) -> u16 {
        let header = self.page.read_header();
        header.free_space
    }

    /// Check if page needs compaction
    pub fn needs_compaction(&self) -> bool {
        let header = self.page.read_header();
        let used_slots = (0..header.num_slots)
            .filter(|&slot_id| !self.read_slot(slot_id).is_empty())
            .count();

        // Compact if more than 30% of slots are empty
        let empty_slots = header.num_slots as usize - used_slots;
        if header.num_slots > 0 {
            (empty_slots as f64 / header.num_slots as f64) > 0.3
        } else {
            false
        }
    }

    fn find_free_slot(&self) -> Option<SlotId> {
        let header = self.page.read_header();

        for slot_id in 0..header.num_slots {
            let slot = self.read_slot(slot_id);
            if slot.is_empty() {
                return Some(slot_id);
            }
        }

        None
    }

    fn read_slot(&self, slot_id: SlotId) -> Slot {
        let offset = PAGE_HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        let bytes = &self.page.data[offset..offset + SLOT_SIZE];
        bincode::deserialize(bytes).unwrap()
    }

    fn write_slot(&mut self, slot_id: SlotId, slot: &Slot) {
        let offset = PAGE_HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        let bytes = bincode::serialize(slot).unwrap();
        self.page.data[offset..offset + SLOT_SIZE]
            .copy_from_slice(&bytes[..SLOT_SIZE]);
    }

    fn get_last_record_offset(&self) -> usize {
        let header = self.page.read_header();
        let mut max_offset = self.page.data.len();

        for slot_id in 0..header.num_slots {
            let slot = self.read_slot(slot_id);
            if !slot.is_empty() && (slot.offset as usize) < max_offset {
                max_offset = slot.offset as usize;
            }
        }

        max_offset
    }

    pub fn into_page(self) -> Page {
        self.page
    }
}

/// Page splitting for when a page becomes too full
pub struct PageSplitter {
    threshold: f64,
}

impl PageSplitter {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Check if page should be split
    pub fn should_split(&self, page: &SlottedPage, page_size: usize) -> bool {
        let used_space = page_size - page.free_space() as usize;
        let utilization = used_space as f64 / page_size as f64;
        utilization > self.threshold
    }

    /// Split a page into two pages
    pub fn split(&self, page: &mut SlottedPage, new_page_id: PageId) -> SlottedPage {
        let header = page.page.read_header();
        let mut records = Vec::new();

        // Collect all valid records
        for slot_id in 0..header.num_slots {
            if let Some(data) = page.get_record(slot_id) {
                records.push(data);
            }
        }

        // Sort by size for better distribution
        records.sort_by_key(|r| r.len());

        let mid = records.len() / 2;
        let page_size = page.page.data.len();

        // Create new page with second half
        let mut new_page = SlottedPage::new(new_page_id, page_size);
        for record in records.iter().skip(mid) {
            new_page.insert_record(record);
        }

        // Reset original page with first half
        *page = SlottedPage::new(page.page.id, page_size);
        for record in records.iter().take(mid) {
            page.insert_record(record);
        }

        new_page
    }
}

/// Page merging for when two pages can be combined
pub struct PageMerger {
    threshold: f64,
}

impl PageMerger {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Check if two pages should be merged
    pub fn should_merge(
        &self,
        page1: &SlottedPage,
        page2: &SlottedPage,
        page_size: usize,
    ) -> bool {
        let total_free = page1.free_space() as usize + page2.free_space() as usize;
        let utilization = 1.0 - (total_free as f64 / (2.0 * page_size as f64));
        utilization < self.threshold
    }

    /// Merge two pages into one
    pub fn merge(&self, page1: &mut SlottedPage, page2: &SlottedPage) -> bool {
        let header2 = page2.page.read_header();

        // Try to insert all records from page2 into page1
        for slot_id in 0..header2.num_slots {
            if let Some(data) = page2.get_record(slot_id) {
                if page1.insert_record(&data).is_none() {
                    return false; // Merge failed - not enough space
                }
            }
        }

        true
    }
}

/// Variable-length record with overflow support
pub struct VariableLengthRecord {
    inline_data: Vec<u8>,
    overflow_pages: Vec<PageId>,
    total_length: usize,
}

impl VariableLengthRecord {
    pub fn new(data: Vec<u8>, max_inline: usize) -> Self {
        let total_length = data.len();

        if data.len() <= max_inline {
            Self {
                inline_data: data,
                overflow_pages: Vec::new(),
                total_length,
            }
        } else {
            Self {
                inline_data: data[..max_inline].to_vec(),
                overflow_pages: Vec::new(), // Would be populated by storage manager
                total_length,
            }
        }
    }

    pub fn is_overflow(&self) -> bool {
        !self.overflow_pages.is_empty()
    }

    pub fn inline_size(&self) -> usize {
        self.inline_data.len()
    }

    pub fn total_size(&self) -> usize {
        self.total_length
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

impl Serialize for VariableLengthRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VariableLengthRecord", 3)?;
        state.serialize_field("inline_data", &self.inline_data)?;
        state.serialize_field("overflow_pages", &self.overflow_pages)?;
        state.serialize_field("total_length", &self.total_length)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VariableLengthRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct VarRecord {
            inline_data: Vec<u8>,
            overflow_pages: Vec<PageId>,
            total_length: usize,
        }

        let record = VarRecord::deserialize(deserializer)?;
        Ok(VariableLengthRecord {
            inline_data: record.inline_data,
            overflow_pages: record.overflow_pages,
            total_length: record.total_length,
        })
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

    #[test]
    fn test_slotted_page_insert() {
        let mut page = SlottedPage::new(1, 4096);
        let data = b"Hello, World!";
        let slot_id = page.insert_record(data).unwrap();
        assert_eq!(slot_id, 0);

        let retrieved = page.get_record(slot_id).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_slotted_page_delete() {
        let mut page = SlottedPage::new(1, 4096);
        let data = b"Test data";
        let slot_id = page.insert_record(data).unwrap();

        assert!(page.delete_record(slot_id));
        assert!(page.get_record(slot_id).is_none());
    }

    #[test]
    fn test_slotted_page_update() {
        let mut page = SlottedPage::new(1, 4096);
        let data = b"Original";
        let slot_id = page.insert_record(data).unwrap();

        let new_data = b"Updated";
        assert!(page.update_record(slot_id, new_data));

        let retrieved = page.get_record(slot_id).unwrap();
        assert_eq!(retrieved, new_data);
    }

    #[test]
    fn test_page_compaction() {
        let mut page = SlottedPage::new(1, 4096);

        // Insert multiple records
        for i in 0..10 {
            page.insert_record(format!("Record {}", i).as_bytes());
        }

        // Delete some records
        page.delete_record(2);
        page.delete_record(5);
        page.delete_record(7);

        let free_before = page.free_space();
        page.compact();
        let free_after = page.free_space();

        assert!(free_after >= free_before);
    }

    #[test]
    fn test_page_splitting() {
        let mut page = SlottedPage::new(1, 4096);
        let splitter = PageSplitter::new(0.8);

        // Fill page
        for i in 0..50 {
            page.insert_record(format!("Record number {}", i).as_bytes());
        }

        if splitter.should_split(&page, 4096) {
            let new_page = splitter.split(&mut page, 2);
            assert!(new_page.page.id == 2);
        }
    }

    #[test]
    fn test_variable_length_record() {
        let data = vec![1, 2, 3, 4, 5];
        let record = VariableLengthRecord::new(data.clone(), 1024);

        assert!(!record.is_overflow());
        assert_eq!(record.total_size(), 5);
    }

    #[test]
    fn test_checksum() {
        let mut page = Page::new(1, 4096);
        page.update_checksum();
        assert!(page.verify_checksum());

        // Corrupt data
        page.data[100] = 255;
        assert!(!page.verify_checksum());
    }
}


