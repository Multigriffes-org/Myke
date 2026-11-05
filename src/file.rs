use regex::Regex;
use std::{
    ffi::{OsStr, OsString},
    fs::read_to_string,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::path::{HeaderPath, ObjectPath, SourcePath};

#[derive(Clone)]
pub struct SourceFile {
    file_name: OsString,
    file_stem: OsString,
    source_path: SourcePath,
    header_path: Option<HeaderPath>,
    last_modified_combined: SystemTime,
    object_path: Option<ObjectPath>,
    last_modified_object: SystemTime,
    dependencies: Vec<(SourcePath, HeaderPath)>,
}

impl SourceFile {
    fn new(
        file_name: OsString,
        file_stem: OsString,
        source_path: SourcePath,
        header_path: Option<HeaderPath>,
    ) -> Self {
        let result = SourceFile {
            file_name,
            file_stem,
            source_path,
            header_path,
            last_modified_combined: SystemTime::now(),
            object_path: None,
            last_modified_object: UNIX_EPOCH,
            dependencies: Vec::new(),
        };
        result
    }

    pub fn update_object_path(&mut self, debug_mode: bool) {
        let target_dir = if debug_mode {
            Path::new("target/debug/")
        } else {
            Path::new("target/release/")
        };

        if let Some(parent) = self.source_path.path().parent() {
            let object_path = parent
                .join(target_dir)
                .join(&self.file_stem)
                .with_added_extension("o");

            self.object_path = Some(ObjectPath::new(object_path))
        };
    }

    pub fn update_last_modified(&mut self) -> SystemTime {
        let mut last_modified_combined;

        match self.source_path.path().metadata() {
            Err(_) => last_modified_combined = SystemTime::now(),
            Ok(metadata) => match metadata.modified() {
                Err(_) => last_modified_combined = SystemTime::now(),
                Ok(time) => last_modified_combined = time,
            },
        }
        match self.header_path {
            Some(ref header_path) => match header_path.path().metadata() {
                Ok(metadata) => match metadata.modified() {
                    Err(_) => {}
                    Ok(time) => {
                        if time > last_modified_combined {
                            last_modified_combined = time;
                        }
                    }
                },
                Err(_) => {}
            },
            None => {}
        }
        self.last_modified_combined = last_modified_combined;

        match self.object_path {
            Some(ref object_path) => match object_path.path().metadata() {
                Err(_) => self.last_modified_object = UNIX_EPOCH,
                Ok(metadata) => match metadata.modified() {
                    Err(_) => self.last_modified_object = UNIX_EPOCH,
                    Ok(time) => self.last_modified_object = time,
                },
            },
            None => self.last_modified_object = UNIX_EPOCH,
        }

        if self.last_modified_combined >= self.last_modified_object {
            self.last_modified_combined
        } else {
            self.last_modified_object
        }
    }

    pub fn update_dependencies(
        &mut self,
        regex_matcher: &Regex,
    ) -> Result<Vec<(SourcePath, HeaderPath)>, io::ErrorKind> {
        let file_content = match read_to_string(&self.source_path.path()) {
            Ok(content) => content,
            Err(err) => return Err(err.kind()),
        };

        for line in file_content.lines() {
            if let Some(captures) = regex_matcher.captures(line) {
                // dbg!(&captures.get(0).unwrap());
                if let Some(captured_match) = captures.get(1) {
                    let mut captured_path = PathBuf::from(&captured_match.as_str());
                    // dbg!(&captured_path);

                    if !captured_path.is_absolute() {
                        let Some(parent) = self.source_path.path().parent() else {
                            continue;
                        };
                        captured_path = parent.join(captured_path);
                    }

                    match captured_path.canonicalize() {
                        Ok(_captured_path) => captured_path = _captured_path,
                        Err(_) => {
                            eprintln!(
                                "Header file note found, ignored : {:?}\nDependencie of : {:?}",
                                captured_path,
                                self.source_path.path()
                            );
                            continue;
                        }
                    }

                    let source_path = match captured_path.extension() {
                        Some(extension) if extension == OsStr::new("hpp") => {
                            SourcePath::from(captured_path.with_extension("cpp"))
                        }
                        Some(extension) if extension == OsStr::new("h") => {
                            SourcePath::from(captured_path.with_extension("c"))
                        }
                        None | Some(_) => {
                            eprintln!(
                                "No valide extension : {:?}\nDependencie of : {:?}",
                                captured_path,
                                self.source_path.path()
                            );
                            continue;
                        }
                    };
                    let header_path = HeaderPath::from(captured_path);

                    self.dependencies.push((source_path, header_path));
                }
            }
        }

        self.dependencies.sort_unstable();
        self.dependencies.dedup();

        return Ok(self.dependencies.clone());
    }

    pub fn should_be_built(&self) -> bool {
        self.last_modified_combined > self.last_modified_object
    }
    pub fn get_object_path(&self) -> Option<ObjectPath> {
        self.object_path.clone()
    }
    pub fn get_object_path_ref(&self) -> &Option<ObjectPath> {
        &self.object_path
    }
    pub fn get_source_path(&self) -> SourcePath {
        self.source_path.clone()
    }
}

impl SourceFile {
    pub fn try_new(
        source_path: &SourcePath,
        header_path_option: &Option<HeaderPath>,
    ) -> Result<Self, io::ErrorKind> {
        match header_path_option {
            Some(header_path) => {
                if !header_path.path().exists() {
                    eprintln!("Header file doesn't exists {:?}", header_path.path());
                    return Err(ErrorKind::NotFound);
                };
            }
            None => {}
        }

        if !source_path.path().exists() {
            eprintln!("Source file doesn't exists {:?}", source_path.path());
            return Err(ErrorKind::NotFound);
        };

        let file_name = match source_path.path().file_name() {
            None => "".into(),
            Some(name) => name.to_owned(),
        };
        let file_stem = match source_path.path().file_stem() {
            None => "".into(),
            Some(stem) => stem.to_owned(),
        };

        Ok(SourceFile::new(
            file_name,
            file_stem,
            source_path.clone(),
            header_path_option.clone(),
        ))
    }
}
