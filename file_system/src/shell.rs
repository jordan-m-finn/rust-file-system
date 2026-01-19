use crate::fs::FileSystem;

// process a single command and return the output string
pub fn process_command(fs: &mut FileSystem, line: &str) -> String {
    let line = line.trim();

    // skip empty lines
    if line.is_empty() {
        return String::new(); 
    }

    // parse the command and arguments
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return String::new();
    }

    let command = parts[0];
    let args = &parts[1..];

    // execute the command
    match command {
        "in" => cmd_init(fs),
        "cr" => cmd_create(fs, args),
        "de" => cmd_destroy(fs, args),
        "op" => cmd_open(fs, args),
        "cl" => cmd_close(fs, args),
        "rd" => cmd_read(fs, args),
        "wr" => cmd_write(fs, args),
        "sk" => cmd_seek(fs, args),
        "dr" => cmd_directory(fs),
        "rm" => cmd_read_memory(fs, args),
        "wm" => cmd_write_memory(fs, args),
        _ => "error".to_string(),
    }
}

// initialize the file system
fn cmd_init(fs: &mut FileSystem) -> String {
    fs.init();
    "system initialized".to_string()
}

// create a new file
fn cmd_create(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 1 {
        return "error".to_string();
    }

    let name = args[0];

    match fs.create(name) {
        Ok(()) => format!("{} created", name),
        Err(_) => "error".to_string(),
    }
}

// destroy a file
fn cmd_destroy(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 1 {
        return "error".to_string();
    }

    let name = args[0];

    match fs.destroy(name) {
        Ok(()) => format!("{} destroyed", name),
        Err(_) => "error".to_string(),
    }
}

// open a file
fn cmd_open(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 1 {
        return "error".to_string();
    }

    let name = args[0];

    match fs.open(name) {
        Ok(index) => format!("{} opened {}", name, index),
        Err(_) => "error".to_string(),
    }
}

// close a file
fn cmd_close(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 1 {
        return "error".to_string();
    }

    let index: usize = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    match fs.close(index) {
        Ok(()) => format!("{} closed", index),
        Err(_) => "error".to_string(),
    }
}

// read from a file
fn cmd_read(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 3 {
        return "error".to_string();
    }

    let index: usize = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    let mem_pos: usize = match args[1].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    let count: usize = match args[2].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    match fs.read(index, mem_pos, count) {
        Ok(bytes_read) => format!("{} bytes read from {}", bytes_read, index),
        Err(_) => "error".to_string(),
    }
}

// write to a file
fn cmd_write(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 3 {
        return "error".to_string();
    }

    let index: usize = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    let mem_pos: usize = match args[1].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    let count: usize = match args[2].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    match fs.write(index, mem_pos, count) {
        Ok(bytes_written) => format!("{} bytes written to {}", bytes_written, index),
        Err(_) => "error".to_string(),
    }
}

// seek within a file
fn cmd_seek(fs: &mut FileSystem, args: &[&str]) -> String {
    if args.len() != 2 {
        return "error".to_string();
    }

    let index: usize = match args[0].parse() {
        Ok(n) => n,
        Err(_) => return "error".to_string(),
    };

    let pos: usize = match args[1].parse() {
        Ok(n) => n,
        Err(_) => "error".to_string(),
    };

    match fs.seek(index, pos) {
        Ok(()) => format!("position is {}", pos),
        Err(_) => "error".to_string(),
    }
}
