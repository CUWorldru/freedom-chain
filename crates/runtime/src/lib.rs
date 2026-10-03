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


#[derive(Clone, Debug, Default)]
pub struct Mempool {
    transactions: std::collections::BTreeMap<Hash, Transaction>,
}

impl Mempool {
    pub fn new() -> Self { Self::default() }

    pub fn len(&self) -> usize { self.transactions.len() }

    pub fn is_empty(&self) -> bool { self.transactions.is_empty() }

    pub fn submit(&mut self, state: &ChainState, tx: Transaction) -> Result<Hash, String> {
        if tx.chain_id != CHAIN_ID { return Err("invalid chain id".into()); }
        let account = state.accounts.get(&tx.sender).ok_or_else(|| "sender account not found".to_string())?;
        if tx.nonce < account.nonce { return Err("stale transaction nonce".into()); }
        if tx.signature.len() != 64 { return Err("invalid signature length".into()); }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&tx.signature);
        if !verify(&account.public_key, &tx.signing_bytes(), &sig) {
            return Err("invalid transaction signature".into());
        }
        let hash = tx.hash();
        if self.transactions.contains_key(&hash) { return Err("duplicate transaction".into()); }
        self.transactions.insert(hash, tx);
        Ok(hash)
    }

    pub fn remove(&mut self, hash: &Hash) -> Option<Transaction> {
        self.transactions.remove(hash)
    }

    pub fn ordered(&self, state: &ChainState, max: usize) -> Vec<Transaction> {
        let mut candidates: Vec<Transaction> = self.transactions.values().cloned().collect();
        candidates.sort_by(|a,b| {
            a.sender.cmp(&b.sender)
                .then(a.nonce.cmp(&b.nonce))
                .then(a.hash().cmp(&b.hash()))
        });

        let mut working = state.clone();
        let mut selected = Vec::new();
        for tx in candidates {
            if selected.len() >= max { break; }
            if execute_transaction(&mut working, &tx).is_ok() {
                selected.push(tx);
            }
        }
        selected
    }

    pub fn remove_committed(&mut self, transactions: &[Transaction]) {
        for tx in transactions { self.transactions.remove(&tx.hash()); }
    }
}

pub fn build_next_block(
    state: &ChainState,
    mempool: &Mempool,
    proposer: freedom_primitives::Address,
    timestamp: u64,
    epoch: u64,
    max_transactions: usize,
) -> Result<Block, String> {
    if proposer == [0; 32] { return Err("invalid proposer".into()); }
    let transactions = mempool.ordered(state, max_transactions);
    let mut working = state.clone();
    let mut receipts = Vec::with_capacity(transactions.len());
    for tx in &transactions {
        receipts.push(execute_transaction(&mut working, tx)?);
    }
    let height = state.height + 1;
    working.height = height;
    let tx_root = tx_root(&transactions);
    let receipts_root = receipts_root(&receipts);
    let state_root = working.state_root();
    Ok(Block {
        header: freedom_types::BlockHeader {
            height,
            previous_hash: state.last_block_hash,
            state_root,
            tx_root,
            receipts_root,
            timestamp,
            epoch,
            proposer,
        },
        transactions,
        receipts: Vec::new(),
    })
}
