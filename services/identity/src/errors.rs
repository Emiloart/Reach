use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("account not found")]
    AccountNotFound,
    #[error("device not found")]
    DeviceNotFound,
    #[error("device registration conflict")]
    DeviceRegistrationConflict,
}
