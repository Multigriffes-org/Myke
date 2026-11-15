use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};

use crossbeam::queue::SegQueue;
use regex::Regex;

use crate::{file::SourceFile, path::SourcePath};

/// Worker that process the queue and return all the created SourceFile and if an error occured
/// TODO: return a proper Error type instead of bool
/// TODO: duplicate in the queue aren't checked. It's checked when the SourceFile update it's dependencies
pub fn dependencies_worker(
    source_file_list: Arc<SegQueue<SourceFile>>,
    task_counter: Arc<AtomicU32>,
    already_knowned_source_path: Arc<Mutex<HashSet<SourcePath>>>,
    regex_matcher: &Regex,
) -> (Vec<SourceFile>, bool) {
    let mut worker_source_file_list = Vec::new();
    let mut error = false;
    loop {
        // Verify if the worker needs to wait or stop
        if task_counter.load(SeqCst) == 0 {
            break;
        }

        // Take a dependencie tuple from the queue and update its dependencies
        let mut included_path_list = match source_file_list.pop() {
            Some(mut source_file) => {
                println!(
                    "Dependencie found: {:?}",
                    source_file.get_source_path().path()
                );

                match source_file.update_dependencies(regex_matcher) {
                    Ok(dependencie_tuple) => dependencie_tuple,
                    Err(err) => {
                        eprintln!("{err}");
                        error = true;
                        continue;
                    }
                }
            }
            None => {
                continue;
            }
        };

        // Make the guard drop after use
        {
            let mut already_knowned_source_path_hashset = match already_knowned_source_path.lock() {
                Ok(already_knowned_source_path_hashset) => already_knowned_source_path_hashset,
                Err(poisonned_guard) => poisonned_guard.into_inner(),
            };

            // Filter the headers of the dependencie
            included_path_list.retain(|depenencie_tuple| {
                !already_knowned_source_path_hashset.contains(&depenencie_tuple.0)
            });

            // Add the new ones to the knowed list
            included_path_list.iter().for_each(|depenencie_tuple| {
                already_knowned_source_path_hashset.insert(depenencie_tuple.0.clone());
            });
        }

        // Create corresponding SourceFile from dependencie tuple
        let included_source_file_list: Vec<SourceFile> = included_path_list
            .into_iter()
            .flat_map(|(source_path, header_path)| {
                match SourceFile::try_new(&source_path, &Some(header_path)) {
                    Ok(source_file) => Ok(source_file),
                    Err(err) => {
                        eprintln!("{err}");
                        error = true;
                        Err(err)
                    }
                }
            })
            .collect();

        // Add new tasks
        included_source_file_list
            .into_iter()
            .for_each(|source_file| {
                source_file_list.push(source_file.clone());
                task_counter.fetch_add(1, SeqCst);
                worker_source_file_list.push(source_file);
            });

        // Finish task
        task_counter.fetch_sub(1, SeqCst);
    }
    return (worker_source_file_list, error);
}
