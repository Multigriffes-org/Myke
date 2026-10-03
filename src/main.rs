use myke::*;
use std::{env, path::PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: myke [OPTIONS] <file_path>");
        std::process::exit(1);
    }

    let mut filepath_index = 1;
    let mut debug_mode = false;

    for i in 1..args.len() {
        match args[i].as_str() {
            "-g" => debug_mode = true,
            _ => {
                filepath_index = i;
                break;
            }
        }
    }
    //
    //
    //
    //
    let bin_file = PathBuf::from(args.get(filepath_index).unwrap());
    let mut included_files: Vec<PathBuf> = Vec::new();
    included_files.push(bin_file.clone());

    find_included_files_recursive(bin_file.clone(), &mut included_files);

    let builded_files = build_obj(&included_files, debug_mode);

    build(bin_file, &builded_files, debug_mode);
}
