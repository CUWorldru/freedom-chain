use freedom_consensus::{block_hash,receipts_root,tx_root,validate_genesis,ConsensusConfig};
use freedom_crypto::verify;
use freedom_primitives::{CHAIN_ID,Hash};
use freedom_state::{ChainState,StateError};
use freedom_types::{Block,Genesis,Receipt,Transaction};
pub fn collect_validator_set_certificate(
    state: &ChainState,
    votes: &[freedom_types::ValidatorSetVote],
) -> Result<freedom_types::ValidatorSetCertificate, String> {
    if votes.is_empty() { return Err("no validator set votes".into()); }
    let mut unique = std::collections::BTreeSet::new();
    for vote in votes {
        freedom_network::validate_validator_set_vote(state, vote)?;
        if !unique.insert(vote.validator_id) {
            return Err("duplicate validator set vote".into());
        }
    }
    if !freedom_consensus::stake_quorum_reached(state, unique.iter()) {
        return Err("validator set vote quorum not reached".into());
    }
    Ok(freedom_types::ValidatorSetCertificate {
        from_epoch: state.epoch,
        to_epoch: state.epoch.saturating_add(1),
        previous_set_hash: freedom_consensus::validator_set_hash(&state.active_validator_set),
        new_set_hash: freedom_consensus::validator_set_hash(&state.pending_validator_set),
        votes: votes.to_vec(),
    })
}

pub fn commit_validator_set_certificate(
    state: &mut ChainState,
    certificate: &freedom_types::ValidatorSetCertificate,
) -> Result<(), String> {
    freedom_consensus::validate_validator_set_certificate(state, certificate)?;
    state.validator_set_certificate = Some(certificate.clone());
    Ok(())
}

pub fn initialize(g:&Genesis)->Result<ChainState,String>{if g.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}let s=ChainState::genesis(g);validate_genesis(&s,&ConsensusConfig::default())?;s.assert_invariants();Ok(s)}
pub fn validate_transaction_admission(s:&ChainState,tx:&Transaction)->Result<(),String>{
 if tx.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}
 let a=s.accounts.get(&tx.sender).ok_or_else(||"sender account not found".to_string())?;
 if tx.nonce < a.nonce{return Err("stale transaction nonce".into())}
 if tx.signature.len()!=64{return Err("invalid signature length".into())}
 let mut sig=[0u8;64];sig.copy_from_slice(&tx.signature);
 if !verify(&a.public_key,&tx.signing_bytes(),&sig){return Err("invalid transaction signature".into())}
 match tx.kind {
  freedom_types::TxKind::BridgeMint => validate_bridge_mint_admission(s,tx)?,
  _ => {}
 }
 Ok(())
}

fn validate_bridge_mint_admission(s:&ChainState,tx:&Transaction)->Result<(),String>{
 let p=freedom_types::BridgeMintTransactionPayload::decode(&tx.payload).map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
 let policy=s.bridge_security_policies.get(&p.message.bridge_id).ok_or_else(||"InvalidBridgeMintTransaction".to_string())?;
 let keys=s.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<std::collections::BTreeMap<freedom_primitives::Hash,[u8;32]>>();
 let expected=freedom_types::BridgeMintAuthorization::from_authenticated_message(&p.message,&p.deposit,&p.approvals,policy,&keys)
   .map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
 if expected!=p.authorization{return Err("InvalidBridgeMintTransaction".into())}
 if p.message.destination_chain_id!=CHAIN_ID{return Err("InvalidBridgeMintTransaction".into())}
 if !p.message.validate_hash() || !p.message.matches_verified_deposit(&p.deposit){return Err("InvalidBridgeMintTransaction".into())}
 if s.bridge_message_status.contains_key(&p.message.replay_key()){return Err("DuplicateBridgeMessage".into())}
 Ok(())
}

