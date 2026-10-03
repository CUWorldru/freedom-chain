use freedom_consensus::{block_hash, proposer_for_height_round};
use freedom_network::{
    build_swarm, decode_message, make_proposal, make_sync_request, make_timeout, make_validator_set_vote, publish,
    NetworkMessage, SyncResponse, SyncWireRequest, SyncWireResponse,
};
use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY, Hash};
use freedom_runtime::{build_next_block, execute_block, initialize, Mempool, ValidatorProtocol};
use freedom_storage::ChainStore;
use freedom_types::{Genesis, Validator};
use libp2p::{Multiaddr, SwarmEvent};
use libp2p::futures::StreamExt;
use std::collections::BTreeMap;
use tokio::time::{interval, Duration};

const ROUND_TIMEOUT_SECS: u64 = 5;
const SYNC_INTERVAL_SECS: u64 = 10;
const DEFAULT_SNAPSHOT_INTERVAL: u64 = 1000;

fn request_id(validator_id: Hash, from_height: u64) -> Hash {
    hash_bytes(&[b"FREEDOM-SYNC-V1", &validator_id, &from_height.to_le_bytes()].concat())
}

fn send_snapshot_request(
    swarm: &mut libp2p::Swarm<freedom_network::FreedomBehaviour>,
    peer: libp2p::PeerId,
    validator_id: Hash,
    height: u64,
) -> Hash {
    let request_id = hash_bytes(&[b"FREEDOM-SNAPSHOT-V1", &validator_id, &height.to_le_bytes()].concat());
    swarm.behaviour_mut().sync.send_request(
        &peer,
        SyncWireRequest::Snapshot(freedom_network::SnapshotSyncRequest { request_id, height }),
    );
    println!("snapshot_request peer={} height={} request_id={}", peer, height, hex::encode(request_id));
    request_id
}

fn send_sync_request(
    swarm: &mut libp2p::Swarm<freedom_network::FreedomBehaviour>,
    peer: libp2p::PeerId,
    validator_id: Hash,
    height: u64,
) -> Hash {
    let from = height.saturating_add(1);
    let id = request_id(validator_id, from);
    swarm.behaviour_mut().sync.send_request(&peer, SyncWireRequest::Blocks(freedom_network::SyncRequest {
        request_id: id,
        from_height: from,
        to_height: from.saturating_add(freedom_network::MAX_SYNC_BLOCKS.saturating_sub(1)),
    }));
    println!("sync_request peer={} from={} request_id={}", peer, from, hex::encode(id));
    id
}

#[tokio::main]
async fn make_evidence_message(evidence: freedom_types::EquivocationEvidence, sender: Hash, key: &ed25519_dalek::SigningKey) -> freedom_types::EvidenceMessage {
    let mut msg=freedom_types::EvidenceMessage{evidence,sender,signature:Vec::new()};
    msg.signature=freedom_crypto::sign(key,&msg.signing_bytes()).to_vec();
    msg
}

