use freedom_primitives::{hash_bytes, Hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REGISTRY_PROTOCOL_VERSION: u16 = 1;
pub const REGISTRY_OPERATION_DOMAIN: &[u8] = b"321MORE-REGISTRY-OPERATION-V1";
pub const REGISTRY_AUTH_DOMAIN: &[u8] = b"321MORE-REGISTRY-AUTH-V1";
pub const MIN_REGISTRY_THRESHOLD: u16 = 4;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum ChainStatus { Active = 1, Paused = 2, Retired = 3 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetStatus { Active = 1, Paused = 2, Revoked = 3 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryOperation {
    RegisterChain {
        chain_id: u32,
        name: String,
        network: String,
        finality_depth: u64,
    },
    UpdateChainStatus { chain_id: u32, status: ChainStatus },
    RegisterCanonicalAsset {
        canonical_asset_id: Hash,
        symbol: String,
        name: String,
        decimals: u8,
    },
    AddAssetRepresentation {
        canonical_asset_id: Hash,
        chain_id: u32,
        external_asset_id: Vec<u8>,
        asset_kind: u8,
        decimals: u8,
    },
    UpdateAssetStatus { canonical_asset_id: Hash, status: AssetStatus },
    RevokeAssetRepresentation {
        canonical_asset_id: Hash,
        chain_id: u32,
        external_asset_id: Vec<u8>,
    },
    UpdateRegistryAuthorizationPolicy {
        policy_id: Hash,
        threshold: u16,
        members: BTreeMap<Hash, [u8; 32]>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrySignature {
    pub signer_id: Hash,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryAuthorization {
    pub signatures: Vec<RegistrySignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryOperationEnvelope {
    pub protocol_version: u16,
    pub network_id: u32,
    pub policy_id: Hash,
    pub nonce: u64,
    pub operation: RegistryOperation,
    pub authorization: RegistryAuthorization,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryAuthorizationPolicy {
    pub policy_id: Hash,
    pub threshold: u16,
    /// signer_id -> Ed25519 public key
    pub members: BTreeMap<Hash, [u8; 32]>,
}

fn put_u16(out: &mut Vec<u8>, value: u16) { out.extend_from_slice(&value.to_le_bytes()); }
fn put_u32(out: &mut Vec<u8>, value: u32) { out.extend_from_slice(&value.to_le_bytes()); }
fn put_u64(out: &mut Vec<u8>, value: u64) { out.extend_from_slice(&value.to_le_bytes()); }
fn put_bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let len = u32::try_from(value.len()).map_err(|_| "registry field exceeds u32 length".to_string())?;
    put_u32(out, len);
    out.extend_from_slice(value);
    Ok(())
}
fn put_string(out: &mut Vec<u8>, value: &str) -> Result<(), String> {
    if value.is_empty() { return Err("registry string must not be empty".into()); }
    put_bytes(out, value.as_bytes())
}

impl RegistryOperation {
    /// Stable, explicit binary encoding. Do not replace with serde/bincode:
    /// these bytes are part of the authorization and consensus identity.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        out.extend_from_slice(REGISTRY_OPERATION_DOMAIN);
        out.push(1); // operation codec version
        match self {
            Self::RegisterChain { chain_id, name, network, finality_depth } => {
                out.push(1); put_u32(&mut out, *chain_id);
                put_string(&mut out, name)?; put_string(&mut out, network)?;
                put_u64(&mut out, *finality_depth);
            }
            Self::UpdateChainStatus { chain_id, status } => {
                out.push(2); put_u32(&mut out, *chain_id); out.push(*status as u8);
            }
            Self::RegisterCanonicalAsset { canonical_asset_id, symbol, name, decimals } => {
                out.push(3); out.extend_from_slice(canonical_asset_id);
                put_string(&mut out, symbol)?; put_string(&mut out, name)?; out.push(*decimals);
            }
            Self::AddAssetRepresentation { canonical_asset_id, chain_id, external_asset_id, asset_kind, decimals } => {
                out.push(4); out.extend_from_slice(canonical_asset_id); put_u32(&mut out, *chain_id);
                put_bytes(&mut out, external_asset_id)?; out.push(*asset_kind); out.push(*decimals);
            }
            Self::UpdateAssetStatus { canonical_asset_id, status } => {
                out.push(5); out.extend_from_slice(canonical_asset_id); out.push(*status as u8);
            }
            Self::RevokeAssetRepresentation { canonical_asset_id, chain_id, external_asset_id } => {
                out.push(6); out.extend_from_slice(canonical_asset_id); put_u32(&mut out, *chain_id);
                put_bytes(&mut out, external_asset_id)?;
            }
            Self::UpdateRegistryAuthorizationPolicy { policy_id, threshold, members } => {
                out.push(7); out.extend_from_slice(policy_id); put_u16(&mut out, *threshold);
                let count = u16::try_from(members.len()).map_err(|_| "too many registry policy members".to_string())?;
                put_u16(&mut out, count);
                for (signer_id, public_key) in members {
                    out.extend_from_slice(signer_id);
                    out.extend_from_slice(public_key);
                }
            }
        }
        Ok(out)
    }

    pub fn operation_hash(&self) -> Result<Hash, String> {
        Ok(hash_bytes(&self.encode_canonical()?))
    }
}

impl RegistryOperationEnvelope {
    pub fn signing_message(&self) -> Result<Vec<u8>, String> {
        if self.protocol_version != REGISTRY_PROTOCOL_VERSION {
            return Err("unsupported registry protocol version".into());
        }
        let operation_hash = self.operation.operation_hash()?;
        let mut out = Vec::with_capacity(REGISTRY_AUTH_DOMAIN.len() + 2 + 4 + 32 + 8 + 32);
        out.extend_from_slice(REGISTRY_AUTH_DOMAIN);
        put_u16(&mut out, self.protocol_version);
        put_u32(&mut out, self.network_id);
        out.extend_from_slice(&self.policy_id);
        put_u64(&mut out, self.nonce);
        out.extend_from_slice(&operation_hash);
        Ok(out)
    }

    pub fn encode_canonical(&self) -> Result<Vec<u8>, String> {
        let mut out = self.signing_message()?;
        let count = u16::try_from(self.authorization.signatures.len())
            .map_err(|_| "too many registry signatures".to_string())?;
        put_u16(&mut out, count);
        let mut previous: Option<Hash> = None;
        for item in &self.authorization.signatures {
            if previous.is_some_and(|p| p >= item.signer_id) {
                return Err("registry signatures must be sorted and unique".into());
            }
            previous = Some(item.signer_id);
            out.extend_from_slice(&item.signer_id);
            if item.signature.len() != 64 {
                return Err("registry signature must be exactly 64 bytes".into());
            }
            out.extend_from_slice(&item.signature);
        }
        Ok(out)
    }

    pub fn verify_authorization(&self, policy: &RegistryAuthorizationPolicy) -> Result<(), String> {
        if self.policy_id != policy.policy_id {
            return Err("registry policy id mismatch".into());
        }
        if policy.threshold < MIN_REGISTRY_THRESHOLD {
            return Err("registry threshold cannot be below four".into());
        }
        if policy.members.len() < policy.threshold as usize {
            return Err("registry policy has fewer members than its threshold".into());
        }
        if self.authorization.signatures.len() < policy.threshold as usize {
            return Err("insufficient registry signatures".into());
        }
        let message = self.signing_message()?;
        let mut seen = BTreeSet::new();
        let mut previous: Option<Hash> = None;
        for item in &self.authorization.signatures {
            if previous.is_some_and(|p| p >= item.signer_id) {
                return Err("registry signatures must be sorted and unique".into());
            }
            previous = Some(item.signer_id);
            if !seen.insert(item.signer_id) {
                return Err("duplicate registry signer".into());
            }
            let public_key = policy.members.get(&item.signer_id)
                .ok_or_else(|| "registry signer is not an active policy member".to_string())?;
            let signature: &[u8; 64] = item.signature.as_slice().try_into()
                .map_err(|_| "registry signature must be exactly 64 bytes".to_string())?;
            if !freedom_crypto::verify(public_key, &message, signature) {
                return Err("invalid registry Ed25519 signature".into());
            }
        }
        Ok(())
    }
}

impl RegistryAuthorizationPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.threshold < MIN_REGISTRY_THRESHOLD {
            return Err("registry threshold cannot be below four".into());
        }
        if self.members.len() < self.threshold as usize {
            return Err("registry policy has fewer members than its threshold".into());
        }
        if self.members.keys().any(|id| *id == [0; 32]) {
            return Err("zero registry signer id is invalid".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn policy_and_keys() -> (RegistryAuthorizationPolicy, Vec<SigningKey>) {
        let keys: Vec<_> = (1u8..=5).map(|n| SigningKey::from_bytes(&[n; 32])).collect();
        let members = keys.iter().map(|key| {
            let public_key = key.verifying_key().to_bytes();
            (hash_bytes(&public_key), public_key)
        }).collect();
        (RegistryAuthorizationPolicy { policy_id: [9; 32], threshold: 4, members }, keys)
    }

    fn signed_envelope(keys: &[SigningKey], operation: RegistryOperation) -> RegistryOperationEnvelope {
        let mut envelope = RegistryOperationEnvelope {
            protocol_version: REGISTRY_PROTOCOL_VERSION,
            network_id: 1,
            policy_id: [9; 32],
            nonce: 1,
            operation,
            authorization: RegistryAuthorization { signatures: Vec::new() },
        };
        let message = envelope.signing_message().unwrap();
        envelope.authorization.signatures = keys.iter().take(4).map(|key| {
            let public_key = key.verifying_key().to_bytes();
            RegistrySignature {
                signer_id: hash_bytes(&public_key),
                signature: key.sign(&message).to_bytes().to_vec(),
            }
        }).collect();
        envelope.authorization.signatures.sort_by_key(|s| s.signer_id);
        envelope
    }

    #[test]
    fn four_of_five_registry_signatures_are_accepted() {
        let (policy, keys) = policy_and_keys();
        let envelope = signed_envelope(&keys, RegistryOperation::RegisterChain {
            chain_id: 100, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 64,
        });
        assert!(envelope.verify_authorization(&policy).is_ok());
        assert!(envelope.encode_canonical().is_ok());
    }

    #[test]
    fn fewer_than_four_signatures_are_rejected() {
        let (policy, keys) = policy_and_keys();
        let mut envelope = signed_envelope(&keys, RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        });
        envelope.authorization.signatures.pop();
        assert!(envelope.verify_authorization(&policy).is_err());
    }

    #[test]
    fn tampered_operation_invalidates_signatures() {
        let (policy, keys) = policy_and_keys();
        let mut envelope = signed_envelope(&keys, RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        });
        envelope.operation = RegistryOperation::UpdateChainStatus {
            chain_id: 101, status: ChainStatus::Paused,
        };
        assert!(envelope.verify_authorization(&policy).is_err());
    }

    #[test]
    fn duplicate_or_unsorted_signers_are_rejected() {
        let (policy, keys) = policy_and_keys();
        let mut envelope = signed_envelope(&keys, RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        });
        envelope.authorization.signatures[1].signer_id = envelope.authorization.signatures[0].signer_id;
        assert!(envelope.verify_authorization(&policy).is_err());
    }

    #[test]
    fn policy_cannot_lower_threshold_below_four() {
        let mut policy = policy_and_keys().0;
        policy.threshold = 3;
        assert!(policy.validate().is_err());
    }
}
