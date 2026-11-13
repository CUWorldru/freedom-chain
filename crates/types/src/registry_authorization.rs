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
    /// Validate operation fields before they become part of a signed payload.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::RegisterChain { chain_id, name, network, finality_depth } => {
                if *chain_id == 0 { return Err("registry chain id must be non-zero".into()); }
                if name.trim().is_empty() || network.trim().is_empty() {
                    return Err("registry chain name and network must not be empty".into());
                }
                if *finality_depth == 0 { return Err("registry finality depth must be non-zero".into()); }
            }
            Self::UpdateChainStatus { chain_id, .. } => {
                if *chain_id == 0 { return Err("registry chain id must be non-zero".into()); }
            }
            Self::RegisterCanonicalAsset { canonical_asset_id, symbol, name, .. } => {
                if *canonical_asset_id == [0; 32] { return Err("canonical asset id must be non-zero".into()); }
                if symbol.trim().is_empty() || name.trim().is_empty() {
                    return Err("canonical asset symbol and name must not be empty".into());
                }
            }
            Self::AddAssetRepresentation { canonical_asset_id, chain_id, external_asset_id, .. }
            | Self::RevokeAssetRepresentation { canonical_asset_id, chain_id, external_asset_id } => {
                if *canonical_asset_id == [0; 32] { return Err("canonical asset id must be non-zero".into()); }
                if *chain_id == 0 { return Err("registry chain id must be non-zero".into()); }
                if external_asset_id.is_empty() { return Err("external asset id must not be empty".into()); }
            }
            Self::UpdateAssetStatus { canonical_asset_id, .. } => {
                if *canonical_asset_id == [0; 32] { return Err("canonical asset id must be non-zero".into()); }
            }
            Self::UpdateRegistryAuthorizationPolicy { policy_id, threshold, members } => {
                let policy = RegistryAuthorizationPolicy {
                    policy_id: *policy_id, threshold: *threshold, members: members.clone(),
                };
                policy.validate()?;
            }
        }
        Ok(())
    }

}

