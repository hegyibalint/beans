//! Class containers — read classes out of `.jmod` and `.jar` archives.
//!
//! Per backlog item 037 this layer hands out *raw class bytes* keyed by
//! binary name (`java.lang.String`, `java.util.Map$Entry`); decoding
//! those bytes into the JVM model is a separate layer (backlog 012)
//! that takes bare `&[u8]`, because class bytes also arrive from loose
//! files (build output directories), not just archives.
//!
//! Both formats are zip archives of classfiles; only the layout
//! differs:
//!
//! - `.jmod` — 4-byte magic prefix (`JM` + version), classes under
//!   `classes/`, non-class sections (`lib/`, `conf/`, `bin/`, …)
//!   ignored.
//! - `.jar` — classes at the archive root; `META-INF/` is skipped
//!   entirely, which also defers multi-release overlays (backlog 036).
//!
//! `module-info.class` entries are skipped until the JPMS module
//! registry (backlog 013) needs them.
//!
//! [`Jmod`] and [`Jar`] are distinct public types sharing a private
//! inner — no public trait, per ADR-0001 (cohesive, not extensible).
//! A consumer that needs "either container" uniformly can add an enum
//! wrapper when one exists.

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use zip::ZipArchive;

/// Magic prefix of a `.jmod` file: `JM` followed by format version
/// `0x01 0x00`. The zip archive starts right after these four bytes.
const JMOD_MAGIC: [u8; 2] = *b"JM";

/// Local-file-header signature every non-empty zip (and thus jar)
/// starts with.
const ZIP_MAGIC: [u8; 2] = *b"PK";

#[derive(Debug)]
pub enum ContainerError {
    Io(io::Error),
    /// The file does not start with the magic bytes its format
    /// requires — e.g. a jar passed to [`Jmod::open`].
    MagicMismatch {
        expected: &'static str,
        found: [u8; 2],
    },
    Zip(zip::result::ZipError),
    /// No class with the requested binary name exists in the archive.
    ClassNotFound(String),
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerError::Io(e) => write!(f, "container I/O error: {e}"),
            ContainerError::MagicMismatch { expected, found } => write!(
                f,
                "magic mismatch: expected {expected}, found {:?}",
                String::from_utf8_lossy(found)
            ),
            ContainerError::Zip(e) => write!(f, "container zip error: {e}"),
            ContainerError::ClassNotFound(name) => {
                write!(f, "class not found in container: {name}")
            }
        }
    }
}

impl std::error::Error for ContainerError {}

impl From<io::Error> for ContainerError {
    fn from(e: io::Error) -> Self {
        ContainerError::Io(e)
    }
}

impl From<zip::result::ZipError> for ContainerError {
    fn from(e: zip::result::ZipError) -> Self {
        ContainerError::Zip(e)
    }
}

/// A JDK `.jmod` archive. Enumerates the `classes/` section only.
pub struct Jmod {
    inner: Inner,
}

impl fmt::Debug for Jmod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Jmod")
            .field("classes", &self.inner.classes.len())
            .finish_non_exhaustive()
    }
}

impl Jmod {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        let inner = Inner::open(path.as_ref(), &JMOD_MAGIC, "JM (jmod)", "classes/")?;
        Ok(Self { inner })
    }

    /// Binary names of all classes in the container, in archive order.
    pub fn class_names(&self) -> impl Iterator<Item = &str> {
        self.inner.class_names()
    }

    /// Raw classfile bytes for a binary name (`java.lang.String`).
    pub fn class_bytes(&mut self, binary_name: &str) -> Result<Vec<u8>, ContainerError> {
        self.inner.class_bytes(binary_name)
    }
}

/// A `.jar` archive. Enumerates base classes only; `META-INF/` —
/// including multi-release `versions/` overlays (backlog 036) — is
/// skipped.
pub struct Jar {
    inner: Inner,
}

impl fmt::Debug for Jar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Jar")
            .field("classes", &self.inner.classes.len())
            .finish_non_exhaustive()
    }
}

impl Jar {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        let inner = Inner::open(path.as_ref(), &ZIP_MAGIC, "PK (jar/zip)", "")?;
        Ok(Self { inner })
    }

    /// Binary names of all classes in the container, in archive order.
    pub fn class_names(&self) -> impl Iterator<Item = &str> {
        self.inner.class_names()
    }

    /// Raw classfile bytes for a binary name (`java.lang.String`).
    pub fn class_bytes(&mut self, binary_name: &str) -> Result<Vec<u8>, ContainerError> {
        self.inner.class_bytes(binary_name)
    }
}

/// Shared implementation: a zip archive plus the layout prefix that
/// distinguishes jmod (`classes/`) from jar (empty prefix).
struct Inner {
    archive: ZipArchive<File>,
    class_prefix: &'static str,
    /// Binary names of every class entry, in archive order. Byte reads
    /// go through `by_name` with the path rebuilt from the binary name.
    classes: Vec<String>,
}

