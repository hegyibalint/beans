use std::{fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum ParseError {
    Toml(toml::de::Error),
    Root { path: PathBuf, error: io::Error },
    UnknownDependency { module: String, dependency: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(error) => write!(f, "{error}"),
            Self::Root { path, error } => {
                write!(
                    f,
                    "cannot resolve workspace root {}: {error}",
                    path.display()
                )
            }
            Self::UnknownDependency { module, dependency } => {
                write!(
                    f,
                    "module {module:?} depends on unknown module {dependency:?}"
                )
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(error) => Some(error),
            Self::Root { error, .. } => Some(error),
            Self::UnknownDependency { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Read { path: PathBuf, error: io::Error },
    Parse { path: PathBuf, error: ParseError },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, error } => write!(f, "cannot read {}: {error}", path.display()),
            Self::Parse { path, error } => write!(f, "cannot parse {}: {error}", path.display()),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { error, .. } => Some(error),
            Self::Parse { error, .. } => Some(error),
        }
    }
}
