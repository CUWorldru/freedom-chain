use freedom_consensus::{block_hash, proposer_for_height_round};
use freedom_network::{
    build_swarm, decode_message, make_proposal, make_sync_request, make_timeout, publish,
    NetworkMessage, SyncResponse,
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

fn request_id(validator_id: Hash, from_height: u64) -> Hash {
    hash_bytes(&[b"FREEDOM-SYNC-V1", &validator_id, &from_height.to_le_bytes()].concat())
}

fn send_sync_request(
    swarm: &mut libp2p::Swarm<freedom_network::FreedomBehaviour>,
    validator_id: Hash,
    height: u64,
) -> Result<Hash, Box<dyn std::error::Error>> {
    let from = height.saturating_add(1);
    let id = request_id(validator_id, from);
    publish(swarm, &make_sync_request(id, from))?;
    println!("sync_request from={} request_id={}", from, hex::encode(id));
    Ok(id)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    if stored_height > 0 {
        for block in store.range(1, stored_height)? {
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

    if proposer_for_height_round(&state, state.height + 1, round) == Some(owner) {
        let mempool = Mempool::new();
        let block = build_next_block(&state, &mempool, owner, 1, 0, round, 100)?;
        let proposal = make_proposal(block.clone(), round, &consensus_key);
        blocks.insert(proposal.block_hash, block);
        println!("proposing height={} round={} hash={}", state.height + 1, round, hex::encode(proposal.block_hash));
        publish(&mut swarm, &NetworkMessage::Proposal(proposal))?;
    }

    loop {
        tokio::select! {
            _ = timer.tick() => {
                if timeout_round_sent != Some(round) {
                    let timeout = make_timeout(state.height + 1, round, validator_id, &consensus_key);
                    publish(&mut swarm, &NetworkMessage::Timeout(timeout))?;
                    timeout_round_sent = Some(round);
                    println!("timeout height={} round={}", state.height + 1, round);
                }
            }
            _ = sync_timer.tick() => {
                pending_sync = Some(send_sync_request(&mut swarm, validator_id, state.height)?);
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("peer_connected={peer_id}");
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                        pending_sync = Some(send_sync_request(&mut swarm, validator_id, state.height)?);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => println!("peer_disconnected={peer_id}"),
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Identify(libp2p::identify::Event::Received { peer_id, info })) => {
                        for addr in info.listen_addrs {
                            let _ = swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
                    }
                    SwarmEvent::Behaviour(freedom_network::FreedomEvent::Gossipsub(event)) => {
                        if let libp2p::gossipsub::Event::Message { message, .. } = event {
                            match decode_message(&message.data) {
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
                                        if block.header.round != block.header.epoch { break; }
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
                                    if proposal.height != state.height + 1 || proposal.round != round { continue; }
                                    if let Err(e) = freedom_network::validate_proposal(&state, &proposal) {
                                        eprintln!("proposal rejected: {e}");
                                        continue;
                                    }
                                    blocks.insert(proposal.block_hash, proposal.block.clone());
                                    let vote = protocol.receive_proposal(&state, &proposal)?;
                                    publish(&mut swarm, &NetworkMessage::Vote(vote))?;
                                    timeout_round_sent = None;
                                }
                                Ok(NetworkMessage::Vote(vote)) => {
                                    if vote.height != state.height + 1 || vote.round != round { continue; }
                                    let hash = vote.block_hash;
                                    let ready = protocol.receive_vote(&state, vote, hash, state.height + 1, round)?;
                                    if ready {
                                        if let Some(block) = blocks.get(&hash).cloned() {
                                            let committed = protocol.finalize_if_quorum(&mut state, &block, round)?;
                                            store.put_block(&committed)?;
                                            println!("FINALIZED height={} round={} hash={}", committed.header.height, round, hex::encode(block_hash(&committed)));
                                            protocol.reset_round();
                                            blocks.clear();
                                            round = 0;
                                            timeout_round_sent = None;
                                            let next_height = state.height + 1;
                                            if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                let mempool = Mempool::new();
                                                let next_block = build_next_block(&state, &mempool, owner, 1, round, 100)?;
                                                let next_proposal = make_proposal(next_block.clone(), round, &consensus_key);
                                                blocks.insert(next_proposal.block_hash, next_block);
                                                println!("proposing height={} round={} hash={}", next_height, round, hex::encode(next_proposal.block_hash));
                                                publish(&mut swarm, &NetworkMessage::Proposal(next_proposal))?;
                                            }
                                        }
                                    }
                                }
                                Ok(NetworkMessage::Timeout(timeout)) => {
                                    if timeout.height != state.height + 1 || timeout.round != round { continue; }
                                    if protocol.receive_timeout(&state, timeout, state.height + 1, round)? {
                                        round = round.saturating_add(1);
                                        protocol.reset_round();
                                        blocks.clear();
                                        timeout_round_sent = None;
                                        println!("VIEW_CHANGE height={} new_round={}", state.height + 1, round);
                                        if proposer_for_height_round(&state, state.height + 1, round) == Some(owner) {
                                            let mempool = Mempool::new();
                                            let block = build_next_block(&state, &mempool, owner, 1, round, 100)?;
                                            let proposal = make_proposal(block.clone(), round, &consensus_key);
                                            blocks.insert(proposal.block_hash, block);
                                            println!("proposing height={} round={} hash={}", state.height + 1, round, hex::encode(proposal.block_hash));
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
