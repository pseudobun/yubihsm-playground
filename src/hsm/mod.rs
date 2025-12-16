pub mod client;
pub mod error;
pub mod operations;

// Re-export commonly used items
pub use client::{HsmClient, HsmConfig, SessionManager};
pub use operations::{
    ObjectSummary, delete_object, get_object_info, get_public_key, list_object_summaries,
    list_objects, sign, verify,
};
