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
