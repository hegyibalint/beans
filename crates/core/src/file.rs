struct VirtualFile {
    pub uri: String,
    pub content: String,
}

impl VirtualFile {
    pub fn content_hash(&self) -> u64 {
        todo!()
    }
}
