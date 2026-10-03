use freedom_primitives::{domain_hash, hash_bytes, Hash};
use freedom_state::ChainState;
use freedom_types::{Block, Event, Transaction, TxKind};

#[derive(Clone, Debug)]
pub struct ConsensusConfig { pub min_validators: usize }
impl Default for ConsensusConfig { fn default() -> Self { Self { min_validators: 3 } } }

pub fn validate_genesis(state: &ChainState, config: &ConsensusConfig) -> Result<(), String> {
    if state.validators.len() < config.min_validators {
        return Err(format!("M0 requires at least {} validators", config.min_validators));
    }
    if state.supply.total != 0 { return Err("genesis supply must be zero".into()); }
    Ok(())
}

fn push_u32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }
fn push_u64(out: &mut Vec<u8>, v: u64) { out.extend_from_slice(&v.to_le_bytes()); }
fn push_bytes(out: &mut Vec<u8>, b: &[u8]) { push_u64(out, b.len() as u64); out.extend_from_slice(b); }

fn canonical_tx(out: &mut Vec<u8>, tx: &Transaction) {
    push_u32(out, tx.chain_id); out.extend_from_slice(&tx.sender); push_u64(out, tx.nonce);
    out.push(tx.kind.code()); push_bytes(out, &tx.payload); out.extend_from_slice(&tx.fee.to_le_bytes()); push_bytes(out, &tx.signature);
}
fn canonical_event(out: &mut Vec<u8>, event: &Event) {
    match event {
        Event::FdmTransferred{from,to,amount} => { out.push(1); out.extend_from_slice(from); out.extend_from_slice(to); out.extend_from_slice(&amount.to_le_bytes()); }
        Event::TicketCreated{ticket_id,draw_id,owner} => { out.push(2); out.extend_from_slice(ticket_id); out.extend_from_slice(draw_id); out.extend_from_slice(owner); }
        Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve} => { out.push(3); out.extend_from_slice(draw_id); out.extend_from_slice(&prize_pool.to_le_bytes()); out.extend_from_slice(&cu_fund.to_le_bytes()); out.extend_from_slice(&annual_reserve.to_le_bytes()); }
        Event::DrawExecuted{draw_id,randomness_seed,selection_proof} => { out.push(4); out.extend_from_slice(draw_id); out.extend_from_slice(randomness_seed); out.extend_from_slice(selection_proof); }
        Event::PrizePaid{draw_id,ticket_id,winner,amount} => { out.push(5); out.extend_from_slice(draw_id); out.extend_from_slice(ticket_id); out.extend_from_slice(winner); out.extend_from_slice(&amount.to_le_bytes()); }
    }
}
pub fn canonical_block_bytes(block: &Block) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"FREEDOM-BLOCK-V1");
    let h=&block.header;
    push_u64(&mut out,h.height); out.extend_from_slice(&h.previous_hash); out.extend_from_slice(&h.state_root);
    out.extend_from_slice(&h.tx_root); out.extend_from_slice(&h.receipts_root); push_u64(&mut out,h.timestamp);
    push_u64(&mut out,h.epoch); out.extend_from_slice(&h.proposer);
    push_u64(&mut out,block.transactions.len() as u64);
    for tx in &block.transactions { canonical_tx(&mut out,tx); }
    out
}
pub fn block_hash(block: &Block) -> Hash { domain_hash(b"FREEDOM-BLOCK-HASH-V1", &[&canonical_block_bytes(block)]) }
pub fn validate_state_root(state: &ChainState, block: &Block) -> Result<(), String> {
    let actual = state.state_root();
    if actual != block.header.state_root { return Err(format!("state root mismatch: expected {}, got {}", hex::encode(block.header.state_root), hex::encode(actual))); }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use freedom_primitives::{merkle_proof, merkle_root, verify_merkle_proof};
    use freedom_types::{BlockHeader, Block, TxKind, Transaction};
    #[test] fn canonical_block_hash_is_stable() {
        let tx=Transaction{chain_id:1,sender:[1;32],nonce:0,kind:TxKind::Transfer,payload:vec![2,3],fee:7,signature:vec![4;64]};
        let b=Block{header:BlockHeader{height:1,previous_hash:[0;32],state_root:[1;32],tx_root:[2;32],receipts_root:[3;32],timestamp:4,epoch:5,proposer:[6;32]},transactions:vec![tx]};
        assert_eq!(block_hash(&b),block_hash(&b));
    }
    #[test] fn merkle_proof_verifies() {
        let leaves=vec![hash_bytes(b"a"),hash_bytes(b"b"),hash_bytes(b"c")];
        let root=merkle_root(&leaves);
        let proof=merkle_proof(&leaves,2).unwrap();
        assert!(verify_merkle_proof(root,leaves[2],&proof));
    }
}
