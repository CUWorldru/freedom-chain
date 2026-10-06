use freedom_consensus::{block_hash, proposer_for_height_round};
use freedom_network::{
    build_swarm, decode_message, make_proposal, make_sync_request, make_timeout, make_validator_set_vote, publish,
    NetworkMessage, SyncResponse, SyncWireRequest, SyncWireResponse,
};
use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY, Hash, NETWORK_NAME};
use freedom_runtime::{build_next_block, execute_block, initialize, Mempool, ValidatorProtocol};
use freedom_storage::ChainStore;
use freedom_types::{Genesis, Validator};
use libp2p::{Multiaddr, SwarmEvent};
use libp2p::futures::StreamExt;
use std::collections::{BTreeMap, BTreeSet};
use tokio::time::{interval, Duration};

const ROUND_TIMEOUT_SECS: u64 = 5;
const SYNC_INTERVAL_SECS: u64 = 10;
const DEFAULT_SNAPSHOT_INTERVAL: u64 = 1000;

fn request_id(validator_id: Hash, from_height: u64) -> Hash {
    hash_bytes(&[b"FREEDOM-SYNC-V1", &validator_id, &from_height.to_le_bytes()].concat())
}

fn select_sync_peer(peers: &[libp2p::PeerId], cursor: &mut usize) -> Option<libp2p::PeerId> {
    if peers.is_empty() {
        *cursor = 0;
        return None;
    }
    *cursor %= peers.len();
    let peer = peers[*cursor];
    *cursor = (*cursor + 1) % peers.len();
    Some(peer)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SyncMisbehaviorEvidence {
    peer: libp2p::PeerId,
    from_height: u64,
    request_id: Hash,
    batch_fingerprint: Hash,
    reason: String,
}

const PEER_MISBEHAVIOR_THRESHOLD: u32 = 3;
const PEER_SCORE_COOLDOWN_ROUNDS: u64 = 3;
const PEER_SCORE_DECAY_INTERVAL: u64 = 10;
const PEER_EVIDENCE_LIMIT: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum PeerEvidenceKind {
    SyncConflict,
    SnapshotConflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum PeerEvidenceSeverity {
    Normal,
    Elevated,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PeerReputationEvidence {
    peer: libp2p::PeerId,
    height: u64,
    request_id: Hash,
    fingerprint: Hash,
    kind: PeerEvidenceKind,
    severity: PeerEvidenceSeverity,
    reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PeerReputation {
    score: u32,
    cooldown_until_height: u64,
    last_decay_height: u64,
    evidence: Vec<PeerReputationEvidence>,
    #[serde(default)]
    dedup_index: BTreeSet<Hash>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PeerReputationPolicy {
    misbehavior_threshold: u32,
    cooldown_rounds: u64,
    decay_interval: u64,
    evidence_limit: usize,
    sync_conflict_weight: u32,
    snapshot_conflict_weight: u32,
    normal_severity_multiplier: u32,
    elevated_severity_multiplier: u32,
    critical_severity_multiplier: u32,
}

const PEER_REPUTATION_POLICY: PeerReputationPolicy = PeerReputationPolicy {
    misbehavior_threshold: PEER_MISBEHAVIOR_THRESHOLD,
    cooldown_rounds: PEER_SCORE_COOLDOWN_ROUNDS,
    decay_interval: PEER_SCORE_DECAY_INTERVAL,
    evidence_limit: PEER_EVIDENCE_LIMIT,
    sync_conflict_weight: 1,
    snapshot_conflict_weight: 1,
    normal_severity_multiplier: 1,
    elevated_severity_multiplier: 2,
    critical_severity_multiplier: 3,
};

fn peer_evidence_weight(kind: &PeerEvidenceKind, severity: PeerEvidenceSeverity) -> u32 {
    let base = match kind {
        PeerEvidenceKind::SyncConflict => PEER_REPUTATION_POLICY.sync_conflict_weight,
        PeerEvidenceKind::SnapshotConflict => PEER_REPUTATION_POLICY.snapshot_conflict_weight,
    };
    let multiplier = match severity {
        PeerEvidenceSeverity::Normal => PEER_REPUTATION_POLICY.normal_severity_multiplier,
        PeerEvidenceSeverity::Elevated => PEER_REPUTATION_POLICY.elevated_severity_multiplier,
        PeerEvidenceSeverity::Critical => PEER_REPUTATION_POLICY.critical_severity_multiplier,
    };
    base.saturating_mul(multiplier)
}

fn peer_evidence_id(evidence: &PeerReputationEvidence) -> Hash {
    let bytes = serde_json::to_vec(&(
        b"freedom/peer-reputation/evidence/v1",
        evidence.peer,
        evidence.height,
        evidence.request_id,
        evidence.fingerprint,
        &evidence.kind,
    )).expect("peer reputation evidence serialization must succeed");
    hash_bytes(&bytes)
}

fn rebuild_peer_reputation_dedup_index(
    reputations: &mut BTreeMap<libp2p::PeerId, PeerReputation>,
) -> bool {
    let mut changed = false;
    for reputation in reputations.values_mut() {
        for evidence in &reputation.evidence {
            if reputation.dedup_index.insert(peer_evidence_id(evidence)) {
                changed = true;
            }
        }
    }
    changed
}

fn compact_peer_evidence(reputation: &mut PeerReputation) -> bool {
    if reputation.evidence.len() <= PEER_REPUTATION_POLICY.evidence_limit {
        return false;
    }
    let excess = reputation.evidence.len() - PEER_REPUTATION_POLICY.evidence_limit;
    reputation.evidence.drain(0..excess);
    true
}

fn classify_peer_evidence(
    reputation: &PeerReputation,
    kind: &PeerEvidenceKind,
    confirming_peers: usize,
) -> PeerEvidenceSeverity {
    let recent = reputation.evidence.iter().rev().take(PEER_REPUTATION_POLICY.evidence_limit);
    let mut same_kind_recent = 0usize;
    let mut recent_total = 0usize;
    for evidence in recent {
        recent_total += 1;
        if &evidence.kind == kind {
            same_kind_recent += 1;
        }
        if recent_total >= 3 {
            break;
        }
    }
    let repetition = same_kind_recent.saturating_add(1);
    let confirmation = confirming_peers.max(1);
    if confirmation >= 3 {
        return PeerEvidenceSeverity::Critical;
    }
    if confirmation == 2 {
        return PeerEvidenceSeverity::Elevated;
    }
    match repetition {
        1 | 2 => PeerEvidenceSeverity::Normal,
        3 => PeerEvidenceSeverity::Elevated,
        _ => PeerEvidenceSeverity::Critical,
    }
}

fn record_peer_evidence_with_confirmations(
    reputations: &mut BTreeMap<libp2p::PeerId, PeerReputation>,
    mut evidence: PeerReputationEvidence,
    confirming_peers: usize,
) {
    let peer = evidence.peer;
    let height = evidence.height;
    let entry = reputations.entry(peer).or_insert(PeerReputation {
        score: 0,
        cooldown_until_height: height,
        last_decay_height: height,
        evidence: Vec::new(),
        dedup_index: BTreeSet::new(),
    });
    let evidence_id = peer_evidence_id(&evidence);
    if !entry.dedup_index.insert(evidence_id) {
        return;
    }
    evidence.severity = classify_peer_evidence(entry, &evidence.kind, confirming_peers);
    entry.score = entry.score.saturating_add(peer_evidence_weight(
        &evidence.kind,
        evidence.severity,
    ));
    entry.last_decay_height = height;
    entry.evidence.push(evidence);
    compact_peer_evidence(entry);
    apply_peer_reputation_policy(entry, height);
}

fn record_peer_evidence(
    reputations: &mut BTreeMap<libp2p::PeerId, PeerReputation>,
    evidence: PeerReputationEvidence,
) {
    record_peer_evidence_with_confirmations(reputations, evidence, 1);
}
fn apply_peer_reputation_policy(reputation: &mut PeerReputation, current_height: u64) {
    if reputation.score >= PEER_REPUTATION_POLICY.misbehavior_threshold {
        reputation.cooldown_until_height = reputation.cooldown_until_height.max(
            current_height.saturating_add(PEER_REPUTATION_POLICY.cooldown_rounds)
        );
    }
}

fn peer_is_eligible(
    reputations: &BTreeMap<libp2p::PeerId, PeerReputation>,
    peer: libp2p::PeerId,
    current_height: u64,
) -> bool {
    reputations.get(&peer)
        .map(|reputation| {
            current_height >= reputation.cooldown_until_height
                && reputation.score < PEER_REPUTATION_POLICY.misbehavior_threshold
        })
        .unwrap_or(true)
}

fn decay_peer_reputation(
    reputations: &mut BTreeMap<libp2p::PeerId, PeerReputation>,
    current_height: u64,
) -> bool {
    let mut changed = false;
    for reputation in reputations.values_mut() {
        let elapsed = current_height.saturating_sub(reputation.last_decay_height);
        let steps = elapsed / PEER_REPUTATION_POLICY.decay_interval;
        if steps > 0 {
            let decay = steps.min(reputation.score as u64) as u32;
            if decay > 0 {
                reputation.score = reputation.score.saturating_sub(decay);
                changed = true;
            }
            reputation.last_decay_height = reputation.last_decay_height.saturating_add(
                steps.saturating_mul(PEER_SCORE_DECAY_INTERVAL)
            );
            compact_peer_evidence(reputation);
        }
    }
    changed
}

fn peer_is_eligible(
    reputations: &BTreeMap<libp2p::PeerId, PeerReputation>,
    peer: libp2p::PeerId,
    current_height: u64,
) -> bool {
    reputations.get(&peer)
        .map(|reputation| current_height >= reputation.cooldown_until_height && reputation.score < PEER_MISBEHAVIOR_THRESHOLD)
        .unwrap_or(true)
}

fn send_snapshot_request(
    swarm: &mut libp2p::Swarm<freedom_network::FreedomBehaviour>,
    peer: libp2p::PeerId,
    validator_id: Hash,
    height: u64,
) -> Result<(Hash, libp2p::request_response::OutboundRequestId), String> {
    let request_id = hash_bytes(&[b"FREEDOM-SNAPSHOT-V1", &validator_id, &height.to_le_bytes()].concat());
    let request = freedom_network::SnapshotSyncRequest { request_id, height };
    freedom_network::validate_snapshot_request(&request)?;
    let outbound_id = swarm.behaviour_mut().sync.send_request(
        &peer,
        SyncWireRequest::Snapshot(request),
    );
    println!("snapshot_request peer={} height={} request_id={} outbound_id={:?}", peer, height, hex::encode(request_id), outbound_id);
    Ok((request_id, outbound_id))
}

fn build_local_proposal(
    state: &freedom_state::ChainState,
    owner: freedom_primitives::Address,
    consensus_key: &ed25519_dalek::SigningKey,
    round: u64,
) -> Result<(freedom_types::BlockProposal, freedom_types::Block), Box<dyn std::error::Error>> {
    let mempool = Mempool::new();
    let next_height = state.height
        .checked_add(1)
        .ok_or_else(|| "block height exceeds u64::MAX".to_string())?;
    let epoch = if state.epoch_length > 0 && next_height % state.epoch_length == 0 {
        state.epoch
            .checked_add(1)
            .ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())?
    } else {
        state.epoch
    };
    let block = build_next_block(state, &mempool, owner, 1, epoch, round, 100)?;
    let proposal = make_proposal(state, block.clone(), round, consensus_key, None, Vec::new(), None)?;
    Ok((proposal, block))
}

fn send_sync_request(
    swarm: &mut libp2p::Swarm<freedom_network::FreedomBehaviour>,
    peer: libp2p::PeerId,
    validator_id: Hash,
    height: u64,
) -> Result<(Hash, libp2p::request_response::OutboundRequestId), String> {
    let from = height
        .checked_add(1)
        .ok_or_else(|| "sync start height exceeds u64::MAX".to_string())?;
    let to = from
        .checked_add(freedom_network::MAX_SYNC_BLOCKS.saturating_sub(1))
        .ok_or_else(|| "sync request end height exceeds u64::MAX".to_string())?;
    let id = request_id(validator_id, from);
    let request = freedom_network::SyncRequest {
        request_id: id,
        from_height: from,
        to_height: to,
    };
    freedom_network::validate_sync_request(&request)?;
    let outbound_id = swarm.behaviour_mut().sync.send_request(
        &peer,
        SyncWireRequest::Blocks(request),
    );
    println!("sync_request peer={} from={} request_id={} outbound_id={:?}", peer, from, hex::encode(id), outbound_id);
    Ok((id, outbound_id))
}

#[tokio::main]
fn make_evidence_message(
    state: &freedom_state::ChainState,
    evidence: freedom_types::EquivocationEvidence,
    sender: Hash,
    key: &ed25519_dalek::SigningKey,
) -> Result<freedom_types::EvidenceMessage, String> {
    let validator = state.active_validator_set.get(&sender)
        .ok_or_else(|| "evidence sender is not active validator".to_string())?;
    if validator.consensus_key != key.verifying_key().to_bytes() {
        return Err("evidence sender key mismatch".into());
    }
    let mut msg = freedom_types::EvidenceMessage {
        evidence,
        sender,
        signature: Vec::new(),
    };
    msg.signature = freedom_crypto::sign(key, &msg.signing_bytes()).to_vec();
    Ok(msg)
}

#[cfg(test)]
mod m31_tests {
    #[test]
    fn sync_peer_selection_rotates_and_handles_empty() {
        let a = libp2p::PeerId::random();
        let b = libp2p::PeerId::random();
        let peers = vec![a, b];
        let mut cursor = 0;
        assert_eq!(select_sync_peer(&peers, &mut cursor), Some(a));
        assert_eq!(select_sync_peer(&peers, &mut cursor), Some(b));
        assert_eq!(select_sync_peer(&peers, &mut cursor), Some(a));
        assert_eq!(select_sync_peer(&[], &mut cursor), None);
        assert_eq!(cursor, 0);
    }


    use super::*;
    use freedom_primitives::hash_bytes;


    #[test]
    fn snapshot_misbehavior_evidence_carries_request_and_fingerprint() {
        let peer = libp2p::PeerId::random();
        let fingerprint = [7u8; 32];
        let evidence = SnapshotMisbehaviorEvidence {
            peer,
            height: 42,
            request_id: [9u8; 32],
            snapshot_fingerprint: fingerprint,
            reason: "snapshot commitment disagrees with another fully validated response".to_string(),
        };
        assert_eq!(evidence.peer, peer);
        assert_eq!(evidence.height, 42);
        assert_eq!(evidence.request_id, [9u8; 32]);
        assert_eq!(evidence.snapshot_fingerprint, fingerprint);
    }

    #[test]
    fn snapshot_commitment_fingerprint_changes_when_state_root_changes() {
        let base = hash_bytes(
            &[
                &7u64.to_le_bytes(),
                &[1u8; 32],
                &[2u8; 32],
                &[3u8; 32],
                &[4u8; 32],
                &[0u8; 32],
            ].concat(),
        );
        let changed = hash_bytes(
            &[
                &7u64.to_le_bytes(),
                &[1u8; 32],
                &[9u8; 32],
                &[3u8; 32],
                &[4u8; 32],
                &[0u8; 32],
            ].concat(),
        );
        assert_ne!(base, changed);
    }

    #[test]
    fn peer_reputation_dedup_keeps_distinct_fingerprints() {
        let peer = libp2p::PeerId::random();
        let base = |fingerprint| PeerReputationEvidence {
            peer,
            height: 10,
            request_id: [7u8; 32],
            fingerprint,
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "distinct".to_string(),
        };
        let mut reputations = BTreeMap::new();
        record_peer_evidence(&mut reputations, base([8u8; 32]));
        record_peer_evidence(&mut reputations, base([9u8; 32]));
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, 2);
        assert_eq!(reputation.evidence.len(), 2);
    }

    #[test]
    fn peer_reputation_legacy_state_rebuilds_dedup_index() {
        let peer = libp2p::PeerId::random();
        let evidence = PeerReputationEvidence {
            peer,
            height: 7,
            request_id: [3u8; 32],
            fingerprint: [4u8; 32],
            kind: PeerEvidenceKind::SnapshotConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "legacy".to_string(),
        };
        let mut reputation = PeerReputation {
            score: 1,
            cooldown_until_height: 10,
            last_decay_height: 7,
            evidence: vec![evidence.clone()],
            dedup_index: BTreeSet::new(),
        };
        let mut legacy = serde_json::to_value(&reputation).unwrap();
        legacy.as_object_mut().unwrap().remove("dedup_index");
        let decoded: PeerReputation = serde_json::from_value(legacy).unwrap();
        assert!(decoded.dedup_index.is_empty());

        let mut reputations = BTreeMap::new();
        reputations.insert(peer, decoded);
        assert!(rebuild_peer_reputation_dedup_index(&mut reputations));
        assert_eq!(
            reputations.get(&peer).unwrap().dedup_index.len(),
            1
        );

        record_peer_evidence(&mut reputations, evidence);
        assert_eq!(reputations.get(&peer).unwrap().score, 1);
        assert_eq!(reputations.get(&peer).unwrap().evidence.len(), 1);
    }

    #[test]
    fn peer_reputation_dedup_survives_evidence_compaction() {
        let peer = libp2p::PeerId::random();
        let make_evidence = |height| PeerReputationEvidence {
            peer,
            height,
            request_id: [height as u8; 32],
            fingerprint: [9u8; 32],
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "compaction".to_string(),
        };
        let mut reputations = BTreeMap::new();
        for height in 1..=(PEER_EVIDENCE_LIMIT as u64 + 5) {
            record_peer_evidence(&mut reputations, make_evidence(height));
        }
        let before = reputations.get(&peer).unwrap().score;
        assert_eq!(reputations.get(&peer).unwrap().evidence.len(), PEER_EVIDENCE_LIMIT);
        record_peer_evidence(&mut reputations, make_evidence(1));
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, before);
        assert_eq!(reputation.evidence.len(), PEER_EVIDENCE_LIMIT);
        assert_eq!(reputation.dedup_index.len(), PEER_EVIDENCE_LIMIT + 5);
    }

    #[test]
    fn peer_reputation_evidence_is_idempotent() {
        let peer = libp2p::PeerId::random();
        let evidence = PeerReputationEvidence {
            peer,
            height: 10,
            request_id: [7u8; 32],
            fingerprint: [8u8; 32],
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "duplicate".to_string(),
        };
        let mut reputations = BTreeMap::new();
        record_peer_evidence_with_confirmations(&mut reputations, evidence.clone(), 1);
        record_peer_evidence_with_confirmations(&mut reputations, evidence, 3);
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, PEER_REPUTATION_POLICY.normal_severity_multiplier);
        assert_eq!(reputation.evidence.len(), 1);
    }

    #[test]
    fn peer_reputation_recording_uses_confirmation_count() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        record_peer_evidence_with_confirmations(&mut reputations, PeerReputationEvidence {
            peer,
            height: 10,
            request_id: [1u8; 32],
            fingerprint: [2u8; 32],
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "confirmed".to_string(),
        }, 2);
        assert_eq!(
            reputations.get(&peer).unwrap().evidence[0].severity,
            PeerEvidenceSeverity::Elevated
        );
        assert_eq!(
            reputations.get(&peer).unwrap().score,
            PEER_REPUTATION_POLICY.elevated_severity_multiplier
        );
    }

    #[test]
    fn peer_reputation_confirmation_escalates_severity() {
        let reputation = PeerReputation {
            score: 0,
            cooldown_until_height: 0,
            last_decay_height: 0,
            evidence: Vec::new(),
            dedup_index: BTreeSet::new(),
        };
        assert_eq!(
            classify_peer_evidence(&reputation, &PeerEvidenceKind::SyncConflict, 1),
            PeerEvidenceSeverity::Normal
        );
        assert_eq!(
            classify_peer_evidence(&reputation, &PeerEvidenceKind::SyncConflict, 2),
            PeerEvidenceSeverity::Elevated
        );
        assert_eq!(
            classify_peer_evidence(&reputation, &PeerEvidenceKind::SyncConflict, 3),
            PeerEvidenceSeverity::Critical
        );
    }

    #[test]
    fn peer_reputation_severity_scales_policy_weight() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        for height in 10..=13 {
            record_peer_evidence(&mut reputations, PeerReputationEvidence {
                peer,
                height,
                request_id: [height as u8; 32],
                fingerprint: [5u8; 32],
                kind: PeerEvidenceKind::SyncConflict,
                severity: PeerEvidenceSeverity::Normal,
                reason: "severity".to_string(),
            });
        }
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(
            reputation.evidence.iter().map(|e| e.severity).collect::<Vec<_>>(),
            vec![
                PeerEvidenceSeverity::Normal,
                PeerEvidenceSeverity::Normal,
                PeerEvidenceSeverity::Elevated,
                PeerEvidenceSeverity::Critical,
            ]
        );
        assert_eq!(
            reputation.score,
            PEER_REPUTATION_POLICY.normal_severity_multiplier
                + PEER_REPUTATION_POLICY.normal_severity_multiplier
                + PEER_REPUTATION_POLICY.elevated_severity_multiplier
                + PEER_REPUTATION_POLICY.critical_severity_multiplier
        );
    }

    #[test]
    fn peer_reputation_evidence_kinds_use_policy_weights() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        record_peer_evidence(&mut reputations, PeerReputationEvidence {
            peer,
            height: 10,
            request_id: [1u8; 32],
            fingerprint: [2u8; 32],
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "sync".to_string(),
        });
        record_peer_evidence(&mut reputations, PeerReputationEvidence {
            peer,
            height: 11,
            request_id: [3u8; 32],
            fingerprint: [4u8; 32],
            kind: PeerEvidenceKind::SnapshotConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "snapshot".to_string(),
        });
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(
            reputation.score,
            PEER_REPUTATION_POLICY.sync_conflict_weight
                + PEER_REPUTATION_POLICY.snapshot_conflict_weight
        );
        assert_eq!(reputation.evidence.len(), 2);
    }

    #[test]
    fn peer_reputation_policy_preserves_thresholds() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        for height in 1..=PEER_REPUTATION_POLICY.misbehavior_threshold as u64 {
            record_peer_evidence(&mut reputations, PeerReputationEvidence {
                peer,
                height,
                request_id: [height as u8; 32],
                fingerprint: [3u8; 32],
                kind: PeerEvidenceKind::SnapshotConflict,
            severity: PeerEvidenceSeverity::Normal,
                reason: "test".to_string(),
            });
        }
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, PEER_REPUTATION_POLICY.misbehavior_threshold);
        assert_eq!(reputation.cooldown_until_height, 6);
        assert!(!peer_is_eligible(&reputations, peer, 5));
        assert!(peer_is_eligible(&reputations, peer, 6));
    }

    #[test]
    fn peer_reputation_evidence_is_bounded() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        for height in 0..(PEER_EVIDENCE_LIMIT as u64 + 5) {
            record_peer_evidence(&mut reputations, PeerReputationEvidence {
                peer,
                height,
                request_id: [height as u8; 32],
                fingerprint: [2u8; 32],
                kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
                reason: "test".to_string(),
            });
        }
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, PEER_EVIDENCE_LIMIT as u32 + 5);
        assert_eq!(reputation.evidence.len(), PEER_EVIDENCE_LIMIT);
        assert_eq!(reputation.evidence.first().unwrap().height, 5);
        assert_eq!(reputation.evidence.last().unwrap().height, PEER_EVIDENCE_LIMIT as u64 + 4);
    }

    #[test]
    fn peer_reputation_decays_without_rewriting_cooldown() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        for _ in 0..3 {
            record_peer_evidence(&mut reputations, PeerReputationEvidence {
                peer,
                height: 10,
                request_id: [1u8; 32],
                fingerprint: [2u8; 32],
                kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
                reason: "test".to_string(),
            });
        }
        assert!(!peer_is_eligible(&reputations, peer, 10));
        assert!(decay_peer_reputation(&mut reputations, 20));
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, 2);
        assert!(!peer_is_eligible(&reputations, peer, 10));
        assert!(peer_is_eligible(&reputations, peer, 20));
        assert_eq!(reputation.cooldown_until_height, 13);
    }

    #[test]
    fn peer_reputation_accepts_multiple_evidence_kinds() {
        let peer = libp2p::PeerId::random();
        let mut reputations = BTreeMap::new();
        for kind in [PeerEvidenceKind::SyncConflict, PeerEvidenceKind::SnapshotConflict] {
            record_peer_evidence(&mut reputations, PeerReputationEvidence {
                peer,
                height: 20,
                request_id: [1u8; 32],
                fingerprint: [2u8; 32],
                kind,
            severity: PeerEvidenceSeverity::Normal,
                reason: "test".to_string(),
            });
        }
        let reputation = reputations.get(&peer).unwrap();
        assert_eq!(reputation.score, 2);
        assert_eq!(reputation.evidence.len(), 2);
        assert!(peer_is_eligible(&reputations, peer, 20));
    }

    #[test]
    fn peer_sync_score_enters_and_exits_cooldown() {
        let peer = libp2p::PeerId::random();
        let mut scores = BTreeMap::new();
        record_peer_evidence(&mut scores, PeerReputationEvidence {
            peer,
            height: 10,
            request_id: [0u8; 32],
            fingerprint: [0u8; 32],
            kind: PeerEvidenceKind::SyncConflict,
            severity: PeerEvidenceSeverity::Normal,
            reason: "test".to_string(),
        });
        assert!(peer_is_eligible(&scores, peer, 10));
        for _ in 0..2 {
            record_peer_evidence(&mut scores, PeerReputationEvidence {
                peer,
                height: 10,
                request_id: [0u8; 32],
                fingerprint: [0u8; 32],
                kind: PeerEvidenceKind::SnapshotConflict,
            severity: PeerEvidenceSeverity::Normal,
                reason: "test".to_string(),
            });
        }
        assert!(!peer_is_eligible(&scores, peer, 10));
        assert!(peer_is_eligible(&scores, peer, 13));
    }

    #[test]
    fn sync_misbehavior_evidence_carries_request_and_fingerprint() {
        let peer = libp2p::PeerId::random();
        let fingerprint = hash_bytes(b"offending-batch");
        let evidence = SyncMisbehaviorEvidence {
            peer,
            from_height: 12,
            request_id: [7; 32],
            batch_fingerprint: fingerprint,
            reason: "test".to_string(),
        };
        assert_eq!(evidence.from_height, 12);
        assert_eq!(evidence.request_id, [7; 32]);
        assert_eq!(evidence.batch_fingerprint, fingerprint);
        assert_eq!(evidence.peer, peer);
    }

    #[test]
    fn sync_conflict_is_detected_by_batch_fingerprint() {
        let a = vec![1u8, 2, 3];
        let b = vec![1u8, 2, 4];
        let ha = hash_bytes(&a);
        let hb = hash_bytes(&b);
        assert_ne!(ha, hb);
    }

    #[test]
    fn recovered_state_builds_the_next_contiguous_proposal() {
        let validators: Vec<Validator> = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: address_from_public_key(&key),
                consensus_key: key,
                stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
        let state = initialize(&genesis).unwrap();
        let owner = state.active_validator_set.values().next().unwrap().owner;
        let key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let (proposal, block) = build_local_proposal(&state, owner, &key, 0).unwrap();

        assert_eq!(proposal.height, state.height + 1);
        assert_eq!(proposal.block_hash, block_hash(&block));
        assert_eq!(block.header.previous_hash, state.last_block_hash);
        assert_eq!(block.header.proposer, owner);
        assert_eq!(proposer_for_height_round(&state, block.header.height, 0), Some(owner));
    }

    #[test]
    fn replay_rejects_previous_hash_corruption_without_mutating_state() {
        let validators: Vec<Validator> = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: address_from_public_key(&key),
                consensus_key: key,
                stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
        let mut state = initialize(&genesis).unwrap();
        let owner = state.active_validator_set.values().next().unwrap().owner;
        let key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let (_, mut block) = build_local_proposal(&state, owner, &key, 0).unwrap();
        block.header.previous_hash = [9u8; 32];
        let before = state.clone();

        let err = replay_persisted_blocks(&mut state, &[block]).unwrap_err();

        assert!(err.contains("previous hash mismatch"));
        assert_eq!(state, before);
    }

    #[test]
    fn replay_rejects_proposer_corruption_before_execution() {
        let validators: Vec<Validator> = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: address_from_public_key(&key),
                consensus_key: key,
                stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
        let mut state = initialize(&genesis).unwrap();
        let owner = state.active_validator_set.values().next().unwrap().owner;
        let key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let (_, mut block) = build_local_proposal(&state, owner, &key, 0).unwrap();
        block.header.proposer = [8u8; 32];
        let before = state.clone();

        let err = replay_persisted_blocks(&mut state, &[block]).unwrap_err();

        assert!(err.contains("proposer mismatch"));
        assert_eq!(state, before);
    }

    #[test]
    fn replay_rejects_height_gap_before_execution() {
        let validators: Vec<Validator> = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: address_from_public_key(&key),
                consensus_key: key,
                stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
        let mut state = initialize(&genesis).unwrap();
        let owner = state.active_validator_set.values().next().unwrap().owner;
        let key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let (_, mut block) = build_local_proposal(&state, owner, &key, 0).unwrap();
        block.header.height = 2;
        let before = state.clone();

        let err = replay_persisted_blocks(&mut state, &[block]).unwrap_err();

        assert!(err.contains("height gap"));
        assert_eq!(state, before);
    }
}