pub fn execute_transaction(s:&mut ChainState,tx:&Transaction)->Result<Receipt,String>{
 validate_transaction_admission(s,tx)?;
 let h:Hash=tx.hash();let before=s.events.len();
 s.execute_transaction(tx).map_err(format_state_error)?;
 s.assert_invariants();
 let events=s.events[before..].to_vec();
 Ok(Receipt{tx_hash:h,success:true,gas_used:1,events})
}
pub fn execute_block(s:&mut ChainState,block:&Block)->Result<Block,String>{
if block.header.height!=s.height+1{return Err("invalid block height".into())}
if block.header.previous_hash!=s.last_block_hash{return Err("invalid previous block hash".into())}
let expected_epoch=if s.epoch_length>0 && block.header.height%s.epoch_length==0{block.header.height/s.epoch_length}else{s.epoch};
if block.header.epoch!=expected_epoch{return Err("invalid block epoch".into())}
freedom_consensus::validate_epoch_transition_certificate(s, block)?;
if block.header.tx_root!=tx_root(&block.transactions){return Err("transaction root mismatch".into())}
let mut working=s.clone();
let mut receipts=Vec::with_capacity(block.transactions.len());
for tx in &block.transactions{receipts.push(execute_transaction(&mut working,tx)?);}
if block.header.receipts_root!=receipts_root(&receipts){return Err("receipts root mismatch".into())}
if let Some(cert) = &block.header.epoch_transition_certificate {
    freedom_consensus::validate_validator_set_certificate(s, cert)?;
    working.validator_set_certificate = Some(cert.clone());
}
working.height=block.header.height;
working.maybe_advance_epoch();
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
        validate_transaction_admission(state, &tx)?;
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
    round: u64,
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
    let epoch_transition_certificate = if epoch != state.epoch { state.validator_set_certificate.clone() } else { None };
    if let Some(cert) = &epoch_transition_certificate {
        working.validator_set_certificate = Some(cert.clone());
    }
    let state_root = working.state_root();
    Ok(Block {
        header: freedom_types::BlockHeader {
            height,
            round,
            previous_hash: state.last_block_hash,
            state_root,
            tx_root,
            receipts_root,
            timestamp,
            epoch,
            proposer,
            epoch_transition_certificate,
        },
        transactions,
        receipts,
    })
}


