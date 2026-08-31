pub enum Resolvable<Src, Res, Fail: FailedResolution> {
    Unresolved(Src),
    Resolved(Res),
    Failed(Fail),
}

pub trait FailedResolution {
    fn reason() -> &str;
}
