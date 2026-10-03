use freedom_primitives::{domain_hash, hash_bytes, merkle_root, Hash};
use freedom_state::ChainState;
use freedom_types::{Block, Event, Receipt, Transaction};
#[derive(Clone, Debug)] pub struct ConsensusConfig { pub min_validators: usize }
impl Default for ConsensusConfig { fn default()->Self{Self{min_validators:3}}}
pub fn validate_genesis(state:&ChainState,config:&ConsensusConfig)->Result<(),String>{if state.validators.len()<config.min_validators{return Err(format!("M0 requires at least {} validators",config.min_validators));}if state.supply.total!=0{return Err("genesis supply must be zero".into())}Ok(())}
fn push_u32(o:&mut Vec<u8>,v:u32){o.extend_from_slice(&v.to_le_bytes())} fn push_u64(o:&mut Vec<u8>,v:u64){o.extend_from_slice(&v.to_le_bytes())} fn push_bytes(o:&mut Vec<u8>,b:&[u8]){push_u64(o,b.len() as u64);o.extend_from_slice(b)}
fn canonical_tx(o:&mut Vec<u8>,tx:&Transaction){push_u32(o,tx.chain_id);o.extend_from_slice(&tx.sender);push_u64(o,tx.nonce);o.push(tx.kind.code());push_bytes(o,&tx.payload);o.extend_from_slice(&tx.fee.to_le_bytes());push_bytes(o,&tx.signature)}
fn canonical_event(o:&mut Vec<u8>,e:&Event){match e{Event::FdmTransferred{from,to,amount}=>{o.push(1);o.extend_from_slice(from);o.extend_from_slice(to);o.extend_from_slice(&amount.to_le_bytes())},Event::TicketCreated{ticket_id,draw_id,owner}=>{o.push(2);o.extend_from_slice(ticket_id);o.extend_from_slice(draw_id);o.extend_from_slice(owner)},Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve}=>{o.push(3);o.extend_from_slice(draw_id);o.extend_from_slice(&prize_pool.to_le_bytes());o.extend_from_slice(&cu_fund.to_le_bytes());o.extend_from_slice(&annual_reserve.to_le_bytes())},Event::DrawExecuted{draw_id,randomness_seed,selection_proof}=>{o.push(4);o.extend_from_slice(draw_id);o.extend_from_slice(randomness_seed);o.extend_from_slice(selection_proof)},Event::PrizePaid{draw_id,ticket_id,winner,amount}=>{o.push(5);o.extend_from_slice(draw_id);o.extend_from_slice(ticket_id);o.extend_from_slice(winner);o.extend_from_slice(&amount.to_le_bytes())}}}
pub fn canonical_receipt_bytes(r:&Receipt)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-RECEIPT-V1");o.extend_from_slice(&r.tx_hash);o.push(r.success as u8);o.extend_from_slice(&r.gas_used.to_le_bytes());o.extend_from_slice(&(r.events.len() as u64).to_le_bytes());for e in &r.events{canonical_event(&mut o,e)}o}
pub fn tx_root(txs:&[Transaction])->Hash{let leaves:Vec<Hash>=txs.iter().map(|t|domain_hash(b"FREEDOM-TX-LEAF-V1",&[&t.hash()])).collect();merkle_root(&leaves)}
pub fn receipts_root(rs:&[Receipt])->Hash{let leaves:Vec<Hash>=rs.iter().map(|r|domain_hash(b"FREEDOM-RECEIPT-LEAF-V1",&[&hash_bytes(&canonical_receipt_bytes(r))])).collect();merkle_root(&leaves)}
pub fn canonical_block_bytes(block:&Block)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BLOCK-V1");let h=&block.header;push_u64(&mut o,h.height);o.extend_from_slice(&h.previous_hash);o.extend_from_slice(&h.state_root);o.extend_from_slice(&h.tx_root);o.extend_from_slice(&h.receipts_root);push_u64(&mut o,h.timestamp);push_u64(&mut o,h.epoch);o.extend_from_slice(&h.proposer);push_u64(&mut o,block.transactions.len() as u64);for tx in &block.transactions{canonical_tx(&mut o,tx)}push_u64(&mut o,block.receipts.len() as u64);for r in &block.receipts{push_bytes(&mut o,&canonical_receipt_bytes(r))}o}
pub fn block_hash(block:&Block)->Hash{domain_hash(b"FREEDOM-BLOCK-HASH-V1",&[&canonical_block_bytes(block)])}
pub fn proposer_for_height_round(state:&ChainState,height:u64,round:u64)->Option<freedom_primitives::Address>{if state.validators.is_empty(){return None}let mut ids:Vec<_>=state.validators.keys().copied().collect();ids.sort();let index=((height-1).saturating_add(round)) as usize % ids.len();let id=ids[index];state.validators.get(&id).map(|v|v.owner)}
pub fn proposer_for_height(state:&ChainState,height:u64)->Option<freedom_primitives::Address>{proposer_for_height_round(state,height,0)}

