use std::{
    collections::HashSet,
    io::{self, ErrorKind, Write, stderr, stdout},
    process::Command,
    sync::{Arc, Mutex, atomic::AtomicU8},
    thread::{available_parallelism, spawn},
    time::{SystemTime, UNIX_EPOCH},
};

use crossbeam::queue::SegQueue;
use regex::regex;

use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    file::SourceFile,
    path::{BinaryPath, ObjectPath, SourcePath},
    worker::dependencies_worker,
};

pub fn compute_dependencies(
    initial_source_path: SourcePath,
    debug_mode: bool,
) -> Result<(Vec<SourceFile>, BinaryPath), io::ErrorKind> {
    let mut main_file = SourceFile::try_new(&initial_source_path, &None)?;
    main_file.update_object_path(debug_mode);

    let binary_path = BinaryPath::new(match main_file.get_object_path_ref() {
        None => {
            eprintln!("Can't find binary path");
            return Err(ErrorKind::NotFound);
        }
        Some(object_path) => object_path.path().with_extension(""),
    });
    let regex_matcher = regex!(r#"^#include[\t ]*"((?:/?[\w\-. ]+)+\.(?:hpp|h))"[\t ]*$"#);

    let mut general_source_file_list = vec![main_file.clone()];
    let already_knowned_source_path = Arc::new(Mutex::new(HashSet::from([initial_source_path])));
    let general_busyness = Arc::new(AtomicU8::new(1));

    let source_file_queue = Arc::new(SegQueue::new());
    source_file_queue.push(main_file);

    let num_worker = match available_parallelism() {
        Ok(num) => num.get(),
        Err(_) => 1,
    };
    // dbg!(num_worker);

    let mut join_handles = Vec::new();
    for _ in 0..num_worker {
        let _source_file_queue = source_file_queue.clone();
        let _general_busyness = general_busyness.clone();
        let _already_knowned_source_path = already_knowned_source_path.clone();

        join_handles.push(spawn(move || {
            dependencies_worker(
                _source_file_queue,
                _general_busyness,
                _already_knowned_source_path,
                regex_matcher,
            )
        }));
    }

    join_handles
        .into_iter()
        .for_each(|handle| match handle.join() {
            Ok(ref mut worker_source_file_list) => {
                worker_source_file_list
                    .iter_mut()
                    .for_each(|source_file| source_file.update_object_path(debug_mode));
                general_source_file_list.append(worker_source_file_list);
            }
            Err(err) => {
                eprintln!("{:?}", err);
            }
        });

    return Ok((general_source_file_list, binary_path));
}

pub fn update_object_path(source_file_list: &mut Vec<SourceFile>, debug_mode: bool) {
    source_file_list
        .par_iter_mut()
        .for_each(|source_file| source_file.update_object_path(debug_mode));
}

pub fn compute_need_rebuild_path(
    source_file_list: &mut Vec<SourceFile>,
) -> (Vec<(SourcePath, Option<ObjectPath>)>, SystemTime) {
    let last_modification_of_all: SystemTime = source_file_list
        .par_iter_mut()
        .map(|x| x.update_last_modified())
        .reduce(
            || UNIX_EPOCH,
            |acc, next| if next > acc { next } else { acc },
        );

    let paths_list: Vec<(SourcePath, Option<ObjectPath>)> = source_file_list
        .par_iter()
        .filter_map(|source_file| {
            if source_file.should_be_built() {
                Some((source_file.get_source_path(), source_file.get_object_path()))
            } else {
                None
            }
        })
        .collect();

    (paths_list, last_modification_of_all)
}

pub fn compile(path_list: Vec<(SourcePath, Option<ObjectPath>)>, debug_mode: bool) -> bool {
    path_list
        .par_iter()
        .map(|tuple| compile_tuple(tuple, debug_mode))
        .reduce(|| false, |acc, x| acc | x)
}

fn compile_tuple(tuple: &(SourcePath, Option<ObjectPath>), debug_mode: bool) -> bool {
    match tuple {
        (source_path, Some(object_path)) => {
            let mut mkdir = Command::new("mkdir");
            mkdir.arg("-p");
            match object_path.path().parent() {
                None => {}
                Some(path) => {
                    mkdir.arg(path);
                }
            };
            if let Ok(output) = mkdir.output() {
                let _ = io::stdout().write_all(&output.stdout);
                let _ = io::stdout().write_all(&output.stderr);
            };

            let mut gcc = Command::new("g++");
            if debug_mode {
                gcc.arg("-g");
            }
            match gcc
                .arg("-Wall")
                .arg("-Wextra")
                .arg("-Wpedantic")
                .arg("-c")
                .arg(source_path.path())
                .arg("-o")
                .arg(object_path.path())
                .output()
            {
                Err(err) => {
                    eprintln!("{}", err);
                    true
                }
                Ok(output) => {
                    println!("Compiling : {:?}", object_path.path());
                    let _ = stdout().write_all(&output.stdout);
                    let _ = stderr().write_all(&output.stderr);
                    output.status.success()
                }
            }
        }
        (source_path, None) => {
            eprintln!("No object for {:?}", source_path.path());
            true
        }
    }
}

pub fn link(source_file_list: &Vec<SourceFile>, binary_path: &BinaryPath) -> i32 {
    let mut mkdir = Command::new("mkdir");
    mkdir.arg("-p");
    match binary_path.path().parent() {
        None => {}
        Some(path) => {
            // dbg!(path);
            mkdir.arg(path);
        }
    };
    if let Ok(output) = mkdir.output() {
        let _ = io::stdout().write_all(&output.stdout);
        let _ = io::stdout().write_all(&output.stderr);
    };

    let mut gcc = Command::new("g++");
    for source_file in source_file_list {
        match source_file.get_object_path() {
            Some(object_path) => {
                gcc.arg(object_path.path());
            }
            None => {
                eprintln!("No object for {:?}", source_file.get_source_path().path());
                continue;
            }
        }
    }
    gcc.arg("-o").arg(binary_path.path().clone());

    match gcc.output() {
        Err(err) => {
            eprintln!("{}", err);
            1
        }
        Ok(output) => {
            println!("Linking : {:?}", binary_path.path());
            let _ = stdout().write_all(&output.stdout);
            let _ = stderr().write_all(&output.stderr);
            match output.status.code() {
                None => 1,
                Some(code) => code,
            }
        }
    }
}
