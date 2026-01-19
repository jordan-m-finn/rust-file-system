// block and disk geometry
// -- bytes per block
// -- total blocks on disk
pub const BLOCK_SIZE: usize = 512;
pub const NUM_BLOCKS: usize = 64;

// reserved blocks:
// -- block 0 holds the bitmap
// -- blocks 1-6 hold descriptors
// -- first directory block
pub const BITMAP_BLOCK: usize = 0;
pub const DIRECTORY_BLOCK: usize = 7;

// file descriptors:
// -- total file descriptors
// -- 512 / 16 = 32
// -- 4 integers * 4 bytes
pub const NUM_DESCRIPTORS: usize = 192;
pub const DESCRIPTORS_PER_BLOCK: usize = 32;
pub const DESCRIPTOR_SIZE: usize = 16;

pub const OFT_SIZE: usize = 4;

pub const MAX_FILE_BLOCKS: usize = 3;
// 1536 bytes
pub const MAX_FILE_SIZE: usize = MAX_FILE_BLOCKS * BLOCK_SIZE;

// directory entries
// -- 4-byte name + 4-byte index
// -- max 3 chars + null terminator
pub const DIR_ENTRY_SIZE: usize = 8;
