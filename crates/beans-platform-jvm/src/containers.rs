trait ContainerHandler {
    fn accept(path: std::path::PathBuf) -> bool;
    fn process(path: std::path::PathBuf) -> Result<crate::PlatformEntry, String>;

pub(crate) fn accept(path: std::path::PathBuf) -> bool {
    todo!()
}

pub(crate) fn process(path: std::path::PathBuf) -> Result<crate::PlatformEntry, String> {
    todo!()
}