impl RegistryOperation {
    /// Stable, explicit binary encoding. Do not replace with serde/bincode:
    /// these bytes are part of the authorization and consensus identity.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
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
        if self.network_id == 0 { return Err("registry network id must be non-zero".into()); }
        if self.nonce == 0 { return Err("registry nonce must be non-zero".into()); }
        if self.policy_id == [0; 32] { return Err("registry policy id must be non-zero".into()); }
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
        policy.validate()?;
        if self.policy_id != policy.policy_id {
            return Err("registry policy id mismatch".into());
        }
        if policy.threshold < MIN_REGISTRY_THRESHOLD {
            return Err("registry threshold cannot be below four".into());
        }
        if policy.members.len() < policy.threshold as usize {
            return Err("registry policy has fewer members than its threshold".into());
        }
        if self.authorization.signatures.len() > u16::MAX as usize {
            return Err("too many registry signatures".into());
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
        if self.policy_id == [0; 32] { return Err("registry policy id must be non-zero".into()); }
        if self.threshold < MIN_REGISTRY_THRESHOLD {
            return Err("registry threshold cannot be below four".into());
        }
        if self.members.len() < self.threshold as usize {
            return Err("registry policy has fewer members than its threshold".into());
        }
        if self.members.len() > u16::MAX as usize {
            return Err("too many registry policy members".into());
        }
        if self.members.keys().any(|id| *id == [0; 32]) {
            return Err("zero registry signer id is invalid".into());
        }
        for (signer_id, public_key) in &self.members {
            if hash_bytes(public_key) != *signer_id {
                return Err("registry signer id must equal the BLAKE3 hash of its Ed25519 public key".into());
            }
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

    #[test]
    fn policy_rejects_signer_id_not_bound_to_public_key() {
        let mut policy = policy_and_keys().0;
        let first = *policy.members.keys().next().unwrap();
        let public_key = policy.members.remove(&first).unwrap();
        policy.members.insert([0xA5; 32], public_key);
        assert!(policy.validate().is_err());
    }

    #[test]
    fn authorization_rejects_malformed_policy_before_signature_checks() {
        let (mut policy, keys) = policy_and_keys();
        let first = *policy.members.keys().next().unwrap();
        let public_key = policy.members.remove(&first).unwrap();
        policy.members.insert([0xA5; 32], public_key);
        let envelope = signed_envelope(&keys, RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        });
        assert!(envelope.verify_authorization(&policy).is_err());
    }
    #[test]
    fn invalid_operation_fields_are_rejected_before_encoding() {
        let cases = vec![
            RegistryOperation::RegisterChain { chain_id: 0, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 64 },
            RegistryOperation::RegisterChain { chain_id: 1, name: " ".into(), network: "mainnet".into(), finality_depth: 64 },
            RegistryOperation::RegisterChain { chain_id: 1, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 0 },
            RegistryOperation::RegisterCanonicalAsset { canonical_asset_id: [0; 32], symbol: "ABC".into(), name: "Asset".into(), decimals: 18 },
            RegistryOperation::AddAssetRepresentation { canonical_asset_id: [1; 32], chain_id: 1, external_asset_id: Vec::new(), asset_kind: 1, decimals: 18 },
            RegistryOperation::RevokeAssetRepresentation { canonical_asset_id: [1; 32], chain_id: 0, external_asset_id: vec![1] },
        ];
        for operation in cases {
            assert!(operation.validate().is_err());
            assert!(operation.encode_canonical().is_err());
        }
    }

    #[test]
    fn policy_update_requires_a_valid_new_policy() {
        let mut policy = policy_and_keys().0;
        policy.threshold = 3;
        let operation = RegistryOperation::UpdateRegistryAuthorizationPolicy {
            policy_id: policy.policy_id, threshold: policy.threshold, members: policy.members,
        };
        assert!(operation.validate().is_err());
        assert!(operation.encode_canonical().is_err());
    }

    #[test]
    fn envelope_rejects_zero_network_nonce_and_policy_id() {
        let (policy, keys) = policy_and_keys();
        let mut envelope = signed_envelope(&keys, RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        });
        envelope.network_id = 0;
        assert!(envelope.signing_message().is_err());
        envelope.network_id = 1;
        envelope.nonce = 0;
        assert!(envelope.signing_message().is_err());
        envelope.nonce = 1;
        envelope.policy_id = [0; 32];
        assert!(envelope.verify_authorization(&policy).is_err());
    }

    #[test]
    fn registry_collection_limits_match_u16_canonical_encoding() {
        assert!(u16::try_from(u16::MAX as usize).is_ok());
        assert!(u16::try_from(u16::MAX as usize + 1).is_err());
    }

}


/// Transport encoding is separate from signing_message() and encode_canonical():
/// those remain the authorization preimage plus signatures; this payload carries
/// the operation bytes needed by nodes.
pub const REGISTRY_PAYLOAD_DOMAIN: &[u8] = b"321MORE-REGISTRY-PAYLOAD-V1";
const MAX_REGISTRY_PAYLOAD_BYTES: usize = 1_048_576;
const MAX_REGISTRY_FIELD_BYTES: usize = MAX_REGISTRY_PAYLOAD_BYTES;
const MAX_REGISTRY_POLICY_MEMBERS: usize = 1_024;
const MAX_REGISTRY_SIGNATURES: usize = 1_024;

