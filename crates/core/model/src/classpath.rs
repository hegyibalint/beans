use std::path::PathBuf;

pub struct Classpath {
    elements: Vec<ClasspathElement>,
}

pub struct ClasspathElement {
    path: PathBuf,
    hash: blake3::Hash,
}
