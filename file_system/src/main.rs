mod constants;
mod disk;
mod bitmap;
mod byte_utils;
mod descriptor;
mod oft;
mod fs;
mod shell;

use std::env;
use std::fs as std_fs;
use std::io::Write;

use fs::FileSystem;
use shell::{run_interactive, run_from_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut file_system = FileSystem::new();

    if args.len() < 2 {
        eprintln!("Usage: {} <input_file> [output_file]", args[0]);
        return;
    }

    let input_file = &args[1];
    let output_file = if args.len() > 2 {
        args[2].clone()
    } else {
        "output.txt".to_string() 
    };
    
    // Read input file
    let input = match std_fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_file, e);
            return;
        }
    };
               
    // Process commands
    let output = run_from_file(&mut file_system, &input);
        
    // Write output file
    match std_fs::File::create(&output_file) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(output.as_bytes()) {
                eprintln!("Error writing {}: {}", output_file, e);
            }
        }
        Err(e) => {
            eprintln!("Error creating {}: {}", output_file, e);
        }
    }
}
