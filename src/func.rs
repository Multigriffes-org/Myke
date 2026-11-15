use std::{
    collections::HashSet,
    fs::create_dir_all,
    io::{self, ErrorKind},
    process::Command,
    sync::{Arc, Mutex, atomic::AtomicU32},
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

/// Fonction that spawn the processes working on the dependencie queue
///
/// Each worker will take work from the queue and increaze the task_counter if they encounter a new dependencie
pub fn compute_dependencies(
    initial_source_file: SourceFile,
    debug_mode: bool,
) -> Result<(Vec<SourceFile>, BinaryPath), io::ErrorKind> {
    let binary_path = BinaryPath::new(match initial_source_file.get_object_path_ref() {
        None => {
            eprintln!("Can't find binary path");
            return Err(ErrorKind::NotFound);
        }
        Some(object_path) => object_path.path().with_extension(""),
    });
    let mut general_source_file_list = vec![initial_source_file.clone()];
    let mut dependencie_error = false;

    // Initializes shared objects
    let regex_matcher = regex!(r#"^#include[\t ]*"((?:/?[\w\-. ]+)+\.(?:hpp|h))"[\t ]*$"#);
    let already_knowned_source_path = Arc::new(Mutex::new(HashSet::from([
        initial_source_file.get_source_path()
    ])));
    let task_counter = Arc::new(AtomicU32::new(1));
    let source_file_queue = Arc::new(SegQueue::new());
    source_file_queue.push(initial_source_file);

    let num_worker = match available_parallelism() {
        Ok(num) => num.get(),
        Err(_) => 1,
    };

    #[cfg(debug_assertions)]
    dbg!(num_worker);

    // Spawn workers
    let mut join_handles = Vec::new();
    for _ in 0..num_worker {
        let _source_file_queue = source_file_queue.clone();
        let _task_counter = task_counter.clone();
        let _already_knowned_source_path = already_knowned_source_path.clone();

        join_handles.push(spawn(move || {
            dependencies_worker(
                _source_file_queue,
                _task_counter,
                _already_knowned_source_path,
                regex_matcher,
            )
        }));
    }

    // Join workers and collect error
    for handle in join_handles {
        match handle.join() {
            Ok((ref mut worker_source_file_list, worker_error)) => {
                dependencie_error |= worker_error;
                general_source_file_list.append(worker_source_file_list);
            }
            Err(err) => {
                eprintln!("{:?}", err);
            }
        }
    }

    general_source_file_list
        .par_iter_mut()
        .for_each(|source_file| source_file.update_object_path(debug_mode));

    if dependencie_error {
        return Err(ErrorKind::NotFound);
    } else {
        return Ok((general_source_file_list, binary_path));
    }
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

pub fn compile(
    path_list: Vec<(SourcePath, Option<ObjectPath>)>,
    debug_mode: bool,
) -> Result<(), ()> {
    if path_list
        .par_iter()
        .map(|tuple| compile_tuple(tuple, debug_mode).is_err())
        .reduce(|| false, |acc, x| acc | x)
    {
        Err(())
    } else {
        Ok(())
    }
}

fn compile_tuple(tuple: &(SourcePath, Option<ObjectPath>), debug_mode: bool) -> Result<(), ()> {
    match tuple {
        (source_path, Some(object_path)) => {
            if let Some(parent) = object_path.path().parent() {
                let _ = create_dir_all(parent);
            };

            let mut gcc = Command::new("g++");
            if debug_mode {
                gcc.arg("-g");
            } else {
                gcc.arg("-O2");
            }

            gcc.arg("-Wall")
                .arg("-Wextra")
                .arg("-Wpedantic")
                .arg("-c")
                .arg(source_path.path())
                .arg("-o")
                .arg(object_path.path());

            println!("Compiling : {:?}", object_path.path());

            match gcc.status() {
                Err(err) => {
                    eprintln!("{err}");
                    Err(())
                }
                Ok(exit_status) => {
                    if exit_status.success() {
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            }
        }
        (source_path, None) => {
            eprintln!("No object path for {:?}", source_path.path());
            Err(())
        }
    }
}

pub fn link(source_file_list: &Vec<SourceFile>, binary_path: &BinaryPath) -> i32 {
    if let Some(parent) = binary_path.path().parent() {
        let _ = create_dir_all(parent);
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
    gcc.arg("-o").arg(binary_path.path());

    println!("Linking : {:?}", binary_path.path());
    match gcc.status() {
        Err(err) => {
            eprintln!("{err}");
            1
        }
        Ok(exit_status) => match exit_status.code() {
            None => 1,
            Some(code) => code,
        },
    }
}
