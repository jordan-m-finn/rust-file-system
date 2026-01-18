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

// result type for file system operations
pub type FsResult<T> = Result<T, &'static str>;

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


    // ======================== Core File Operations ======================== // 

    // Seek: set the current position of file at OFT index to `pos`
    //
    // @returns Ok(()) on success, Err on failure
    pub fn seek(&mut self, oft_index: usize, pos: usize) -> FsResult<()> {
        // Validate OFT index
        if oft_index >= OFT_SIZE {
            return Err("Invalid OFT index");
        }

        // get the OFT entry
        let entry = self.oft.get(oft_index).ok_or("Invalid OFT index")?;

        // check if entry is in use
        if entry.is_free() {
            return Err("File not open");
        }

        // check if position is valid (can seek to end of file, but not past it)
        if pos > entry.size as usize {
            return Err("Position exceeds file size");
        }

        // determine which block of the file contains position 'pos'
        // 0, 1, or 2
        let new_block_index = pos / BLOCK_SIZE;

        // get current block in buffer
        let entry = self.oft.get(oft_index).unwrap();
        let current_block_index = entry.current_block();

        // if we need a different block, swap buffers
        if new_block_index != current_block_index && entry.size > 0 {
            // get the descriptor to find block numbers
            let desc_index = entry.descriptor_index as usize;
            let desc = self.read_descriptor(desc_index);

            // write current buffer back to disk
            let current_disk_block = desc.blocks[current_block_index] as usize;
            if current_disk_block > 0 {
                let entry = self.oft.get(oft_index).unwrap();
                self.disk.output_buffer.copy_from_slice(&entry.buffer);
                self.disk.write_block(current_disk_block)?;
            }

            // load the new block into buffer
            let new_disk_block = desc.blocks[new_block_index] as usize;
            if new_disk_block > 0  {
                self.disk.read_block(new_disk_block)?;
                let entry = self.oft.get_mut(oft_index).unwrap();
                entry.buffer.copy_from_slice(&self.disk.input_buffer);
            }
        }

        // update current position
        let entry = self.oft.get_mut(oft_index).unwrap();
        entry.current_pos = pos as i32;

        Ok(())
    }

    // Read: copy 'count' bytes from file at OFT index to memory M starting at 'mem_pos'
    //
    // Returns Ok(bytes_read) on success
    pub fn read(&mut self, oft_index: usize, mem_pos: usize, count: usize) -> FsResult<usize> {
        // validate OFT index
        if oft_index >= OFT_SIZE {
            return Err("Invalid OFT index");
        }

        // validate memory position
        if mem_pos >= BLOCK_SIZE {
            return Err("Memory position out of range");
        }

        // check if entry is in use
        {
            let entry = self.oft.get(oft_index).ok_or("Invalid OFT index")?;
            if entry.is_free() {
                return Err("File not open");
            }
        }

        let mut bytes_read = 0;

        while bytes_read < count {
            // get current state (must re-borrow each iteration due to mutations)
            let (current_pos, file_size, desc_index, buffer_offset) = {
                let entry = self.oft.get(oft_index).unwrap();
                (
                    entry.current_pos as usize,
                    entry.size as usize,
                    entry.descriptor_index as usize,
                    entry.buffer_offset(),
                )
            };

            // check for end of file
            if current_pos >= file_size {
                break;
            }

            // check for end of memory buffer m
            if mem_pos + bytes_read >= BLOCK_SIZE {
                break;
            }

            // calculate how many bytes we can read in this iteration
            let bytes_remaining_in_buffer = BLOCK_SIZE - buffer_offset;
            let bytes_remaining_in_file = file_size - current_pos;
            let bytes_remaining_to_read = count - bytes_read;
            let bytes_remaining_in_memory = BLOCK_SIZE - (mem_pos + bytes_read);

            let bytes_this_iteration = bytes_remaining_in_buffer
                .min(bytes_remaining_in_file)
                .min(bytes_remaining_to_read)
                .min(bytes_remaining_in_memory);
            
            // copy bytes from OFT buffer to memory M
            {
                let entry = self.oft.get(oft_index).unwrap();
                let src_start = buffer_offset;
                let src_end = buffer_offset + bytes_this_iteration;
                let dst_start = mem_pos + bytes_read;
                let dst_end = dst_start + bytes_this_iteration;

                self.memory[dst_start..dst_end]
                    .copy_from_slice(&entry.buffer[src_start..src_end]);
            }

            // update position and count
            bytes_read += bytes_this_iteration;
            {
                let entry = self.oft.get_mut(oft_index).unwrap();
                entry.current_pos += bytes_this_iteration as i32;
            }

            // check if we hit end of buffer and need to load next block
            let (new_buffer_offset, current_pos, file_size) = {
                let entry = self.oft.get(oft_index).unwrap();
                (entry.buffer_offset(), entry.current_pos as usize, entry.size as usize)
            };

            if new_buffer_offset == 0 && current_pos < file_size && bytes_read < count {
                // we've crossed into a new block, need to swap buffers
                self.swap_buffer(oft_index);
            }
        }

        Ok(bytes_read)
    }

    // Write: copy 'count' bytes from memory M[mem_pos...] to file at OFT index
    //
    // @returns Ok(bytes_written) on success
    pub fn write(&mut self, oft_index: usize, mem_pos: usize, count: usize) -> FsResult<usize> {
        use crate::constants::MAX_FILE_SIZE;
        
        // validate OFT index
        if oft_index >= OFT_SIZE {
            return Err("Invalid OFT index");
        }

        // validate memory position
        if mem_pos >= BLOCK_SIZE {
            return Err("Memory position out of range");
        }

        // check if entry is in use
        {
            let entry = self.oft.get(oft_index).ok_or("Invalid OFT index")?;
            if entry.is_free() {
                return Err("File not open");
            }
        }

        let mut bytes_written = 0;

        while bytes_written < count {
            // get current state
            let (current_pos, file_size, desc_index, buffer_offset) = {
                let entry = self.oft.get(oft_index).unwrap();;
                (
                    entry.current_pos as usize,
                    entry.size as usize,
                    entry.descriptor_index as usize,
                    entry.buffer_offset(),
                )
            };

            if current_pos >= MAX_FILE_SIZE {
                break;
            }

            // check for the end of memory buffer M
            if mem_pos + bytes_written >= BLOCK_SIZE {
                break;
            }

            // calculate how many bytes we can write in this iteration
            let bytes_remaining_in_buffer = BLOCK_SIZE - buffer_offset;
            let bytes_remaining_in_file = MAX_FILE_SIZE - current_pos;
            let bytes_remaining_to_write = count - bytes_written;
            let bytes_remaining_in_memory = BLOCK_SIZE - (mem_pos + bytes_written);

            let bytes_this_iteration = bytes_remaining_in_buffer
                .min(bytes_remaining_in_file)
                .min(bytes_remaining_to_write)
                .min(bytes_remaining_in_memory);

            // copy bytes from memory M to OFT buffer
            {
                let entry = self.oft.get_mut(oft_index).unwrap();
                let src_start = mem_pos + bytes_written;
                let src_end = src_start + bytes_this_iteration;
                let dst_start = buffer_offset;
                let dst_end = buffer_offset + bytes_this_iteration;

                entry.buffer[dst_start..dst_end]
                    .copy_from_slice(&self.memory[src_start..src_end]);
            }

            // update position and count
            bytes_written += bytes_this_iteration;
            {
                let entry = self.oft.get_mut(oft_index).unwrap();
                entry.current_pos += bytes_this_iteration as i32;

                // update file size if we've extended the file
                if entry.current_pos > entry.size {
                    entry.size = entry.current_pos;
                }
            }

            // check if we hit end of buffer and need to move to next block
            let (new_buffer_offset, current_pos) = {
                let entry = self.oft.get(oft_index).unwrap();
                (entry.buffer_offset(), entry.current_pos as usize)
            };

            if new_buffer_offset == 0 && current_pos < MAX_FILE_SIZE && bytes_written < count {
                // we've crossed into a new block
                self.write_swap_buffer(oft_index)?;
            }
        }

        // update the descriptor with new file size
        {
            let entry = self.oft.get(oft_index).unwrap();
            let desc_index = entry.descriptor_index as usize;
            let new_size = entry.size;

            let mut desc = self.read_descriptor(desc_index);
            desc.size = new_size;
            self.write_descriptor(desc_index, &desc);
        }

        Ok(bytes_written)
    }

    // open a file by name
    // @returns the OFT index on success
    pub fn open(&mut self, name: &str) -> FsResult<usize> {
        // validate name length
        if name.is_empty() || name.len() > 4 {
            return Err("Invalid file name");
        }

        // search directory for the file
        let desc_index = match self.find_file_in_directory(name)? {
            Some(idx) => idx,
            None => return Err("File not found");
        };

        // find a free OFT entry (skip index 0, reserved for directory)
        let oft_index = match self.oft.find_free() {
            Some(idx) => idx,
            None => return Err("Too many files open");
        };

        // get file descriptor
        let desc = self.read_descriptor(desc_index);

        // set upo the OFT entry
        {
            let entry = self.oft.get_mut(oft_index).unwrap();
            entry.current_pos = 0;
            entry.size = desc.size;
            entry.descriptor_index = desc_index as i32;
            entry.buffer.fill(0);
        }

        // if file has content, load first block into buffer
        if desc.size > 0 && desc.blocks[0] > 0 {
            let block_num = desc.blocks[0] as usize;
            self.disk.read_block(block_num)?;
            let entry = self.oft.get_mut(oft_index).unwrap();
            entry.buffer.copy_from_slice(&self.disk.input_buffer);
        } else if desc.size == 0 {
            // empty file so allocate first block if not alread allocated
            let mut desc = desc;
            if desc.blocks[0] == 0 {
                let new_block = self.allocate_block().ok_or("Disk full");
                desc.blocks[0] = new_block as i32;
                self.write_descriptor(desc_index, &desc);
            }
        }
        
        Ok(oft_index)
    }

    // close a file via OFT index
    pub fn close(&mut self, oft_index: usize) -> FsResult<()> {
        // validate OFT index (can't close directory at index 0)
        if oft_index == 0 {
            return Err("Cannot close directory");
        }

        if oft_index >= OFT_SIZE {
            return Err("Invalid OFT index");
        }

        // check if entry is in use
        {
            let entry = self.oft.get(oft_index).ok_or("Invalid OFT index");
            if entry.is_free() {
                return Err("File not open");
            }
        }

        // get info we need before modifying
        let desc_index, current_pos, size) = {
            let entry = self.oft.get(oft_index).unwrap();
            (
                entry.descriptor_index as usize,
                entry.current_pos as usize,
                entry.size,
            )
        };

        // write buffer back to disk
        let desc = self.read_descriptor(desc_index);
        let current_block_idnex = current_pos / BLOCK_SIZE;
        
        // only write if there's a valid block
        if current_block_index < 3 {
            let disk_block = desc.blocks[current_block_index] as usize;
            if disk_block > 0 {
                let entry = self.oft.get(oft_index).unwrap();
                self.disk.output_buffer.copy_from_slice(&entry.buffer);
                self.disk.write_block(disk_block)?;
            }
        }

        // update file size in descriptor
        let mut desc = self.read_descriptor(desc_index);
        desc.size = size;
        self.write_descriptor(desc_index, &desc);

        // flush the reserved cache to persist descriptor changes
        self.flush_reserved_cache();

        // mark OFT entry as free
        let entry = self.oft.get_mut(oft_index).unwrap();
        entry.current_pos = -1;
        entry.size = 0;
        entry.descriptor_index = 0;
        entry.buffer.fill(0);

        Ok(())
    }

    // creates a new file 
    pub fn create(&mut self, name: &str) -> FsResult<()> {
        // validate name
        if name.is_empty() || name.len() > 4 {
            return Err("Invalid file name");
        }

        // check if file already exists
        if self.find_file_in_directory(name)?.is_some() {
            return Err("File already exists");
        }

        // find a free descriptor
        let desc_index = match self.find_free_descriptor() {
            Some(idx) => idx,
            None => return Err("Too many files");
        };

        // find a free directory entry
        let dir_pos = match self.find_free_directory_entry()? {
            Some(pos) => pos,
            None => return Err("Directory full");
        };

        // initialize the descriptor (size=0, no blocks allocated yet)
        let new_desc = FileDescriptor::new_empty();
        self.write_descriptor(desc_index, &new_desc);

        // write directory entry
        self.write_directory_entry(dir_pos, name, desc_index)?;

        // flush changes to disk
        self.flush_reserved_cache();

        Ok(())
    }

    // deletes a file
    pub fn destroy(&mut self, name: &str) -> FsResult<()> {
        use crate::constants::DIR_ENTRY_SIZE;

        // search directory for the file, keeping track of position
        self.seek(0, 0)?;

        let dir_size = self.oft.get(0).unwrap().size as usize;
        let mut pos = 0;
        let mut found_desc_index: Option<usize> = None;
        let mut found_pos: Option<usize> = None;

        while pos < dir_size {
            let bytes_read = self.read(0, 0, DIR_ENTRY_SIZE)?;
            if bytes_read < DIR_ENTRY_SIZE {
                break;
            }

            let entry_name = self.extract_name_from_memory(0);
            let desc_index = read_i32(&self.memory, 4) as usize;

            if entry_name == name {
                found_desc_index = Some(desc_index);
                found_pos = Some(pos);
                break;
            }

            pos += DIR_ENTRY_SIZE;
        }

        // check if file was found 
        let desc_index = found_desc_index.ok_or("File not found")?;
        let entry_pos = found_pos.unwrap();

        // check if file is currently open
        for i in 1..OFT_SIZE {
            if let Some(entry) = self.oft.get(i) {
                if !entry.is_free() && entry.descriptor__index as usize == desc_index {
                    return Err("File is open");
                }
            }
        }

        // get descriptor and free its blocks
        let desc = self.read_descriptor(desc_index);
        for &block_num in &desc.blocks {
            if block_num > 0 {
                self.free_block(block_num as usize);
            }
        }

        // mark descriptor as free
        let free_desc = FileDescriptor::new_free();
        self.write_descriptor(desc_index, &free_desc);

        // clear directory entry
        self.clear_directory_entry(entry_pos)?;

        // flush changes to disk
        self.flush_reserved_cache();

        Ok(())
    }

    // list all files in the directory
    // @returns a vector of (name, size) pairs
    pub fn directory(&mut self) -> FsResult<Vec<(String, i32)>> {
        use crate::constants::DIR_ENTRY_SIZE;

        let mut files = Vec::new();

        // seek to start of directory
        self.seek(0, 0)?;

        let dir_size = self.oft.get(0).unwrap().size as usize;
        let mut pos = 0;

        while pos < dir_size {
            let bytes_read = self.read(0, 0, DIR_ENTRY_SIZE)?;
            if bytes_read < DIR_ENTRY_SIZE {
                break;
            }

            // check if entry is in use (name not zero)
            if self.memory[0] != 0 {
                let name = self.extract_name_from_memory(0);
                let desc_index = read_i32(&self.memory, 4) as usize;

                // get file size from descriptor
                let desc = self.read_descriptor(desc_index);
                files.push((name, desc.size));
            }

            pos += DIR_ENTRY_SIZE;
        }

        Ok(files)
    }

    // =========== HELPER FUNCTIONS FOR THE CORE OPERATIONS ============== //

    // Helper: swap the buffer for an open file to match current position
    // writes current buffer to disk, loads new block
    fn swap_buffer(&mut self, oft_index: usize) -> FsResult<()> {
        let (desc_index, old_block_index, new_block_index) = {
            let entry = self.oft.get(oft_index).unwrap();
            let current_pos = entry.current_pos as usize;
            let new_block = current_pos / BLOCK_SIZE;
            // old block is one less since we just crossed the boundary
            let old_block = if new_block > 0 { new_block - 1 } else { 0 };
            (entry.descriptor_index as usize, old_block, new_block)
        };

        let desc = self.read_descriptor(desc_index);

        // write old buffer to disk
        let old_disk_block = desc.blocks[old_block_index] as usize;
        if old_disk_block > 0 {
            let entry = self.oft.get(oft_index).unwrap();
            self.disk.output_buffer.copy_from_slice(&entry.buffer);
            self.disk.write_block(old_disk_block)?;
        }

        // load new block into buffer
        let new_disk_block = desc.blocks[new_block_index] as usize;
        if new_disk_block > 0 {
            self.disk.read_block(new_disk_block)?;
            let entry = self.oft.get_mut(oft_index).unwrap();
            entry.buffer.copy_from_slice(&self.disk.input_buffer);
        } else {
            // new block doesn't exist yet, zero the buffer
            let entry = self.oft.get_mut(oft_index).unwrap();
            entry.buffer.fill(0);
        }

        Ok(())
    }

    // Helper: swap buffer during write operation, similar to the fn above but also allocates new
    // blocks if needed
    fn write_swap_buffer(&mut self, oft_index: usize) -> FsResult<()> {
        let (desc_index, old_block_index, new_block_index) = {
            let entry = self.oft.get(oft_index).unwrap();
            let current_pos = entry.current_pos as usize;
            let new_block = current_pos / BLOCK_SIZE;
            let old_block = if new_block > 0 { new_block - 1 } else { 0 };
            (entry.descriptor_index as usize, old_block, new_block)
        };

        let mut desc = self.read_descriptor(desc_index);

        // write old buffer to disk
        let old_disk_block = desc.blocks[old_block_index] as usize;
        if old_disk_block > 0 {
            let entry = self.oft.get(oft_index).unwrap();
            self.disk.output_buffer.copy_from_slice(&entry.buffer);
            self.disk.write_block(old_disk_block);
        }

        // check if new block exists, allocate if not
        let mut new_disk_block = desc.blocks[new_block_index] as usize;
        if new_disk_block == 0 {
            // allocate a new block
            new_disk_block = self.allocate_block().ok_or("Disk full")?;
            desc.blocks[new_block_index] = new_disk_block as i32;
            self.write_descriptor(desc_index, &desc);
        }

        // load new block into buffer (or zero it if freshly allocated)
        self.disk.read_block(new_disk_block)?;
        let entry = self.oft.get_mut(oft_index).unwrap();
        entry.buffer.copy_from_slice(&self.disk.input_buffer);

        Ok(())
    }

    
    // ======================= Memory Operations ============================ //
    
    // write a string to memory M starting at position `mem_pos`
    // @returns the number of bytes written
    pub fn write_memory(&mut self, mem_pos: usize, data: &str) -> FsResult<usize> {
        let bytes = data.as_bytes();
        let len = bytes.len();

        if mem_pos + len > BLOCK_SIZE {
            return Err("Data exceeds memory bounds");
        }

        self.memory[mem_pos..mem_pos + len].copy_from_slice(bytes);
        Ok(len)
    }

    // read 'count' bytes from memory M starting at position 'mem_pos'
    // @returns the bytes as a String (lossy conversion for non-UTF8)
    pub fn read_memory(&self, mem_pos: usize, count: usize) -> FsResult<String> {
        if mem_pos + count > BLOCK_SIZE {
            return Err("Read exceeds memory bounds");
        }

        let bytes = &self.memory[mem_pos..mem_pos + count];
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    // ======================= Directory Helpers ============================ //
    
    // search the directory for a file by name
    // @returns Some(descriptor_index) if found, None if not found
    fn find_file_in_directory(&mut self, name: &str) -> FsResult<Option<usize>> {
        use crate::constants::DIR_ENTRY_SIZE;

        // seek to start of directory
        self.seek(0, 0)?;

        let dir_size = self.oft.get(0).unwrap().size as usize;
        let mut pos = 0;

        while pos < dir_size {
            // read directory entry into memory
            let bytes_read = self.read(0, 0, DIR_ENTRY_SIZE)?;
            if bytes_read < DIR_ENTRY_SIZE {
                break;
            }

            // extract name (first 4 bytes)
            let entry_name = self.extract_name_from_memory(0);

            // extract descriptor index (next 4 bytes)
            let desc_index = read_i32(&self.memory, 4) as usize;

            // check if this entry matches
            if entry_name == name {
                return Ok(Some(desc_index));
            }

            pos += DIR_ENTRY_SIZE;
        }

        Ok(None)
    }

    // find a free entry in the directory
    // @returns Some(position) of the free entry, None if directory is full
    fn find_free_directory_entry(&mut self) -> FsResult<Option<usize>> {
        use crate::constants::{DIR_ENTRY_SIZE, MAX_FILE_SIZE};

        // seek to start of directory
        self.seek(0, 0)?;

        let dir_size = self.oft.get(0).unwrap().size as usize;
        let mut pos = 0;

        // first, search existing entries for a free slot (name = 0)
        while pos < dir_size {
            let bytes_read = self.read(0, 0, DIR_ENTRY_SIZE);
            if bytes_read < DIR_ENTRY_SIZE {
                break;
            }

            // check if name field is zero (aka free entry)
            if self.memory[0] == 0 {
                // found one! return its position
                return Ok(Some(pos));
            }

            pos += DIR_ENTRY_SIZE;
        }

        // no free entry found in existing entries
        // check if we can append a new entry
        if dir_size + DIR_ENTRY_SIZE <= MAX_FILE_SIZE {
            // we can add at the end
            return Ok(Some(dir_size));
        }
        
        // directory is full :/
        Ok(None)
    }

    // write a directory entry at the specified position
    fn write_directory_entry(&mut self, pos: usize, name: &str, desc_index: usize) {
        use crate::constants::DIR_ENTRY_SIZE;

        // prepare the entry in memory
        // first 4 bytes: name (null-padded)
        self.memory[0..4].fill(0);
        let name_bytes = name.as_bytes();
        let copy_len = name_bytes.len().min(4);
        self.memory[0..copy_len].copy_from_slice(&name_bytes[0..copy_len]);

        // next 4 bytes: descriptor index
        write_i32(&mut self.memory, 4, desc_index as i32);

        // seek to position and write
        self.seek(0, pos)?;
        self.write(0, 0, DIR_ENTRY_SIZE)?;

        Ok(())
    }

    // clear a directory entry at the specified position (set name to 0)
    fn clear_directory_entry(&mut self, pos: usize) -> FsResult<()> {
        use crate::constants::DIR_ENTRY_SIZE;

        // zero out the entry in memory
        self.memory[0..DIR_ENTRY_SIZE].fill(0);

        // seek to position and write
        self.seek(0, pos)?;
        self.write(0, 0, DIR_ENTRY_SIZE)?;

        Ok(())
    }

    // extract a null-terminated name from memory at the given offset
    fn extract_name_from_memory(&self, offset: usize) -> String {
        let mut name = String::new();

        for i in 0..4 {
            let byte = self.memory[offset + 1];
            if byte == 0 {
                break;
            }
            name.push(byte as char);
        }
        name
    }
}
