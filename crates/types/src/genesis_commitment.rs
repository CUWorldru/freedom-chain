use crate::registry_authorization::RegistryAuthorizationPolicy;
use crate::Validator;
use freedom_primitives::{hash_bytes, Hash, CHAIN_ID};
use std::collections::BTreeSet;

pub const GENESIS_COMMITMENT_DOMAIN: &[u8] = b"321MORE-GENESIS-COMMITMENT-V1";
pub const GENESIS_COMMITMENT_VERSION: u16 = 1;

/// Consensus-critical genesis configuration. The expected commitment must be
/// pinned independently by the node release/configuration; hashing a mutable
/// manifest alone does not establish trust.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenesisCommitmentManifest {
    pub chain_id: u32,
    pub network_id: u32,
    pub initial_supply: u128,
    /// Validator order is preserved because it may be meaningful to genesis
    /// initialization and must therefore be committed exactly.
    pub validators: Vec<Validator>,
    pub registry_policy: RegistryAuthorizationPolicy,
    pub registry_activation_height: u64,
}

impl GenesisCommitmentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.chain_id != CHAIN_ID {
            return Err("genesis commitment has an unexpected chain id".into());
        }
        if self.network_id == 0 {
            return Err("genesis network id must be non-zero".into());
        }
        if self.validators.is_empty() {
            return Err("genesis validator set must not be empty".into());
        }
        let mut validator_ids = BTreeSet::new();
        for validator in &self.validators {
            if validator.validator_id == [0; 32] || !validator_ids.insert(validator.validator_id) {
                return Err("genesis validator ids must be non-zero and unique".into());
            }
            if hash_bytes(&validator.consensus_key) != validator.validator_id {
                return Err("genesis validator id must match its consensus public key".into());
            }
        }
        self.registry_policy.validate()?;
        Ok(())
    }

    /// Explicit canonical encoding. Do not replace with serde/bincode.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let validator_count = u32::try_from(self.validators.len())
            .map_err(|_| "too many genesis validators".to_string())?;
        let policy_count = u16::try_from(self.registry_policy.members.len())
            .map_err(|_| "too many genesis registry members".to_string())?;

        let mut out = Vec::new();
        out.extend_from_slice(GENESIS_COMMITMENT_DOMAIN);
        out.extend_from_slice(&GENESIS_COMMITMENT_VERSION.to_le_bytes());
        out.extend_from_slice(&self.chain_id.to_le_bytes());
        out.extend_from_slice(&self.network_id.to_le_bytes());
        out.extend_from_slice(&self.initial_supply.to_le_bytes());
        out.extend_from_slice(&self.registry_activation_height.to_le_bytes());

        out.extend_from_slice(&validator_count.to_le_bytes());
        for validator in &self.validators {
            out.extend_from_slice(&validator.validator_id);
            out.extend_from_slice(&validator.owner);
            out.extend_from_slice(&validator.consensus_key);
            out.extend_from_slice(&validator.stake.to_le_bytes());
            out.extend_from_slice(&validator.unbonding_stake.to_le_bytes());
            out.extend_from_slice(&validator.blocks_proposed.to_le_bytes());
            out.extend_from_slice(&validator.blocks_finalized.to_le_bytes());
            out.extend_from_slice(&validator.missed_votes.to_le_bytes());
        }

        out.extend_from_slice(&self.registry_policy.policy_id);
        out.extend_from_slice(&self.registry_policy.threshold.to_le_bytes());
        out.extend_from_slice(&policy_count.to_le_bytes());
        for (signer_id, public_key) in &self.registry_policy.members {
            out.extend_from_slice(signer_id);
            out.extend_from_slice(public_key);
        }
        Ok(out)
    }

    pub fn commitment_hash(&self) -> Result<Hash, String> {
        Ok(hash_bytes(&self.encode_canonical()?))
    }

    /// trusted_expected must come from a separately trusted/pinned source,
    /// not from the same replaceable manifest file.
    pub fn verify_pinned(&self, trusted_expected: &Hash) -> Result<Hash, String> {
        if *trusted_expected == [0; 32] {
            return Err("trusted genesis commitment must not be zero".into());
        }
        let actual = self.commitment_hash()?;
        if actual != *trusted_expected {
            return Err("genesis commitment does not match the trusted network anchor".into());
        }
        Ok(actual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use std::collections::BTreeMap;

    fn manifest() -> GenesisCommitmentManifest {
        let keys: Vec<_> = (1u8..=4).map(|n| SigningKey::from_bytes(&[n; 32])).collect();
        let members: BTreeMap<_, _> = keys.iter().map(|key| {
            let public_key = key.verifying_key().to_bytes();
            (hash_bytes(&public_key), public_key)
        }).collect();
        let validators: Vec<_> = keys.iter().take(3).map(|key| {
            let consensus_key = key.verifying_key().to_bytes();
            Validator {
                validator_id: hash_bytes(&consensus_key),
                owner: freedom_primitives::address_from_public_key(&consensus_key),
                consensus_key,
                stake: 0,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        }).collect();
        GenesisCommitmentManifest {
            chain_id: CHAIN_ID,
            network_id: 1,
            initial_supply: 0,
            validators,
            registry_policy: RegistryAuthorizationPolicy {
                policy_id: [9; 32],
                threshold: 4,
                members,
            },
            registry_activation_height: 0,
        }
    }

    #[test]
    fn canonical_commitment_is_deterministic() {
        let value = manifest();
        assert_eq!(value.commitment_hash().unwrap(), value.commitment_hash().unwrap());
        assert_eq!(value.encode_canonical().unwrap(), value.encode_canonical().unwrap());
    }

    #[test]
    fn pinned_anchor_accepts_exact_manifest_and_rejects_tampering() {
        let value = manifest();
        let pinned = value.commitment_hash().unwrap();
        assert_eq!(value.verify_pinned(&pinned).unwrap(), pinned);

        let mut changed = value.clone();
        changed.network_id = 2;
        assert!(changed.verify_pinned(&pinned).is_err());
    }

    #[test]
    fn rejects_duplicate_or_mismatched_validator_identity() {
        let mut value = manifest();
        value.validators.push(value.validators[0].clone());
        assert!(value.validate().is_err());

        let mut value = manifest();
        value.validators[0].validator_id = [0xAA; 32];
        assert!(value.validate().is_err());
    }

    #[test]
    fn rejects_zero_trust_anchor() {
        assert!(manifest().verify_pinned(&[0; 32]).is_err());
    }
}
