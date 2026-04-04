use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyServiceError {
    #[error("device key bundle not found")]
    KeyBundleNotFound,
    #[error("signed prekey not found")]
    SignedPrekeyNotFound,
    #[error("no one-time prekeys available")]
    NoAvailableOneTimePrekeys,
}
