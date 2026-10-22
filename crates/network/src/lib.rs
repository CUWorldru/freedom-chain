use freedom_consensus::block_hash;
use freedom_crypto::{sign, verify};
use freedom_primitives::{Address, Hash};
use freedom_state::ChainState;
use freedom_types::{BlockProposal, ValidatorVote, ValidatorTimeout};
use libp2p::{
    connection_limits,
    request_response::{self, ProtocolSupport},
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify, ping,
    swarm::NetworkBehaviour,
    PeerId, Swarm, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub const NETWORK_PROTOCOL: &str = "/freedom/1";
pub const PROPOSAL_TOPIC: &str = "freedom/1/proposals";
pub const VOTE_TOPIC: &str = "freedom/1/votes";
pub const PRECOMMIT_TOPIC: &str = "freedom/1/precommits";
pub const TIMEOUT_TOPIC: &str = "freedom/1/timeouts";
pub const EVIDENCE_TOPIC: &str = "freedom/1/evidence";
pub const VALSET_VOTE_TOPIC: &str = "freedom/1/validator-set-votes";
pub const VALSET_CERT_TOPIC: &str = "freedom/1/validator-set-certificates";
pub const SYNC_TOPIC: &str = "freedom/1/sync";
pub const MAX_SYNC_BLOCKS: u64 = 32;
/// Hard cap for a request/response JSON payload before serde deserialization.
pub const MAX_SYNC_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_SYNC_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum concurrent sync request/response streams per peer connection.
/// This bounds simultaneous codec buffers; one sync and one snapshot may coexist.
pub const MAX_CONCURRENT_SYNC_STREAMS: usize = 2;
/// Global cap across all peers, enforced by libp2p's connection-limits behaviour.
pub const MAX_ESTABLISHED_CONNECTIONS: u32 = 64;
pub const MAX_ESTABLISHED_INCOMING_CONNECTIONS: u32 = 48;
pub const MAX_ESTABLISHED_OUTGOING_CONNECTIONS: u32 = 16;
pub const MAX_PENDING_INCOMING_CONNECTIONS: u32 = 32;
pub const MAX_PENDING_OUTGOING_CONNECTIONS: u32 = 16;
pub const MAX_SNAPSHOT_STATE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SNAPSHOT_TRANSITIONS: usize = 4096;
pub const MAX_SNAPSHOT_VALIDATORS_PER_SET: usize = 4096;
pub const MAX_SNAPSHOT_TOTAL_VALIDATOR_ENTRIES: usize = 16_384;
pub const MAX_SNAPSHOT_CERTIFICATE_VOTES: usize = 4096;
pub const MAX_SNAPSHOT_TOTAL_CERTIFICATE_VOTES: usize = 65_536;
pub const SYNC_PROTOCOL: &str = "/freedom/1/sync/1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRequest { pub request_id: Hash, pub from_height: u64, pub to_height: u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncResponse { pub request_id: Hash, pub from_height: u64, pub blocks: Vec<freedom_types::Block> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSyncRequest { pub request_id: Hash, pub height: u64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSyncResponse {
    pub request_id: Hash,
    pub chain_id: u64,
    pub height: u64,
    pub block_hash: Hash,
    pub state_root: Hash,
    pub checkpoint: freedom_types::Block,
    pub certificate: freedom_types::FinalityCertificate,
    #[serde(default)]
    pub finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    #[serde(default)]
    pub epoch_transition_certificate: Option<freedom_types::ValidatorSetCertificate>,
    /// Ordered certificates proving each validator-set transition after the
    /// locally trusted epoch. Each item carries the resulting set so the next
    /// certificate can be verified against the immediately preceding set.
    #[serde(default)]
    pub validator_set_transitions: Vec<SnapshotValidatorSetTransition>,
    pub state_json: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncWireRequest { Blocks(SyncRequest), Snapshot(SnapshotSyncRequest) }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncWireResponse { Blocks(SyncResponse), Snapshot(SnapshotSyncResponse) }

pub fn validate_sync_request(req: &SyncRequest) -> Result<(), String> {
    if req.from_height == 0 { return Err("sync request cannot start at genesis".into()); }
    if req.to_height < req.from_height { return Err("sync request range is inverted".into()); }
    let span = req.to_height
        .checked_sub(req.from_height)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "sync request range overflows".to_string())?;
    if span > MAX_SYNC_BLOCKS { return Err("sync request exceeds maximum block range".into()); }
    Ok(())
}

pub fn validate_snapshot_request(req: &SnapshotSyncRequest) -> Result<(), String> { if req.height == 0 { return Err("snapshot request cannot target genesis".into()); } Ok(()) }

struct BoundedSizeWriter {
    written: usize,
    maximum: usize,
    exceeded: bool,
}

impl BoundedSizeWriter {
    fn new(maximum: usize) -> Self {
        Self { written: 0, maximum, exceeded: false }
    }
}

impl Write for BoundedSizeWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.written);
        if bytes.len() > remaining {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "encoded size limit exceeded",
            ));
        }
        // The remaining-capacity check above proves this addition cannot overflow.
        self.written += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedVecWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}

impl BoundedVecWriter {
    fn new(maximum: usize) -> Self {
        Self {
            // Avoid reserving the full protocol cap for small snapshots.
            bytes: Vec::with_capacity(maximum.min(64 * 1024)),
            maximum,
            exceeded: false,
        }
    }
}

impl Write for BoundedVecWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.bytes.len());
        if bytes.len() > remaining {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "encoded size limit exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serialize JSON into a bounded buffer. Unlike to_vec(), this never retains
/// more than `maximum` payload bytes while preparing a network response.
pub fn serialize_bounded_json<T: Serialize>(
    value: &T,
    maximum: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let mut writer = BoundedVecWriter::new(maximum);
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        if writer.exceeded {
            return Err(format!("{label} exceeds maximum encoded size"));
        }
        return Err(format!("{label} serialization failed: {error}"));
    }
    Ok(writer.bytes)
}

fn serialized_size_bounded<T: Serialize>(value: &T, maximum: usize, label: &str) -> Result<usize, String> {
    // Count encoded bytes without retaining a second full JSON buffer. The
    // writer aborts serialization as soon as the configured limit is crossed.
    let mut writer = BoundedSizeWriter::new(maximum);
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        if writer.exceeded {
            return Err(format!("{label} exceeds maximum encoded size"));
        }
        return Err(format!("{label} serialization failed: {error}"));
    }
    Ok(writer.written)
}

fn validate_serialized_size<T: Serialize>(value: &T, maximum: usize, label: &str) -> Result<(), String> {
    serialized_size_bounded(value, maximum, label).map(|_| ())
}

/// Enforce the outbound request cap before handing a request to libp2p.
pub fn validate_sync_wire_request_size(request: &SyncWireRequest) -> Result<(), String> {
    validate_serialized_size(request, MAX_SYNC_REQUEST_BYTES, "sync request")
}

/// Enforce the outbound response cap before handing a response to libp2p.
pub fn validate_sync_wire_response_size(response: &SyncWireResponse) -> Result<(), String> {
    validate_serialized_size(response, MAX_SYNC_RESPONSE_BYTES, "sync response")
}

/// Measure an outbound response using the same bounded serializer used by the
/// size validator. Returns the encoded size without allocating a JSON buffer.
pub fn sync_wire_response_encoded_size(response: &SyncWireResponse) -> Result<usize, String> {
    serialized_size_bounded(response, MAX_SYNC_RESPONSE_BYTES, "sync response")
}

pub fn validate_sync_response(response: &SyncResponse) -> Result<(), String> {
    if response.blocks.is_empty() { return Err("sync response is empty".into()); }
    if response.blocks.len() > MAX_SYNC_BLOCKS as usize { return Err("sync response exceeds maximum block count".into()); }
    if response.from_height == 0 { return Err("sync response cannot start at genesis".into()); }
    for (i, b) in response.blocks.iter().enumerate() {
        let expected_height = response.from_height
            .checked_add(i as u64)
            .ok_or_else(|| "sync response height overflows".to_string())?;
        if b.header.height != expected_height {
            return Err("sync response contains non-contiguous block heights".into());
        }
    }
    if response.blocks.first().map(|b| b.header.height) != Some(response.from_height) {
        return Err("sync response first block does not match from_height".into());
    }
    Ok(())
}