struct RegistryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> RegistryReader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() > MAX_REGISTRY_PAYLOAD_BYTES { return Err("registry payload exceeds maximum size".into()); }
        Ok(Self { bytes, offset: 0 })
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self.offset.checked_add(len).ok_or_else(|| "registry payload length overflow".to_string())?;
        let value = self.bytes.get(self.offset..end).ok_or_else(|| "truncated registry payload".to_string())?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, String> { Ok(self.take(1)?[0]) }
    fn u16(&mut self) -> Result<u16, String> { Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("fixed length"))) }
    fn u32(&mut self) -> Result<u32, String> { Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("fixed length"))) }
    fn u64(&mut self) -> Result<u64, String> { Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("fixed length"))) }
    fn hash(&mut self) -> Result<Hash, String> { Ok(self.take(32)?.try_into().expect("fixed length")) }
    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let len = self.u32()? as usize;
        if len > MAX_REGISTRY_FIELD_BYTES { return Err("registry field exceeds maximum size".into()); }
        Ok(self.take(len)?.to_vec())
    }
    fn string(&mut self) -> Result<String, String> {
        let value = String::from_utf8(self.bytes()?).map_err(|_| "registry string is not valid UTF-8".to_string())?;
        if value.is_empty() { return Err("registry string must not be empty".into()); }
        Ok(value)
    }
    fn finish(self) -> Result<(), String> {
        if self.offset != self.bytes.len() { return Err("trailing bytes in registry payload".into()); }
        Ok(())
    }
}

impl RegistryOperation {
    /// Decode the explicit operation wire format and reject non-canonical forms.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = RegistryReader::new(bytes)?;
        if reader.take(REGISTRY_OPERATION_DOMAIN.len())? != REGISTRY_OPERATION_DOMAIN { return Err("invalid registry operation domain".into()); }
        if reader.u8()? != 1 { return Err("unsupported registry operation codec version".into()); }
        let operation = match reader.u8()? {
            1 => Self::RegisterChain { chain_id: reader.u32()?, name: reader.string()?, network: reader.string()?, finality_depth: reader.u64()? },
            2 => {
                let chain_id = reader.u32()?;
                let status = match reader.u8()? {
                    1 => ChainStatus::Active, 2 => ChainStatus::Paused, 3 => ChainStatus::Retired,
                    _ => return Err("unknown registry chain status".into()),
                };
                Self::UpdateChainStatus { chain_id, status }
            }
            3 => Self::RegisterCanonicalAsset { canonical_asset_id: reader.hash()?, symbol: reader.string()?, name: reader.string()?, decimals: reader.u8()? },
            4 => Self::AddAssetRepresentation { canonical_asset_id: reader.hash()?, chain_id: reader.u32()?, external_asset_id: reader.bytes()?, asset_kind: reader.u8()?, decimals: reader.u8()? },
            5 => {
                let canonical_asset_id = reader.hash()?;
                let status = match reader.u8()? {
                    1 => AssetStatus::Active, 2 => AssetStatus::Paused, 3 => AssetStatus::Revoked,
                    _ => return Err("unknown registry asset status".into()),
                };
                Self::UpdateAssetStatus { canonical_asset_id, status }
            }
            6 => Self::RevokeAssetRepresentation { canonical_asset_id: reader.hash()?, chain_id: reader.u32()?, external_asset_id: reader.bytes()? },
            7 => {
                let policy_id = reader.hash()?;
                let threshold = reader.u16()?;
                let count = reader.u16()? as usize;
                if count > MAX_REGISTRY_POLICY_MEMBERS { return Err("registry policy member count exceeds limit".into()); }
                let mut members = BTreeMap::new();
                let mut previous: Option<Hash> = None;
                for _ in 0..count {
                    let signer_id = reader.hash()?;
                    let public_key: [u8; 32] = reader.take(32)?.try_into().expect("fixed length");
                    if previous.is_some_and(|value| value >= signer_id) { return Err("registry policy members must be sorted and unique".into()); }
                    previous = Some(signer_id);
                    members.insert(signer_id, public_key);
                }
                Self::UpdateRegistryAuthorizationPolicy { policy_id, threshold, members }
            }
            _ => return Err("unknown registry operation variant".into()),
        };
        reader.finish()?;
        operation.validate()?;
        if operation.encode_canonical()?.as_slice() != bytes { return Err("non-canonical registry operation encoding".into()); }
        Ok(operation)
    }
}

