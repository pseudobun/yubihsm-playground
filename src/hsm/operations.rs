use super::client::HsmClient;
use super::error::{HsmError, HsmResult};
use sha2::{Digest, Sha256};

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
