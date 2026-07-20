pub mod analysis;
pub mod file;
pub mod language;
pub mod model;
pub mod storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct EntryId(usize);

pub struct VirtualFile {
    pub uri: String,
    pub contents: String,
}