/// Check whether a remotely finalized checkpoint can be appended to the local
/// canonical history with the storage model currently supported by this node.
/// Fast-sync checkpoints above the next height require an ancestry-proof/trusted
/// checkpoint design and must not be activated by overwriting local metadata.
pub fn validate_snapshot_extension(
    local_height: u64,
    local_hash: freedom_primitives::Hash,
    checkpoint: &freedom_types::Block,
) -> Result<(), String> {
    let expected_height = local_height
        .checked_add(1)
        .ok_or_else(|| "local height exceeds u64::MAX".to_string())?;
    if checkpoint.header.height != expected_height {
        return Err("checkpoint is not the next local height; ancestry proof is unsupported".into());
    }
    if checkpoint.header.previous_hash != local_hash {
        return Err("checkpoint parent does not match local canonical tip".into());
    }
    Ok(())
}

/// Validate a bounded block ancestry from a locally trusted canonical tip to a
/// remote checkpoint. The ancestry slice contains intermediate blocks only;
/// the checkpoint is supplied separately and is checked as the final link.
/// This proves hash/height continuity, not finality-set trust or state execution.
pub fn validate_snapshot_ancestry(
    local_height: u64,
    local_hash: freedom_primitives::Hash,
    ancestry: &[freedom_types::Block],
    checkpoint: &freedom_types::Block,
) -> Result<(), String> {
    if ancestry.len() > MAX_SYNC_BLOCKS as usize {
        return Err("snapshot ancestry exceeds maximum block count".into());
    }

    let mut expected_height = local_height
        .checked_add(1)
        .ok_or_else(|| "local height exceeds u64::MAX".to_string())?;
    let mut expected_parent = local_hash;

    for block in ancestry {
        if block.header.height != expected_height {
            return Err("snapshot ancestry contains non-contiguous heights".into());
        }
        if block.header.previous_hash != expected_parent {
            return Err("snapshot ancestry parent hash mismatch".into());
        }
        expected_parent = freedom_consensus::block_hash(block);
        expected_height = expected_height
            .checked_add(1)
            .ok_or_else(|| "snapshot ancestry height exceeds u64::MAX".to_string())?;
    }

    if checkpoint.header.height != expected_height {
        return Err("checkpoint height does not follow supplied ancestry".into());
    }
    if checkpoint.header.previous_hash != expected_parent {
        return Err("checkpoint parent hash does not match supplied ancestry".into());
    }
    Ok(())
}