impl RegistryOperationEnvelope {
    /// Complete consensus transport payload: operation + envelope metadata + signatures.
    pub fn encode_payload(&self) -> Result<Vec<u8>, String> {
        self.signing_message()?;
        let operation = self.operation.encode_canonical()?;
        let mut out = Vec::new();
        out.extend_from_slice(REGISTRY_PAYLOAD_DOMAIN);
        put_bytes(&mut out, &operation)?;
        put_u16(&mut out, self.protocol_version);
        put_u32(&mut out, self.network_id);
        out.extend_from_slice(&self.policy_id);
        put_u64(&mut out, self.nonce);
        let count = u16::try_from(self.authorization.signatures.len()).map_err(|_| "too many registry signatures".to_string())?;
        if count as usize > MAX_REGISTRY_SIGNATURES { return Err("registry signature count exceeds limit".into()); }
        put_u16(&mut out, count);
        let mut previous: Option<Hash> = None;
        for item in &self.authorization.signatures {
            if previous.is_some_and(|value| value >= item.signer_id) { return Err("registry signatures must be sorted and unique".into()); }
            previous = Some(item.signer_id);
            if item.signature.len() != 64 { return Err("registry signature must be exactly 64 bytes".into()); }
            out.extend_from_slice(&item.signer_id);
            out.extend_from_slice(&item.signature);
        }
        if out.len() > MAX_REGISTRY_PAYLOAD_BYTES { return Err("registry payload exceeds maximum size".into()); }
        Ok(out)
    }

    /// Decode a complete transport payload; reject truncation, trailing bytes,
    /// unknown discriminants, oversized fields and non-canonical ordering.
    pub fn decode_payload(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = RegistryReader::new(bytes)?;
        if reader.take(REGISTRY_PAYLOAD_DOMAIN.len())? != REGISTRY_PAYLOAD_DOMAIN { return Err("invalid registry payload domain".into()); }
        let operation = RegistryOperation::decode_canonical(&reader.bytes()?)?;
        let protocol_version = reader.u16()?;
        let network_id = reader.u32()?;
        let policy_id = reader.hash()?;
        let nonce = reader.u64()?;
        let count = reader.u16()? as usize;
        if count > MAX_REGISTRY_SIGNATURES { return Err("registry signature count exceeds limit".into()); }
        let mut signatures = Vec::with_capacity(count);
        let mut previous: Option<Hash> = None;
        for _ in 0..count {
            let signer_id = reader.hash()?;
            if previous.is_some_and(|value| value >= signer_id) { return Err("registry signatures must be sorted and unique".into()); }
            previous = Some(signer_id);
            signatures.push(RegistrySignature { signer_id, signature: reader.take(64)?.to_vec() });
        }
        reader.finish()?;
        let envelope = Self {
            protocol_version, network_id, policy_id, nonce, operation,
            authorization: RegistryAuthorization { signatures },
        };
        if envelope.encode_payload()?.as_slice() != bytes { return Err("non-canonical registry payload encoding".into()); }
        Ok(envelope)
    }
}


#[cfg(test)]
mod registry_payload_codec_tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_envelope(operation: RegistryOperation) -> RegistryOperationEnvelope {
        let keys: Vec<_> = (1u8..=5).map(|n| SigningKey::from_bytes(&[n; 32])).collect();
        let mut envelope = RegistryOperationEnvelope {
            protocol_version: REGISTRY_PROTOCOL_VERSION,
            network_id: 1,
            policy_id: [9; 32],
            nonce: 7,
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
        envelope.authorization.signatures.sort_by_key(|item| item.signer_id);
        envelope
    }

