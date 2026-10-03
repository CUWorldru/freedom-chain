use freedom_consensus::{block_hash, proposer_for_height};
use freedom_network::{build_swarm, decode_message, make_proposal, publish, NetworkMessage};
use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY, Hash};
use freedom_runtime::{build_next_block, initialize, Mempool, ValidatorProtocol};
use freedom_types::{Genesis, Validator};
use libp2p::{Multiaddr, SwarmEvent};
use libp2p::futures::StreamExt;
use std::collections::BTreeMap;

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
    let mut protocol = ValidatorProtocol::new(validator_id, consensus_key.clone());
    let mut blocks: BTreeMap<Hash, freedom_types::Block> = BTreeMap::new();

    let mut swarm = build_swarm()?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?)?;

    if let Ok(peers) = std::env::var("FREEDOM_PEERS") {
        for raw in peers.split(',').filter(|s| !s.trim().is_empty()) {
            swarm.dial(raw.trim().parse::<Multiaddr>()?)?;
        }
    }

    println!("Freedom Chain M16 validator={validator_index}");
    println!("peer_id={}", swarm.local_peer_id());
    println!("validators={}", state.validators.len());

    let proposer = proposer_for_height(&state, 1).ok_or("no proposer")?;
    if proposer == state.validators.get(&validator_id).ok_or("validator not found")?.owner {
        let mempool = Mempool::new();
        let block = build_next_block(&state, &mempool, proposer, 1, 1, 100)?;
        let proposal = make_proposal(block.clone(), &consensus_key);
        let hash = proposal.block_hash;
        blocks.insert(hash, block);
        println!("proposing height=1 hash={}", hex::encode(hash));
        publish(&mut swarm, &NetworkMessage::Proposal(proposal))?;
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
            SwarmEvent::ConnectionEstablished { peer_id, .. } => println!("peer_connected={peer_id}"),
            SwarmEvent::ConnectionClosed { peer_id, .. } => println!("peer_disconnected={peer_id}"),
            SwarmEvent::Behaviour(freedom_network::FreedomEvent::Gossipsub(event)) => {
                if let libp2p::gossipsub::Event::Message { message, .. } = event {
                    match decode_message(&message.data) {
                        Ok(NetworkMessage::Proposal(proposal)) => {
                            if let Err(e) = freedom_network::validate_proposal(&state, &proposal) {
                                eprintln!("proposal rejected: {e}");
                                continue;
                            }
                            blocks.insert(proposal.block_hash, proposal.block.clone());
                            let vote = protocol.receive_proposal(&state, &proposal)?;
                            publish(&mut swarm, &NetworkMessage::Vote(vote))?;
                        }
                        Ok(NetworkMessage::Vote(vote)) => {
                            let hash = vote.block_hash;
                            let height = vote.height;
                            let ready = protocol.receive_vote(&state, vote, hash, height)?;
                            if ready {
                                if let Some(block) = blocks.get(&hash).cloned() {
                                    let committed = protocol.finalize_if_quorum(&mut state, &block)?;
                                    println!("FINALIZED height={} hash={}", committed.header.height, hex::encode(block_hash(&committed)));
                                    protocol.reset_round();
                                    blocks.clear();
                                    let next_height = state.height + 1;
                                    if let Some(next_proposer) = proposer_for_height(&state, next_height) {
                                        if next_proposer == state.validators.get(&validator_id).unwrap().owner {
                                            let mempool = Mempool::new();
                                            let next_block = build_next_block(&state, &mempool, next_proposer, next_height, next_height, 100)?;
                                            let next_proposal = make_proposal(next_block.clone(), &consensus_key);
                                            blocks.insert(next_proposal.block_hash, next_block);
                                            println!("proposing height={} hash={}", next_height, hex::encode(next_proposal.block_hash));
                                            publish(&mut swarm, &NetworkMessage::Proposal(next_proposal))?;
                                        }
                                    }
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
