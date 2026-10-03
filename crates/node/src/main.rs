use freedom_consensus::{block_hash, proposer_for_height_round};
use freedom_network::{build_swarm, decode_message, make_proposal, make_timeout, publish, NetworkMessage};
use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY, Hash};
use freedom_runtime::{build_next_block, initialize, Mempool, ValidatorProtocol};
use freedom_types::{Genesis, Validator};
use libp2p::{Multiaddr, SwarmEvent};
use libp2p::futures::StreamExt;
use std::collections::BTreeMap;
use tokio::time::{interval, Duration};

const ROUND_TIMEOUT_SECS: u64 = 5;

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
    let mut round = 0u64;
    let mut timeout_round_sent = None::<u64>;
    let mut timer = interval(Duration::from_secs(ROUND_TIMEOUT_SECS));

    let mut swarm = build_swarm()?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?)?;

    if let Ok(peers) = std::env::var("FREEDOM_PEERS") {
        for raw in peers.split(',').filter(|s| !s.trim().is_empty()) {
            let addr: Multiaddr = raw.trim().parse()?;
            swarm.dial(addr)?;
        }
    }

    println!("Freedom Chain M17 validator={validator_index}");
    println!("peer_id={}", swarm.local_peer_id());
    println!("validators={}", state.validators.len());
    println!("round_timeout={}s", ROUND_TIMEOUT_SECS);

    if proposer_for_height_round(&state, 1, round) == Some(owner) {
        let mempool = Mempool::new();
        let block = build_next_block(&state, &mempool, owner, 1, round, 100)?;
        let proposal = make_proposal(block.clone(), round, &consensus_key);
        blocks.insert(proposal.block_hash, block);
        println!("proposing height=1 round={} hash={}", round, hex::encode(proposal.block_hash));
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
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("peer_connected={peer_id}");
                        let _ = swarm.behaviour_mut().kademlia.bootstrap();
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
                                            println!("FINALIZED height={} round={} hash={}", committed.header.height, round, hex::encode(block_hash(&committed)));
                                            protocol.reset_round();
                                            blocks.clear();
                                            round = 0;
                                            timeout_round_sent = None;
                                            let next_height = state.height + 1;
                                            if proposer_for_height_round(&state, next_height, round) == Some(owner) {
                                                let mempool = Mempool::new();
                                                let next_block = build_next_block(&state, &mempool, owner, next_height, round, 100)?;
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
                                            let block = build_next_block(&state, &mempool, owner, state.height + 1, round, 100)?;
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
