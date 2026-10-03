use freedom_consensus::{block_hash,receipts_root,tx_root,validate_genesis,ConsensusConfig};
use freedom_crypto::verify;
use freedom_primitives::{CHAIN_ID,Hash};
use freedom_state::{ChainState,StateError};
use freedom_types::{Block,Genesis,Receipt,Transaction};
pub fn initialize(g:&Genesis)->Result<ChainState,String>{if g.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}let s=ChainState::genesis(g);validate_genesis(&s,&ConsensusConfig::default())?;s.assert_invariants();Ok(s)}
pub fn execute_transaction(s:&mut ChainState,tx:&Transaction)->Result<Receipt,String>{if tx.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}let a=s.accounts.get(&tx.sender).ok_or_else(||"sender account not found".to_string())?;if tx.signature.len()!=64{return Err("invalid signature length".into())}let mut sig=[0u8;64];sig.copy_from_slice(&tx.signature);if !verify(&a.public_key,&tx.signing_bytes(),&sig){return Err("invalid transaction signature".into())}let h:Hash=tx.hash();let before=s.events.len();s.execute_transaction(tx).map_err(format_state_error)?;s.assert_invariants();let events=s.events[before..].to_vec();Ok(Receipt{tx_hash:h,success:true,gas_used:1,events})}
pub fn execute_block(s:&mut ChainState,block:&Block)->Result<Block,String>{
if block.header.height!=s.height+1{return Err("invalid block height".into())}
if block.header.previous_hash!=s.last_block_hash{return Err("invalid previous block hash".into())}
if block.header.tx_root!=tx_root(&block.transactions){return Err("transaction root mismatch".into())}
let mut working=s.clone();
let mut receipts=Vec::with_capacity(block.transactions.len());
for tx in &block.transactions{receipts.push(execute_transaction(&mut working,tx)?);}
if block.header.receipts_root!=receipts_root(&receipts){return Err("receipts root mismatch".into())}
working.height=block.header.height;
if block.header.state_root!=working.state_root(){return Err("state root mismatch".into())}
let committed=Block{header:block.header.clone(),transactions:block.transactions.clone(),receipts};
let bh=block_hash(&committed);
working.last_block_hash=bh;
working.assert_invariants();
*s=working;
Ok(committed)}
fn format_state_error(e:StateError)->String{format!("{e:?}")}
