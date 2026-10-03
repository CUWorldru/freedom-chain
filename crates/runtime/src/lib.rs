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


#[cfg(test)]
mod tests {
    use super::*;
    use freedom_crypto::sign;
    use freedom_primitives::Supply;
    use freedom_state::Account;
    use freedom_types::{Genesis, TransferPayload, TxKind, Validator};

    fn test_state() -> ChainState {
        let validators = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            let owner = freedom_primitives::address_from_public_key(&key);
            let id = freedom_primitives::hash_bytes(&[b"validator", &[i + 1]].concat());
            (id, Validator {
                validator_id: id,
                owner,
                consensus_key: key,
                stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            })
        }).collect::<Vec<_>>();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: validators.into_iter().map(|(_,v)| v).collect() };
        let mut state = initialize(&genesis).unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let public = signing_key.verifying_key().to_bytes();
        state.insert_account(Account::new(public, 1_000));
        state.supply = Supply { total: 1_000, circulating: 1_000, locked: 0, staked: 0, treasury: 0 };
        state.assert_invariants();
        state
    }

    #[test]
    fn mempool_orders_and_block_builder_commits_transfer() {
        let mut state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let recipient = freedom_primitives::address_from_public_key(&[9; 32]);
        let payload = TransferPayload { to: recipient, amount: 250 }.encode();
        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload,
            fee: 10,
            signature: Vec::new(),
        };
        tx.signature = sign(&signing_key, &tx.signing_bytes()).to_vec();

        let mut mempool = Mempool::new();
        let hash = mempool.submit(&state, tx.clone()).unwrap();
        assert_eq!(mempool.len(), 1);

        let proposer = freedom_consensus::proposer_for_height(&state, 1).unwrap();
        let block = build_next_block(&state, &mempool, proposer, 1, 1, 100).unwrap();
        assert_eq!(block.transactions, vec![tx.clone()]);
        assert_eq!(block.header.tx_root, tx_root(&block.transactions));
        assert_eq!(block.header.receipts_root, receipts_root(&{
            let mut preview = state.clone();
            block.transactions.iter().map(|t| execute_transaction(&mut preview, t).unwrap()).collect::<Vec<_>>()
        }));

        let committed = execute_block(&mut state, &block).unwrap();
        assert_eq!(state.height, 1);
        assert_eq!(state.last_block_hash, block_hash(&committed));
        assert_eq!(state.accounts.get(&recipient).unwrap().balance, 250);
        mempool.remove_committed(&committed.transactions);
        assert_eq!(mempool.len(), 0);
        assert!(mempool.remove(&hash).is_none());
    }

    #[test]
    fn mempool_rejects_stale_nonce_and_duplicate_hash() {
        let state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let recipient = freedom_primitives::address_from_public_key(&[8; 32]);
        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 1 }.encode(),
            fee: 0,
            signature: Vec::new(),
        };
        tx.signature = sign(&signing_key, &tx.signing_bytes()).to_vec();
        let mut mempool = Mempool::new();
        mempool.submit(&state, tx.clone()).unwrap();
        assert!(mempool.submit(&state, tx).is_err());

        let mut stale = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 1,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 1 }.encode(),
            fee: 0,
            signature: Vec::new(),
        };
        stale.signature = sign(&signing_key, &stale.signing_bytes()).to_vec();
        let mut next_state = state.clone();
        next_state.accounts.get_mut(&sender).unwrap().nonce = 2;
        assert!(mempool.submit(&next_state, stale).is_err());
    }
}
