use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;

pub fn generate_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

pub fn sign(key: &SigningKey, message: &[u8]) -> [u8; 64] {
    key.sign(message).to_bytes()
}

pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else { return false };
    key.verify(message, &Signature::from_bytes(signature)).is_ok()
}


pub fn bls_verify(public_key:&[u8;48],message:&[u8],signature:&[u8;96])->bool{
    let Ok(pk)=blst::min_pk::PublicKey::from_bytes(public_key) else{return false};
    let Ok(sig)=blst::min_pk::Signature::from_bytes(signature) else{return false};
    sig.verify(true,message,b"FREEDOM-ETH-BLS-V1",&pk,true)==blst::BLST_ERROR::BLST_SUCCESS
}

pub fn bls_verify_aggregate(public_keys:&[[u8;48]],message:&[u8],signature:&[u8;96])->bool{
    if public_keys.is_empty(){return false;}
    let Ok(sig)=blst::min_pk::Signature::from_bytes(signature) else{return false};
    let refs=public_keys.iter().filter_map(|b|blst::min_pk::PublicKey::from_bytes(b).ok()).collect::<Vec<_>>();
    if refs.len()!=public_keys.len(){return false;}
    let pks=refs.iter().collect::<Vec<_>>();
    sig.fast_aggregate_verify(true,message,b"FREEDOM-ETH-BLS-V1",&pks)==blst::BLST_ERROR::BLST_SUCCESS
}