fn replay_persisted_blocks(
    state: &mut freedom_state::ChainState,
    blocks: &[freedom_types::Block],
) -> Result<(), String> {
    for block in blocks {
        let expected_height = state
            .height
            .checked_add(1)
            .ok_or_else(|| "stored chain height exceeds u64::MAX".to_string())?;
        if block.header.height != expected_height {
            return Err(format!("stored chain height gap at {}", block.header.height));
        }
        if block.header.previous_hash != state.last_block_hash {
            return Err(format!("stored chain previous hash mismatch at {}", block.header.height));
        }
        let expected = proposer_for_height_round(state, block.header.height, block.header.round)
            .ok_or_else(|| "no proposer for stored block".to_string())?;
        if expected != block.header.proposer {
            return Err(format!("stored block proposer mismatch at {}", block.header.height));
        }
        let committed = execute_block(state, block)
            .map_err(|e| format!("stored block execution rejected at {}: {e}", block.header.height))?;
        if block_hash(&committed) != block_hash(block) {
            return Err(format!("stored block hash mismatch at {}", block.header.height));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validators: Vec<Validator> = (0u8..3).map(|i| {
        let key = [i + 1; 32];
        Validator {
            validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
            owner: address_from_public_key(&key),
            consensus_key: key,
            stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
        }
    }).collect();

    let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
    let mut state = initialize(&genesis)?;
    let validator_index = std::env::var("NETWORK_321MORE_VALIDATOR_INDEX").or_else(|_| std::env::var("FREEDOM_VALIDATOR_INDEX")).ok()
        .and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    if validator_index >= 3 { return Err("NETWORK_321MORE_VALIDATOR_INDEX must be 0..2".into()); }

    let consensus_key = ed25519_dalek::SigningKey::from_bytes(&[(validator_index as u8) + 1; 32]);
    let validator_id = hash_bytes(&[b"validator", &[(validator_index as u8) + 1]].concat());
    let owner = state.validators.get(&validator_id).ok_or("validator not found")?.owner;
    let mut protocol = ValidatorProtocol::new(validator_id, consensus_key.clone());
    let mut blocks: BTreeMap<Hash, freedom_types::Block> = BTreeMap::new();

    let db_path = std::env::var("NETWORK_321MORE_DB_PATH").or_else(|_| std::env::var("FREEDOM_DB_PATH")).unwrap_or_else(|_| "./data/321more".to_string());
    let store = ChainStore::open(&db_path)?;
    let stored_height = store.latest_height()?;
    store.validate_tip_metadata()
        .map_err(|e| format!("stored tip metadata is inconsistent: {e}"))?;
    let snapshot_interval = std::env::var("NETWORK_321MORE_SNAPSHOT_INTERVAL").or_else(|_| std::env::var("FREEDOM_SNAPSHOT_INTERVAL")).ok()
        .and_then(|v| v.parse::<u64>().ok()).filter(|v| *v > 0)
        .unwrap_or(DEFAULT_SNAPSHOT_INTERVAL);
    let replay_from = if let Some(snapshot) = store.latest_snapshot()? {
        if snapshot.height > stored_height {
            return Err(format!(
                "snapshot height {} is ahead of stored chain height {}",
                snapshot.height, stored_height
            ).into());
        }
        if snapshot.height == 0 {
            if snapshot.block_hash != [0; 32] || snapshot.state.last_block_hash != [0; 32] {
                return Err("genesis snapshot hash linkage mismatch".into());
            }
        } else {
            let checkpoint = store.get_block(snapshot.height)?.ok_or("snapshot checkpoint block missing")?;
            if block_hash(&checkpoint) != snapshot.block_hash || checkpoint.header.state_root != snapshot.state_root {
                return Err(format!("snapshot checkpoint mismatch at {}", snapshot.height).into());
            }
        }
        state = snapshot.state;
        if state.last_block_hash != snapshot.block_hash || state.height != snapshot.height {
            return Err(format!("snapshot state linkage mismatch at {}", snapshot.height).into());
        }
        snapshot.height
            .checked_add(1)
            .ok_or_else(|| "snapshot height exceeds u64::MAX".to_string())?
    } else {
        store.save_snapshot(&state, [0;32])?;
        1
    };
    if stored_height >= replay_from {
        let replay_blocks = store.range(replay_from, stored_height)?;
        replay_persisted_blocks(&mut state, &replay_blocks).map_err(|e| e.into())?;
        if state.last_block_hash != store.latest_hash()? {
            return Err("stored latest hash metadata mismatch".into());
        }
        println!("restored height={} hash={}", state.height, hex::encode(state.last_block_hash));
    }

    let mut swarm = build_swarm()?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?)?;

    if let Ok(peers) = std::env::var("NETWORK_321MORE_PEERS").or_else(|_| std::env::var("FREEDOM_PEERS")) {
        for raw in peers.split(',').filter(|s| !s.trim().is_empty()) {
            swarm.dial(raw.trim().parse::<Multiaddr>()?)?;
        }
    }

    println!("{NETWORK_NAME} validator={validator_index}");
    println!("peer_id={}", swarm.local_peer_id());
    println!("validators={}", state.validators.len());
    println!("height={}", state.height);
    println!("db={}", db_path);

    let mut round = 0u64;
    let mut timeout_round_sent = None::<u64>;
    let mut timer = interval(Duration::from_secs(ROUND_TIMEOUT_SECS));
    let mut sync_timer = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
    let mut pending_sync = Vec::<(libp2p::PeerId, libp2p::request_response::OutboundRequestId, Hash, u64)>::new();
    let mut sync_candidates = BTreeMap::<Hash, (u64, SyncResponse)>::new();
    let mut sync_required = 1usize;
    let mut sync_round_peers = BTreeMap::<libp2p::PeerId, u64>::new();
    let mut sync_conflicts = BTreeMap::<libp2p::PeerId, u64>::new();
    let mut sync_misbehavior = Vec::<SyncMisbehaviorEvidence>::new();
    let mut peer_reputations = store
        .load_peer_reputation::<BTreeMap<libp2p::PeerId, PeerReputation>>()
        .unwrap_or_default();
    if rebuild_peer_reputation_dedup_index(&mut peer_reputations) {
        store.save_peer_reputation(&peer_reputations)?;
    }
    let mut snapshot_misbehavior = Vec::<SnapshotMisbehaviorEvidence>::new();
    let mut pending_snapshot = Vec::<(libp2p::PeerId, libp2p::request_response::OutboundRequestId, Hash, u64)>::new();
    let mut snapshot_candidates = BTreeMap::<Hash, (u64, freedom_network::SnapshotSyncResponse)>::new();
    let mut snapshot_required = 1usize;
    let mut connected_peers = Vec::<libp2p::PeerId>::new();
    let mut sync_peer_cursor = 0usize;
    let mut valset_votes: BTreeMap<Hash, freedom_types::ValidatorSetVote> = BTreeMap::new();
    let mut valset_certificate_broadcasted: Option<Hash> = None;

    let next_height = state.height
        .checked_add(1)
        .ok_or_else(|| "block height exceeds u64::MAX".to_string())?;
    if proposer_for_height_round(&state, next_height, round) == Some(owner) {
        let (proposal, block) = build_local_proposal(&state, owner, &consensus_key, round)?;
        blocks.insert(proposal.block_hash, block);
        println!("proposing height={} round={} hash={}", next_height, round, hex::encode(proposal.block_hash));
        publish(&mut swarm, &NetworkMessage::Proposal(proposal))?;
    }

    loop {
        let next_height = state.height
            .checked_add(1)
            .ok_or_else(|| "block height exceeds u64::MAX".to_string())?;
        tokio::select! {
            _ = timer.tick() => {
                if state.epoch_length > 0 && (next_height) % state.epoch_length == 0 {
                    let new_set_hash = freedom_consensus::validator_set_hash(&state.pending_validator_set);
                    if let Ok(vote) = make_validator_set_vote(&state, validator_id, state.epoch.checked_add(1).ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())?, new_set_hash, &consensus_key) {
                        valset_votes.insert(vote.validator_id, vote.clone());
                        publish(&mut swarm, &NetworkMessage::ValidatorSetVote(vote))?;
                    }
                }
                if timeout_round_sent != Some(round) {
                    let timeout = make_timeout(&state, next_height, round, validator_id, &consensus_key)?;
                    publish(&mut swarm, &NetworkMessage::Timeout(timeout))?;
                    timeout_round_sent = Some(round);
                    println!("timeout height={} round={}", next_height, round);
                }
            }
            _ = sync_timer.tick() => {
                if decay_peer_reputation(&mut peer_reputations, state.height) {
                    store.save_peer_reputation(&peer_reputations)?;
                }
                if !connected_peers.is_empty() && pending_sync.is_empty() {
                    let from_height = next_height;
                    let peer_count = connected_peers.len().min(2);
                    sync_required = peer_count.max(1);
                    sync_candidates.clear();
                    sync_round_peers.clear();
                    sync_conflicts.clear();
                    let mut selected = 0usize;
                    let max_attempts = connected_peers.len();
                    while selected < peer_count && selected < max_attempts {
                        let peer = match select_sync_peer(&connected_peers, &mut sync_peer_cursor) {
                            Some(peer) => peer,
                            None => break,
                        };
                        if !peer_is_eligible(&peer_reputations, peer, state.height) {
                            continue;
                        }
                        let (sync_id, sync_outbound_id) = match send_sync_request(&mut swarm, peer, validator_id, state.height) {
                            Ok(value) => value,
                            Err(e) => {
                                eprintln!("sync request rejected: {e}");
                                continue;
                            }
                        };
                        pending_sync.push((peer, sync_outbound_id, sync_id, from_height));
                        sync_round_peers.insert(peer, from_height);
                        selected += 1;
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("peer_connected={peer_id}");
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                        if !connected_peers.contains(&peer_id) {
                            connected_peers.push(peer_id);
                        }
                        if !connected_peers.is_empty() {
                            sync_peer_cursor %= connected_peers.len();
                        }
                        if pending_sync.is_empty() {
                            sync_required = 1;
                            let (sync_id, sync_outbound_id) = match send_sync_request(&mut swarm, peer_id, validator_id, state.height) {
                                Ok(value) => value,
                                Err(e) => {
                                    eprintln!("sync request rejected: {e}");
                                    continue;
                                }
                            };
                            pending_sync.push((peer_id, sync_outbound_id, sync_id, next_height));
                            sync_round_peers.insert(peer_id, next_height);
                        }
                        if !pending_snapshot.iter().any(|(pending_peer, ..)| *pending_peer == peer_id)
                            && pending_snapshot.len() < 2
                            && peer_is_eligible(&peer_reputations, peer_id, state.height)
                        {
                            if pending_snapshot.is_empty() {
                                snapshot_candidates.clear();
                            }
                            let (snapshot_request_id, snapshot_outbound_id) = send_snapshot_request(&mut swarm, peer_id, validator_id, u64::MAX)?;
                            pending_snapshot.push((peer_id, snapshot_outbound_id, snapshot_request_id, u64::MAX));
                            snapshot_required = pending_snapshot.len().max(1);
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        connected_peers.retain(|p| *p != peer_id);
                        if !connected_peers.is_empty() {
                            sync_peer_cursor %= connected_peers.len();
                        } else {
                            sync_peer_cursor = 0;
                        }
                        pending_sync.retain(|(pending_peer, ..)| *pending_peer != peer_id);
                        if pending_sync.is_empty() {
                            sync_candidates.clear();
                            sync_round_peers.clear();
                            sync_conflicts.clear();
                            sync_misbehavior.clear();
                            sync_required = 1;
                        }
                        pending_snapshot.retain(|(pending_peer, ..)| *pending_peer != peer_id);
                        if pending_snapshot.is_empty() {
                            snapshot_candidates.clear();
                            snapshot_required = 1;
                        }
                        println!("peer_disconnected={peer_id}");
                    },
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Identify(libp2p::identify::Event::Received { peer_id, info })) => {
                        for addr in info.listen_addrs {
                            let _ = swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                    }
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Sync(event)) => {
                        if let libp2p::request_response::Event::Message { peer, message } = event {
                            match message {
                                libp2p::request_response::Message::Request { request, channel, .. } => {
                                    match request {
                                        SyncWireRequest::Blocks(req) => {
                                            let latest = state.height;
                                            if freedom_network::validate_sync_request(&req).is_err() {
                                                continue;
                                            }
                                            let to = req.to_height.min(latest);
                                            if req.from_height <= to {
                                                if let Ok(blocks) = store.range(req.from_height, to) {
                                                    let response = SyncWireResponse::Blocks(SyncResponse { request_id: req.request_id, from_height: req.from_height, blocks });
                                                    let _ = swarm.behaviour_mut().sync.send_response(channel, response);
                                                }
                                            }
                                        }
                                        SyncWireRequest::Snapshot(req) => {
                                            if freedom_network::validate_snapshot_request(&req).is_err() { eprintln!("SNAPSHOT_REQUEST_REJECTED height={}", req.height); continue; }
                                            if let Ok(Some(snapshot)) = store.latest_snapshot_at_or_below(req.height) {
                                                if let (Some(certificate), Ok(Some(checkpoint))) = (snapshot.certificate.clone(), store.get_block(snapshot.height)) {
                                                    if let Ok(state_json) = serde_json::to_vec(&snapshot.state) {
                                                        let response = SyncWireResponse::Snapshot(freedom_network::SnapshotSyncResponse {
                                                            request_id: req.request_id,
                                                            height: snapshot.height,
                                                            block_hash: snapshot.block_hash,
                                                            state_root: snapshot.state_root,
                                                            checkpoint,
                                                            certificate,
                                                            finality_validator_set: snapshot.finality_validator_set.clone(),
                                                            epoch_transition_certificate: snapshot.epoch_transition_certificate.clone(),
                                                            chain_id: freedom_primitives::CHAIN_ID,
                                                            state_json,
                                                        });
                                                        if let SyncWireResponse::Snapshot(ref snapshot_response) = response {
                                                            if freedom_network::validate_snapshot_response(snapshot_response).is_err() {
                                                                eprintln!("SNAPSHOT_RESPONSE_REJECTED height={}", snapshot_response.height);
                                                                continue;
                                                            }
                                                        }
                                                        let _ = swarm.behaviour_mut().sync.send_response(channel, response);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                libp2p::request_response::Message::Response { request_id: outbound_id, response } => {
                                    match response {
                                        SyncWireResponse::Blocks(response) => {
                                            let pending_index = pending_sync.iter().position(|(expected_peer, expected_outbound_id, expected_request_id, expected_from)| {
                                                *expected_peer == peer
                                                    && *expected_outbound_id == outbound_id
                                                    && *expected_request_id == response.request_id
                                                    && *expected_from == response.from_height
                                            });
                                            if pending_index.is_none() || freedom_network::validate_sync_response(&response).is_err() {
                                                continue;
                                            }
                                            pending_sync.remove(pending_index.unwrap());

                                            let fingerprint = hash_bytes(&serde_json::to_vec(&response.blocks)?);
                                            if sync_required > 1 && !sync_candidates.is_empty() && !sync_candidates.contains_key(&fingerprint) {
                                                sync_conflicts.insert(peer, response.from_height);
                                                let evidence = PeerReputationEvidence {
                                                    peer,
                                                    height: response.from_height,
                                                    request_id: response.request_id,
                                                    fingerprint,
                                                    kind: PeerEvidenceKind::SyncConflict,
                                                    severity: PeerEvidenceSeverity::Normal,
                                                    reason: "batch disagrees with another valid sync response".to_string(),
                                                };
                                                sync_misbehavior.push(SyncMisbehaviorEvidence {
                                                    peer: evidence.peer,
                                                    from_height: evidence.height,
                                                    request_id: evidence.request_id,
                                                    batch_fingerprint: evidence.fingerprint,
                                                    reason: evidence.reason.clone(),
                                                });
                                                let confirming_peers = sync_candidates
                                                    .values()
                                                    .map(|(count, _)| *count as usize)
                                                    .max()
                                                    .unwrap_or(1);
                                                record_peer_evidence_with_confirmations(
                                                    &mut peer_reputations,
                                                    evidence,
                                                    confirming_peers,
                                                );
                                                store.save_peer_reputation(&peer_reputations)?;
                                                eprintln!(
                                                    "SYNC_CONFLICT peer={} from={} fingerprint={} score={:?} existing_candidates={}",
                                                    peer,
                                                    response.from_height,
                                                    hex::encode(fingerprint),
                                                    peer_reputations.get(&peer).map(|r| r.score),
                                                    sync_candidates.len()
                                                );
                                            }
                                            let entry = sync_candidates.entry(fingerprint).or_insert((0, response.clone()));
                                            entry.0 += 1;
                                            let required = sync_required;
                                            if entry.0 < required {
                                                continue;
                                            }

                                            let mut working_state = state.clone();
                                            let mut accepted_blocks = Vec::new();
                                            for block in entry.1.blocks.clone() {
                                                let expected_height = match working_state.height.checked_add(1) {
                                                    Some(height) => height,
                                                    None => {
                                                        eprintln!("sync block rejected: state height exceeds u64::MAX");
                                                        break;
                                                    }
                                                };
                                                if block.header.height != expected_height { break; }
                                                let expected = match proposer_for_height_round(&working_state, block.header.height, block.header.round) { Some(v) => v, None => break };
                                                if expected != block.header.proposer { break; }
                                                if let Err(e) = freedom_network::validate_block_transition(&working_state, &block) { eprintln!("sync block rejected at {}: {e}", block.header.height); break; }
                                                let committed = match execute_block(&mut working_state, &block) { Ok(b) => b, Err(e) => { eprintln!("sync execution rejected at {}: {e}", block.header.height); break; } };
                                                accepted_blocks.push(committed);
                                            }
                                            if accepted > 0 {
                                                if let Err(e) = store.put_blocks_atomic(&accepted_blocks) {
                                                    eprintln!("sync persistence error: {e}");
                                                    continue;
                                                }
                                                state = working_state;
                                                println!("SYNCED blocks={} height={}", accepted, state.height);
                                                pending_sync.clear();
                                                sync_candidates.clear();
                                                sync_round_peers.clear();
                                                sync_conflicts.clear();
                                                sync_misbehavior.clear();
                                                sync_required = 1;
                                                round=0;
                                                timeout_round_sent=None;
                                                if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                    let (next_proposal, next_block) = build_local_proposal(&state, owner, &consensus_key, round)?;
                                                    blocks.insert(next_proposal.block_hash, next_block);
                                                    println!("proposing height={} round={} hash={}", next_height, round, hex::encode(next_proposal.block_hash));
                                                    publish(&mut swarm, &NetworkMessage::Proposal(next_proposal))?;
                                                }
                                            }
                                        }
                                        SyncWireResponse::Snapshot(response) => {
                                            let snapshot_pending_index = pending_snapshot.iter().position(|(expected_peer, expected_outbound_id, expected_request_id, target_height)| {
                                                *expected_peer == peer
                                                    && *expected_outbound_id == outbound_id
                                                    && *expected_request_id == response.request_id
                                                    && response.height <= *target_height
                                            });
                                            if snapshot_pending_index.is_none() {
                                                eprintln!("SNAPSHOT_REJECTED unsolicited, stale, wrong-peer, or wrong-request response at {}", response.height);
                                                continue;
                                            }
                                            pending_snapshot.remove(snapshot_pending_index.unwrap());
                                            if response.height <= state.height || freedom_network::validate_snapshot_response(&response).is_err() { continue; }
                                            if response.chain_id != freedom_primitives::CHAIN_ID {
                                                eprintln!("SNAPSHOT_REJECTED chain id mismatch");
                                                continue;
                                            }
                                            if response.height == 0 {
                                                eprintln!("SNAPSHOT_REJECTED genesis is not a remote finalized checkpoint");
                                                continue;
                                            }

                                            if response.block_hash != freedom_consensus::block_hash(&response.checkpoint)
                                                || response.checkpoint.header.height != response.height
                                                || response.checkpoint.header.state_root != response.state_root
                                                || response.certificate.block_hash != response.block_hash
                                                || response.certificate.height != response.height
                                                || response.certificate.round != response.checkpoint.header.round {
                                                eprintln!("SNAPSHOT_REJECTED checkpoint commitment mismatch at {}", response.height);
                                                continue;
                                            }
                                            let snapshot_state: freedom_state::ChainState = match serde_json::from_slice(&response.state_json) {
                                                Ok(s) => s,
                                                Err(e) => { eprintln!("SNAPSHOT_REJECTED invalid state encoding: {e}"); continue; }
                                            };
                                            let expected_finality_set_hash = if let Some(cert) = &response.checkpoint.header.epoch_transition_certificate {
                                                cert.previous_set_hash
                                            } else {
                                                freedom_consensus::validator_set_hash(&snapshot_state.active_validator_set)
                                            };
                                            if freedom_consensus::validator_set_hash(&response.finality_validator_set) != expected_finality_set_hash {
                                                eprintln!("SNAPSHOT_REJECTED finality validator set hash mismatch");
                                                continue;
                                            }
                                            if let Err(e) = freedom_consensus::validate_finality_certificate_for_set(
                                                &response.finality_validator_set,
                                                &response.checkpoint,
                                                &response.certificate,
                                            ) {
                                                eprintln!("SNAPSHOT_REJECTED invalid checkpoint finality certificate: {e}");
                                                continue;
                                            }
                                            if response.certificate.block_hash != response.block_hash
                                                || response.certificate.height != response.height
                                                || response.certificate.round != response.checkpoint.header.round {
                                                eprintln!("SNAPSHOT_REJECTED finality certificate checkpoint mismatch");
                                                continue;
                                            }
                                            if response.certificate.height != snapshot_state.height {
                                                eprintln!("SNAPSHOT_REJECTED finality certificate height does not match snapshot");
                                                continue;
                                            }
                                            if let Some(cert) = &response.epoch_transition_certificate {
                                                if freedom_consensus::validator_set_hash(&response.finality_validator_set) != cert.previous_set_hash {
                                                    eprintln!("SNAPSHOT_REJECTED finality set is not the transition previous set");
                                                    continue;
                                                }
                                                if cert.to_epoch != snapshot_state.epoch
                                                    || cert.new_set_hash != freedom_consensus::validator_set_hash(&snapshot_state.active_validator_set)
                                                    || snapshot_state.validator_set_certificate.as_ref().map(|c| c.hash()) != Some(cert.hash())
                                                    || response.checkpoint.header.epoch != cert.to_epoch
                                                    || response.checkpoint.header.epoch_transition_certificate.as_ref().map(|c| c.hash()) != Some(cert.hash()) {
                                                    eprintln!("SNAPSHOT_REJECTED invalid epoch transition certificate");
                                                    continue;
                                                }
                                            }

                                            if snapshot_state.height != response.height
                                                || snapshot_state.last_block_hash != response.block_hash
                                                || snapshot_state.state_root() != response.state_root {
                                                eprintln!("SNAPSHOT_REJECTED state commitment mismatch at {}", response.height);
                                                continue;
                                            }

                                            let snapshot_fingerprint = hash_bytes(
                                                &[
                                                    &response.height.to_le_bytes(),
                                                    &response.block_hash,
                                                    &response.state_root,
                                                    &hash_bytes(&serde_json::to_vec(&response.certificate)?),
                                                    &freedom_consensus::validator_set_hash(&response.finality_validator_set),
                                                    &response.epoch_transition_certificate.as_ref().map(|c| c.hash()).unwrap_or([0; 32]),
                                                ].concat(),
                                            );
                                            let snapshot_conflict = snapshot_required > 1
                                                && snapshot_candidates
                                                    .keys()
                                                    .any(|candidate| *candidate != snapshot_fingerprint);
                                            let snapshot_entry = snapshot_candidates
                                                .entry(snapshot_fingerprint)
                                                .or_insert((0, response.clone()));
                                            snapshot_entry.0 += 1;
                                            let snapshot_candidate_count = snapshot_candidates.len();
                                            if snapshot_conflict {
                                                let evidence = PeerReputationEvidence {
                                                    peer,
                                                    height: response.height,
                                                    request_id: response.request_id,
                                                    fingerprint: snapshot_fingerprint,
                                                    kind: PeerEvidenceKind::SnapshotConflict,
                                                    severity: PeerEvidenceSeverity::Normal,
                                                    reason: "snapshot commitment disagrees with another fully validated response".to_string(),
                                                };
                                                snapshot_misbehavior.push(SnapshotMisbehaviorEvidence {
                                                    peer: evidence.peer,
                                                    height: evidence.height,
                                                    request_id: evidence.request_id,
                                                    snapshot_fingerprint: evidence.fingerprint,
                                                    reason: evidence.reason.clone(),
                                                });
                                                let confirming_peers = snapshot_candidates
                                                    .values()
                                                    .map(|(count, _)| *count as usize)
                                                    .max()
                                                    .unwrap_or(1);
                                                record_peer_evidence_with_confirmations(
                                                    &mut peer_reputations,
                                                    evidence,
                                                    confirming_peers,
                                                );
                                                store.save_peer_reputation(&peer_reputations)?;
                                                eprintln!(
                                                    "SNAPSHOT_CONFLICT peer={} height={} fingerprint={} score={:?} candidates={}",
                                                    peer,
                                                    response.height,
                                                    hex::encode(snapshot_fingerprint),
                                                    peer_reputations.get(&peer).map(|r| r.score),
                                                    snapshot_candidate_count
                                                );
                                            }
                                            if snapshot_entry.0 < snapshot_required {
                                                continue;
                                            }
                                            let verified_snapshot = snapshot_entry.1.clone();

                                            if let Err(e) = store.put_block_and_snapshot(
                                                &verified_snapshot.checkpoint,
                                                &snapshot_state,
                                                verified_snapshot.certificate.clone(),
                                                verified_snapshot.finality_validator_set.clone(),
                                            ) {
                                                &response.checkpoint,
                                                &snapshot_state,
                                                response.certificate.clone(),
                                                response.finality_validator_set.clone(),
                                            ) {
                                                eprintln!("snapshot activation persistence error: {e}");
                                                continue;
                                            }
                                            state = snapshot_state;
                                            let next_height = state
                                                .height
                                                .checked_add(1)
                                                .ok_or_else(|| "block height exceeds u64::MAX".to_string())?;
                                            pending_sync.clear();
                                            snapshot_candidates.clear();
                                            snapshot_misbehavior.clear();
                                            snapshot_required = 1;
                                            pending_snapshot.clear();
                                            round = 0;
                                            timeout_round_sent = None;
                                            protocol.reset_round();
                                            blocks.clear();
                                            println!("SNAPSHOT_ACTIVATED height={} hash={} bytes={}", response.height, hex::encode(response.block_hash), response.state_json.len());
                                            if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                let (next_proposal, next_block) = build_local_proposal(&state, owner, &consensus_key, round)?;
                                                blocks.insert(next_proposal.block_hash, next_block);
                                                println!("proposing height={} round={} hash={}", next_height, round, hex::encode(next_proposal.block_hash));
                                                publish(&mut swarm, &NetworkMessage::Proposal(next_proposal))?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Gossipsub(event)) => {
                        if let libp2p::gossipsub::Event::Message { message, .. } = event {
                            match decode_message(&message.data) {
                                Ok(NetworkMessage::ValidatorSetVote(vote)) => {
                                    if !protocol.remember_message(vote.message_id()) { continue; }
                                    if freedom_network::validate_validator_set_vote(&state, &vote).is_err() { continue; }
                                    valset_votes.insert(vote.validator_id, vote);
                                    let votes: Vec<_> = valset_votes.values().cloned().collect();
                                    if let Ok(cert) = freedom_runtime::collect_validator_set_certificate(&state, &votes) {
                                        if valset_certificate_broadcasted != Some(cert.hash()) {
                                            valset_certificate_broadcasted = Some(cert.hash());
                                            freedom_runtime::commit_validator_set_certificate(&mut state, &cert)?;
                                            publish(&mut swarm, &NetworkMessage::ValidatorSetCertificate(cert))?;
                                        }
                                    }
                                }
                                Ok(NetworkMessage::ValidatorSetCertificate(cert)) => {
                                    if !protocol.remember_message(cert.hash()) { continue; }
                                    if freedom_runtime::commit_validator_set_certificate(&mut state, &cert).is_ok() {
                                        // Certificate is committed only through the runtime admission boundary.
                                    }
                                }
                                Ok(NetworkMessage::SyncRequest(_req)) => {
                                    eprintln!("IGNORED_LEGACY_GOSSIPSUB_SYNC_REQUEST");
                                }
                                Ok(NetworkMessage::SyncResponse(_response)) => {
                                    eprintln!("IGNORED_LEGACY_GOSSIPSUB_SYNC_RESPONSE");
                                }
                                Ok(NetworkMessage::Proposal(proposal)) => {
                                    if !protocol.remember_message(proposal.message_id()) { continue; }
                                    if proposal.height != next_height || proposal.round != round { continue; }
                                    if let Err(e) = freedom_network::validate_proposal(&state, &proposal) {
                                        eprintln!("proposal rejected: {e}");
                                        continue;
                                    }
                                    blocks.insert(proposal.block_hash, proposal.block.clone());
                                    let vote = match protocol.receive_proposal(&mut state, &proposal) {
                                        Ok(v) => v,
                                        Err(e) if e == "validator proposal equivocation detected" => {
                                            let key=(proposal.proposer,proposal.height,proposal.round);
                                            if let Some(existing)=protocol.proposals.get(&key) {
                                                let evidence=freedom_consensus::make_evidence(
                                                    freedom_types::EquivocationKind::DoubleProposal,
                                                    proposal.height,proposal.round,proposal.proposer,
                                                    existing.signing_bytes(),proposal.signing_bytes());
                                                state.record_equivocation(evidence.clone())?;
                                                let msg=make_evidence_message(&state,evidence,validator_id,&consensus_key)?;
                                                publish(&mut swarm,&NetworkMessage::Evidence(msg))?;
                                            }
                                            continue;
                                        }
                                        Err(e) => return Err(e),
                                    };
                                    publish(&mut swarm, &NetworkMessage::Vote(vote))?;
                                    timeout_round_sent = None;
                                }
                                Ok(NetworkMessage::Vote(vote)) => {
                                    if !protocol.remember_message(vote.message_id()) { continue; }
                                    if vote.height != next_height || vote.round != round { continue; }
                                    let hash = vote.block_hash;
                                    if let Some(existing)=protocol.votes.get(&vote.validator_id) {
                                        if existing.block_hash != vote.block_hash || existing.round != vote.round {
                                            let evidence=freedom_consensus::make_evidence(
                                                freedom_types::EquivocationKind::DoublePrevote,
                                                vote.height,vote.round,vote.validator_id,
                                                existing.signing_bytes(),vote.signing_bytes());
                                            state.record_equivocation(evidence.clone())?;
                                            let msg=make_evidence_message(evidence,validator_id,&consensus_key);
                                            publish(&mut swarm,&NetworkMessage::Evidence(msg))?;
                                            continue;
                                        }
                                    }
                                    let ready = protocol.receive_vote(&state, vote, hash, next_height, round)?;
                                    if ready {
                                        let block = blocks.get(&hash).ok_or_else(|| "prevote quorum block missing".to_string())?;
                                        let precommit = protocol.lock_on_prevote_quorum(&state, hash, next_height, round, block)?;
                                        publish(&mut swarm, &NetworkMessage::Precommit(precommit))?;
                                    }
                                }
                                Ok(NetworkMessage::Precommit(precommit)) => {
                                    if !protocol.remember_message(precommit.message_id()) { continue; }
                                    if precommit.height != next_height || precommit.round != round { continue; }
                                    let hash = precommit.block_hash;
                                    if let Some(existing)=protocol.precommits.get(&precommit.validator_id) {
                                        if existing.block_hash != precommit.block_hash || existing.round != precommit.round {
                                            let evidence=freedom_consensus::make_evidence(
                                                freedom_types::EquivocationKind::DoublePrecommit,
                                                precommit.height,precommit.round,precommit.validator_id,
                                                existing.signing_bytes(),precommit.signing_bytes());
                                            state.record_equivocation(evidence.clone())?;
                                            let msg=make_evidence_message(evidence,validator_id,&consensus_key);
                                            publish(&mut swarm,&NetworkMessage::Evidence(msg))?;
                                            continue;
                                        }
                                    }
                                    let ready = protocol.receive_precommit(&state, precommit, hash, next_height, round)?;
                                    if ready {
                                        if let Some(block) = blocks.get(&hash).cloned() {
                                            let finality_validator_set = state.active_validator_set.clone();
                                            let committed = protocol.finalize_if_quorum(&mut state, &block, round)?;
                                            store.put_block(&committed)?;
                                            if committed.header.height % snapshot_interval == 0 {
                                                let certificate = freedom_types::FinalityCertificate {
                                                    block_hash: freedom_consensus::block_hash(&committed),
                                                    height: committed.header.height,
                                                    round,
                                                    precommits: protocol.precommits.values().cloned().collect(),
                                                };
                                                store.save_snapshot_with_certificate_and_validator_set(
                                                    &state,
                                                    freedom_consensus::block_hash(&committed),
                                                    certificate,
                                                    finality_validator_set,
                                                )?;
                                            }
                                            println!("FINALIZED height={} round={} hash={}", committed.header.height, round, hex::encode(block_hash(&committed)));
                                            protocol.reset_height();
                                            blocks.clear();
                                            round = 0;
                                            timeout_round_sent = None;
                                            let next_height = state
                                                .height
                                                .checked_add(1)
                                                .ok_or_else(|| "block height exceeds u64::MAX".to_string())?;
                                            if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                let (next_proposal, next_block) = build_local_proposal(&state, owner, &consensus_key, round)?;
                                                blocks.insert(next_proposal.block_hash, next_block);
                                                println!("proposing height={} round={} hash={}", next_height, round, hex::encode(next_proposal.block_hash));
                                                publish(&mut swarm, &NetworkMessage::Proposal(next_proposal))?;
                                            }
                                        }
                                    }
                                }
                                Ok(NetworkMessage::Evidence(msg)) => {
                                    if !protocol.remember_message(msg.message_id()) { continue; }
                                    freedom_consensus::validate_evidence_message(&state, &msg)?;
                                    state.record_equivocation(msg.evidence.clone())?;
                                    println!("EVIDENCE_RECEIVED id={}", hex::encode(msg.evidence.evidence_hash));
                                }
                                Ok(NetworkMessage::Timeout(timeout)) => {
                                    if !protocol.remember_message(timeout.message_id()) { continue; }
                                    if timeout.height != next_height || timeout.round != round { continue; }
                                    if let Some(existing)=protocol.timeouts.get(&timeout.validator_id) {
                                        if existing.signing_bytes() != timeout.signing_bytes() {
                                            eprintln!("timeout equivocation observed validator={}", hex::encode(timeout.validator_id));
                                            continue;
                                        }
                                    }
                                    if protocol.receive_timeout(&state, timeout, next_height, round)? {
                                        let timeout_certificate = freedom_types::TimeoutCertificate {
                                            height: next_height,
                                            round,
                                            timeouts: protocol.timeouts.values().cloned().collect(),
                                        };
                                        let previous_round = round;
                                        round = round.checked_add(1).ok_or_else(|| "round exceeds u64::MAX".to_string())?;
                                        let locked_block = protocol.locked_block.clone();
                                        let locked_round = protocol.locked_round;
                                        let locked_precommits = protocol.precommits.values().cloned().collect::<Vec<_>>();
                                        protocol.reset_round();
                                        blocks.clear();
                                        timeout_round_sent = None;
                                        println!("VIEW_CHANGE height={} new_round={}", next_height, round);
                                        if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                            let (block, valid_round, valid_precommits) = if let (Some(block), Some(lr)) = (locked_block, locked_round) {
                                                (block, Some(lr), locked_precommits)
                                            } else {
                                                let mempool = Mempool::new();
                                                (build_next_block(&state, &mempool, owner, 1, if state.epoch_length > 0 && (next_height) % state.epoch_length == 0 { state.epoch.checked_add(1).ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())? } else { state.epoch }, round, 100)?, None, Vec::new())
                                            };
                                            let proposal = make_proposal(&state, block.clone(), round, &consensus_key, valid_round, valid_precommits, Some(timeout_certificate))?;
                                            blocks.insert(proposal.block_hash, block);
                                            println!("proposing height={} round={} hash={} previous_round={}", next_height, round, hex::encode(proposal.block_hash), previous_round);
                                            publish(&mut swarm, &NetworkMessage::Proposal(proposal))?;
                                        }
                                    }
                                }
                                Err(e) => eprintln!("message decode error: {e}"),
                            }
                        }
                    }
                    SwarmEvent::Behaviour(_) => {}
                    _ => {}
                }
            }
        }
    }
}
