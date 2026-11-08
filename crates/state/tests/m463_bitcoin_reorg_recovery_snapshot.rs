use ed25519_dalek::{Signer, SigningKey};
use freedom_primitives::hash_bytes;
use freedom_state::ChainState;
use freedom_types::{
    BitcoinReorgEvidence, BitcoinReorgPolicy, BitcoinReorgRecovery,
    BitcoinReorgRecoverySignature, Genesis, Validator,
};

const BRIDGE_ID: [u8; 32] = [91; 32];

fn state_with_recovery(valid_signature: bool, include_evidence: bool) -> ChainState {
    let key = SigningKey::from_bytes(&[7; 32]);
    let consensus_key = key.verifying_key().to_bytes();
    let validator_id = hash_bytes(&consensus_key);
    let validator = Validator {
        validator_id,
        owner: [8; 32],
        consensus_key,
        stake: 0,
        unbonding_stake: 0,
        blocks_proposed: 0,
        blocks_finalized: 0,
        missed_votes: 0,
    };
    let mut state = ChainState::genesis(&Genesis {
        chain_id: freedom_primitives::CHAIN_ID,
        initial_supply: 0,
        validators: vec![validator],
    });

    state.bitcoin_reorg_policy.insert(
        BRIDGE_ID,
        BitcoinReorgPolicy {
            elevated_confirmations: 6,
            critical_chainwork_margin: [1; 32],
        },
    );

    let mut evidence = BitcoinReorgEvidence {
        checkpoint_height: 100,
        checkpoint_hash: [10; 32],
        candidate_tip_height: 101,
        candidate_tip_hash: [11; 32],
        candidate_chainwork: [12; 32],
        fingerprint: [0; 32],
    };
    evidence.fingerprint = evidence.compute_fingerprint();
    if include_evidence {
        state.bitcoin_reorg_evidence.insert(BRIDGE_ID, vec![evidence.clone()]);
    }

    let mut recovery = BitcoinReorgRecovery {
        bridge_id: BRIDGE_ID,
        evidence_fingerprint: evidence.fingerprint,
        nonce: 1,
        canonical_height: 101,
        canonical_block_hash: [13; 32],
        canonical_chainwork: [14; 32],
        validator_ids: vec![validator_id],
        signatures: Vec::new(),
        recovery_hash: [0; 32],
    };
    let signature = if valid_signature {
        key.sign(&recovery.signing_bytes()).to_bytes().to_vec()
    } else {
        vec![0; 64]
    };
    recovery.signatures.push(BitcoinReorgRecoverySignature {
        validator_id,
        signature,
    });
    recovery.recovery_hash = recovery.compute_hash();
    state.bitcoin_reorg_recovery.insert(BRIDGE_ID, recovery);
    state
}

#[test]
fn persisted_reorg_recovery_rejects_invalid_validator_signatures() {
    let state = state_with_recovery(false, true);
    assert!(state.validate_finality_state_integrity().is_err());
}

#[test]
fn persisted_reorg_recovery_requires_matching_evidence() {
    let state = state_with_recovery(true, false);
    assert!(state.validate_finality_state_integrity().is_err());
}

#[test]
fn persisted_reorg_recovery_accepts_authenticated_recovery_with_evidence() {
    let state = state_with_recovery(true, true);
    assert!(state.validate_finality_state_integrity().is_ok());
}

#[test]
fn persisted_reorg_recovery_rejects_unknown_validator_ids() {
    let mut state = state_with_recovery(true, true);
    let recovery = state.bitcoin_reorg_recovery.get_mut(&BRIDGE_ID).unwrap();
    let unknown_id = [99; 32];
    recovery.validator_ids.push(unknown_id);
    recovery.signatures.push(BitcoinReorgRecoverySignature {
        validator_id: unknown_id,
        signature: vec![0; 64],
    });
    recovery.recovery_hash = recovery.compute_hash();
    assert!(state.validate_finality_state_integrity().is_err());
}
