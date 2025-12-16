use super::client::HsmClient;
use super::error::{HsmError, HsmResult};
use hex;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use yubihsm::Algorithm;
use yubihsm::asymmetric::PublicKey;
use yubihsm::object::{Id, Info, Label, SequenceId, Type};

/// sign data using an ECDSA key (secp256r1/P-256) stored in the HSM
/// First hashes the data with SHA-256, then signs the hash
pub fn sign(client: &HsmClient, key_id: u16, data: &[u8]) -> HsmResult<Vec<u8>> {
    if data.is_empty() {
        return Err(HsmError::InvalidInput("Data cannot be empty".to_string()));
    }

    // Hash the data with SHA-256 first
    let hash = Sha256::digest(data);
    let hash_vec = hash.to_vec();

    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::SigningFailed(format!("Failed to lock client: {}", e)))?;

    // Sign the hash using sign_ecdsa_prehash_raw
    let signature = hsm
        .sign_ecdsa_prehash_raw(key_id, hash_vec)
        .map_err(|e| HsmError::SigningFailed(format!("{:?}", e)))?;

    Ok(signature)
}

pub fn verify(client: &HsmClient, key_id: u16, data: &[u8], signature: &[u8]) -> HsmResult<bool> {
    if data.is_empty() {
        return Err(HsmError::InvalidInput("Data cannot be empty".to_string()));
    }

    // Hash the data with SHA-256 (same as during signing)
    let hash = Sha256::digest(data);

    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::VerificationFailed(format!("Failed to lock client: {}", e)))?;

    // Get the public key from the HSM
    let public_key = hsm
        .get_public_key(key_id)
        .map_err(|e| HsmError::InvalidKey(format!("Failed to get public key: {:?}", e)))?;

    // Use p256 crate for ECDSA verification
    use p256::ecdsa::{Signature as EcdsaSignature, VerifyingKey};
    use signature::hazmat::PrehashVerifier;

    // YubiHSM returns public key as raw bytes (64 bytes: x || y for P-256)
    // We need to convert it to uncompressed SEC1 format (0x04 || x || y)
    let pk_bytes = public_key.as_ref();

    // Try to parse as uncompressed point first (if it's already 65 bytes with 0x04 prefix)
    let verifying_key = if pk_bytes.len() == 65 && pk_bytes[0] == 0x04 {
        VerifyingKey::from_sec1_bytes(pk_bytes)
            .map_err(|e| HsmError::InvalidKey(format!("Invalid public key (SEC1): {}", e)))?
    } else if pk_bytes.len() == 64 {
        // If it's 64 bytes (raw x || y), add the 0x04 prefix
        let mut uncompressed = vec![0x04];
        uncompressed.extend_from_slice(pk_bytes);

        VerifyingKey::from_sec1_bytes(&uncompressed)
            .map_err(|e| HsmError::InvalidKey(format!("Invalid public key (raw): {}", e)))?
    } else {
        return Err(HsmError::InvalidKey(format!(
            "Unexpected public key length: {} bytes (expected 64 or 65)",
            pk_bytes.len()
        )));
    };

    // Parse the signature
    // YubiHSM returns DER-encoded signature (typically 70 bytes, but can vary)
    // p256::ecdsa::Signature::from_slice() expects raw format (64 bytes: r || s)
    // So we need to handle DER format and convert to raw if needed
    let sig = if signature.len() > 64 && signature[0] == 0x30 {
        // DER format: starts with 0x30 (SEQUENCE tag) and is longer than 64 bytes
        // DER structure: SEQUENCE { INTEGER r, INTEGER s }
        // We'll use the ecdsa crate's DER parsing capability
        EcdsaSignature::from_der(signature)
            .map_err(|e| HsmError::InvalidInput(format!("Invalid DER signature format: {}", e)))?
    } else if signature.len() == 64 {
        // Raw format: r || s (32 bytes each)
        EcdsaSignature::from_slice(signature)
            .map_err(|e| HsmError::InvalidInput(format!("Invalid raw signature format: {}", e)))?
    } else {
        return Err(HsmError::InvalidInput(format!(
            "Invalid signature length: {} bytes (expected 64 for raw or >64 for DER)",
            signature.len()
        )));
    };

    // Verify the signature against the hash
    // Since we used sign_ecdsa_prehash_raw, we need to use verify_prehash
    match verifying_key.verify_prehash(&hash, &sig) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// List all objects visible to the current authentication key on the HSM.
