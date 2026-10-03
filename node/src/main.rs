use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID, GENESIS_SUPPLY};
use freedom_runtime::initialize;
use freedom_types::{Genesis, Validator};

fn main() {
    let validators: Vec<Validator> = (0u8..3)
        .map(|i| {
            let public_key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: address_from_public_key(&public_key),
                consensus_key: public_key,
                stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        })
        .collect();

    let genesis = Genesis {
        chain_id: CHAIN_ID,
        initial_supply: GENESIS_SUPPLY,
        validators,
    };

    let state = initialize(&genesis).expect("invalid genesis");

    println!("Freedom Chain M0");
    println!("chain_id={}", genesis.chain_id);
    println!("height={}", state.height);
    println!("validators={}", state.validators.len());
    println!("genesis_supply={}", state.supply.total);
}
