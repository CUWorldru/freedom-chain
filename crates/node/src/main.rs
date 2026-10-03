use freedom_consensus::proposer_for_height;
use freedom_network::build_swarm;
use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY};
use freedom_runtime::initialize;
use freedom_types::{Genesis, Validator};
use libp2p::{Multiaddr, SwarmEvent};
use libp2p::futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validators: Vec<Validator> = (0u8..3).map(|i| {
        let key = [i + 1; 32];
        Validator {
            validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
            owner: address_from_public_key(&key),
            consensus_key: key,
            stake: 0,
            blocks_proposed: 0,
            blocks_finalized: 0,
            missed_votes: 0,
        }
    }).collect();

    let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: GENESIS_SUPPLY, validators };
    let state = initialize(&genesis)?;
    let mut swarm = build_swarm()?;
    let listen: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(listen)?;

    println!("Freedom Chain M15");
    println!("peer_id={}", swarm.local_peer_id());
    println!("validators={}", state.validators.len());
    println!("next_proposer={:?}", proposer_for_height(&state, 1));

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("listening={address}"),
            SwarmEvent::ConnectionEstablished { peer_id, .. } => println!("peer_connected={peer_id}"),
            SwarmEvent::ConnectionClosed { peer_id, .. } => println!("peer_disconnected={peer_id}"),
            SwarmEvent::Behaviour(_) => {}
            _ => {}
        }
    }
}