    #[test]
    fn every_registry_operation_variant_roundtrips_canonically() {
        let mut members = BTreeMap::new();
        for n in 1u8..=4 {
            let key = SigningKey::from_bytes(&[n; 32]).verifying_key().to_bytes();
            members.insert(hash_bytes(&key), key);
        }
        let operations = vec![
            RegistryOperation::RegisterChain { chain_id: 10, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 64 },
            RegistryOperation::UpdateChainStatus { chain_id: 10, status: ChainStatus::Paused },
            RegistryOperation::RegisterCanonicalAsset { canonical_asset_id: [1; 32], symbol: "USD".into(), name: "Dollar".into(), decimals: 6 },
            RegistryOperation::AddAssetRepresentation { canonical_asset_id: [1; 32], chain_id: 10, external_asset_id: vec![1, 2, 3], asset_kind: 1, decimals: 6 },
            RegistryOperation::UpdateAssetStatus { canonical_asset_id: [1; 32], status: AssetStatus::Revoked },
            RegistryOperation::RevokeAssetRepresentation { canonical_asset_id: [1; 32], chain_id: 10, external_asset_id: vec![4, 5] },
            RegistryOperation::UpdateRegistryAuthorizationPolicy { policy_id: [2; 32], threshold: 4, members },
        ];
        for operation in operations {
            let encoded = operation.encode_canonical().unwrap();
            assert_eq!(RegistryOperation::decode_canonical(&encoded).unwrap(), operation);
        }
    }

    #[test]
    fn complete_payload_roundtrips_and_preserves_four_of_five_signatures() {
        let envelope = signed_envelope(RegistryOperation::RegisterChain {
            chain_id: 100, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 64,
        });
        let encoded = envelope.encode_payload().unwrap();
        let decoded = RegistryOperationEnvelope::decode_payload(&encoded).unwrap();
        assert_eq!(decoded, envelope);
        assert_eq!(decoded.authorization.signatures.len(), 4);
        assert_eq!(decoded.signing_message().unwrap(), envelope.signing_message().unwrap());
    }

    #[test]
    fn payload_decoder_rejects_truncation_and_trailing_bytes() {
        let encoded = signed_envelope(RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        }).encode_payload().unwrap();
        assert!(RegistryOperationEnvelope::decode_payload(&encoded[..encoded.len() - 1]).is_err());
        let mut trailing = encoded;
        trailing.push(0);
        assert!(RegistryOperationEnvelope::decode_payload(&trailing).is_err());
    }

    #[test]
    fn payload_decoder_rejects_unknown_domain_and_operation_variant() {
        let encoded = signed_envelope(RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        }).encode_payload().unwrap();
        let mut bad_domain = encoded.clone();
        bad_domain[0] ^= 1;
        assert!(RegistryOperationEnvelope::decode_payload(&bad_domain).is_err());

        let mut bad_operation = RegistryOperation::RegisterChain {
            chain_id: 100, name: "Ethereum".into(), network: "mainnet".into(), finality_depth: 64,
        }.encode_canonical().unwrap();
        bad_operation[REGISTRY_OPERATION_DOMAIN.len() + 1] = 0xFF;
        assert!(RegistryOperation::decode_canonical(&bad_operation).is_err());
    }

    #[test]
    fn payload_decoder_rejects_noncanonical_signature_order_and_oversize() {
        let mut encoded = signed_envelope(RegistryOperation::UpdateChainStatus {
            chain_id: 100, status: ChainStatus::Paused,
        }).encode_payload().unwrap();
        // Locate signature entries: domain + operation length/data + fixed envelope + count.
        let op_len_at = REGISTRY_PAYLOAD_DOMAIN.len();
        let op_len = u32::from_le_bytes(encoded[op_len_at..op_len_at + 4].try_into().unwrap()) as usize;
        let signatures_at = op_len_at + 4 + op_len + 2 + 4 + 32 + 8 + 2;
        let first_signer = encoded[signatures_at..signatures_at + 32].to_vec();
        let second_signer = encoded[signatures_at + 96..signatures_at + 128].to_vec();
        encoded[signatures_at..signatures_at + 32].copy_from_slice(&second_signer);
        encoded[signatures_at + 96..signatures_at + 128].copy_from_slice(&first_signer);
        assert!(RegistryOperationEnvelope::decode_payload(&encoded).is_err());
        assert!(RegistryOperationEnvelope::decode_payload(&vec![0; MAX_REGISTRY_PAYLOAD_BYTES + 1]).is_err());
    }
}
