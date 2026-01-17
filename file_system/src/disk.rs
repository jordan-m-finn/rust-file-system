use crate::constants::{BLOCK_SIZE, NUM_BLOCKS};

// DISK: 2D array of bytes
// D[64][512] - 64 blocks, each 512 bytes
pub struct Disk {
    data: [[u8; BLOCK_SIZE]; NUM_BLOCKS],

    // I/O buffers
    pub input_buffer: [u8; BLOCK_SIZE], // I[512]
    pub output_buffer: [u8; BLOCK_SIZE], // O[512]
}
