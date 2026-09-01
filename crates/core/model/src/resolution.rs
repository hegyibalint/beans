use std::sync::OnceLock;

pub struct Resolvable<S, R> {
    source: S,
    resolution: OnceLock<R>,
}

impl<S, R> Resolvable<S, R> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            resolution: OnceLock::new(),
        }
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn resolution(&self) -> Option<&R> {
        self.resolution.get()
    }

    pub fn get_or_init(&self, resolve: impl FnOnce(&S) -> R) -> &R {
        self.resolution.get_or_init(|| resolve(&self.source))
    }
}
