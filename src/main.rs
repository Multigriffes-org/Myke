use std::{env, path::PathBuf, process::Command};

use myke::{
    file::SourceFile,
    func::{compile, compute_dependencies, compute_need_rebuild_path, link},
    path::SourcePath,
};
fn print_help() {
    println!("Usage: myke (build|run) [Option] <File>");
    println!("Option:");
    println!("\t-r --release");
    println!("\t-g --debug");
    println!("By default it's a debug build");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_help();
        return;
    }

    let mut run_mode = false;
    let mut debug_mode = true;
    let mut path_index = 0;

    match args.get(1) {
        None => {
            print_help();
            return;
        }
        Some(mode) => match mode.as_str() {
            "build" => {}
            "run" => run_mode = true,
            _ => {
                print_help();
                return;
            }
        },
    }

    for i in 2..args.len() {
        match args.get(i) {
            None => {
                print_help();
                return;
            }
            Some(arg) => match arg.as_str() {
                "-r" | "--release" => debug_mode = false,
                "-g" | "--debug" => debug_mode = true,
                _ => {
                    path_index = i;
                    break;
                }
            },
        }
    }

    if debug_mode {
        println!("Debug mode");
    } else {
        println!("Release mode");
    }

    if path_index == 0 {
        print_help();
        return;
    }

    //_________________________________________
    let input_path = match PathBuf::from(&args[path_index].trim_matches('"')).canonicalize() {
        Ok(input_path) => SourcePath::from(input_path),
        Err(_) => {
            eprintln!("Can't find initial path, stopping...");
            return;
        }
    };
    let Ok(mut main_file) = SourceFile::try_new(&input_path, &None) else {
        eprintln!("Can't find initial path, stopping...");
        return;
    };
    main_file.update_object_path(debug_mode);

    let Ok((mut source_file_list, binary_path)) = compute_dependencies(main_file, debug_mode)
    else {
        eprintln!("Error in dependencies searching, stopping...");
        return;
    };

    let (build_path_list, all_last_modified) = compute_need_rebuild_path(&mut source_file_list);


    if compile(need_build_tuple_list, debug_mode).is_err() {
        eprintln!("Error in compilation process, stopping...");
        return;
    }

    let link_exit_code = match binary_path.path().metadata() {
        Ok(metadata) => match metadata.modified() {
            Ok(binary_last_modified) if binary_last_modified >= all_last_modified => {
                println!("No need to link again");
                0
            }
            _ => link(&source_file_list, &binary_path),
        },
        Err(_) => link(&source_file_list, &binary_path),
    };

    if link_exit_code == 0 && run_mode {
        let mut run_command = Command::new(&binary_path.path());
        run_command.args(&args[path_index + 1..]);
        let _ = run_command.spawn();
    }
}
