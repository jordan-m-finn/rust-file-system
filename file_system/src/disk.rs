use crate::constants::{BLOCK_SIZE, NUM_BLOCKS};

// DISK: 2D array of bytes
// D[64][512] - 64 blocks, each 512 bytes
pub struct Disk {
    data: [[u8; BLOCK_SIZE]; NUM_BLOCKS],

    // I/O buffers
    pub input_buffer: [u8; BLOCK_SIZE], // I[512]
    pub output_buffer: [u8; BLOCK_SIZE], // O[512]
}

imp Disk {
    // create a new disk with all bytes initialized to zero
    pub fn new() -> Self {
        Disk {
            data: [[0u8; BLOCK_SIZE]; NUM_BLOCKS],
            input_buffer: [0u8; BLOCK_SIZE],
            output_buffer: [0u8; BLOCK_SIZE],
        }
    }

    // read block `block_num` from disk into the input buffer
    pub fn read_block(&mut self, block_num: usize) {
        // copy the entire block into the input_buffer
        self.input_buffer.copy_from_slice(&self.data[block_num]);
    }

    // write the output buffer contents to block `block_num` on disk
    pub fn write_block(&mut self, block_num: usize) {
        // copy output_buffer into the disk block
        self.data[block_num].copy_from_slice(&self.output_buffer);
    }
}
