use std::fmt;

/// Result type for HSM operations
pub type HsmResult<T> = Result<T, HsmError>;

/// Errors that can occur during HSM operations
#[derive(Debug)]
pub enum HsmError {
    /// Failed to authenticate with the HSM
    AuthenticationFailed(String),

    /// Signing operation failed
    SigningFailed(String),

    /// Listing objects/keys failed
    ListingFailed(String),

    /// Verification operation failed
    VerificationFailed(String),

    /// Key not found or invalid
    InvalidKey(String),

    /// Invalid input data
    InvalidInput(String),

    /// Failed to get public key
    GetPublicKeyFailed(String),
}

impl fmt::Display for HsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HsmError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            HsmError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            HsmError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            HsmError::InvalidKey(msg) => write!(f, "Invalid key: {}", msg),
            HsmError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            HsmError::ListingFailed(msg) => write!(f, "Listing failed: {}", msg),
            HsmError::GetPublicKeyFailed(msg) => write!(f, "Failed to get public key: {}", msg),
        }
    }
}

impl std::error::Error for HsmError {}