#[cfg(test)]
mod m31_tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    #[test]
    fn evidence_message_is_signed_by_local_validator() {
        let key=ed25519_dalek::SigningKey::from_bytes(&[11u8;32]);
        let evidence=freedom_consensus::make_evidence(
            freedom_types::EquivocationKind::DoublePrevote,
            10,2,hash_bytes(b"bad-validator"),vec![1,2],vec![3,4]);
        let msg=make_evidence_message(evidence,hash_bytes(b"sender"),&key);
        assert_eq!(msg.signature.len(),64);
        assert_ne!(msg.message_id(),[0u8;32]);
    }
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
    let validator_index = std::env::var("FREEDOM_VALIDATOR_INDEX").ok()
        .and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    if validator_index >= 3 { return Err("FREEDOM_VALIDATOR_INDEX must be 0..2".into()); }

    let consensus_key = ed25519_dalek::SigningKey::from_bytes(&[(validator_index as u8) + 1; 32]);
    let validator_id = hash_bytes(&[b"validator", &[(validator_index as u8) + 1]].concat());
    let owner = state.validators.get(&validator_id).ok_or("validator not found")?.owner;
    let mut protocol = ValidatorProtocol::new(validator_id, consensus_key.clone());
    let mut blocks: BTreeMap<Hash, freedom_types::Block> = BTreeMap::new();

    let db_path = std::env::var("FREEDOM_DB_PATH").unwrap_or_else(|_| "./data/freedom".to_string());
    let store = ChainStore::open(&db_path)?;
    let stored_height = store.latest_height()?;
    let snapshot_interval = std::env::var("FREEDOM_SNAPSHOT_INTERVAL").ok()
        .and_then(|v| v.parse::<u64>().ok()).filter(|v| *v > 0)
        .unwrap_or(DEFAULT_SNAPSHOT_INTERVAL);
    let replay_from = if let Some(snapshot) = store.latest_snapshot()? {
        let checkpoint = store.get_block(snapshot.height)?.ok_or("snapshot checkpoint block missing")?;
        if block_hash(&checkpoint) != snapshot.block_hash || checkpoint.header.state_root != snapshot.state_root {
            return Err(format!("snapshot checkpoint mismatch at {}", snapshot.height).into());
        }
        state = snapshot.state;
        if state.last_block_hash != snapshot.block_hash || state.height != snapshot.height {
            return Err(format!("snapshot state linkage mismatch at {}", snapshot.height).into());
        }
        snapshot.height.saturating_add(1)
    } else {
        store.save_snapshot(&state, [0;32])?;
        1
    };
    if stored_height >= replay_from {
        for block in store.range(replay_from, stored_height)? {
            if block.header.height != state.height + 1 {
                return Err(format!("stored chain height gap at {}", block.header.height).into());
            }
            if block.header.previous_hash != state.last_block_hash {
                return Err(format!("stored chain previous hash mismatch at {}", block.header.height).into());
            }
            let expected = proposer_for_height_round(&state, block.header.height, block.header.round)
                .ok_or("no proposer for stored block")?;
            if expected != block.header.proposer {
                return Err(format!("stored block proposer mismatch at {}", block.header.height).into());
            }
            let committed = execute_block(&mut state, &block)?;
            if block_hash(&committed) != block_hash(&block) {
                return Err(format!("stored block hash mismatch at {}", block.header.height).into());
            }
        }
        if state.last_block_hash != store.latest_hash()? {
            return Err("stored latest hash metadata mismatch".into());
        }
        println!("restored height={} hash={}", state.height, hex::encode(state.last_block_hash));
    }

    let mut swarm = build_swarm()?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?)?;

    if let Ok(peers) = std::env::var("FREEDOM_PEERS") {
        for raw in peers.split(',').filter(|s| !s.trim().is_empty()) {
            swarm.dial(raw.trim().parse::<Multiaddr>()?)?;
        }
    }

    println!("Freedom Chain M18 validator={validator_index}");
    println!("peer_id={}", swarm.local_peer_id());
    println!("validators={}", state.validators.len());
    println!("height={}", state.height);
    println!("db={}", db_path);

    let mut round = 0u64;
    let mut timeout_round_sent = None::<u64>;
    let mut timer = interval(Duration::from_secs(ROUND_TIMEOUT_SECS));
    let mut sync_timer = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
    let mut pending_sync = None::<Hash>;
    let mut connected_peers = Vec::<libp2p::PeerId>::new();
    let mut valset_votes: BTreeMap<Hash, freedom_types::ValidatorSetVote> = BTreeMap::new();
    let mut valset_certificate_broadcasted: Option<Hash> = None;

    if proposer_for_height_round(&state, state.height + 1, round) == Some(owner) {
        let mempool = Mempool::new();
        let block = build_next_block(&state, &mempool, owner, 1, if state.epoch_length > 0 && (state.height + 1) % state.epoch_length == 0 { state.epoch.saturating_add(1) } else { state.epoch }, round, 100)?;
        let proposal = make_proposal(block.clone(), round, &consensus_key, None, Vec::new(), None);
        blocks.insert(proposal.block_hash, block);
        println!("proposing height={} round={} hash={}", state.height + 1, round, hex::encode(proposal.block_hash));
        publish(&mut swarm, &NetworkMessage::Proposal(proposal))?;
    }

    loop {
        tokio::select! {
            _ = timer.tick() => {
                if state.epoch_length > 0 && (state.height + 1) % state.epoch_length == 0 {
                    let new_set_hash = freedom_consensus::validator_set_hash(&state.pending_validator_set);
                    if let Ok(vote) = make_validator_set_vote(&state, validator_id, state.epoch.saturating_add(1), new_set_hash, &consensus_key) {
                        valset_votes.insert(vote.validator_id, vote.clone());
                        publish(&mut swarm, &NetworkMessage::ValidatorSetVote(vote))?;
                    }
                }
                if timeout_round_sent != Some(round) {
                    let timeout = make_timeout(state.height + 1, round, validator_id, &consensus_key);
                    publish(&mut swarm, &NetworkMessage::Timeout(timeout))?;
                    timeout_round_sent = Some(round);
                    println!("timeout height={} round={}", state.height + 1, round);
                }
            }
            _ = sync_timer.tick() => {
                if let Some(peer) = connected_peers.first().copied() { pending_sync = Some(send_sync_request(&mut swarm, peer, validator_id, state.height)); }
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("peer_connected={peer_id}");
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                        if !connected_peers.contains(&peer_id) { connected_peers.push(peer_id); }
                        pending_sync = Some(send_sync_request(&mut swarm, peer_id, validator_id, state.height));
                        if state.height == 0 {
                            let _ = send_snapshot_request(&mut swarm, peer_id, validator_id, 0);
                        }
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => { connected_peers.retain(|p| *p != peer_id); println!("peer_disconnected={peer_id}"); },
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Identify(libp2p::identify::Event::Received { peer_id, info })) => {
                        for addr in info.listen_addrs {
                            let _ = swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                    }
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Sync(event)) => {
                        if let libp2p::request_response::Event::Message { message, .. } = event {
                            match message {
                                libp2p::request_response::Message::Request { request, channel, .. } => {
                                    match request {
                                        SyncWireRequest::Blocks(req) => {
                                            let latest = state.height;
                                            let to = req.to_height.min(latest).min(req.from_height.saturating_add(freedom_network::MAX_SYNC_BLOCKS.saturating_sub(1)));
                                            if req.from_height > 0 && req.from_height <= to {
                                                if let Ok(blocks) = store.range(req.from_height, to) {
                                                    let response = SyncWireResponse::Blocks(SyncResponse { request_id: req.request_id, from_height: req.from_height, blocks });
                                                    let _ = swarm.behaviour_mut().sync.send_response(channel, response);
                                                }
                                            }
                                        }
                                        SyncWireRequest::Snapshot(req) => {
                                            if let Ok(Some(snapshot)) = if req.height == 0 { store.latest_snapshot() } else { store.load_snapshot(req.height) } {
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
                                                        let _ = swarm.behaviour_mut().sync.send_response(channel, response);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                libp2p::request_response::Message::Response { response, .. } => {
                                    match response {
                                        SyncWireResponse::Blocks(response) => {
                                            if pending_sync != Some(response.request_id) || response.blocks.is_empty() { continue; }
                                            let mut accepted = 0u64;
                                            for block in response.blocks {
                                                if block.header.height != state.height + 1 { break; }
                                                let expected = match proposer_for_height_round(&state, block.header.height, block.header.round) { Some(v) => v, None => break };
                                                if expected != block.header.proposer { break; }
                                                if let Err(e) = freedom_network::validate_block_transition(&state, &block) { eprintln!("sync block rejected at {}: {e}", block.header.height); break; }
                                                let committed = match execute_block(&mut state, &block) { Ok(b) => b, Err(e) => { eprintln!("sync execution rejected at {}: {e}", block.header.height); break; } };
                                                if let Err(e) = store.put_block(&committed) { eprintln!("sync persistence error: {e}"); break; }
                                                if committed.header.height % snapshot_interval == 0 { if let Err(e) = store.save_snapshot(&state, block_hash(&committed)) { eprintln!("snapshot persistence error: {e}"); } }
                                                accepted += 1;
                                            }
                                            if accepted > 0 { println!("SYNCED blocks={} height={}", accepted, state.height); pending_sync=None; round=0; timeout_round_sent=None; }
                                        }
                                        SyncWireResponse::Snapshot(response) => {
                                            if response.height <= state.height { continue; }
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
                                            state = snapshot_state;
                                            if let Err(e) = store.put_block(&response.checkpoint) {
                                                eprintln!("snapshot checkpoint persistence error: {e}");
                                                continue;
                                            }
                                            if let Err(e) = store.save_snapshot_with_certificate_and_validator_set(
                                                &state,
                                                response.block_hash,
                                                response.certificate.clone(),
                                                response.finality_validator_set.clone(),
                                            ) {
                                                eprintln!("snapshot persistence error: {e}");
                                                continue;
                                            }
                                            pending_sync = None;
                                            round = 0;
                                            timeout_round_sent = None;
                                            protocol.reset_round();
                                            blocks.clear();
                                            println!("SNAPSHOT_ACTIVATED height={} hash={} bytes={}", response.height, hex::encode(response.block_hash), response.state_json.len());
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
                                            state.validator_set_certificate = Some(cert.clone());
                                            publish(&mut swarm, &NetworkMessage::ValidatorSetCertificate(cert))?;
                                        }
                                    }
                                }
                                Ok(NetworkMessage::ValidatorSetCertificate(cert)) => {
                                    if !protocol.remember_message(cert.hash()) { continue; }
                                    if freedom_consensus::validate_validator_set_certificate(&state, &cert).is_ok() {
                                        state.validator_set_certificate = Some(cert);
                                    }
                                }
                                Ok(NetworkMessage::SyncRequest(req)) => {
                                    if req.from_height == 0 || req.from_height > req.to_height {
                                        continue;
                                    }
                                    let latest = state.height;
                                    let to = req.to_height.min(latest).min(req.from_height.saturating_add(31));
                                    if req.from_height <= to {
                                        match store.range(req.from_height, to) {
                                            Ok(blocks) => {
                                                let response = NetworkMessage::SyncResponse(SyncResponse {
                                                    request_id: req.request_id,
                                                    from_height: req.from_height,
                                                    blocks,
                                                });
                                                let _ = publish(&mut swarm, &response);
                                            }
                                            Err(e) => eprintln!("sync read error: {e}"),
                                        }
                                    }
                                }
                                Ok(NetworkMessage::SyncResponse(response)) => {
                                    if pending_sync != Some(response.request_id) || response.blocks.is_empty() {
                                        continue;
                                    }
                                    let mut accepted = 0u64;
                                    for block in response.blocks {
                                        if block.header.height != state.height + 1 { break; }
                                        let expected = match proposer_for_height_round(&state, block.header.height, block.header.round) {
                                            Some(v) => v,
                                            None => break,
                                        };
                                        if expected != block.header.proposer { break; }
                                        if let Err(e) = freedom_network::validate_block_transition(&state, &block) {
                                            eprintln!("sync block rejected at {}: {e}", block.header.height);
                                            break;
                                        }
                                        let committed = match execute_block(&mut state, &block) {
                                            Ok(b) => b,
                                            Err(e) => { eprintln!("sync execution rejected at {}: {e}", block.header.height); break; }
                                        };
                                        if let Err(e) = store.put_block(&committed) {
                                            eprintln!("sync persistence error: {e}");
                                            break;
                                        }
                                        accepted += 1;
                                    }
                                    if accepted > 0 {
                                        println!("SYNCED blocks={} height={}", accepted, state.height);
                                        pending_sync = None;
                                        round = 0;
                                        timeout_round_sent = None;
                                    }
                                }
                                Ok(NetworkMessage::Proposal(proposal)) => {
                                    if !protocol.remember_message(proposal.message_id()) { continue; }
                                    if proposal.height != state.height + 1 || proposal.round != round { continue; }
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
                                                let msg=make_evidence_message(evidence,validator_id,&consensus_key);
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
                                    if vote.height != state.height + 1 || vote.round != round { continue; }
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
                                    let ready = protocol.receive_vote(&state, vote, hash, state.height + 1, round)?;
                                    if ready {
                                        let block = blocks.get(&hash).ok_or_else(|| "prevote quorum block missing".to_string())?;
                                        let precommit = protocol.lock_on_prevote_quorum(&state, hash, state.height + 1, round, block)?;
                                        publish(&mut swarm, &NetworkMessage::Precommit(precommit))?;
                                    }
                                }
                                Ok(NetworkMessage::Precommit(precommit)) => {
                                    if !protocol.remember_message(precommit.message_id()) { continue; }
                                    if precommit.height != state.height + 1 || precommit.round != round { continue; }
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
                                    let ready = protocol.receive_precommit(&state, precommit, hash, state.height + 1, round)?;
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
                                            let next_height = state.height + 1;
                                            if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                let mempool = Mempool::new();
                                                let next_block = build_next_block(&state, &mempool, owner, 1, if state.epoch_length > 0 && (state.height + 1) % state.epoch_length == 0 { state.epoch.saturating_add(1) } else { state.epoch }, round, 100)?;
                                                let next_proposal = make_proposal(next_block.clone(), round, &consensus_key, None, Vec::new(), None);
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
                                    if timeout.height != state.height + 1 || timeout.round != round { continue; }
                                    if let Some(existing)=protocol.timeouts.get(&timeout.validator_id) {
                                        if existing.signing_bytes() != timeout.signing_bytes() {
                                            eprintln!("timeout equivocation observed validator={}", hex::encode(timeout.validator_id));
                                            continue;
                                        }
                                    }
                                    if protocol.receive_timeout(&state, timeout, state.height + 1, round)? {
                                        let timeout_certificate = freedom_types::TimeoutCertificate {
                                            height: state.height + 1,
                                            round,
                                            timeouts: protocol.timeouts.values().cloned().collect(),
                                        };
                                        let previous_round = round;
                                        round = round.saturating_add(1);
                                        let locked_block = protocol.locked_block.clone();
                                        let locked_round = protocol.locked_round;
                                        let locked_precommits = protocol.precommits.values().cloned().collect::<Vec<_>>();
                                        protocol.reset_round();
                                        blocks.clear();
                                        timeout_round_sent = None;
                                        println!("VIEW_CHANGE height={} new_round={}", state.height + 1, round);
                                        if proposer_for_height_round(&state, state.height + 1, round) == Some(owner) {
                                            let (block, valid_round, valid_precommits) = if let (Some(block), Some(lr)) = (locked_block, locked_round) {
                                                (block, Some(lr), locked_precommits)
                                            } else {
                                                let mempool = Mempool::new();
                                                (build_next_block(&state, &mempool, owner, 1, if state.epoch_length > 0 && (state.height + 1) % state.epoch_length == 0 { state.epoch.saturating_add(1) } else { state.epoch }, round, 100)?, None, Vec::new())
                                            };
                                            let proposal = make_proposal(block.clone(), round, &consensus_key, valid_round, valid_precommits, Some(timeout_certificate));
                                            blocks.insert(proposal.block_hash, block);
                                            println!("proposing height={} round={} hash={} previous_round={}", state.height + 1, round, hex::encode(proposal.block_hash), previous_round);
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