/// Anchor the peer-supplied finality validator set to a set already trusted by
/// the local pre-checkpoint state. A snapshot's own state must not be used to
/// authenticate the certificate that is supposed to authenticate that snapshot.
pub fn validate_snapshot_validator_set_anchor(
    trusted_set_hash: freedom_primitives::Hash,
    supplied_set: &std::collections::BTreeMap<freedom_primitives::Hash, freedom_types::ValidatorSetEntry>,
) -> Result<(), String> {
    if supplied_set.is_empty() {
        return Err("snapshot finality validator set is empty".into());
    }
    if freedom_consensus::validator_set_hash(supplied_set) != trusted_set_hash {
        return Err("snapshot finality validator set is not anchored to trusted local state".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotValidatorSetTransition {
    pub certificate: freedom_types::ValidatorSetCertificate,
    pub new_validator_set: std::collections::BTreeMap<freedom_primitives::Hash, freedom_types::ValidatorSetEntry>,
}

/// Check the structural continuity of a validator-set transition sequence.
/// This is not a substitute for signature/quorum validation; it ensures that
/// no epoch is skipped, repeated, reordered, or paired with the wrong set.
pub fn validate_snapshot_transition_sequence(
    start_epoch: u64,
    transitions: &[SnapshotValidatorSetTransition],
    target_epoch: u64,
) -> Result<(), String> {
    if transitions.len() > MAX_SNAPSHOT_TRANSITIONS {
        return Err("snapshot validator-set history exceeds maximum transition count".into());
    }

    let mut expected_from = start_epoch;
    for transition in transitions {
        let cert = &transition.certificate;
        let expected_to = expected_from
            .checked_add(1)
            .ok_or_else(|| "validator-set history epoch exceeds u64::MAX".to_string())?;
        if cert.from_epoch != expected_from || cert.to_epoch != expected_to {
            return Err("validator-set history contains a gap, duplicate, or out-of-order transition".into());
        }
        if transition.new_validator_set.is_empty()
            || cert.new_set_hash != freedom_consensus::validator_set_hash(&transition.new_validator_set)
        {
            return Err("validator-set history new set hash mismatch".into());
        }
        expected_from = cert.to_epoch;
    }

    if expected_from != target_epoch {
        return Err("validator-set history does not reach target epoch".into());
    }
    Ok(())
}

/// Verify an ordered validator-set history from a locally trusted state. Each
/// certificate is checked by the existing consensus validator against the
/// immediately preceding set, including signature, validator-count, and stake
/// quorum rules. Peer-provided snapshot state is never used as the trust anchor.
pub fn validate_snapshot_validator_set_history(
    trusted_state: &ChainState,
    transitions: &[SnapshotValidatorSetTransition],
    target_epoch: u64,
    target_set: &std::collections::BTreeMap<freedom_primitives::Hash, freedom_types::ValidatorSetEntry>,
) -> Result<(), String> {
    let mut working = trusted_state.clone();
    if working.active_validator_set.is_empty() {
        return Err("trusted validator set is empty".into());
    }
    validate_snapshot_transition_sequence(trusted_state.epoch, transitions, target_epoch)?;
    for transition in transitions {
        let cert = &transition.certificate;
        let expected_to = working.epoch.checked_add(1)
            .ok_or_else(|| "validator-set history epoch exceeds u64::MAX".to_string())?;
        if cert.from_epoch != working.epoch || cert.to_epoch != expected_to {
            return Err("validator-set history contains a gap or out-of-order transition".into());
        }
        if cert.previous_set_hash != freedom_consensus::validator_set_hash(&working.active_validator_set) {
            return Err("validator-set history previous set does not link to trusted predecessor".into());
        }
        if transition.new_validator_set.is_empty()
            || cert.new_set_hash != freedom_consensus::validator_set_hash(&transition.new_validator_set) {
            return Err("validator-set history new set hash mismatch".into());
        }
        working.pending_validator_set = transition.new_validator_set.clone();
        working.validator_set_history_sets.insert(
            working.epoch,
            working.active_validator_set.iter().map(|(id, entry)| (*id, entry.consensus_key)).collect(),
        );
        working.validator_set_history_entries.insert(working.epoch, working.active_validator_set.clone());
        freedom_consensus::validate_validator_set_certificate(&working, cert)
            .map_err(|e| format!("validator-set history certificate invalid: {e}"))?;
        working.active_validator_set = transition.new_validator_set.clone();
        working.epoch = cert.to_epoch;
        working.validator_set_history.insert(working.epoch, cert.new_set_hash);
        working.validator_set_history_sets.insert(
            working.epoch,
            working.active_validator_set.iter().map(|(id, entry)| (*id, entry.consensus_key)).collect(),
        );
        working.validator_set_history_entries.insert(working.epoch, working.active_validator_set.clone());
        working.validator_set_history_key_hashes.insert(working.epoch, cert.new_set_hash);
    }
    if working.epoch != target_epoch {
        return Err("validator-set history does not reach target epoch".into());
    }
    if working.active_validator_set.is_empty()
        || freedom_consensus::validator_set_hash(&working.active_validator_set)
            != freedom_consensus::validator_set_hash(target_set) {
        return Err("validator-set history target set mismatch".into());
    }
    Ok(())
}

/// Bind the transition history's final validator set to the set that signed the
/// checkpoint finality certificate. Structural continuity alone is insufficient:
/// otherwise a peer could attach a well-formed history that ends in another set.
pub fn validate_snapshot_transition_target_set(
    transitions: &[SnapshotValidatorSetTransition],
    target_epoch: u64,
    target_set: &std::collections::BTreeMap<freedom_primitives::Hash, freedom_types::ValidatorSetEntry>,
) -> Result<(), String> {
    if target_set.is_empty() {
        return Err("snapshot target validator set is empty".into());
    }
    if target_epoch == 0 {
        if !transitions.is_empty() {
            return Err("genesis validator epoch must not contain prior transitions".into());
        }
        return Ok(());
    }

    let last = transitions.last()
        .ok_or_else(|| "snapshot validator-set history has no transition for target epoch".to_string())?;
    if last.certificate.to_epoch != target_epoch {
        return Err("snapshot validator-set history ends at the wrong epoch".into());
    }
    if freedom_consensus::validator_set_hash(&last.new_validator_set)
        != freedom_consensus::validator_set_hash(target_set)
    {
        return Err("snapshot validator-set history target set does not match finality set".into());
    }
    Ok(())
}

fn validate_snapshot_resource_limits(
    finality_validator_set: &std::collections::BTreeMap<freedom_primitives::Hash, freedom_types::ValidatorSetEntry>,
    certificate: &freedom_types::FinalityCertificate,
    epoch_transition_certificate: Option<&freedom_types::ValidatorSetCertificate>,
    transitions: &[SnapshotValidatorSetTransition],
) -> Result<(), String> {
    if transitions.len() > MAX_SNAPSHOT_TRANSITIONS {
        return Err("snapshot validator-set history exceeds maximum transition count".into());
    }
    if finality_validator_set.len() > MAX_SNAPSHOT_VALIDATORS_PER_SET {
        return Err("snapshot finality validator set exceeds maximum validator count".into());
    }
    if certificate.precommits.len() > MAX_SNAPSHOT_CERTIFICATE_VOTES {
        return Err("snapshot finality certificate exceeds maximum vote count".into());
    }

    let mut total_entries = finality_validator_set.len();
    let mut total_votes = certificate.precommits.len();
    if let Some(cert) = epoch_transition_certificate {
        if cert.votes.len() > MAX_SNAPSHOT_CERTIFICATE_VOTES {
            return Err("snapshot epoch transition certificate exceeds maximum vote count".into());
        }
        total_votes = total_votes.checked_add(cert.votes.len())
            .ok_or_else(|| "snapshot certificate vote count overflows".to_string())?;
    }

    for transition in transitions {
        if transition.new_validator_set.len() > MAX_SNAPSHOT_VALIDATORS_PER_SET {
            return Err("snapshot transition validator set exceeds maximum validator count".into());
        }
        total_entries = total_entries.checked_add(transition.new_validator_set.len())
            .ok_or_else(|| "snapshot validator entry count overflows".to_string())?;
        if total_entries > MAX_SNAPSHOT_TOTAL_VALIDATOR_ENTRIES {
            return Err("snapshot validator history exceeds maximum aggregate validator entries".into());
        }
        if transition.certificate.votes.len() > MAX_SNAPSHOT_CERTIFICATE_VOTES {
            return Err("snapshot validator transition certificate exceeds maximum vote count".into());
        }
        total_votes = total_votes.checked_add(transition.certificate.votes.len())
            .ok_or_else(|| "snapshot certificate vote count overflows".to_string())?;
        if total_votes > MAX_SNAPSHOT_TOTAL_CERTIFICATE_VOTES {
            return Err("snapshot validator history exceeds maximum aggregate certificate votes".into());
        }
    }
    if total_votes > MAX_SNAPSHOT_TOTAL_CERTIFICATE_VOTES {
        return Err("snapshot certificates exceed maximum aggregate vote count".into());
    }
    Ok(())
}

pub fn validate_snapshot_response(response: &SnapshotSyncResponse) -> Result<(), String> {
    validate_snapshot_resource_limits(
        &response.finality_validator_set,
        &response.certificate,
        response.epoch_transition_certificate.as_ref(),
        &response.validator_set_transitions,
    )?;
    if response.chain_id != u64::from(freedom_primitives::CHAIN_ID) {
        return Err("snapshot chain id mismatch".into());
    }
    if response.height == 0 {
        return Err("snapshot response cannot target genesis".into());
    }
    if response.state_json.len() > MAX_SNAPSHOT_STATE_BYTES {
        return Err("snapshot state exceeds maximum encoded size".into());
    }
    if response.checkpoint.header.height != response.height {
        return Err("snapshot checkpoint height mismatch".into());
    }
    if freedom_consensus::block_hash(&response.checkpoint) != response.block_hash {
        return Err("snapshot checkpoint hash mismatch".into());
    }
    if response.checkpoint.header.state_root != response.state_root {
        return Err("snapshot checkpoint state root mismatch".into());
    }
    if response.certificate.height != response.height || response.certificate.block_hash != response.block_hash {
        return Err("snapshot certificate identity mismatch".into());
    }
    if response.certificate.round != response.checkpoint.header.round {
        return Err("snapshot certificate round mismatch".into());
    }
    if response.finality_validator_set.is_empty() {
        return Err("snapshot finality validator set is empty".into());
    }
    if let Some(checkpoint_cert) = &response.checkpoint.header.epoch_transition_certificate {
        if freedom_consensus::validator_set_hash(&response.finality_validator_set) != checkpoint_cert.previous_set_hash {
            return Err("snapshot finality validator set hash mismatch".into());
        }
    } else if freedom_consensus::validator_set_hash(&response.finality_validator_set) == [0u8;32] {
        return Err("snapshot finality validator set hash invalid".into());
    }
    freedom_consensus::validate_finality_certificate_for_set(
        &response.finality_validator_set, &response.checkpoint, &response.certificate,
    ).map_err(|e| format!("snapshot finality certificate invalid: {e}"))?;
    if response.certificate.height != response.checkpoint.header.height {
        return Err("snapshot finality height mismatch".into());
    }
    if let Some(checkpoint_cert) = &response.checkpoint.header.epoch_transition_certificate {
        let supplied_cert = response.epoch_transition_certificate.as_ref()
            .ok_or_else(|| "snapshot checkpoint transition certificate missing".to_string())?;
        if supplied_cert.hash() != checkpoint_cert.hash() {
            return Err("snapshot epoch transition certificate mismatch".into());
        }
        if checkpoint_cert.from_epoch.checked_add(1) != Some(checkpoint_cert.to_epoch)
            || checkpoint_cert.to_epoch != response.checkpoint.header.epoch
        {
            return Err("snapshot checkpoint transition epoch does not match checkpoint header".into());
        }
    } else if response.epoch_transition_certificate.is_some() {
        return Err("snapshot supplied unexpected epoch transition certificate".into());
    }

    // Wire responses must include the complete transition prefix from genesis
    // through the set that finalized the checkpoint. The receiver separately
    // anchors the suffix after its locally trusted epoch.
    let target_epoch = response.checkpoint.header.epoch_transition_certificate
        .as_ref()
        .map(|cert| cert.from_epoch)
        .unwrap_or(response.checkpoint.header.epoch);
    validate_snapshot_transition_sequence(0, &response.validator_set_transitions, target_epoch)?;
    validate_snapshot_transition_target_set(
        &response.validator_set_transitions,
        target_epoch,
        &response.finality_validator_set,
    )?;
    Ok(())
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "FreedomEvent")]
pub struct FreedomBehaviour {
    pub connection_limits: connection_limits::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub kademlia: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub sync: request_response::json::Behaviour<SyncWireRequest, SyncWireResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMessage {
    Proposal(BlockProposal),
    ValidatorSetVote(freedom_types::ValidatorSetVote),
    ValidatorSetCertificate(freedom_types::ValidatorSetCertificate),
    Vote(ValidatorVote),
    Precommit(freedom_types::ValidatorPrecommit),
    Timeout(ValidatorTimeout),
    Evidence(freedom_types::EvidenceMessage),
    SyncRequest(SyncRequest),
    SyncResponse(SyncResponse),
}

pub enum FreedomEvent {
    ConnectionLimits,
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Ping(ping::Event),
    Kademlia(libp2p::kad::Event),
    Sync(request_response::Event<SyncWireRequest, SyncWireResponse>),
}

impl From<()> for FreedomEvent {
    fn from(_: ()) -> Self { Self::ConnectionLimits }
}
impl From<std::convert::Infallible> for FreedomEvent {
    fn from(event: std::convert::Infallible) -> Self { match event {} }
}
impl From<gossipsub::Event> for FreedomEvent {
    fn from(event: gossipsub::Event) -> Self { Self::Gossipsub(event) }
}
impl From<identify::Event> for FreedomEvent {
    fn from(event: identify::Event) -> Self { Self::Identify(event) }
}
impl From<ping::Event> for FreedomEvent {
    fn from(event: ping::Event) -> Self { Self::Ping(event) }
}
impl From<request_response::Event<SyncWireRequest, SyncWireResponse>> for FreedomEvent { fn from(event: request_response::Event<SyncWireRequest, SyncWireResponse>) -> Self { Self::Sync(event) } }
impl From<libp2p::kad::Event> for FreedomEvent {
    fn from(event: libp2p::kad::Event) -> Self { Self::Kademlia(event) }
}

pub fn make_proposal(
    state: &ChainState,
    block: freedom_types::Block,
    round: u64,
    consensus_key: &ed25519_dalek::SigningKey,
    valid_round: Option<u64>,
    valid_precommits: Vec<freedom_types::ValidatorPrecommit>,
    timeout_certificate: Option<freedom_types::TimeoutCertificate>,
) -> Result<BlockProposal, String> {
    let proposer = block.header.proposer;
    if block.header.round != round {
        return Err("proposal round does not match block round".into());
    }
    let expected_height = state.height
        .checked_add(1)
        .ok_or_else(|| "proposal height exceeds u64::MAX".to_string())?;
    if block.header.height != expected_height {
        return Err("proposal height is not the next block height".into());
    }
    let expected_proposer = freedom_consensus::proposer_for_height_round(state, block.header.height, round)
        .ok_or_else(|| "no proposer available for proposal height/round".to_string())?;
    if proposer != expected_proposer {
        return Err("proposal proposer is not deterministic for height/round".into());
    }
    let validator = state.active_validator_set.values()
        .find(|entry| entry.owner == proposer)
        .ok_or_else(|| "proposal proposer is not an active validator".to_string())?;
    if validator.consensus_key != consensus_key.verifying_key().to_bytes() {
        return Err("proposal proposer key mismatch".into());
    }
    validate_block_transition(state, &block)?;
    let block_hash = block_hash(&block);
    let mut proposal = BlockProposal {
        block_hash,
        height: block.header.height,
        round,
        proposer,
        block,
        valid_round,
        valid_precommits,
        timeout_certificate,
        signature: Vec::new(),
    };
    sign_proposal(&mut proposal, consensus_key);
    Ok(proposal)
}

fn sign_proposal(proposal: &mut BlockProposal, consensus_key: &ed25519_dalek::SigningKey) {
    proposal.signature = sign(consensus_key, &proposal.signing_bytes()).to_vec();
}

pub fn validate_block_transition(state: &ChainState, block: &freedom_types::Block) -> Result<(), String> {
    freedom_consensus::validate_epoch_transition_certificate(state, block)?;
    let expected_height = state.height
        .checked_add(1)
        .ok_or_else(|| "proposal height exceeds u64::MAX".to_string())?;
    if block.header.height != expected_height { return Err("invalid proposal height".into()); }
    if block.header.previous_hash != state.last_block_hash { return Err("invalid proposal previous hash".into()); }
    if freedom_consensus::tx_root(&block.transactions) != block.header.tx_root { return Err("proposal transaction root mismatch".into()); }

    let mut bridge_ids = std::collections::BTreeSet::new();
    for tx in &block.transactions {
        if tx.kind == freedom_types::TxKind::BridgeMint {
            let payload = freedom_types::BridgeMintTransactionPayload::decode(&tx.payload)
                .map_err(|_| "proposal bridge mint payload invalid".to_string())?;
            let canonical = payload.encode()
                .map_err(|_| "proposal bridge mint payload non-canonical".to_string())?;
            if canonical != tx.payload {
                return Err("proposal bridge mint payload non-canonical".into());
            }
            let id = payload.canonical_id()
                .map_err(|_| "proposal bridge mint canonical id invalid".to_string())?;
            if !bridge_ids.insert(id) {
                return Err("proposal contains duplicate bridge mint canonical id".into());
            }
        }
    }

    let mut working = state.clone();
    let mut receipts = Vec::with_capacity(block.transactions.len());
    for tx in &block.transactions {
        let account = working.accounts.get(&tx.sender).ok_or_else(|| "proposal sender not found".to_string())?;
        if tx.signature.len() != 64 { return Err("proposal transaction signature length invalid".into()); }
        let mut sig = [0u8;64]; sig.copy_from_slice(&tx.signature);
        if !freedom_crypto::verify(&account.public_key, &tx.signing_bytes(), &sig) {
            return Err("proposal transaction signature invalid".into());
        }
        let before = working.events.len();
        working.execute_transaction(tx).map_err(|e| format!("proposal transaction rejected: {e:?}"))?;
        let events = working.events[before..].to_vec();
        receipts.push(freedom_types::Receipt { tx_hash: tx.hash(), success: true, gas_used: 1, events });
    }
    if receipts != block.receipts { return Err("proposal receipts mismatch".into()); }
    if freedom_consensus::receipts_root(&receipts) != block.header.receipts_root { return Err("proposal receipts root mismatch".into()); }
    working.height = block.header.height;
    if working.state_root() != block.header.state_root { return Err("proposal state root mismatch".into()); }
    Ok(())
}

pub fn validate_proposal(state: &ChainState, proposal: &BlockProposal) -> Result<(), String> {
    if proposal.height != proposal.block.header.height { return Err("proposal height mismatch".into()); }
    if proposal.round != proposal.block.header.round {
        return Err("proposal round mismatch".into());
    }
    freedom_consensus::validate_valid_round_proof(
        state,
        proposal.block_hash,
        proposal.height,
        proposal.round,
        proposal.valid_round,
        &proposal.valid_precommits,
    )?;
    if proposal.proposer != proposal.block.header.proposer { return Err("proposal proposer mismatch".into()); }
    let expected = freedom_consensus::proposer_for_height_round(state, proposal.height, proposal.round).ok_or_else(|| "no proposer for round".to_string())?;
    if expected != proposal.proposer { return Err("unexpected proposer for round".into()); }
    if block_hash(&proposal.block) != proposal.block_hash { return Err("proposal block hash mismatch".into()); }
    validate_block_transition(state, &proposal.block)?;
    if let Some(tc) = &proposal.timeout_certificate {
        freedom_consensus::validate_timeout_certificate(state, tc, proposal.height, proposal.round)?;
    }
    let validator = state.active_validator_set.values().find(|v| v.owner == proposal.proposer)
        .ok_or_else(|| "unknown proposal proposer".to_string())?;
    if proposal.signature.len() != 64 { return Err("invalid proposal signature length".into()); }
    let mut sig = [0u8;64]; sig.copy_from_slice(&proposal.signature);
    if !verify(&validator.consensus_key, &proposal.signing_bytes(), &sig) {
        return Err("invalid proposal signature".into());
    }
    Ok(())
}

pub fn make_vote(state: &ChainState, validator_id: Hash, proposal: &BlockProposal, consensus_key: &ed25519_dalek::SigningKey) -> Result<ValidatorVote, String> {
    validate_proposal(state, proposal)?;
    let validator = state.active_validator_set.get(&validator_id)
        .ok_or_else(|| "unknown active voting validator".to_string())?;
    if validator.consensus_key != consensus_key.verifying_key().to_bytes() {
        return Err("voting validator key mismatch".into());
    }
    let mut vote = ValidatorVote {
        block_hash: proposal.block_hash,
        height: proposal.height,
        round: proposal.round,
        validator_id,
        signature: Vec::new(),
    };
    vote.signature = sign(consensus_key, &vote.signing_bytes()).to_vec();
    Ok(vote)
}

pub fn evidence_topic() -> IdentTopic { IdentTopic::new(EVIDENCE_TOPIC) }
pub fn validator_set_vote_topic() -> IdentTopic { IdentTopic::new(VALSET_VOTE_TOPIC) }
pub fn validator_set_certificate_topic() -> IdentTopic { IdentTopic::new(VALSET_CERT_TOPIC) }
pub fn publish_validator_set_vote(
    topic: &IdentTopic,
    vote: &freedom_types::ValidatorSetVote,
) -> Result<Vec<u8>, String> {
    let msg = NetworkMessage::ValidatorSetVote(vote.clone());
    serde_json::to_vec(&msg).map_err(|e| format!("encode validator set vote: {e}"))
}

pub fn publish_validator_set_certificate(
    topic: &IdentTopic,
    certificate: &freedom_types::ValidatorSetCertificate,
) -> Result<Vec<u8>, String> {
    let msg = NetworkMessage::ValidatorSetCertificate(certificate.clone());
    serde_json::to_vec(&msg).map_err(|e| format!("encode validator set certificate: {e}"))
}


pub fn timeout_topic() -> IdentTopic { IdentTopic::new(TIMEOUT_TOPIC) }
pub fn sync_topic() -> IdentTopic { IdentTopic::new(SYNC_TOPIC) }

pub fn make_validator_set_vote(
    state: &ChainState,
    validator_id: Hash,
    to_epoch: u64,
    new_set_hash: Hash,
    consensus_key: &ed25519_dalek::SigningKey,
) -> Result<freedom_types::ValidatorSetVote, String> {
    let expected_to_epoch = state.epoch
        .checked_add(1)
        .ok_or_else(|| "validator set vote target epoch exceeds u64::MAX".to_string())?;
    if to_epoch != expected_to_epoch {
        return Err("invalid validator set vote target epoch".into());
    }
    let previous_set_hash = freedom_consensus::validator_set_hash(&state.active_validator_set);
    let expected_new_set_hash = freedom_consensus::validator_set_hash(&state.pending_validator_set);
    if new_set_hash != expected_new_set_hash {
        return Err("validator set vote new set hash mismatch".into());
    }
    let validator = state.active_validator_set.get(&validator_id)
        .ok_or_else(|| "validator set vote signer is not active".to_string())?;
    if validator.consensus_key != consensus_key.verifying_key().to_bytes() {
        return Err("validator set vote key mismatch".into());
    }
    let mut vote = freedom_types::ValidatorSetVote {
        from_epoch: state.epoch,
        to_epoch,
        previous_set_hash,
        new_set_hash,
        validator_id,
        signature: Vec::new(),
    };
    vote.signature = sign(consensus_key, &vote.signing_bytes()).to_vec();
    Ok(vote)
}

pub fn validate_validator_set_vote(
    state: &ChainState,
    vote: &freedom_types::ValidatorSetVote,
) -> Result<(), String> {
    let expected_to_epoch = state.epoch
        .checked_add(1)
        .ok_or_else(|| "validator set vote target epoch exceeds u64::MAX".to_string())?;
    if vote.from_epoch != state.epoch || vote.to_epoch != expected_to_epoch {
        return Err("invalid validator set vote epoch".into());
    }
    if vote.previous_set_hash != freedom_consensus::validator_set_hash(&state.active_validator_set) {
        return Err("validator set vote previous set mismatch".into());
    }
    if vote.new_set_hash != freedom_consensus::validator_set_hash(&state.pending_validator_set) {
        return Err("validator set vote new set mismatch".into());
    }
    let validator = state.active_validator_set.get(&vote.validator_id)
        .ok_or_else(|| "validator set vote signer is not active".to_string())?;
    if vote.signature.len() != 64 { return Err("invalid validator set vote signature length".into()); }
    let mut sig=[0u8;64]; sig.copy_from_slice(&vote.signature);
    if !verify(&validator.consensus_key, &vote.signing_bytes(), &sig) {
        return Err("invalid validator set vote signature".into());
    }
    Ok(())
}

pub fn make_timeout(
    state: &ChainState,
    height: u64,
    round: u64,
    validator_id: Hash,
    consensus_key: &ed25519_dalek::SigningKey,
) -> Result<ValidatorTimeout, String> {
    let validator = state.active_validator_set.get(&validator_id)
        .ok_or_else(|| "timeout signer is not active".to_string())?;
    if validator.consensus_key != consensus_key.verifying_key().to_bytes() {
        return Err("timeout signer key mismatch".into());
    }
    let mut t = ValidatorTimeout {
        height,
        round,
        validator_id,
        signature: Vec::new(),
    };
    t.signature = sign(consensus_key, &t.signing_bytes()).to_vec();
    Ok(t)
}

pub fn proposal_topic() -> IdentTopic {
    IdentTopic::new(PROPOSAL_TOPIC)
}


pub fn precommit_topic() -> IdentTopic { IdentTopic::new(PRECOMMIT_TOPIC) }

pub fn vote_topic() -> IdentTopic {
    IdentTopic::new(VOTE_TOPIC)
}

pub fn build_swarm() -> Result<Swarm<FreedomBehaviour>, Box<dyn std::error::Error + Send + Sync>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mut config = gossipsub::ConfigBuilder::default();
            config.validation_mode(ValidationMode::Strict);
            let config = config.build().map_err(|e| format!("gossipsub config: {e}"))?;
            let peer_id = PeerId::from(key.public());
            let mut gossipsub = gossipsub::Behaviour::new(
                MessageAuthenticity::Signed(key.clone()),
                config,
            ).map_err(|e| format!("gossipsub init: {e}"))?;
            gossipsub.subscribe(&proposal_topic())?;
            gossipsub.subscribe(&vote_topic())?;
            gossipsub.subscribe(&IdentTopic::new(PRECOMMIT_TOPIC))?;
            gossipsub.subscribe(&IdentTopic::new(TIMEOUT_TOPIC))?;
            gossipsub.subscribe(&sync_topic())?;
            Ok(FreedomBehaviour {
                connection_limits: connection_limits::Behaviour::new(
                    connection_limits::ConnectionLimits::default()
                        .with_max_established(Some(MAX_ESTABLISHED_CONNECTIONS))
                        .with_max_established_incoming(Some(MAX_ESTABLISHED_INCOMING_CONNECTIONS))
                        .with_max_established_outgoing(Some(MAX_ESTABLISHED_OUTGOING_CONNECTIONS))
                        .with_max_pending_incoming(Some(MAX_PENDING_INCOMING_CONNECTIONS))
                        .with_max_pending_outgoing(Some(MAX_PENDING_OUTGOING_CONNECTIONS)),
                ),
                gossipsub,
                identify: identify::Behaviour::new(
                    identify::Config::new(NETWORK_PROTOCOL.to_string(), key.public()),
                ),
                ping: ping::Behaviour::default(),
                kademlia: libp2p::kad::Behaviour::new(peer_id, libp2p::kad::store::MemoryStore::new(peer_id)),
                sync: request_response::json::Behaviour::with_codec(
                    request_response::json::codec::Codec::<SyncWireRequest, SyncWireResponse>::default()
                        .set_request_size_maximum(MAX_SYNC_REQUEST_BYTES as u64)
                        .set_response_size_maximum(MAX_SYNC_RESPONSE_BYTES as u64),
                    std::iter::once((libp2p::StreamProtocol::new(SYNC_PROTOCOL), ProtocolSupport::Full)),
                    request_response::Config::default()
                        .with_max_concurrent_streams(MAX_CONCURRENT_SYNC_STREAMS),
                ),
            })
        })?
        .build();

    Ok(swarm)
}

