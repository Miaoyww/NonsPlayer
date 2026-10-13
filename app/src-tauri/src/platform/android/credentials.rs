//! Keystore-backed compatibility surface for host and scoped plugin credentials.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;

#[derive(Debug)]
pub enum Error {
    NoEntry,
    Unavailable,
}
pub struct Entry {
    service: String,
    key: String,
}
#[derive(Deserialize)]
struct Response {
    value: Option<String>,
}
impl Entry {
    pub fn new(service: &str, key: &str) -> Result<Self, Error> {
        if service.is_empty() || key.is_empty() || key.len() > 1024 {
            return Err(Error::Unavailable);
        }
        Ok(Self {
            service: service.into(),
            key: key.into(),
        })
    }
    fn call(&self, operation: &str, value: Option<String>) -> Result<Response, Error> {
        super::HANDLE
            .get()
            .ok_or(Error::Unavailable)?
            .run_mobile_plugin(
                "credentials",
                serde_json::json!({
                    "service": self.service, "key": self.key, "operation": operation, "value": value
                }),
            )
            .map_err(|_| Error::Unavailable)
    }
    pub fn get_secret(&self) -> Result<Vec<u8>, Error> {
        let value = self.call("read", None)?.value.ok_or(Error::NoEntry)?;
        STANDARD.decode(value).map_err(|_| Error::Unavailable)
    }
    pub fn get_password(&self) -> Result<String, Error> {
        String::from_utf8(self.get_secret()?).map_err(|_| Error::Unavailable)
    }
    pub fn set_secret(&self, value: &[u8]) -> Result<(), Error> {
        self.call("write", Some(STANDARD.encode(value)))?;
        Ok(())
    }
    pub fn set_password(&self, value: &str) -> Result<(), Error> {
        self.set_secret(value.as_bytes())
    }
    pub fn delete_credential(&self) -> Result<(), Error> {
        self.call("delete", None)?;
        Ok(())
    }
}
