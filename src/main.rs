use std::path::PathBuf;

use myke::{
    func::{compile, compute_dependencies, compute_need_rebuild_path, link},
    path::SourcePath,
};

fn main() {
    let input_path = SourcePath::from(PathBuf::from("/home/jonas/GEII/exos/9/main.cpp"));
    let debug_mode = true;

    let Ok((mut source_file_list, binary_path)) = compute_dependencies(input_path, debug_mode)
    else {
        println!("Can't find file");
        return;
    };

    let (build_path_list, all_last_modified) = compute_need_rebuild_path(&mut source_file_list);

    compile(build_path_list, debug_mode);

    match binary_path.path().metadata() {
        Ok(metadata) => match metadata.modified() {
            Ok(binary_last_modified) if binary_last_modified >= all_last_modified => {
                println!("No need to link again")
            }
            _ => {
                link(&source_file_list, binary_path);
            }
        },
        Err(_) => link(&source_file_list, binary_path),
    }
}