pub fn local_peer_id(swarm: &Swarm<FreedomBehaviour>) -> PeerId {
    *swarm.local_peer_id()
}

pub fn encode_message(message: &NetworkMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(message)
}

pub fn decode_message(bytes: &[u8]) -> Result<NetworkMessage, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn publish(
    swarm: &mut Swarm<FreedomBehaviour>,
    message: &NetworkMessage,
) -> Result<gossipsub::MessageId, Box<dyn std::error::Error>> {
    let (topic, bytes) = match message {
        NetworkMessage::Proposal(_) => (proposal_topic(), encode_message(message)?),
        NetworkMessage::Vote(_) => (vote_topic(), encode_message(message)?),
        NetworkMessage::Precommit(_) => (IdentTopic::new(PRECOMMIT_TOPIC), encode_message(message)?),
        NetworkMessage::Timeout(_) => (timeout_topic(), encode_message(message)?),
        NetworkMessage::Evidence(_) => (IdentTopic::new(EVIDENCE_TOPIC), encode_message(message)?),
        NetworkMessage::ValidatorSetVote(_) => (validator_set_vote_topic(), encode_message(message)?),
        NetworkMessage::ValidatorSetCertificate(_) => (validator_set_certificate_topic(), encode_message(message)?),
        NetworkMessage::SyncRequest(_) | NetworkMessage::SyncResponse(_) => (sync_topic(), encode_message(message)?),
    };
    Ok(swarm.behaviour_mut().gossipsub.publish(topic, bytes)?)
}

