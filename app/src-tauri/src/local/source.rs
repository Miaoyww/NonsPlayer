//! Read-only local music access. Document locators retain an opaque SAF root and
//! provider display names; only the relative name portion uses Path operations.
use std::{
    ffi::OsString,
    fs::File,
    io,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

#[cfg(target_os = "android")]
pub(crate) mod android;

const DOCUMENT_PREFIX: &str = "nons-saf";

pub(crate) fn is_document(path: impl AsRef<Path>) -> bool {
    path.as_ref().components().next() == Some(Component::Normal(DOCUMENT_PREFIX.as_ref()))
}

// The first two components identify the grant; the third is its display name.
// Never treat a content URI's path segments as provider file names.
fn document_parts(path: &Path) -> io::Result<(String, PathBuf, PathBuf)> {
    let mut parts = path.components();
    if parts.next() != Some(Component::Normal(DOCUMENT_PREFIX.as_ref())) {
        return Err(io::Error::other("不是文档目录位置"));
    }
    let token = match parts.next() {
        Some(Component::Normal(token)) => token.to_string_lossy().into_owned(),
        _ => return Err(io::Error::other("文档目录标识无效")),
    };
    let root_name = match parts.next() {
        Some(Component::Normal(name)) => name.to_owned(),
        _ => return Err(io::Error::other("文档目录名称无效")),
    };
    let root = Path::new(DOCUMENT_PREFIX).join(&token).join(root_name);
    let mut relative = PathBuf::new();
    for part in parts {
        match part {
            Component::Normal(name) => relative.push(name),
            Component::CurDir => (),
            Component::ParentDir if relative.pop() => (),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "路径超出授权音乐目录",
                ))
            }
        }
    }
    if relative.components().count() >= 128 {
        return Err(io::Error::other("音乐目录嵌套超过 128 层"));
    }
    Ok((token, root, relative))
}

pub(crate) fn display(path: &str) -> String {
    if is_document(path) {
        document_parts(Path::new(path))
            .ok()
            .and_then(|(_, root, relative)| {
                Some(
                    root.file_name()?.to_string_lossy().into_owned()
                        + if relative.as_os_str().is_empty() {
                            ""
                        } else {
                            "/"
                        }
                        + &relative.to_string_lossy(),
                )
            })
            .unwrap_or_else(|| "音乐文件夹".into())
    } else {
        path.to_owned()
    }
}

#[derive(Clone)]
pub(crate) struct Metadata {
    len: u64,
    modified: Option<SystemTime>,
    file: bool,
    directory: bool,
}
impl Metadata {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        self.modified
            .ok_or_else(|| io::Error::other("文件修改时间不可用"))
    }
    pub fn is_file(&self) -> bool {
        self.file
    }
    pub fn is_dir(&self) -> bool {
        self.directory
    }
}

#[derive(Clone)]
pub(crate) struct Entry {
    path: PathBuf,
    metadata: Metadata,
}
impl Entry {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file_name(&self) -> OsString {
        self.path.file_name().unwrap_or_default().to_owned()
    }
    pub fn file_type(&self) -> io::Result<Metadata> {
        Ok(self.metadata.clone())
    }
}

pub(crate) fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    let path = path.as_ref();
    if is_document(path) {
        #[cfg(target_os = "android")]
        return android::metadata(path);
        #[cfg(not(target_os = "android"))]
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "文档目录仅支持 Android",
        ));
    }
    let metadata = std::fs::metadata(path)?;
    Ok(Metadata {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        file: metadata.is_file(),
        directory: metadata.is_dir(),
    })
}

pub(crate) fn open(path: impl AsRef<Path>) -> io::Result<File> {
    let path = path.as_ref();
    if is_document(path) {
        #[cfg(target_os = "android")]
        return android::open(path);
        #[cfg(not(target_os = "android"))]
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "文档目录仅支持 Android",
        ));
    }
    File::open(path)
}

pub(crate) fn authorized(path: &Path) -> io::Result<()> {
    if is_document(path) {
        #[cfg(target_os = "android")]
        return android::authorized(path);
        #[cfg(not(target_os = "android"))]
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "文档目录仅支持 Android",
        ));
    }
    Ok(())
}

