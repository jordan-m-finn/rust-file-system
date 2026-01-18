use crate::constants::{
    BLOCK_SIZE,
    NUM_BLOCKS,
    NUM_DESCRIPTORS,
    DESCRIPTORS_PER_BLOCK,
    OFT_SIZE,
    BITMAP_BLOCK,
    DIRECTORY_BLOCK,
};
use crate::disk::Disk;
use crate::bitmap::{
    set_block_occupied,
    set_block_free,
    find_free_block,
    is_block_occupied
};
use crate::byte_utils::{read_i32, write_i32};
use crate::descriptor::{
    FileDescriptor, descriptor_block, read_descriptor, write_descriptor,
};
use crate::oft::{OFT, OFTEntry};

// complete file system state
pub struct FileSystem {
    disk: Disk,
    oft: OFT,

    // main memory buffer M[512] for user data
    pub memory: [u8; BLOCK_SIZE],

    // cache of reserved blocks (bitmap and descriptors)
    // avoids repeatedly reading blocks 0-6 from disk
    // block 0 = bitmap, blocks 1-6 = descriptors
    //
    // As quoted from the spec:
    // "The reserved blocks 0 through 6 must be accessed frequently to access the bitmap
    // and the descriptors. It would not be practical to keep reading the blocks repeatedly
    // from the disk. Instead, these blocks may be copied into a dedicated data structure
    // after each init command."
    reserved_cache: [[u8; BLOCK_SIZE]; 7],
}

impl FileSystem {
    // create and initialize the new file system
    pub fn new() -> Self {
        let mut fs = FileSystem {
            disk: Disk::new(),
            oft: OFT::new(),
            memory: [0u8; BLOCK_SIZE],
            reserved_cache: [[0u8; BLOCK_SIZE]; 7],
        };

        fs.init();
        fs
    }

    // initialize (or reinitialize) the file system to its starting state
    pub fn init(&mut self) {
        // step 1: zero out the disk
        self.disk = Disk::new();

        // step 2: zero out the reserved cache
        for block in self.reserved_cache.iter_mut() {
            block.fill(0);
        }

        // step 3: set up the bitmap (block 0 in cache)
        // mark blocks 0-7 as occupied (bitmap, descriptors, directory)
        for block_num in 0..=DIRECTORY_BLOCK {
            set_block_occupied(&mut self.reserved_cache[BITMAP_BLOCK], block_num);
        }

        // step 4: set up file descriptors (blocks 1-6 in cache)
        // descriptor 0 = directory: size = 0, first block = 7
        let dir_descriptor = FileDescriptor {
            size: 0,
            blocks: [DIRECTORY_BLOCK as i32, 0, 0],
        };
        write_descriptor(&mut self.reserved_cache[1], 0, &dir_descriptor);

        // descriptors 1-191 = free (size=-1)
        for desc_idx in 1..NUM_DESCRIPTORS {
            // should return 1-6
            let block_in_cache = descriptor_block(desc_idx);
            let index_in_block = desc_idx % DESCRIPTORS_PER_BLOCK;

            let free_descriptor = FileDescriptor::new_free();
            write_descriptor(
                &mut self.reserved_cache[block_in_cache],
                index_in_block,
                &free_descriptor,
            );
        }

        // step 5: write reserved cache to disk
        self.flush_reserved_cache();

        // step 6: initialize memory M to zeros
        self.memory.fill(0);

        // step 7: initialize OFT
        self.oft = OFT::new();

        // step 8: open the directory at OFT[0]
        if let Some(dir_entry) = self.oft.get_mut(0) {
            dir_entry.current_pos = 0;
            dir_entry.size = 0;
            dir_entry.descriptor_index = 0;
            //load directory's first block (block 7) into buffer
            self.disk.read_block(DIRECTORY_BLOCK).unwrap();
            dir_entry.buffer.copy_from_slice(&self.disk.input_buffer);
        }
    }

    // write the reserved cache (blocks 0-6) back to disk
    fn flush_reserved_cache(&mut self) {
        for (i, block_data) in self.reserved_cache.iter().enumerate() {
            self.disk.output_buffer.copy_from_slice(block_data);
            self.disk.write_block(i).unwrap();
        }
    }

    // load the reserved blocks from disk into cache which is called after loading a saved disk
    fn load_reserved_cache(&mut self) {
        for i in 0..7 {
            self.disk.read_block(i).unwrap();
            self.reserved_cache[i].copy_from_slice(&self.disk.input_buffer);
        }
    }

    // ======================= BITMAP HELPERS ============================ //

    // find a free block and mark it as occupied
    // @returns the block number, or None if disk is full
    fn allocate_block(&mut self) -> Option<usize> {
        let block_num = find_free_block(&self.reserved_cache[BITMAP_BLOCK])?;
        set_block_occupied(&mut self.reserved_cache[BITMAP_BLOCK], block_num);
        Some(block_num)
    }

    // mark a block as free in the bitmap
    fn free_block(&mut self, block_num: usize) {
        set_block_free(&mut self.reserved_cache[BITMAP_BLOCK], block_num);
    }

    // check if a block is occupied
    fn is_block_occupied(&self, block_num: usize) -> bool {
        is_block_occupied(&self.reserved_cache[BITMAP_BLOCK], block_num)
    }

    // ======================== DESCRIPTOR HELPERS ======================== //

    // read a file descriptor from the cache
    fn read_descriptor(&self, desc_index: usize) -> FileDescriptor {
        let block_in_cache = descriptor_block(desc_index);
        let index_in_block = desc_index % DESCRIPTORS_PER_BLOCK;
        read_descriptor(&self.reserved_cache[block_in_cache], index_in_block)
    }

    // write a file descriptor to the cache
    fn write_descriptor(&mut self, desc_index: usize, desc: &FileDescriptor) {
        let block_in_cache = descriptor_block(desc_index);
        let index_in_block = desc_index % DESCRIPTORS_PER_BLOCK;
        write_descriptor(
            &mut self.reserved_cache[block_in_cache],
            index_in_block,
            desc,
        );
    }

    // find a free descriptor (size == -1), starting from index 1
    fn find_free_descriptor(&self) -> Option<usize> {
        for desc_idx in 1..NUM_DESCRIPTORS {
            if self.read_descriptor(desc_idx).is_free() {
                return Some(desc_idx);
            }
        }
        None
    }


    // ======================== Debug/Testing HELPERS ======================== //
    
    // print the current state of the bitmap (first 16 blocks)
    pub fn debug_bitmap(&self) {
        print!("Bitmap (blocks 0-15): ");
        for i in 0..16 {
            if self.is_block_occupied(i) {
                print!("1");
            } else {
                print!("0");
            }
        }
        println!();
    }

    // print the state of descriptor 0 (directory)
    pub fn debug_directory_descriptor(&self) {
        let desc = self.read_descriptor(0);

        println!(
            "Descriptor 0 (directory): size={}, blocks={:?}",
            desc.size, desc.blocks
        );
    } 

    // print the state of the OFT
    pub fn debug_oft(&self) {
        println!("Open File Table:");
        for i in 0..OFT_SIZE {
            if let Some(entry) = self.oft.get(i) {
                let status = if entry.is_free() { "free" } else { "in use" };
                println!(
                    "  [{}] {} - pos={}, size={}, desc={}",
                    i, status, entry.current_pos, entry.size, entry.descriptor_index
                );
            }
        }
    }
}
