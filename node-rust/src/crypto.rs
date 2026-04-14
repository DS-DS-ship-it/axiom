use anyhow::{anyhow, Result};
use blake3::Hasher;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use rand::rngs::OsRng;

pub fn hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

pub fn generate_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

pub fn signing_key_from_hex(hex_str: &str) -> Result<SigningKey> {
    let raw = hex::decode(hex_str)?;
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| anyhow!("private key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
}

pub fn public_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

pub fn address_from_vk(vk: &VerifyingKey) -> String {
    let hashed = hash_hex(&vk.to_bytes());
    format!("axm_{}", &hashed[..40])
}

pub fn sign_bytes(key: &SigningKey, bytes: &[u8]) -> String {
    let sig: Signature = key.sign(bytes);
    hex::encode(sig.to_bytes())
}

pub fn verify_bytes(vk_hex: &str, bytes: &[u8], sig_hex: &str) -> Result<()> {
    let vk_raw = hex::decode(vk_hex)?;
    let sig_raw = hex::decode(sig_hex)?;
    let vk_arr: [u8; 32] = vk_raw.try_into().map_err(|_| anyhow!("bad public key len"))?;
    let sig_arr: [u8; 64] = sig_raw.try_into().map_err(|_| anyhow!("bad signature len"))?;
    let vk = VerifyingKey::from_bytes(&vk_arr)?;
    let sig = Signature::from_bytes(&sig_arr);
    vk.verify(bytes, &sig)?;
    Ok(())
}
