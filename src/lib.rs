use mystd::remove_elements_in;
use regex::regex;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
    time::SystemTime,
};

/// Take a list of PathBuf pointing at .c or .cpp files
/// Update there corresponding .o in their target/{release/debug}/ directorie
/// Return all the corresponding .o files regardless of whether they have been updated or not
pub fn build_obj(source_files: &Vec<PathBuf>, debug_mode: bool) -> Vec<PathBuf> {
    let mut builded_files = Vec::new();
    // dbg!("Build obj !!!");

    for path in source_files {
        let dir = path.parent().unwrap();
        // dbg!(&dir);
        let stem = path.file_stem().unwrap().to_str().unwrap();
        // dbg!(&stem);
        let name = path.file_name().unwrap().to_str().unwrap();
        // dbg!(&name);
        let target_dir = if debug_mode {
            Path::new("target/debug/")
        } else {
            Path::new("target/release/")
        };
        // dbg!(&target_dir);
        let mut obj_path = dir.join(target_dir).join(stem);
        obj_path.add_extension("o");
        // dbg!(&obj_path);

        if should_be_built(path, &obj_path) {
            println!("Building : {}", obj_path.to_str().unwrap());
            let output = process::Command::new("mkdir")
                .arg("-p")
                .arg(obj_path.parent().unwrap().to_str().unwrap())
                .output()
                .unwrap();
            io::stdout().write_all(&output.stdout).unwrap();
            io::stdout().write_all(&output.stderr).unwrap();

            let mut command = process::Command::new("g++");
            if debug_mode {
                command.arg("-g");
            }
            let output = command
                .arg("-c")
                .arg(format!("{}", dir.join(name).to_str().unwrap()))
                .arg("-o")
                .arg(format!("{}", obj_path.to_str().unwrap()))
                .output()
                .unwrap();

            io::stdout().write_all(&output.stdout).unwrap();
            io::stdout().write_all(&output.stderr).unwrap();
        }
        builded_files.push(obj_path);
    }

    return builded_files;
}

fn should_be_built(source_path: &PathBuf, obj_path: &PathBuf) -> bool {
    let obj_last_modified = match obj_path.metadata() {
        Err(_) => SystemTime::UNIX_EPOCH,
        Ok(metadata) => match metadata.modified() {
            Err(_) => SystemTime::UNIX_EPOCH,
            Ok(date) => date,
        },
    };

    let source_last_modified = match source_path.metadata() {
        Err(err) => {
            println!("Problem reading file metadata: {}", err);
            SystemTime::now()
        }
        Ok(metadata) => match metadata.modified() {
            Err(err) => {
                println!("Problem getting last modified date: {err}");
                SystemTime::now()
            }
            Ok(date) => {
                // dbg!(date);
                date
            }
        },
    };
    return source_last_modified > obj_last_modified;
}

/// Take a list of PathBuf and pass them to gcc with the correct output dir
pub fn build(bin_file: PathBuf, builded_files: &Vec<PathBuf>, debug_mode: bool) {
    let mut command = process::Command::new("g++");
    for obj in builded_files {
        command.arg(obj.to_str().unwrap());
    }

    let target_dir = if debug_mode {
        Path::new("target/debug/")
    } else {
        Path::new("target/release/")
    };

    // dbg!("Build !!!");
    // dbg!(&target_dir);
    let dir = bin_file.parent().unwrap();
    // dbg!(&dir);
    let stem = bin_file.file_stem().unwrap();
    // dbg!(&stem);
    let bin_path = dir.join(target_dir).join(stem);
    // dbg!(&bin_path);
    println!("Linking : {}", bin_path.to_str().unwrap());

    command.arg("-o").arg(bin_path.to_str().unwrap());

    let output = command.output().unwrap();
    io::stdout().write_all(&output.stdout).unwrap();
    io::stdout().write_all(&output.stderr).unwrap();
}

/// Recursively get all included header in all the file if the correspding .c/.cpp exist
/// without duplicates
fn find_included_files_relative(path: &PathBuf) -> Vec<PathBuf> {
    match (path.try_exists(), path.is_file()) {
        (Err(err), _) => {
            println!("Can't reach path : {:?}", err);
            std::process::exit(1);
        }
        (Ok(false), _) => {
            println!("Path doesn't exists");
            std::process::exit(1);
        }
        (Ok(true), false) => {
            println!("Provided path isn't a file");
            std::process::exit(1);
        }
        (Ok(true), true) => {}
    };

    let source_file = match fs::read_to_string(&path) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file: {}", err);
            std::process::exit(1)
        }
    };

    let regex_matcher = regex!(r#"^#include\ *\"((/?[\w\-. ]+)+\.h)\"\ *$"#);

    let mut local_included_files: Vec<PathBuf> = Vec::new();

    for line in source_file.lines() {
        if let Some(captures) = regex_matcher.captures(line) {
            // dbg!(&captures.get(0).unwrap());
            if let Some(captured_match) = captures.get(1) {
                // dbg!(&captured_match);

                let mut captured_path = PathBuf::from(&captured_match.as_str().to_string());
                if !captured_path.is_absolute() {
                    captured_path = path.parent().unwrap().join(captured_path);
                }
                captured_path = captured_path.canonicalize().unwrap();

                if captured_path.with_extension("cpp").is_file() {
                    captured_path = captured_path.with_extension("cpp")
                } else if captured_path.with_extension("c").is_file() {
                    captured_path = captured_path.with_extension("c")
                } else {
                    continue;
                }

                local_included_files.push(captured_path);
                // dbg!(&local_included_files);
                //
            } else {
                continue;
            }
        } else {
            continue;
        }
    }
    return local_included_files;
}

pub fn find_included_files_recursive(path: PathBuf, included_files: &mut Vec<PathBuf>) {
    let mut local_included_files = find_included_files_relative(&path);
    // dbg!(&local_included_files);

    remove_elements_in(&mut local_included_files, included_files);
    // dbg!(&local_included_files);

    let mut temp = local_included_files.clone();
    included_files.append(&mut temp);
    // dbg!(&included_files);

    for included_file in local_included_files {
        find_included_files_recursive(included_file, included_files);
    }
}
