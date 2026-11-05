use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering::SeqCst},
        mpmc::{Receiver, Sender},
    },
    time::Duration,
};

use regex::Regex;

use crate::{file::SourceFile, path::SourcePath};

pub fn dependencies_worker(
    source_file_list_receiver: Receiver<SourceFile>,
    source_file_list_sender: Sender<SourceFile>,
    general_busyness: Arc<AtomicU8>,
    already_knowned_source_path: Arc<Mutex<HashSet<SourcePath>>>,
    regex_matcher: &Regex,
) -> Vec<SourceFile> {
    let mut local_source_file_list = Vec::new();
    loop {
        // Verify if the worker needs to wait or stop
        if general_busyness.load(SeqCst) == 0 {
            break;
        }

        let mut included_path_list =
            match source_file_list_receiver.recv_timeout(Duration::new(1, 0)) {
                Ok(mut source_file) => {
                    println!("updating_deps: {:?}", source_file.get_source_path().path());
                    match source_file.update_dependencies(regex_matcher) {
                        Ok(list) => list,
                        Err(err) => {
                            eprintln!("{err}");
                            continue;
                        }
                    }
                }
                Err(_err) => {
                    continue;
                }
            };

        let mut already_knowned_source_path_hashset = match already_knowned_source_path.lock() {
            Ok(list) => list,
            Err(poisonned_guard) => poisonned_guard.into_inner(),
        };

        included_path_list.retain(|x| !already_knowned_source_path_hashset.contains(&x.0));

        included_path_list.iter().for_each(|x| {
            already_knowned_source_path_hashset.insert(x.0.clone());
        });
        drop(already_knowned_source_path_hashset);

        let included_source_file_list: Vec<SourceFile> = included_path_list
            .iter()
            .flat_map(|(source_path, header_path)| {
                SourceFile::try_new(source_path, &Some(header_path.clone()))
            })
            .collect();

        included_source_file_list.iter().for_each(|x| {
            match source_file_list_sender.send(x.clone()) {
                Err(err) => {
                    eprintln!("{}", err);
                }
                Ok(_) => {
                    general_busyness.fetch_add(1, SeqCst);
                }
            };
        });

        included_source_file_list.iter().for_each(|x| {
            local_source_file_list.push(x.clone());
        });

        general_busyness.fetch_sub(1, SeqCst);
    }
    return local_source_file_list;
}
