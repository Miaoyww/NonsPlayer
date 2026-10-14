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

pub(crate) enum Entry {
    File(PathBuf),
    System(system::Entry),
}

fn map_system(error: system::Error) -> Error {
    match error {
        system::Error::NoEntry => Error::NoEntry,
        _ => Error::Unavailable,
    }
}

impl Entry {
    pub(crate) fn new(service: &str, key: &str) -> Result<Self, Error> {
        if let Some(data) = super::directories::portable() {
            use sha2::{Digest, Sha256};
            let identity = serde_json::to_vec(&(service, key)).map_err(|_| Error::Unavailable)?;
            return Ok(Self::File(
                data.join("credentials")
                    .join(format!("{:x}", Sha256::digest(identity))),
            ));
        }
        system::Entry::new(service, key)
            .map(Self::System)
            .map_err(map_system)
    }

    pub(crate) fn get_secret(&self) -> Result<Vec<u8>, Error> {
        match self {
            Self::System(entry) => entry.get_secret().map_err(map_system),
            Self::File(path) => std::fs::read(path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Error::NoEntry
                } else {
                    Error::Unavailable
                }
            }),
        }
    }

    pub(crate) fn get_password(&self) -> Result<String, Error> {
        match self {
            Self::System(entry) => entry.get_password().map_err(map_system),
            Self::File(_) => String::from_utf8(self.get_secret()?).map_err(|_| Error::Unavailable),
        }
    }

    pub(crate) fn set_secret(&self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry.set_secret(bytes).map_err(map_system),
            Self::File(path) => {
                use std::io::Write;
                let mut file =
                    tempfile::NamedTempFile::new_in(path.parent().ok_or(Error::Unavailable)?)
                        .map_err(|_| Error::Unavailable)?;
                file.write_all(bytes).map_err(|_| Error::Unavailable)?;
                file.as_file().sync_all().map_err(|_| Error::Unavailable)?;
                file.persist(path).map_err(|_| Error::Unavailable)?;
                Ok(())
            }
        }
    }

    pub(crate) fn set_password(&self, value: &str) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry.set_password(value).map_err(map_system),
            Self::File(_) => self.set_secret(value.as_bytes()),
        }
    }

    pub(crate) fn delete_credential(&self) -> Result<(), Error> {
        match self {
            Self::System(entry) => entry.delete_credential().map_err(map_system),
            Self::File(path) => std::fs::remove_file(path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Error::NoEntry
                } else {
                    Error::Unavailable
                }
            }),
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
