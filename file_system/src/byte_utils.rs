// reads a 32-bit signed integer from a byte slice at the given offset
// uses little-endian byte order (least significant byte first)
pub fn read_i32(data: &[u8], offset: usize) -> i32 {
    // extract 4 bytes starting at offset
    let bytes: [u8; 4] = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];

    // convert from little-endian bytes to i32
    i32::from_le_bytes(bytes)
}

// writes a 32-bit signed integer to a byte slice at the given offset
// uses little-endian byte order
pub fn write_i32(data: &mut [u8], offset: usize, value: i32) {
    // convert i32 to little-endian bytes
    let bytes = value.to_le_bytes();

    // copy the 4 bytes into the slice
    data[offset] = bytes[0];
    data[offset + 1] = bytes[1];
    data[offset + 2] = bytes[2];
    data[offset + 3] = bytes[3];
}