pub(crate) fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    let path = path.as_ref();
    if is_document(path) {
        let (_, root, relative) = document_parts(path)?;
        let normalized = root.join(relative);
        metadata(&normalized)?;
        Ok(normalized)
    } else {
        path.canonicalize()
    }
}

pub(crate) fn read_dir(
    path: impl AsRef<Path>,
) -> io::Result<impl Iterator<Item = io::Result<Entry>>> {
    let path = path.as_ref();
    let entries: Vec<io::Result<Entry>> = if is_document(path) {
        #[cfg(target_os = "android")]
        {
            android::read_dir(path)?.into_iter().map(Ok).collect()
        }
        #[cfg(not(target_os = "android"))]
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "文档目录仅支持 Android",
        ));
    } else {
        std::fs::read_dir(path)?
            .map(|entry| {
                let entry = entry?;
                // Keep directory symlinks out of cover discovery and traversal.
                let kind = entry.file_type()?;
                Ok(Entry {
                    path: entry.path(),
                    metadata: Metadata {
                        len: 0,
                        modified: None,
                        file: kind.is_file(),
                        directory: kind.is_dir(),
                    },
                })
            })
            .collect()
    };
    Ok(entries.into_iter())
}

pub(crate) enum Walk {
    Native(walkdir::IntoIter),
    Documents(Vec<std::vec::IntoIter<io::Result<Entry>>>),
}
pub(crate) fn walk(path: impl AsRef<Path>) -> Walk {
    let path = path.as_ref();
    if is_document(path) {
        let entry = metadata(path).map(|metadata| Entry {
            path: path.to_path_buf(),
            metadata,
        });
        Walk::Documents(vec![vec![entry].into_iter()])
    } else {
        Walk::Native(
            walkdir::WalkDir::new(path)
                .follow_links(false)
                .max_open(8)
                .sort_by_file_name()
                .into_iter(),
        )
    }
}
impl Iterator for Walk {
    type Item = io::Result<Entry>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Native(entries) => entries.next().map(|entry| {
                let entry = entry.map_err(io::Error::other)?;
                let kind = entry.file_type();
                // No stat is needed by the scanner for native directory entries.
                Ok(Entry {
                    path: entry.into_path(),
                    metadata: Metadata {
                        len: 0,
                        modified: Some(SystemTime::UNIX_EPOCH),
                        file: kind.is_file(),
                        directory: kind.is_dir(),
                    },
                })
            }),
            Self::Documents(stack) => loop {
                let next = stack.last_mut()?.next();
                let Some(next) = next else {
                    stack.pop();
                    continue;
                };
                if let Ok(entry) = &next {
                    if entry.metadata.is_dir() {
                        if stack.len() >= 128 {
                            return Some(Err(io::Error::other("音乐目录嵌套超过 128 层")));
                        }
                        let mut children = match read_dir(&entry.path) {
                            Ok(children) => children.collect::<Vec<_>>(),
                            Err(error) => vec![Err(error)],
                        };
                        children.sort_by_key(|entry| {
                            entry.as_ref().ok().map(|entry| entry.file_name())
                        });
                        stack.push(children.into_iter());
                    }
                }
                return Some(next);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_names_preserve_siblings_without_parsing_opaque_uri() {
        let path = Path::new("nons-saf/opaque-token/音乐/专辑/01 song.flac");
        let (_, root, relative) = document_parts(&path.with_extension("lrc")).unwrap();
        assert_eq!(root, Path::new("nons-saf/opaque-token/音乐"));
        assert_eq!(relative, Path::new("专辑/01 song.lrc"));
        assert_eq!(
            display(path.to_str().unwrap()).replace('\\', "/"),
            "音乐/专辑/01 song.flac"
        );
        assert!(document_parts(Path::new("nons-saf/token/音乐/../secret.flac")).is_err());
        let (_, _, relative) =
            document_parts(Path::new("nons-saf/token/音乐/专辑/../audio.flac")).unwrap();
        assert_eq!(relative, Path::new("audio.flac"));
        let deep = Path::new("nons-saf/token/音乐").join(vec!["a"; 128].join("/"));
        assert!(document_parts(&deep).is_err());
    }
}