pub fn send_sync_request(swarm: &mut Swarm<FreedomBehaviour>, peer: PeerId, request: SyncWireRequest) -> Result<request_response::OutboundRequestId, String> {
    match &request {
        SyncWireRequest::Blocks(req) => validate_sync_request(req)?,
        SyncWireRequest::Snapshot(req) => validate_snapshot_request(req)?,
    }
    validate_sync_wire_request_size(&request)?;
    Ok(swarm.behaviour_mut().sync.send_request(&peer, request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    fn ancestry_block(height: u64, previous_hash: freedom_primitives::Hash) -> freedom_types::Block {
        freedom_types::Block {
            header: freedom_types::BlockHeader {
                height, round: 0, previous_hash, state_root: [height as u8;32],
                tx_root: [2;32], receipts_root: [3;32], timestamp: height, epoch: 0,
                proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        }
    }

    #[test]
    fn snapshot_ancestry_accepts_contiguous_path_from_trusted_tip() {
        let first = ancestry_block(8, [7;32]);
        let first_hash = freedom_consensus::block_hash(&first);
        let second = ancestry_block(9, first_hash);
        let second_hash = freedom_consensus::block_hash(&second);
        let checkpoint = ancestry_block(10, second_hash);
        assert!(validate_snapshot_ancestry(7, [7;32], &[first, second], &checkpoint).is_ok());
    }

    #[test]
    fn snapshot_ancestry_rejects_missing_intermediate_block() {
        let first = ancestry_block(8, [7;32]);
        let checkpoint = ancestry_block(10, freedom_consensus::block_hash(&first));
        let error = validate_snapshot_ancestry(7, [7;32], &[first], &checkpoint).unwrap_err();
        assert!(error.contains("height"));
    }

    #[test]
    fn snapshot_ancestry_rejects_wrong_parent_link() {
        let first = ancestry_block(8, [7;32]);
        let second = ancestry_block(9, [99;32]);
        let checkpoint = ancestry_block(10, freedom_consensus::block_hash(&second));
        let error = validate_snapshot_ancestry(7, [7;32], &[first, second], &checkpoint).unwrap_err();
        assert!(error.contains("parent hash"));
    }

    #[test]
    fn snapshot_ancestry_rejects_oversized_proof() {
        let ancestry = vec![ancestry_block(0, [0;32]); MAX_SYNC_BLOCKS as usize + 1];
        let checkpoint = ancestry_block(1, [0;32]);
        let error = validate_snapshot_ancestry(0, [0;32], &ancestry, &checkpoint).unwrap_err();
        assert!(error.contains("maximum block count"));
    }

    #[test]
    fn snapshot_ancestry_rejects_height_overflow() {
        let error = validate_snapshot_ancestry(u64::MAX, [7;32], &[], &ancestry_block(u64::MAX, [7;32])).unwrap_err();
        assert!(error.contains("exceeds u64::MAX"));
    }

    #[test]
    fn snapshot_extension_accepts_only_next_height_with_matching_parent() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 8, round: 0, previous_hash: [7;32], state_root: [1;32],
                tx_root: [2;32], receipts_root: [3;32], timestamp: 8, epoch: 0,
                proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(validate_snapshot_extension(7, [7;32], &checkpoint).is_ok());
    }

    #[test]
    fn snapshot_extension_rejects_checkpoint_that_skips_local_history() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 20, round: 0, previous_hash: [7;32], state_root: [1;32],
                tx_root: [2;32], receipts_root: [3;32], timestamp: 20, epoch: 0,
                proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let error = validate_snapshot_extension(7, [7;32], &checkpoint).unwrap_err();
        assert!(error.contains("ancestry proof is unsupported"));
    }

    #[test]
    fn snapshot_extension_rejects_wrong_parent() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 8, round: 0, previous_hash: [9;32], state_root: [1;32],
                tx_root: [2;32], receipts_root: [3;32], timestamp: 8, epoch: 0,
                proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(validate_snapshot_extension(7, [7;32], &checkpoint).unwrap_err().contains("parent"));
    }

    #[test]
    fn snapshot_extension_rejects_height_overflow() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: u64::MAX, round: 0, previous_hash: [7;32], state_root: [1;32],
                tx_root: [2;32], receipts_root: [3;32], timestamp: 1, epoch: 0,
                proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(validate_snapshot_extension(u64::MAX, [7;32], &checkpoint).is_err());
    }

    #[test]
    fn snapshot_transition_target_set_rejects_mismatched_finality_set() {
        let make_set = |seed: u8| {
            let id = hash_bytes(&[b"m475-target-validator".as_slice(), &[seed]].concat());
            let mut set = std::collections::BTreeMap::new();
            set.insert(id, freedom_types::ValidatorSetEntry {
                validator_id: id, owner: [seed; 32], consensus_key: [seed.wrapping_add(1); 32], stake: 10,
            });
            set
        };
        let history_set = make_set(1);
        let different_finality_set = make_set(2);
        let transitions = vec![SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 0, to_epoch: 1, previous_set_hash: [9;32],
                new_set_hash: freedom_consensus::validator_set_hash(&history_set), votes: Vec::new(),
            },
            new_validator_set: history_set,
        }];
        let error = validate_snapshot_transition_target_set(
            &transitions, 1, &different_finality_set,
        ).unwrap_err();
        assert!(error.contains("target set does not match finality set"));
    }

    #[test]
    fn snapshot_transition_target_set_accepts_matching_finality_set() {
        let id = hash_bytes(b"m475-matching-target-validator");
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1;32], consensus_key: [2;32], stake: 10,
        });
        let transitions = vec![SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 0, to_epoch: 1, previous_set_hash: [9;32],
                new_set_hash: freedom_consensus::validator_set_hash(&set), votes: Vec::new(),
            },
            new_validator_set: set.clone(),
        }];
        assert!(validate_snapshot_transition_target_set(&transitions, 1, &set).is_ok());
    }

    #[test]
    fn snapshot_transition_sequence_accepts_complete_epoch_prefix() {
        let make_set = |seed: u8| {
            let id = hash_bytes(&[b"m474-transition-validator".as_slice(), &[seed]].concat());
            let mut set = std::collections::BTreeMap::new();
            set.insert(id, freedom_types::ValidatorSetEntry {
                validator_id: id, owner: [seed; 32], consensus_key: [seed.wrapping_add(1); 32], stake: 10,
            });
            set
        };
        let first_set = make_set(1);
        let second_set = make_set(3);
        let transitions = vec![
            SnapshotValidatorSetTransition {
                certificate: freedom_types::ValidatorSetCertificate {
                    from_epoch: 0, to_epoch: 1, previous_set_hash: [9;32],
                    new_set_hash: freedom_consensus::validator_set_hash(&first_set), votes: Vec::new(),
                },
                new_validator_set: first_set,
            },
            SnapshotValidatorSetTransition {
                certificate: freedom_types::ValidatorSetCertificate {
                    from_epoch: 1, to_epoch: 2, previous_set_hash: [8;32],
                    new_set_hash: freedom_consensus::validator_set_hash(&second_set), votes: Vec::new(),
                },
                new_validator_set: second_set,
            },
        ];
        assert!(validate_snapshot_transition_sequence(0, &transitions, 2).is_ok());
    }

    #[test]
    fn snapshot_transition_sequence_rejects_missing_epoch() {
        let id = hash_bytes(b"m474-gap-validator");
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1;32], consensus_key: [2;32], stake: 10,
        });
        let transition = SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 1, to_epoch: 2, previous_set_hash: [3;32],
                new_set_hash: freedom_consensus::validator_set_hash(&set), votes: Vec::new(),
            },
            new_validator_set: set,
        };
        let error = validate_snapshot_transition_sequence(0, &[transition], 2).unwrap_err();
        assert!(error.contains("gap, duplicate, or out-of-order"));
    }

    #[test]
    fn snapshot_transition_sequence_rejects_duplicate_or_reordered_epoch() {
        let id = hash_bytes(b"m474-reordered-validator");
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1;32], consensus_key: [2;32], stake: 10,
        });
        let make = |from_epoch, to_epoch| SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch, to_epoch, previous_set_hash: [3;32],
                new_set_hash: freedom_consensus::validator_set_hash(&set), votes: Vec::new(),
            },
            new_validator_set: set.clone(),
        };
        let error = validate_snapshot_transition_sequence(0, &[make(0, 1), make(0, 1)], 1).unwrap_err();
        assert!(error.contains("gap, duplicate, or out-of-order"));
    }

    #[test]
    fn snapshot_transition_sequence_rejects_set_hash_mismatch() {
        let id = hash_bytes(b"m474-wrong-set-validator");
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1;32], consensus_key: [2;32], stake: 10,
        });
        let transition = SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 0, to_epoch: 1, previous_set_hash: [3;32],
                new_set_hash: [7;32], votes: Vec::new(),
            },
            new_validator_set: set,
        };
        let error = validate_snapshot_transition_sequence(0, &[transition], 1).unwrap_err();
        assert!(error.contains("new set hash mismatch"));
    }

    #[test]
    fn snapshot_validator_set_history_rejects_epoch_gap() {
        let key = [12u8; 32];
        let id = hash_bytes(b"m472-history-validator");
        let entry = freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1; 32], consensus_key: key, stake: 10,
        };
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, entry);
        let state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: u64::from(freedom_primitives::CHAIN_ID),
            initial_supply: 0,
            validators: vec![freedom_types::Validator {
                validator_id: id, owner: [1; 32], consensus_key: key, stake: 10,
                unbonding_stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }],
        });
        let transition = SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 1, to_epoch: 2, previous_set_hash: freedom_consensus::validator_set_hash(&set),
                new_set_hash: [8; 32], votes: Vec::new(),
            },
            new_validator_set: set.clone(),
        };
        let error = validate_snapshot_validator_set_history(&state, &[transition], 2, &set).unwrap_err();
        assert!(error.contains("gap or out-of-order"));
    }

    #[test]
    fn snapshot_validator_set_history_rejects_target_epoch_mismatch() {
        let key = [13u8; 32];
        let id = hash_bytes(b"m472-target-validator");
        let entry = freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1; 32], consensus_key: key, stake: 10,
        };
        let mut set = std::collections::BTreeMap::new();
        set.insert(id, entry);
        let state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: u64::from(freedom_primitives::CHAIN_ID),
            initial_supply: 0,
            validators: vec![freedom_types::Validator {
                validator_id: id, owner: [1; 32], consensus_key: key, stake: 10,
                unbonding_stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }],
        });
        let error = validate_snapshot_validator_set_history(&state, &[], 1, &set).unwrap_err();
        assert!(error.contains("does not reach target epoch"));
    }

    #[test]
    fn snapshot_validator_set_anchor_accepts_exact_trusted_set() {
        let mut set = std::collections::BTreeMap::new();
        let id = hash_bytes(b"m471-validator");
        set.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id,
            owner: [1; 32],
            consensus_key: [2; 32],
            stake: 10,
        });
        let trusted_hash = freedom_consensus::validator_set_hash(&set);
        assert!(validate_snapshot_validator_set_anchor(trusted_hash, &set).is_ok());
    }

    #[test]
    fn snapshot_validator_set_anchor_rejects_peer_selected_set() {
        let mut trusted = std::collections::BTreeMap::new();
        let id = hash_bytes(b"m471-trusted-validator");
        trusted.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id, owner: [1; 32], consensus_key: [2; 32], stake: 10,
        });
        let mut supplied = trusted.clone();
        supplied.get_mut(&id).unwrap().stake = 11;
        let error = validate_snapshot_validator_set_anchor(
            freedom_consensus::validator_set_hash(&trusted), &supplied,
        ).unwrap_err();
        assert!(error.contains("not anchored"));
    }

    #[test]
    fn snapshot_validator_set_anchor_rejects_empty_set() {
        let empty = std::collections::BTreeMap::new();
        assert!(validate_snapshot_validator_set_anchor([0; 32], &empty)
            .unwrap_err().contains("empty"));
    }

    fn make_resource_test_set(size: usize, offset: usize) -> std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry> {
        let mut set = std::collections::BTreeMap::new();
        for index in offset..offset + size {
            let mut id = [0u8; 32];
            id[..8].copy_from_slice(&(index as u64).to_le_bytes());
            set.insert(id, freedom_types::ValidatorSetEntry {
                validator_id: id, owner: [1; 32], consensus_key: [2; 32], stake: 1,
            });
        }
        set
    }

    #[test]
    fn snapshot_resource_limits_reject_oversized_finality_validator_set() {
        let set = make_resource_test_set(MAX_SNAPSHOT_VALIDATORS_PER_SET + 1, 0);
        let certificate = freedom_types::FinalityCertificate {
            block_hash: [1; 32], height: 1, round: 0, precommits: Vec::new(),
        };
        let error = validate_snapshot_resource_limits(&set, &certificate, None, &[])
            .unwrap_err();
        assert!(error.contains("maximum validator count"));
    }

    #[test]
    fn snapshot_resource_limits_reject_aggregate_validator_history_overflow() {
        let finality_set = make_resource_test_set(MAX_SNAPSHOT_VALIDATORS_PER_SET, 0);
        let mut transitions = Vec::new();
        for index in 1..=4 {
            transitions.push(SnapshotValidatorSetTransition {
                certificate: freedom_types::ValidatorSetCertificate {
                    from_epoch: index - 1, to_epoch: index, previous_set_hash: [3; 32],
                    new_set_hash: [4; 32], votes: Vec::new(),
                },
                new_validator_set: make_resource_test_set(
                    MAX_SNAPSHOT_VALIDATORS_PER_SET,
                    (index as usize) * MAX_SNAPSHOT_VALIDATORS_PER_SET,
                ),
            });
        }
        let certificate = freedom_types::FinalityCertificate {
            block_hash: [1; 32], height: 1, round: 0, precommits: Vec::new(),
        };
        let error = validate_snapshot_resource_limits(
            &finality_set, &certificate, None, &transitions,
        ).unwrap_err();
        assert!(error.contains("aggregate validator entries"));
    }

    #[test]
    fn snapshot_resource_limits_reject_too_many_transitions() {
        let transitions = vec![SnapshotValidatorSetTransition {
            certificate: freedom_types::ValidatorSetCertificate {
                from_epoch: 0, to_epoch: 1, previous_set_hash: [3; 32],
                new_set_hash: [4; 32], votes: Vec::new(),
            },
            new_validator_set: std::collections::BTreeMap::new(),
        }; MAX_SNAPSHOT_TRANSITIONS + 1];
        let certificate = freedom_types::FinalityCertificate {
            block_hash: [1; 32], height: 1, round: 0, precommits: Vec::new(),
        };
        let error = validate_snapshot_resource_limits(
            &std::collections::BTreeMap::new(), &certificate, None, &transitions,
        ).unwrap_err();
        assert!(error.contains("maximum transition count"));
    }

    #[test]
    fn snapshot_response_rejects_transition_certificate_mismatch() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 7, round: 2, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 7, epoch: 1, proposer: [4;32],
                epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let response = SnapshotSyncResponse {
            request_id: [1;32], chain_id: u64::from(freedom_primitives::CHAIN_ID), height: 7,
            block_hash: freedom_consensus::block_hash(&checkpoint), state_root: [1;32], checkpoint,
            certificate: freedom_types::FinalityCertificate { block_hash: [0;32], height: 7, round: 2, precommits: Vec::new() },
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: Some(freedom_types::ValidatorSetCertificate {
                from_epoch: 0, to_epoch: 1, previous_set_hash: [2;32], new_set_hash: [3;32], votes: Vec::new()
            }),
            validator_set_transitions: Vec::new(),
            state_json: Vec::new(),
        };
        assert!(validate_snapshot_response(&response).is_err());
    }

    #[test]
    fn snapshot_response_rejects_wrong_chain_id() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 7, round: 2, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 7, epoch: 0, proposer: [4;32],
                epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let response = SnapshotSyncResponse {
            request_id: [1;32], chain_id: u64::from(freedom_primitives::CHAIN_ID).wrapping_add(1), height: 7,
            block_hash: freedom_consensus::block_hash(&checkpoint), state_root: [1;32], checkpoint,
            certificate: freedom_types::FinalityCertificate { block_hash: [0;32], height: 7, round: 2, precommits: Vec::new() },
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None, validator_set_transitions: Vec::new(), state_json: Vec::new(),
        };
        assert!(validate_snapshot_response(&response).is_err());
    }

    #[test]
    fn snapshot_response_rejects_checkpoint_hash_mismatch() {
        let checkpoint = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 7, round: 2, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 7, epoch: 0, proposer: [4;32],
                epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let response = SnapshotSyncResponse {
            request_id: [1;32], chain_id: u64::from(freedom_primitives::CHAIN_ID), height: 7,
            block_hash: [9;32], state_root: [1;32], checkpoint,
            certificate: freedom_types::FinalityCertificate { block_hash: [9;32], height: 7, round: 2, precommits: Vec::new() },
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None, validator_set_transitions: Vec::new(), state_json: Vec::new(),
        };
        assert!(validate_snapshot_response(&response).is_err());
    }

    #[test]
    fn sync_response_rejects_wrong_first_height() {
        let response = SyncResponse {
            request_id: [1; 32],
            from_height: 5,
            blocks: vec![freedom_types::Block {
                header: freedom_types::BlockHeader {
                    height: 6, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                    receipts_root: [3;32], timestamp: 6, epoch: 0, proposer: [4;32],
                    epoch_transition_certificate: None,
                },
                transactions: vec![], receipts: vec![],
            }],
        };
        assert!(validate_sync_response(&response).is_err());
    }

    #[test]
    fn sync_request_bounds_are_enforced() { let id=[1u8;32]; assert!(validate_sync_request(&SyncRequest{request_id:id,from_height:1,to_height:32}).is_ok()); assert!(validate_sync_request(&SyncRequest{request_id:id,from_height:0,to_height:1}).is_err()); assert!(validate_sync_request(&SyncRequest{request_id:id,from_height:1,to_height:33}).is_err()); }

    #[test]
    fn sync_request_rejects_overflowing_range() {
        let id = [1u8; 32];
        assert!(validate_sync_request(&SyncRequest {
            request_id: id,
            from_height: u64::MAX - 1,
            to_height: u64::MAX,
        }).is_ok());
        assert!(validate_sync_request(&SyncRequest {
            request_id: id,
            from_height: u64::MAX,
            to_height: u64::MAX,
        }).is_ok());
    }

    #[test]
    fn sync_response_rejects_height_overflow() {
        let block = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: u64::MAX, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32],
                epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let response = SyncResponse {
            request_id: [1;32],
            from_height: u64::MAX,
            blocks: vec![block],
        };
        assert!(validate_sync_response(&response).is_ok());
    }

    #[test]
    fn sync_wire_size_guard_rejects_oversized_payload_before_send() {
        let oversized = "x".repeat(64);
        assert!(validate_serialized_size(&oversized, 32, "test response")
            .unwrap_err().contains("maximum encoded size"));
        let small = "ok".to_string();
        assert!(validate_serialized_size(&small, 32, "test response").is_ok());
    }

    #[test]
    fn sync_wire_request_size_guard_accepts_valid_snapshot_request() {
        let request = SyncWireRequest::Snapshot(SnapshotSyncRequest {
            request_id: [1; 32],
            height: 1,
        });
        assert!(validate_sync_wire_request_size(&request).is_ok());
    }

    #[test]
    fn network_messages_round_trip() {
        let block = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 7, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 7, epoch: 7, proposer: [3;32],
                epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        };
        let mut proposal = BlockProposal {
            block_hash: hash_bytes(b"block"),
            height: 7,
            round: 0,
            proposer: [3; 32],
            block,
            valid_round: None,
            valid_precommits: Vec::new(),
            timeout_certificate: None,
            signature: Vec::new(),
        };
        let bytes = encode_message(&NetworkMessage::Proposal(proposal.clone())).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(decoded, NetworkMessage::Proposal(proposal));
    }

    #[test]
    fn evidence_message_round_trip() {
        let evidence = freedom_types::EquivocationEvidence {
            kind: freedom_types::EquivocationKind::DoublePrevote,
            height: 7, round: 1, validator_id: [4;32],
            first: vec![1,2], second: vec![3,4], evidence_hash: [5;32],
        };
        let msg = freedom_types::EvidenceMessage { evidence, sender: [6;32], signature: vec![7;64] };
        let bytes = encode_message(&NetworkMessage::Evidence(msg.clone())).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(decoded, NetworkMessage::Evidence(msg));
    }

    #[test]
    fn swarm_has_authenticated_peer_identity() {
        let swarm = build_swarm().unwrap();
        assert!(!local_peer_id(&swarm).to_bytes().is_empty());
    }
}

