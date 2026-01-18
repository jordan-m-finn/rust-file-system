use crate::constants::{
    BLOCK_SIZE,
    DESCRIPTOR_SIZE,
    DESCRIPTORS_PER_BLOCK,
    NUM_DESCRIPTORS,
};
use crate::byte_utils::{read_i32, write_i32};

// file descriptor in memory (not on-disk format)
#[derive(Debug, Clone, Copy)]
pub struct FileDescriptor {
    pub size: i32,
    pub blocks: [i32; 3],
}

impl FileDescriptor {
    // create a new free descriptor (size = -1)
    pub fn new_free() -> Self {
        FileDescriptor {
            size: -1,
            blocks: [0, 0, 0],
        }
    }

    // create a new empty file descriptor (size = 0)
    pub fn new_empty() -> Self {
        FileDescriptor {
            size: 0,
            blocks: [0, 0, 0],
        }
    }

    // check if this descriptor is free
    pub fn is_free(&self) -> bool {
        self.size == -1
    }
}

// calculate which disk block contains descriptor `index`
// descriptors are in blocks 1-6, with 32 descriptors per block
pub fn descriptor_block(index: usize) -> usize {
    // block 1 has descriptors 0-31
    // block 2 has descriptors 32-63
    // etc.
    1 + (index / DESCRIPTORS_PER_BLOCK)
}

// calculate the byte offset within a block for descriptor `index`
pub fn descriptor_offset(index: usize) -> usize {
    (index % DESCRIPTORS_PER_BLOCK) * DESCRIPTOR_SIZE
}

// read a descriptor from a block's data
pub fn read_descriptor(block_data: &[u8], index_in_block: usize) -> FileDescriptor {
    let offset = index_in_block * DESCRIPTOR_SIZE;

    FileDescriptor {
        size: read_i32(block_data, offset),
        blocks: [
            read_i32(block_data, offset + 4),
            read_i32(block_data, offset + 8),
            read_i32(block_data, offset + 12),
        ],
    }
}

// write a descriptor to a block's data
pub fn write_descriptor(block_data: &mut [u8], index_in_block: usize, desc: &FileDescriptor) {
    let offset = index_in_block * DESCRIPTOR_SIZE;

    write_i32(block_data, offset, desc.size);
    write_i32(block_data, offset + 4, desc.blocks[0]);
    write_i32(block_data, offset + 8, desc.blocks[1]);
    write_i32(block_data, offset + 12, desc.blocks[2]);
}

// find a free descriptor (size == -1)
// takes a function that reads block data, since we need to check multiple blocks
// returns some(index) if found, None if all descriptors are in use
pub fn find_free_descriptor<F>(mut read_block_fn: F) -> Option<usize>
where
    F: FnMut(usize) -> [u8; BLOCK_SIZE],
{
    // start at 1, descriptor 0 is directory
    for desc_index in 1..NUM_DESCRIPTORS {
        let block_num = descriptor_block(desc_index);
        let block_data = read_block_fn(block_num);
        let index_in_block = desc_index % DESCRIPTORS_PER_BLOCK;

        let desc = read_descriptor(&block_data, index_in_block);
        if desc.is_free() {
            return Some(desc_index);
        }
    }
    None
}