/// Returns a human-readable summary string that can be shown in the UI.
pub fn list_objects(client: &HsmClient) -> HsmResult<String> {
    let summaries = list_object_summaries(client)?;

    if summaries.is_empty() {
        return Ok("No objects visible for the current authentication key.".to_string());
    }

    let mut out = String::from("Objects on YubiHSM2 (visible to current auth key):\n");
    for summary in summaries {
        let _ = writeln!(
            &mut out,
            "- id 0x{:04x} ({:?}), algorithm: {:?}, label: {:?}, sequence: {}",
            summary.object_id,
            summary.object_type,
            summary.algorithm,
            summary.label,
            summary.sequence,
        );

        // If this is an asymmetric key, also display its public key
        if let Some(pk_hex) = summary.public_key_hex.as_ref() {
            // Format hex with line breaks for readability (64 chars per line)
            let formatted_hex: String = pk_hex
                .chars()
                .collect::<Vec<_>>()
                .chunks(64)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n    ");

            let _ = writeln!(
                &mut out,
                "  Public Key:\n    Algorithm: {:?}\n    Bytes ({} bytes, hex):\n    {}",
                summary.algorithm,
                pk_hex.len() / 2,
                formatted_hex
            );
        }
    }

    Ok(out)
}

/// Get detailed information about an object (using its ID and type).
pub fn get_object_info(client: &HsmClient, object_id: Id, object_type: Type) -> HsmResult<Info> {
    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::ListingFailed(format!("Failed to lock client: {}", e)))?;

    let info = hsm
        .get_object_info(object_id, object_type)
        .map_err(|e| HsmError::ListingFailed(format!("Failed to get object info: {:?}", e)))?;

    Ok(info)
}

/// Get the public key bytes/algorithm for an asymmetric key object id.
pub fn get_public_key(client: &HsmClient, key_id: Id) -> HsmResult<PublicKey> {
    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::ListingFailed(format!("Failed to lock client: {}", e)))?;

    let public_key = hsm
        .get_public_key(key_id)
        .map_err(|e| HsmError::GetPublicKeyFailed(format!("Failed to get public key: {:?}", e)))?;

    Ok(public_key)
}

/// Structured summary of an HSM object suitable for displaying in a table.
#[derive(Clone, Debug)]
pub struct ObjectSummary {
    pub object_id: Id,
    pub object_type: Type,
    pub algorithm: Algorithm,
    pub label: Label,
    pub sequence: SequenceId,
    /// Hex-encoded public key bytes for asymmetric keys, if available.
    pub public_key_hex: Option<String>,
}

/// Delete an object from the HSM by ID and type.
/// Note: This will NOT delete authentication keys for safety.
pub fn delete_object(client: &HsmClient, object_id: Id, object_type: Type) -> HsmResult<()> {
    if object_type == Type::AuthenticationKey {
        return Err(HsmError::InvalidInput(
            "Deleting authentication keys is not allowed".to_string(),
        ));
    }

    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::DeletionFailed(format!("Failed to lock client: {}", e)))?;

    hsm.delete_object(object_id, object_type)
        .map_err(|e| HsmError::DeletionFailed(format!("Failed to delete object: {:?}", e)))?;

    Ok(())
}

/// List objects and return structured summaries that can be rendered in a table.
pub fn list_object_summaries(client: &HsmClient) -> HsmResult<Vec<ObjectSummary>> {
    let hsm_client = client.client();
    let hsm = hsm_client
        .lock()
        .map_err(|e| HsmError::ListingFailed(format!("Failed to lock client: {}", e)))?;

    // Empty filter list = list all objects visible to this auth key
    let entries = hsm
        .list_objects(&[])
        .map_err(|e| HsmError::ListingFailed(format!("{:?}", e)))?;
    drop(hsm);

    let mut summaries = Vec::new();

    for entry in entries {
        let info = get_object_info(client, entry.object_id, entry.object_type)?;

        let public_key_hex = if info.object_type == Type::AsymmetricKey {
            let public_key = get_public_key(client, info.object_id)?;
            Some(hex::encode(&public_key.bytes))
        } else {
            None
        };

        summaries.push(ObjectSummary {
            object_id: info.object_id,
            object_type: info.object_type,
            algorithm: info.algorithm,
            label: info.label,
            sequence: info.sequence,
            public_key_hex,
        });
    }

    Ok(summaries)
}