impl Inner {
    fn open(
        path: &Path,
        magic: &[u8; 2],
        expected: &'static str,
        class_prefix: &'static str,
    ) -> Result<Self, ContainerError> {
        let mut file = File::open(path)?;

        let mut found = [0u8; 2];
        file.read_exact(&mut found)?;
        if &found != magic {
            return Err(ContainerError::MagicMismatch { expected, found });
        }
        file.seek(SeekFrom::Start(0))?;

        // The zip reader locates the central directory from the end of
        // the file and compensates for prepended data, so the jmod
        // magic prefix needs no offset handling here.
        let archive = ZipArchive::new(file)?;

        let mut classes = Vec::new();
        for index in 0..archive.len() {
            let entry_path = archive.name_for_index(index).expect("index < len");
            if let Some(name) = binary_name(entry_path, class_prefix) {
                classes.push(name);
            }
        }

        Ok(Self {
            archive,
            class_prefix,
            classes,
        })
    }

    fn class_names(&self) -> impl Iterator<Item = &str> {
        self.classes.iter().map(|name| name.as_str())
    }

    fn class_bytes(&mut self, binary_name: &str) -> Result<Vec<u8>, ContainerError> {
        let entry_path = format!(
            "{}{}.class",
            self.class_prefix,
            binary_name.replace('.', "/")
        );
        let mut entry = match self.archive.by_name(&entry_path) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::FileNotFound) => {
                return Err(ContainerError::ClassNotFound(binary_name.to_string()));
            }
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

/// Map a zip entry path to a binary class name, or `None` for entries
/// this layer does not surface (non-class sections, `META-INF/`,
/// `module-info.class`).
///
/// `classes/java/util/Map$Entry.class` (prefix `classes/`) and
/// `java/util/Map$Entry.class` (empty prefix) both map to
/// `java.util.Map$Entry` — the `$` of nested classes is part of the
/// binary name and preserved as-is.
fn binary_name(entry_path: &str, class_prefix: &str) -> Option<String> {
    let path = entry_path.strip_prefix(class_prefix)?;
    let path = path.strip_suffix(".class")?;
    if path.starts_with("META-INF/") {
        return None;
    }
    if path == "module-info" || path.ends_with("/module-info") {
        return None;
    }
    Some(path.replace('/', "."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_test_jdks::Jdk;

    /// Tests run against a provisioned Temurin 21 (downloaded and
    /// cached by `beans-test-jdks`), not `$JAVA_HOME` — which may
    /// point at a runtime without `jmods/` at all.
    fn jdk() -> Jdk {
        beans_test_jdks::jdk(21)
    }

    #[test]
    fn jmod_enumerates_java_base_binary_names() {
        let jmod = Jmod::open(jdk().jmod("java.base")).unwrap();
        let names: Vec<&str> = jmod.class_names().collect();

        assert!(names.contains(&"java.lang.String"));
        assert!(names.contains(&"java.util.List"));
        // Nested classes keep the `$` of their binary name.
        assert!(names.contains(&"java.util.Map$Entry"));
        // module-info and non-class sections are not surfaced.
        assert!(!names.iter().any(|n| n.contains("module-info")));
        assert!(names.iter().all(|n| !n.contains('/')));
        // java.base is big; a tiny count means we misread the layout.
        assert!(names.len() > 1000, "only {} classes found", names.len());
    }

    #[test]
    fn jmod_hands_out_classfile_bytes() {
        let mut jmod = Jmod::open(jdk().jmod("java.base")).unwrap();
        let bytes = jmod.class_bytes("java.lang.String").unwrap();
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);

        let err = jmod.class_bytes("no.such.Class").unwrap_err();
        assert!(matches!(err, ContainerError::ClassNotFound(_)));
    }

    #[test]
    fn jar_enumerates_classes_without_meta_inf() {
        // jrt-fs.jar ships with every JDK — a real jar with no
        // version-specific contents to depend on.
        let jar = Jar::open(jdk().jrt_fs_jar()).unwrap();
        let names: Vec<&str> = jar.class_names().collect();

        assert!(!names.is_empty());
        assert!(names.iter().all(|n| !n.starts_with("META-INF")));
        assert!(names.iter().all(|n| !n.contains('/')));
    }

    #[test]
    fn jar_hands_out_classfile_bytes() {
        let mut jar = Jar::open(jdk().jrt_fs_jar()).unwrap();
        let first = jar.class_names().next().unwrap().to_string();
        let bytes = jar.class_bytes(&first).unwrap();
        assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn magic_mismatch_is_reported_clearly() {
        let as_jmod = Jmod::open(jdk().jrt_fs_jar()).unwrap_err();
        assert!(matches!(as_jmod, ContainerError::MagicMismatch { .. }));

        let as_jar = Jar::open(jdk().jmod("java.base")).unwrap_err();
        assert!(matches!(as_jar, ContainerError::MagicMismatch { .. }));
    }

    #[test]
    fn binary_name_mapping() {
        assert_eq!(
            binary_name("classes/java/lang/String.class", "classes/"),
            Some("java.lang.String".to_string())
        );
        assert_eq!(
            binary_name("java/util/Map$Entry.class", ""),
            Some("java.util.Map$Entry".to_string())
        );
        // Wrong section, wrong suffix, skipped entries.
        assert_eq!(binary_name("lib/libjava.dylib", "classes/"), None);
        assert_eq!(binary_name("java/lang/String.class", "classes/"), None);
        assert_eq!(binary_name("classes/module-info.class", "classes/"), None);
        assert_eq!(binary_name("module-info.class", ""), None);
        assert_eq!(
            binary_name("META-INF/versions/9/module-info.class", ""),
            None
        );
        assert_eq!(binary_name("META-INF/services/foo", ""), None);
    }
}