#[cfg(test)]
mod m29_tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    #[test]
    fn message_dedup_is_bounded_and_deterministic() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        let mut p = ValidatorProtocol::new(hash_bytes(b"v"), key);
        assert!(p.remember_message(hash_bytes(b"a")));
        assert!(!p.remember_message(hash_bytes(b"a")));
        p.max_seen_messages = 2;
        assert!(p.remember_message(hash_bytes(b"b")));
        assert!(p.remember_message(hash_bytes(b"c")));
        assert_eq!(p.seen_message_order.len(), 2);
        assert!(!p.seen_messages.contains(&hash_bytes(b"a")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_crypto::sign;
    use freedom_primitives::Supply;
    use freedom_state::Account;
    use freedom_types::{BridgeMessage, BridgeMessageApprovalSet, BridgeMintAuthorization, BridgeMintTransactionPayload, Genesis, TransferPayload, TxKind, Validator, VerifiedDeposit};

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
                unbonding_stake: 0,
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
    fn runtime_routes_bridge_mint_as_transaction_kind() {
        let mut state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let payload = BridgeMintTransactionPayload {
            message: BridgeMessage {
                protocol_version: 1, source_chain_id: 2, destination_chain_id: CHAIN_ID,
                bridge_id: [1;32], asset_id: [2;32], source_tx_hash: [3;32],
                source_event_index: 0, nonce: 1, amount: 1, recipient: [4;32], message_hash: [5;32],
            },
            deposit: VerifiedDeposit {
                canonical_asset_id:[2;32], representation_asset_id:[6;32], bridge_id:[1;32],
                source_chain_id:2, tx_hash:[3;32], event_index:0, block_or_slot:0,
                confirmations:1, amount:1, recipient:vec![4;20], replay_key:[7;32], proof_hash:[8;32],
            },
            authorization: BridgeMintAuthorization {
                bridge_id:[1;32], message_hash:[5;32], replay_key:[7;32],
                canonical_asset_id:[2;32], representation_asset_id:[6;32],
                source_chain_id:2, source_tx_hash:[3;32], source_event_index:0,
                destination_chain_id:CHAIN_ID, nonce:1, amount:1, recipient:[4;32],
                approval_count:1, authorization_hash:[9;32],
            },
            approvals: BridgeMessageApprovalSet { bridge_id:[1;32], message_hash:[5;32], approvals:vec![] },
        }.encode().unwrap();
        let mut tx=Transaction{chain_id:CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload,fee:0,signature:Vec::new()};
        tx.signature=sign(&signing_key,&tx.signing_bytes()).to_vec();
        assert_eq!(execute_transaction(&mut state,&tx),Err("InvalidBridgeMintTransaction".into()));
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
        let block = build_next_block(&state, &mempool, proposer, 1, 1, 0, 100).unwrap();
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


pub fn finalize_block_with_quorum(
    state: &mut ChainState,
    block: &Block,
    votes: &[freedom_types::ValidatorVote],
) -> Result<Block, String> {
    let hash = block_hash(block);
    freedom_consensus::validate_quorum(state, hash, block.header.height, 0, votes)?;
    let committed = execute_block(state, block)?;
    Ok(committed)
}


#[derive(Clone)]
pub struct ValidatorProtocol {
    pub validator_id: Hash,
    pub consensus_key: ed25519_dalek::SigningKey,
    pub votes: std::collections::BTreeMap<Hash, freedom_types::ValidatorVote>,
    pub precommits: std::collections::BTreeMap<Hash, freedom_types::ValidatorPrecommit>,
    pub timeouts: std::collections::BTreeMap<Hash, freedom_types::ValidatorTimeout>,
    pub locked_round: Option<u64>,
    pub locked_block_hash: Option<Hash>,
    pub locked_block: Option<Block>,
    pub proposals: std::collections::BTreeMap<(Hash,u64,u64), freedom_types::BlockProposal>,
    pub seen_messages: std::collections::BTreeSet<Hash>,
    pub seen_message_order: std::collections::VecDeque<Hash>,
    pub max_seen_messages: usize,
}

impl ValidatorProtocol {
    pub fn new(validator_id: Hash, consensus_key: ed25519_dalek::SigningKey) -> Self {
        Self { validator_id, consensus_key, votes: std::collections::BTreeMap::new(), precommits: std::collections::BTreeMap::new(), timeouts: std::collections::BTreeMap::new(), locked_round: None, locked_block_hash: None, locked_block: None, proposals: std::collections::BTreeMap::new(), seen_messages: std::collections::BTreeSet::new(), seen_message_order: std::collections::VecDeque::new(), max_seen_messages: 4096 }
    }

    pub fn remember_message(&mut self, id: Hash) -> bool {
        if self.seen_messages.contains(&id) { return false; }
        self.seen_messages.insert(id);
        self.seen_message_order.push_back(id);
        while self.seen_message_order.len() > self.max_seen_messages {
            if let Some(old) = self.seen_message_order.pop_front() { self.seen_messages.remove(&old); }
        }
        true
    }

    pub fn reset_round(&mut self) { self.votes.clear(); self.precommits.clear(); self.timeouts.clear(); self.proposals.retain(|(_,_,r),_| *r==self.locked_round.unwrap_or(0)); }

    pub fn reset_height(&mut self) {
        self.reset_round();
        self.locked_round = None;
        self.locked_block_hash = None;
        self.locked_block = None;
    }

    pub fn receive_proposal(&mut self, state: &ChainState, proposal: &freedom_types::BlockProposal) -> Result<freedom_types::ValidatorVote, String> {
        if let Some(locked) = self.locked_block_hash {
            if locked != proposal.block_hash {
                return Err("proposal conflicts with local lock".into());
            }
        }
        let key=(proposal.proposer,proposal.height,proposal.round);
        if let Some(existing)=self.proposals.get(&key) {
            if existing.block_hash != proposal.block_hash {
                return Err("validator proposal equivocation detected".into());
            }
        } else {
            self.proposals.insert(key, proposal.clone());
        }
        freedom_network::make_vote(state, self.validator_id, proposal, &self.consensus_key)
    }

    pub fn receive_timeout(&mut self, state: &ChainState, timeout: freedom_types::ValidatorTimeout, height: u64, round: u64) -> Result<bool, String> { freedom_consensus::validate_timeout(state, &timeout, height, round)?;
if let Some(existing)=self.timeouts.get(&timeout.validator_id){
 if existing.signing_bytes()!=timeout.signing_bytes(){return Err("validator timeout equivocation detected".into());}
 return Ok(freedom_consensus::stake_quorum_reached(state, self.timeouts.keys()));
}
self.timeouts.insert(timeout.validator_id, timeout);
Ok(freedom_consensus::stake_quorum_reached(state, self.timeouts.keys())) }

    pub fn receive_vote(&mut self, state: &ChainState, vote: freedom_types::ValidatorVote, expected_block_hash: Hash, height: u64, round: u64) -> Result<bool, String> {
        freedom_consensus::validate_vote(state, &vote, expected_block_hash, height, round)?;
        if let Some(existing) = self.votes.get(&vote.validator_id) {
            if existing.block_hash != vote.block_hash || existing.round != vote.round {
                return Err("validator prevote equivocation detected".into());
            }
            return Ok(freedom_consensus::stake_quorum_reached(state, self.votes.keys()));
        }
        self.votes.insert(vote.validator_id, vote);
        Ok(freedom_consensus::stake_quorum_reached(state, self.votes.keys()))
    }

    pub fn lock_on_prevote_quorum(
        &mut self,
        state: &ChainState,
        block_hash: Hash,
        height: u64,
        round: u64,
        block: &Block,
    ) -> Result<freedom_types::ValidatorPrecommit, String> {
        let votes: Vec<_> = self.votes.values().cloned().collect();
        freedom_consensus::validate_quorum(state, block_hash, height, round, &votes)?;
        if let Some(locked) = self.locked_block_hash {
            if locked != block_hash {
                return Err("locked on a different block".into());
            }
        }
        self.locked_round = Some(round);
        self.locked_block_hash = Some(block_hash);
        self.locked_block = Some(block.clone());
        let mut precommit = freedom_types::ValidatorPrecommit {
            block_hash,
            height,
            round,
            validator_id: self.validator_id,
            signature: Vec::new(),
        };
        precommit.signature = freedom_crypto::sign(&self.consensus_key, &precommit.signing_bytes()).to_vec();
        self.precommits.insert(self.validator_id, precommit.clone());
        Ok(precommit)
    }

    pub fn receive_precommit(
        &mut self,
        state: &ChainState,
        precommit: freedom_types::ValidatorPrecommit,
        expected_block_hash: Hash,
        height: u64,
        round: u64,
    ) -> Result<bool, String> {
        freedom_consensus::validate_precommit(state, &precommit, expected_block_hash, height, round)?;
        if let Some(existing) = self.precommits.get(&precommit.validator_id) {
            if existing.block_hash != precommit.block_hash || existing.round != precommit.round {
                return Err("validator precommit equivocation detected".into());
            }
            return Ok(freedom_consensus::stake_quorum_reached(state, self.precommits.keys()));
        }
        self.precommits.insert(precommit.validator_id, precommit);
        Ok(freedom_consensus::stake_quorum_reached(state, self.precommits.keys()))
    }

    pub fn finalize_if_quorum(&self, state: &mut ChainState, block: &Block, round: u64) -> Result<Block, String> {
        let precommits: Vec<_> = self.precommits.values().cloned().collect();
        freedom_consensus::validate_precommit_quorum(state, block_hash(block), block.header.height, round, &precommits)?;
        execute_block(state, block)
    }
}


#[cfg(test)]
mod m22_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::ValidatorPrecommit;

    #[test]
    fn precommit_signing_domain_is_distinct() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let mut p = ValidatorPrecommit {
            block_hash: hash_bytes(b"block"),
            height: 10,
            round: 2,
            validator_id: hash_bytes(b"validator"),
            signature: Vec::new(),
        };
        p.signature = freedom_crypto::sign(&key, &p.signing_bytes()).to_vec();
        assert!(freedom_crypto::verify(&key.verifying_key().to_bytes(), &p.signing_bytes(), &p.signature.try_into().unwrap()));
    }
}
