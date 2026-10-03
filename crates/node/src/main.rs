use freedom_consensus::{block_hash,receipts_root,tx_root};
use freedom_primitives::{address_from_public_key,hash_bytes,CHAIN_ID,GENESIS_SUPPLY};
use freedom_runtime::{execute_block,initialize};
use freedom_types::{Block,BlockHeader,Genesis,Validator};

fn main(){
let validators:Vec<Validator>=(0u8..3).map(|i|{let key=[i+1;32];Validator{validator_id:hash_bytes(&[b"validator",&[i+1]].concat()),owner:address_from_public_key(&key),consensus_key:key,stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0}}).collect();
let genesis=Genesis{chain_id:CHAIN_ID,initial_supply:GENESIS_SUPPLY,validators};
let mut state=initialize(&genesis).expect("invalid genesis");
println!("Freedom Chain M12 localnet");
println!("chain_id={} validators={} genesis_supply={}",CHAIN_ID,state.validators.len(),state.supply.total);
for height in 1..=6{
let proposer=freedom_consensus::proposer_for_height(&state,height).expect("no proposer");
let mut candidate=Block{header:BlockHeader{height,previous_hash:state.last_block_hash,state_root:[0;32],tx_root:tx_root(&[]),receipts_root:receipts_root(&[]),timestamp:height,epoch:height,proposer},transactions:vec![],receipts:vec![]};
let mut preview=state.clone();preview.height=height;candidate.header.state_root=preview.state_root();
let committed=execute_block(&mut state,&candidate).expect("block execution failed");
println!("block={} proposer={} txs={} hash={}",height,hex::encode(proposer),committed.transactions.len(),hex::encode(block_hash(&committed)));
}
println!("final_height={} final_state_root={}",state.height,hex::encode(state.state_root()));
}