pub fn make_sync_request(request_id: Hash, from_height: u64) -> NetworkMessage {
    let to_height = from_height
        .checked_add(MAX_SYNC_BLOCKS - 1)
        .unwrap_or(u64::MAX);
    NetworkMessage::SyncRequest(SyncRequest {
        request_id,
        from_height,
        to_height,
    })
}


#[cfg(test)]
mod m160_bridge_proposal_tests {
    use super::*;

    #[test]
    fn global_connection_budget_is_bounded_and_directional_caps_fit() {
        assert!(MAX_ESTABLISHED_INCOMING_CONNECTIONS <= MAX_ESTABLISHED_CONNECTIONS);
        assert!(MAX_ESTABLISHED_OUTGOING_CONNECTIONS <= MAX_ESTABLISHED_CONNECTIONS);
        assert!(
            MAX_ESTABLISHED_INCOMING_CONNECTIONS + MAX_ESTABLISHED_OUTGOING_CONNECTIONS
                <= MAX_ESTABLISHED_CONNECTIONS
        );
        assert!(MAX_PENDING_INCOMING_CONNECTIONS > 0);
        assert!(MAX_PENDING_OUTGOING_CONNECTIONS > 0);
    }

    #[test]
    fn bridge_mint_block_validation_rejects_duplicate_canonical_ids() {
        // The duplicate-ID rule is deterministic at proposal validation time:
        // two differently relayed transactions cannot represent the same proof.
        let mut ids = std::collections::BTreeSet::new();
        let id = freedom_primitives::hash_bytes(b"bridge-proof");
        assert!(ids.insert(id));
        assert!(!ids.insert(id));
    }
}

