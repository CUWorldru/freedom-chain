pub mod bridge_config;
use freedom_consensus::{block_hash,receipts_root,tx_root,validate_genesis,ConsensusConfig};
use freedom_crypto::verify;
use freedom_primitives::{CHAIN_ID,Hash};
use freedom_state::{ChainState,StateError};
use freedom_types::{Block,Genesis,Receipt,Transaction};
pub fn collect_validator_set_certificate(
    state: &ChainState,
    votes: &[freedom_types::ValidatorSetVote],
) -> Result<freedom_types::ValidatorSetCertificate, String> {
    if let Some(existing) = state.validator_set_certificate.as_ref() {
        // A transition certificate is immutable once committed. Do this
        // before collecting new votes so no alternate collection path can
        // manufacture a replacement certificate for the same epoch.
        return Err("validator set certificate is already committed for this transition".into());
    }
    if votes.is_empty() { return Err("no validator set votes".into()); }
    let mut unique = std::collections::BTreeSet::new();
    for vote in votes {
        freedom_network::validate_validator_set_vote(state, vote)?;
        if !unique.insert(vote.validator_id) {
            return Err("duplicate validator set vote".into());
        }
    }
    let to_epoch = state.epoch
        .checked_add(1)
        .ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())?;
    let certificate = freedom_types::ValidatorSetCertificate {
        from_epoch: state.epoch,
        to_epoch,
        previous_set_hash: freedom_consensus::validator_set_hash(&state.active_validator_set),
        new_set_hash: freedom_consensus::validator_set_hash(&state.pending_validator_set),
        votes: votes.to_vec(),
    };
    // Collection must use exactly the same full certificate validation as
    // block admission. Do not publish a locally accepted stake-only
    // certificate that peers would reject for insufficient validator-count
    // quorum or invalid transition fields.
    freedom_consensus::validate_validator_set_certificate(state, &certificate)?;
    if let Some(existing) = state.validator_set_certificate.as_ref() {
        if existing != &certificate {
            return Err("validator set certificate is already committed for this transition".into());
        }
    }
    Ok(certificate)
}

pub fn commit_validator_set_certificate(
    state: &mut ChainState,
    certificate: &freedom_types::ValidatorSetCertificate,
) -> Result<(), String> {
    freedom_consensus::validate_validator_set_certificate(state, certificate)?;
    if let Some(existing) = state.validator_set_certificate.as_ref() {