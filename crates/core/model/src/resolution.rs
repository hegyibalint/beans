use std::sync::OnceLock;

pub struct Resolvable<S, R, F> {
    source: S,
    resolution: OnceLock<Resolution<R, F>>,
}

pub enum Resolution<R, F> {
    Resolved(R),
    Failed(F),
}

impl<S, R, F> Resolvable<S, R, F> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            resolution: OnceLock::new(),
        }
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn resolution(&self) -> Option<&Resolution<R, F>> {
        self.resolution.get()
    }

    pub fn get_or_init(
        &self,
        resolve: impl FnOnce(&S) -> Resolution<R, F>,
    ) -> &Resolution<R, F> {
        self.resolution.get_or_init(|| resolve(&self.source))
    }
}