#[cfg(test)]
mod m481_bounded_serialization_tests {
    use super::*;

    #[test]
    fn bounded_json_serializer_accepts_payload_at_exact_limit() {
        let value = "abc";
        let encoded = serialize_bounded_json(&value, 5, "test payload").unwrap();
        assert_eq!(encoded, br#""abc""#);
    }

    #[test]
    fn bounded_json_serializer_rejects_payload_before_exceeding_limit() {
        let value = "abcd";
        let error = serialize_bounded_json(&value, 5, "test payload").unwrap_err();
        assert!(error.contains("exceeds maximum encoded size"));
    }

    fn bounded_size_writer_accepts_exact_encoded_size_without_buffering_payload() {
        let value = "abc";
        assert!(validate_serialized_size(&value, 5, "test payload").is_ok());
    }

    #[test]
    fn bounded_size_writer_rejects_payload_at_first_limit_crossing() {
        let value = "abc";
        let error = validate_serialized_size(&value, 4, "test payload").unwrap_err();
        assert!(error.contains("exceeds maximum encoded size"));
    }

    #[test]
    fn bounded_size_writer_accepts_zero_length_serialized_value_only_if_limit_allows_it() {
        assert!(validate_serialized_size(&(), 2, "unit payload").is_ok());
        assert!(validate_serialized_size(&(), 1, "unit payload").is_err());
    }
}
