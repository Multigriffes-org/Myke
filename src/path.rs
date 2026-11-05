use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath(PathBuf);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderPath(PathBuf);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectPath(PathBuf);
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinaryPath(PathBuf);

impl ObjectPath {
    pub fn new(path: PathBuf) -> Self {
        Self { 0: path }
    }
    pub fn path(&self) -> &PathBuf {
        &self.0
    }
    pub fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.0
    }
}
impl HeaderPath {
    pub fn new(path: PathBuf) -> Self {
        Self { 0: path }
    }
    pub fn path(&self) -> &PathBuf {
        &self.0
    }
    pub fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.0
    }
}
impl BinaryPath {
    pub fn new(path: PathBuf) -> Self {
        Self { 0: path }
    }
    pub fn path(&self) -> &PathBuf {
        &self.0
    }
    pub fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.0
    }
}
impl SourcePath {
    pub fn new(path: PathBuf) -> Self {
        Self { 0: path }
    }
    pub fn path(&self) -> &PathBuf {
        &self.0
    }
    pub fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.0
    }
}

impl From<PathBuf> for SourcePath {
    fn from(value: PathBuf) -> Self {
        Self { 0: value }
    }
}
impl From<PathBuf> for HeaderPath {
    fn from(value: PathBuf) -> Self {
        Self { 0: value }
    }
}
impl From<PathBuf> for BinaryPath {
    fn from(value: PathBuf) -> Self {
        Self { 0: value }
    }
}
impl From<PathBuf> for ObjectPath {
    fn from(value: PathBuf) -> Self {
        Self { 0: value }
    }
}
