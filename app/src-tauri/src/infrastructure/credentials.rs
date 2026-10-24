//! Portable credentials travel with the application; installed builds use the OS vault.
#[cfg(target_os = "android")]
use crate::platform::android::credentials as system;
#[cfg(not(target_os = "android"))]
use keyring as system;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) enum Error {
    NoEntry,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoEntry => "credential entry not found",
            Self::Unavailable => "credential storage unavailable",
        })
    }
}

pub(crate) enum Entry {
    File(PathBuf),
    System(system::Entry),
}

fn map_system(operation: &str, error: system::Error) -> Error {
    match error {
        system::Error::NoEntry => Error::NoEntry,
        error => {
            #[cfg(not(target_os = "android"))]
            match &error {
                // keyring's Display for Ambiguous includes credential Debug data.
                system::Error::Ambiguous(entries) => super::diagnostics::warn(
                    operation,
                    format_args!("ambiguous credential entries count={}", entries.len()),
                ),
                _ => super::diagnostics::warn(operation, &error),
            }
            #[cfg(target_os = "android")]
            super::diagnostics::warn(operation, &error);
            Error::Unavailable
        }
    }
}

fn map_file(operation: &str, error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::NotFound {
        Error::NoEntry
    } else {
        super::diagnostics::warn(operation, &error);
        Error::Unavailable
    }
}

impl Entry {
    pub(crate) fn new(service: &str, key: &str) -> Result<Self, Error> {
        if let Some(data) = super::directories::portable() {
            use sha2::{Digest, Sha256};
            let identity = serde_json::to_vec(&(service, key))
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "credentials.identity.serialize",
                        error,
                    );
                })
                .map_err(|_| Error::Unavailable)?;
            return Ok(Self::File(
                data.join("credentials")
                    .join(format!("{:x}", Sha256::digest(identity))),
            ));
        }
        system::Entry::new(service, key)
            .map(Self::System)
            .map_err(|error| map_system("credentials.create", error))
    }

    pub(crate) fn get_secret(&self) -> Result<Vec<u8>, Error> {
        match self {
            Self::System(entry) => entry
                .get_secret()
                .map_err(|error| map_system("credentials.read", error)),
            Self::File(path) => {
                std::fs::read(path).map_err(|error| map_file("credentials.read", error))
            }
        }
    }

    pub(crate) fn get_password(&self) -> Result<String, Error> {
        match self {
            Self::System(entry) => entry
                .get_password()
                .map_err(|error| map_system("credentials.decode", error)),
            Self::File(_) => String::from_utf8(self.get_secret()?)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("credentials.decode_utf8", error);
                })
                .map_err(|_| Error::Unavailable),
        }
    }

    pub(crate) fn set_secret(&self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry
                .set_secret(bytes)
                .map_err(|error| map_system("credentials.write", error)),
            Self::File(path) => {
                use std::io::Write;
                let mut file =
                    tempfile::NamedTempFile::new_in(path.parent().ok_or(Error::Unavailable)?)
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "credentials.file.create",
                                error,
                            );
                        })
                        .map_err(|_| Error::Unavailable)?;
                file.write_all(bytes)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("credentials.file.write", error);
                    })
                    .map_err(|_| Error::Unavailable)?;
                file.as_file()
                    .sync_all()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("credentials.file.sync", error);
                    })
                    .map_err(|_| Error::Unavailable)?;
                file.persist(path)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("credentials.file.persist", error);
                    })
                    .map_err(|_| Error::Unavailable)?;
                Ok(())
            }
        }
    }

    pub(crate) fn set_password(&self, value: &str) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry
                .set_password(value)
                .map_err(|error| map_system("credentials.write", error)),
            Self::File(_) => self.set_secret(value.as_bytes()),
        }
    }

    pub(crate) fn delete_credential(&self) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry
                .delete_credential()
                .map_err(|error| map_system("credentials.delete", error)),
            Self::File(path) => {
                std::fs::remove_file(path).map_err(|error| map_file("credentials.delete", error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_credentials_survive_reopen_and_replace() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("secret");
        let entry = Entry::File(path.clone());
        assert!(matches!(entry.get_secret(), Err(Error::NoEntry)));
        entry.set_secret(&[0, 255, 12]).unwrap();
        assert_eq!(
            Entry::File(path.clone()).get_secret().unwrap(),
            [0, 255, 12]
        );
        entry.set_password("更新后的凭据").unwrap();
        assert_eq!(Entry::File(path).get_password().unwrap(), "更新后的凭据");
        entry.delete_credential().unwrap();
        assert!(matches!(entry.delete_credential(), Err(Error::NoEntry)));
    }
}
