pub mod client;
pub mod error;
pub mod operations;

// Re-export commonly used items
pub use client::{HsmClient, HsmConfig};
pub use operations::{get_object_info, get_public_key, list_objects, sign, verify};
