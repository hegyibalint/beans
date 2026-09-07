use std::path::PathBuf;

#[derive(Default)]
pub struct Classpath {
    elements: Vec<ClasspathElement>,
}

pub struct ClasspathElement {
    path: PathBuf,
    hash: blake3::Hash,
}