pub fn validate_block_commitments(state:&ChainState,block:&Block)->Result<(),String>{if block.header.height!=state.height+1{return Err("invalid block height".into())}if block.header.previous_hash!=state.last_block_hash{return Err("invalid previous block hash".into())}if block.transactions.len()!=block.receipts.len(){return Err("transaction/receipt count mismatch".into())}if tx_root(&block.transactions)!=block.header.tx_root{return Err("transaction root mismatch".into())}if receipts_root(&block.receipts)!=block.header.receipts_root{return Err("receipts root mismatch".into())}if state.state_root()!=block.header.state_root{return Err("state root mismatch".into())}Ok(())}
#[cfg(test)]mod tests{use super::*;use freedom_primitives::{merkle_proof,merkle_root,verify_merkle_proof};use freedom_types::{TxKind,Transaction,Receipt};#[test]fn merkle_proof_verifies(){let leaves=vec![hash_bytes(b"a"),hash_bytes(b"b"),hash_bytes(b"c")];let root=merkle_root(&leaves);let proof=merkle_proof(&leaves,2).unwrap();assert!(verify_merkle_proof(root,leaves[2],&proof));}#[test]fn roots_are_order_sensitive(){let a=Transaction{chain_id:1,sender:[1;32],nonce:0,kind:TxKind::Transfer,payload:vec![1],fee:0,signature:vec![2;64]};let b=Transaction{chain_id:1,sender:[1;32],nonce:1,kind:TxKind::Transfer,payload:vec![2],fee:0,signature:vec![3;64]};assert_ne!(tx_root(&[a.clone(),b.clone()]),tx_root(&[b,a]));let r1=Receipt{tx_hash:a.hash(),success:true,gas_used:1,events:vec![]};let r2=Receipt{tx_hash:b.hash(),success:true,gas_used:1,events:vec![]};assert_ne!(receipts_root(&[r1.clone(),r2.clone()]),receipts_root(&[r2,r1]));}}


use freedom_crypto::verify;

pub fn quorum_threshold(validator_count: usize) -> usize {
    if validator_count == 0 { return usize::MAX; }
    (validator_count * 2) / 3 + 1
}

pub fn validate_vote(state: &ChainState, vote: &freedom_types::ValidatorVote, expected_block_hash: Hash, expected_height: u64) -> Result<(), String> {
    if vote.block_hash != expected_block_hash { return Err("vote block hash mismatch".into()); }
    if vote.height != expected_height { return Err("vote height mismatch".into()); }
    let validator = state.validators.get(&vote.validator_id).ok_or_else(|| "unknown validator".to_string())?;
    if vote.signature.len() != 64 { return Err("invalid vote signature length".into()); }
    let mut sig = [0u8;64];
    sig.copy_from_slice(&vote.signature);
    if !verify(&validator.consensus_key, &vote.signing_bytes(), &sig) {
        return Err("invalid vote signature".into());
    }
    Ok(())
}

pub fn validate_quorum(state: &ChainState, block_hash: Hash, height: u64, votes: &[freedom_types::ValidatorVote]) -> Result<(), String> {
    let threshold = quorum_threshold(state.validators.len());
    let mut seen = std::collections::BTreeSet::new();
    for vote in votes {
        validate_vote(state, vote, block_hash, height)?;
        if !seen.insert(vote.validator_id) { return Err("duplicate validator vote".into()); }
    }
    if seen.len() < threshold { return Err(format!("quorum not reached: {}/{}", seen.len(), threshold)); }
    Ok(())
}


#[cfg(test)]
mod quorum_tests {
    use super::*;
    use freedom_crypto::sign;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator, ValidatorVote};
    
    fn state_with_validators() -> ChainState {
        let validators = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            Validator {
                validator_id: hash_bytes(&[b"validator", &[i + 1]].concat()),
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        ChainState::genesis(&Genesis { chain_id: 1, initial_supply: 0, validators })
    }

    fn vote(state: &ChainState, index: u8, block_hash: Hash, height: u64) -> ValidatorVote {
        let id = hash_bytes(&[b"validator", &[index + 1]].concat());
        let key = [index + 1; 32];
        let mut v = ValidatorVote { block_hash, height, validator_id: id, signature: Vec::new() };
        v.signature = sign(&ed25519_dalek::SigningKey::from_bytes(&key), &v.signing_bytes()).to_vec();
        v
    }

    #[test]
    fn two_of_three_votes_finalize_quorum() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let votes = vec![vote(&state, 0, hash, 1), vote(&state, 1, hash, 1)];
        assert_eq!(quorum_threshold(3), 3);
        assert!(validate_quorum(&state, hash, 1, &votes).is_err());
    }

    #[test]
    fn three_of_three_votes_reach_current_threshold() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let votes = vec![vote(&state, 0, hash, 1), vote(&state, 1, hash, 1), vote(&state, 2, hash, 1)];
        assert!(validate_quorum(&state, hash, 1, &votes).is_ok());
    }

    #[test]
    fn duplicate_vote_is_rejected() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let v = vote(&state, 0, hash, 1);
        assert!(validate_quorum(&state, hash, 1, &[v.clone(), v]).is_err());
    }
}

pub fn validate_timeout(state:&ChainState, timeout:&freedom_types::ValidatorTimeout, height:u64, round:u64)->Result<(),String>{
 if timeout.height!=height{return Err("timeout height mismatch".into())}
 if timeout.round!=round{return Err("timeout round mismatch".into())}
 let v=state.validators.get(&timeout.validator_id).ok_or_else(||"unknown validator".to_string())?;
 if timeout.signature.len()!=64{return Err("invalid timeout signature length".into())}
 let mut sig=[0u8;64];sig.copy_from_slice(&timeout.signature);
 if !freedom_crypto::verify(&v.consensus_key,&timeout.signing_bytes(),&sig){return Err("invalid timeout signature".into())}
 Ok(())
}
pub fn validate_timeout_quorum(state:&ChainState,height:u64,round:u64,timeouts:&[freedom_types::ValidatorTimeout])->Result<(),String>{
 let mut seen=std::collections::BTreeSet::new();
 for t in timeouts{validate_timeout(state,t,height,round)?;if !seen.insert(t.validator_id){return Err("duplicate timeout validator".into())}}
 if seen.len()<quorum_threshold(state.validators.len()){return Err("timeout quorum not reached".into())}
 Ok(())
}
