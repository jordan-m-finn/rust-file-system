use crate::constants::NUM_BLOCKS;

// bitmap manages 64 bits (one per block) stored in the first 8 bytes of block 0.
// bit = 0 means free, bit = 1 means occupied

pub fn is_block_occupied(bitmap_data: &[u8], block_num: usize) -> bool {
    if block_num >= NUM_BLOCKS {
        return true;
    }

    let byte_index = block_num / 8;
    let bit_position = block_num % 8;

    // extract the bit using bitwise AND with a mask
    // ex: to check bit 3, mask is 00001000 (1 << 3)
    let mask = 1u8 << bit_position;

    (bitmap_data[byte_index] & mask) != 0
}

// set bit to 1
pub fn set_block_occupied(bitmap_data: &mut [u8], block_num: usize) {
    if block_num >= NUM_BLOCKS {
        return;
    }

    let byte_index = block_num / 8;
    let bit_position = block_num % 8;

    // set the bit using bitwise OR
    // ex: to set bit 3, OR with 00001000
    let mask = 1u8 << bit_position;
    bitmap_data[byte_index] |= mask;
}

// set bit to 0
pub fn set_block_free(bitmap_data: &mut [u8], block_num: usize) {
    if block_num >= NUM_BLOCKS {
        return;
    }

    let byte_index = block_num / 8;
    let bit_position = block_num % 8;
    
    // clear the bit using bitwise AND with inverted mask
    // ex: to clear bit 3, AND with 11110111 (!(1 << 3))
    let mask = !(1u8 << bit_position);
    bitmap_data[byte_index] &= mask;
}

// find the first bit that is 0
// @return Some(block_num) if found, None if disk is full
pub fn find_free_block(bitmap_data: &[u8]) -> Option<usize> {
    for block_num in 0..NUM_BLOCKS {
        if !is_block_occupied(bitmap_data, block_num) {
            return Some(block_num);
        }
    }
    None
}
