use crate::constants::{BLOCK_SIZE, OFT_SIZE};

// an entry in the Open File Table (OFT)
#[derive(Clone)]
pub struct OFTEntry {
    // r/w holding current block
    pub buffer: [u8; BLOCK_SIZE],
    pub current_pos: i32,
    pub size: i32,
    pub descriptor_index: i32,
}

impl OFTEntry {
    // create a new free OFT entry
    pub fn new_free() -> Self {
        OFTEntry {
            buffer: [0u8; BLOCK_SIZE],
            // -1 indicates free
            current_pos: -1,
            size: 0,
            descriptor_index: 0,
        }
    }

    // check if this entry is free
    pub fn is_free(&self) -> bool {
        self.current_pos == -1
    }

    // get which block of the file is currently in the buffer (current_pos / BLOCK_SIZE)
    pub fn current_block(&self) -> usize {
        if self.current_pos < 0 {
            0
        } else {
            (self.current_pos as usize) / BLOCK_SIZE
        }
    }

    // get offset w/in the buffer for the current position (current_pos % BLOCK_SIZE)
    pub fn buffer_offset(&self) -> usize {
        if self.current_pos < 0 {
            0
        } else {
            (self.current_pos as usize) % BLOCK_SIZE
        }
    }
}

// OFT itself
pub struct OFT {
    entries: [OFTEntry; OFT_SIZE],
}

impl OFT {
    // create a new OFT with all entries free
    pub fn new() -> Self {
        OFT {
            entries: [
                OFTEntry::new_free(),
                OFTEntry::new_free(),
                OFTEntry::new_free(),
                OFTEntry::new_free(),
            ],
        }
    }

    // get a reference to an entry
    pub fn get(&self, index: usize) -> Option<&OFTEntry> {
        if index < OFT_SIZE {
            Some(&self.entries[index])
        } else {
            None
        }
    }

    // get a mutable reference to an entry
    pub fn get_mut(&mut self, index: usize) -> Option<&mut OFTEntry> {
        if index < OFT_SIZE {
            Some(&mut self.entries[index])
        } else {
            None
        }
    }

    // find a free OFT entry
    // @returns the index
    pub fn find_free(&self) -> Option<usize> {
        for i in 1..OFT_SIZE {
            if self.entries[i].is_free() {
                return Some(i);
            }
        }
        None
    }
}
