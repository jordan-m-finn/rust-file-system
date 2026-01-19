# File System Simulator ~ Rust

A file system simulator implemented in Rust for UCI CS143B (Operating Systems). This project emulates a simple disk-based file system with support for file creation, destruction, reading, writing, and directory management.

## Quick Start

```bash
# Build the project
cargo build --release

# Run with input/output files
cargo run <input_file> [output_file]

# Example
cargo run input.txt output.txt
```

If no output file is specified, results are written to `output.txt`.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Shell (shell.rs)                       │
│                 Command parsing & output formatting         │
├─────────────────────────────────────────────────────────────┤
│                   File System (fs.rs)                       │
│    create, destroy, open, close, read, write, seek, dir     │
├──────────────────┬──────────────────┬───────────────────────┤
│  Bitmap          │  Descriptors     │  Open File Table      │
│  (bitmap.rs)     │  (descriptor.rs) │  (oft.rs)             │
│  Block tracking  │  File metadata   │  Active file state    │
├──────────────────┴──────────────────┴───────────────────────┤
│                   Emulated Disk (disk.rs)                   │
│                D[64][512] with I/O buffers                  │
└─────────────────────────────────────────────────────────────┘
```

## Disk Layout

| Blocks | Purpose                                        |
| ------ | ---------------------------------------------- |
| 0      | Bitmap (64 bits tracking free/occupied blocks) |
| 1-6    | File descriptors (192 total, 32 per block)     |
| 7      | Directory's first data block                   |
| 8-63   | Available for file data                        |

## File Structure

| File            | Lines | Purpose                               |
| --------------- | ----- | ------------------------------------- |
| `main.rs`       | ~40   | Entry point, file I/O handling        |
| `fs.rs`         | ~380  | Core file system operations           |
| `shell.rs`      | ~115  | Command parser                        |
| `disk.rs`       | ~30   | Emulated disk with read/write block   |
| `bitmap.rs`     | ~35   | Bit manipulation for block allocation |
| `descriptor.rs` | ~45   | File descriptor read/write            |
| `oft.rs`        | ~55   | Open File Table management            |
| `constants.rs`  | ~15   | System constants                      |
| `byte_utils.rs` | ~15   | Byte/integer conversions              |

## Command Reference

| Command          | Description                       | Output                                |
| ---------------- | --------------------------------- | ------------------------------------- |
| `in`             | Initialize file system            | `system initialized`                  |
| `cr <name>`      | Create file                       | `<name> created`                      |
| `de <name>`      | Destroy file                      | `<name> destroyed`                    |
| `op <name>`      | Open file                         | `<name> opened <index>`               |
| `cl <index>`     | Close file                        | `<index> closed`                      |
| `rd <i> <m> <n>` | Read n bytes from file i to M[m]  | `<n> bytes read from <i>`             |
| `wr <i> <m> <n>` | Write n bytes from M[m] to file i | `<n> bytes written to <i>`            |
| `sk <i> <p>`     | Seek to position p in file i      | `position is <p>`                     |
| `dr`             | List directory                    | `<name1> <size1> <name2> <size2> ...` |
| `rm <m> <n>`     | Read n bytes from M[m]            | `<characters>`                        |
| `wm <m> <s>`     | Write string s to M[m]            | `<n> bytes written to M`              |

All errors output: `error`

## Technical Details

### Constraints

- Block size: 512 bytes
- Maximum file size: 1,536 bytes (3 blocks)
- Maximum open files: 4 (including directory)
- Maximum files: 191 (descriptor 0 reserved for directory)
- File name length: 1-4 characters

### Key Design Decisions

1. **Reserved Cache**: Blocks 0-6 (bitmap + descriptors) are cached in memory to minimize disk reads during frequent metadata operations.

2. **Directory as File**: The directory is implemented as a regular file at OFT index 0, using the same read/write mechanisms as user files.

3. **Memory Preservation**: Directory operations save and restore user memory M to prevent internal operations from corrupting user data.

4. **Buffered I/O**: Each open file maintains a 512-byte buffer. Block swapping occurs transparently when read/write operations cross block boundaries.

## Example

**input.txt:**

```
in
cr foo
op foo
wm 0 Hello
wr 1 0 5
sk 1 0
rd 1 10 5
rm 10 5
cl 1
dr
```

**output.txt:**

```
system initialized
foo created
foo opened 1
5 bytes written to M
5 bytes written to 1
position is 0
5 bytes read from 1
Hello
1 closed
foo 5
```

## Building from Source

Requires Rust 1.70+ (tested on 1.90).

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```
