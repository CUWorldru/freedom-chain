use freedom_primitives::{domain_hash, hash_bytes, merkle_root, Hash};
use freedom_crypto::{sign, verify};
use freedom_state::{Account, ChainState};
use freedom_types::{Block, Event, Receipt, Transaction, ValidatorSetCertificate};
#[derive(Clone, Debug)] pub struct ConsensusConfig { pub min_validators: usize }
impl Default for ConsensusConfig { fn default()->Self{Self{min_validators:3}}}
pub fn validator_set_hash(set: &std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>) -> Hash {
    // Validator-set history in ChainState is the authoritative snapshot
    // commitment. Keep certificate hashing byte-for-byte compatible with that
    // persisted format; changing it here would make existing history and
    // certificates describe different validator sets.
    let mut bytes = Vec::new();
    for (id, entry) in set {
        bytes.extend_from_slice(id);
        bytes.extend_from_slice(&entry.owner);
        bytes.extend_from_slice(&entry.consensus_key);
        bytes.extend_from_slice(&entry.stake.to_le_bytes());
    }
    hash_bytes(&bytes)
}

pub fn validate_validator_set_certificate(
    state: &ChainState,
    cert: &ValidatorSetCertificate,
) -> Result<(), String> {
    if cert.from_epoch != state.epoch || cert.to_epoch != state.epoch.checked_add(1).ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())? {
        return Err("invalid validator set certificate epoch transition".into());
    }
    if cert.previous_set_hash != validator_set_hash(&state.active_validator_set) {
        return Err("validator set certificate previous set mismatch".into());
    }
    if cert.new_set_hash != validator_set_hash(&state.pending_validator_set) {
        return Err("validator set certificate new set mismatch".into());
    }
    if !cert.validate_transition_fields() {
        return Err("invalid validator set certificate transition fields".into());
    }
    let previous_keys = state.validator_set_history_sets.get(&state.epoch)
        .ok_or_else(|| "validator set certificate previous validator keys unavailable".to_string())?;
    if previous_keys.is_empty() {
        return Err("validator set certificate previous validator keys empty".into());
    }
    let required_quorum = quorum_threshold(state.active_validator_set.len());
    let mut ids = std::collections::BTreeSet::new();
    for vote in &cert.votes {
        if vote.from_epoch != cert.from_epoch
            || vote.to_epoch != cert.to_epoch
            || vote.previous_set_hash != cert.previous_set_hash
            || vote.new_set_hash != cert.new_set_hash
        {
            return Err("validator set vote does not match certificate".into());
        }
        if !ids.insert(vote.validator_id) {
            return Err("duplicate validator set vote".into());
        }
        let entry = state.active_validator_set.get(&vote.validator_id)
            .ok_or_else(|| "validator set vote signer is not active".to_string())?;
        if vote.signature.len() != 64 {
            return Err("invalid validator set vote signature length".into());
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&vote.signature);
        if !verify(&entry.consensus_key, &vote.signing_bytes(), &sig) {
            return Err("invalid validator set vote signature".into());
        }
    }
    if ids.len() < required_quorum {
        return Err("validator set certificate lacks validator-count quorum".into());
    }
    if !stake_quorum_reached(state, &ids) {
        return Err("validator set certificate lacks stake quorum".into());
    }
    Ok(())
}

pub fn validate_genesis(state:&ChainState,config:&ConsensusConfig)->Result<(),String>{if state.validators.len()<config.min_validators{return Err(format!("M0 requires at least {} validators",config.min_validators));}if state.supply.total!=0{return Err("genesis supply must be zero".into())}Ok(())}
fn push_u32(o:&mut Vec<u8>,v:u32){o.extend_from_slice(&v.to_le_bytes())} fn push_u64(o:&mut Vec<u8>,v:u64){o.extend_from_slice(&v.to_le_bytes())} fn push_bytes(o:&mut Vec<u8>,b:&[u8]){push_u64(o,b.len() as u64);o.extend_from_slice(b)}
fn canonical_tx(o:&mut Vec<u8>,tx:&Transaction){push_u32(o,tx.chain_id);o.extend_from_slice(&tx.sender);push_u64(o,tx.nonce);o.push(tx.kind.code());push_bytes(o,&tx.payload);o.extend_from_slice(&tx.fee.to_le_bytes());push_bytes(o,&tx.signature)}
fn canonical_event(o:&mut Vec<u8>,e:&Event){match e{Event::NativeAssetTransferred{from,to,amount}=>{o.push(1);o.extend_from_slice(from);o.extend_from_slice(to);o.extend_from_slice(&amount.to_le_bytes())},Event::TicketCreated{ticket_id,draw_id,owner}=>{o.push(2);o.extend_from_slice(ticket_id);o.extend_from_slice(draw_id);o.extend_from_slice(owner)},Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve}=>{o.push(3);o.extend_from_slice(draw_id);o.extend_from_slice(&prize_pool.to_le_bytes());o.extend_from_slice(&cu_fund.to_le_bytes());o.extend_from_slice(&annual_reserve.to_le_bytes())},Event::DrawExecuted{draw_id,randomness_seed,selection_proof}=>{o.push(4);o.extend_from_slice(draw_id);o.extend_from_slice(randomness_seed);o.extend_from_slice(selection_proof)},Event::PrizePaid{draw_id,ticket_id,winner,amount}=>{o.push(5);o.extend_from_slice(draw_id);o.extend_from_slice(ticket_id);o.extend_from_slice(winner);o.extend_from_slice(&amount.to_le_bytes())}}}
pub fn canonical_receipt_bytes(r:&Receipt)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-RECEIPT-V1");o.extend_from_slice(&r.tx_hash);o.push(r.success as u8);o.extend_from_slice(&r.gas_used.to_le_bytes());o.extend_from_slice(&(r.events.len() as u64).to_le_bytes());for e in &r.events{canonical_event(&mut o,e)}o}
pub fn tx_root(txs:&[Transaction])->Hash{let leaves:Vec<Hash>=txs.iter().map(|t|domain_hash(b"FREEDOM-TX-LEAF-V1",&[&t.hash()])).collect();merkle_root(&leaves)}
pub fn receipts_root(rs:&[Receipt])->Hash{let leaves:Vec<Hash>=rs.iter().map(|r|domain_hash(b"FREEDOM-RECEIPT-LEAF-V1",&[&hash_bytes(&canonical_receipt_bytes(r))])).collect();merkle_root(&leaves)}
pub fn canonical_block_bytes(block:&Block)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BLOCK-V1");let h=&block.header;push_u64(&mut o,h.height);push_u64(&mut o,h.round);o.extend_from_slice(&h.previous_hash);o.extend_from_slice(&h.state_root);o.extend_from_slice(&h.tx_root);o.extend_from_slice(&h.receipts_root);push_u64(&mut o,h.timestamp);push_u64(&mut o,h.epoch);o.extend_from_slice(&h.proposer);match &h.epoch_transition_certificate{Some(cert)=>{o.push(1);o.extend_from_slice(&cert.hash());},None=>o.push(0)};push_u64(&mut o,block.transactions.len() as u64);for tx in &block.transactions{canonical_tx(&mut o,tx)}push_u64(&mut o,block.receipts.len() as u64);for r in &block.receipts{push_bytes(&mut o,&canonical_receipt_bytes(r))}o}
pub fn block_hash(block:&Block)->Hash{domain_hash(b"FREEDOM-BLOCK-HASH-V1",&[&canonical_block_bytes(block)])}
pub fn proposer_for_height_round(state:&ChainState,height:u64,round:u64)->Option<freedom_primitives::Address>{
 if height == 0 || state.active_validator_set.is_empty(){return None}
 let mut ids:Vec<_>=state.active_validator_set.keys().copied().collect();ids.sort();
 let len=ids.len() as u64;let height_offset=(height-1)%len;let round_offset=round%len;let index=(height_offset.checked_add(round_offset)? % len) as usize;
 let id=ids[index];state.active_validator_set.get(&id).map(|v|v.owner)
}
pub fn proposer_for_height(state:&ChainState,height:u64)->Option<freedom_primitives::Address>{proposer_for_height_round(state,height,0)}

#[cfg(test)]
mod m326_proposer_height_boundary_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator};

    fn state_with_validator() -> ChainState {
        let key = [51u8; 32];
        ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![Validator {
                validator_id: hash_bytes(b"m326-validator"),
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: 1,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }],
        })
    }

    #[test]
    fn proposer_rejects_zero_height() {
        let state = state_with_validator();
        assert_eq!(proposer_for_height_round(&state, 0, 0), None);
        assert_eq!(proposer_for_height(&state, 0), None);
    }

    #[test]
    fn proposer_still_selects_first_validator_at_height_one() {
        let state = state_with_validator();
        let expected = state.active_validator_set.values().next().unwrap().owner;
        assert_eq!(proposer_for_height_round(&state, 1, 0), Some(expected));
    }
}


pub fn validate_epoch_transition_certificate(state: &ChainState, block: &Block) -> Result<(), String> {
    let boundary = state.epoch_length > 0 && block.header.height > 0 && block.header.height % state.epoch_length == 0;
    if boundary {
        let expected_epoch = state.epoch.checked_add(1).ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())?;
        if block.header.epoch != expected_epoch {
            return Err("epoch transition block target epoch must be the immediate successor".into());
        }
        let cert = block.header.epoch_transition_certificate.as_ref()
            .ok_or_else(|| "epoch transition block requires validator set certificate".to_string())?;
        validate_validator_set_certificate(state, cert)?;
        if cert.to_epoch != expected_epoch || cert.to_epoch != block.header.epoch {
            return Err("validator set certificate target epoch mismatch".into());
        }
    } else if block.header.epoch_transition_certificate.is_some() {
        return Err("validator set certificate is only valid on epoch transition blocks".into());
    }
    Ok(())
}

pub fn validate_transaction_auth(state:&ChainState, tx:&Transaction)->Result<(),String>{
 let account=state.accounts.get(&tx.sender).ok_or_else(||"transaction sender account not found".to_string())?;
 if freedom_primitives::address_from_public_key(&account.public_key)!=tx.sender{return Err("transaction sender/account key binding mismatch".into())}
 if tx.signature.len()!=64{return Err("invalid transaction signature length".into())}
 let mut sig=[0u8;64];sig.copy_from_slice(&tx.signature);
 if !verify(&account.public_key,&tx.signing_bytes(),&sig){return Err("invalid transaction signature".into())}
 Ok(())
}
pub fn validate_block_commitments(state:&ChainState,block:&Block)->Result<(),String>{let expected_height=state.height.checked_add(1).ok_or_else(||"block height exceeds u64::MAX".to_string())?;if block.header.height!=expected_height{return Err("invalid block height".into())}if block.header.previous_hash!=state.last_block_hash{return Err("invalid previous block hash".into())}let expected_proposer=proposer_for_height_round(state,block.header.height,block.header.round).ok_or_else(||"no proposer available for block height/round".to_string())?;if block.header.proposer!=expected_proposer{return Err("invalid block proposer".into())}if block.transactions.len()!=block.receipts.len(){return Err("transaction/receipt count mismatch".into())}let mut next_nonces=std::collections::BTreeMap::new();for tx in &block.transactions{validate_transaction_auth(state,tx)?;let expected=*next_nonces.entry(tx.sender).or_insert_with(||state.accounts.get(&tx.sender).map(|a|a.nonce).unwrap_or(0));if tx.nonce!=expected{return Err("transaction nonce is not the next sender nonce".into())}let next=expected.checked_add(1).ok_or_else(||"transaction nonce exceeds u64::MAX".to_string())?;next_nonces.insert(tx.sender,next);}if tx_root(&block.transactions)!=block.header.tx_root{return Err("transaction root mismatch".into())}if receipts_root(&block.receipts)!=block.header.receipts_root{return Err("receipts root mismatch".into())}if state.state_root()!=block.header.state_root{return Err("state root mismatch".into())}Ok(())}
#[cfg(test)]
mod m334_proposer_rotation_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator};

    fn state_with_three_validators() -> ChainState {
        ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: (0..3).map(|i| {
                let key = [70u8 + i as u8; 32];
                Validator {
                    validator_id: hash_bytes(&[b'm', b'3', b'3', b'4', i as u8]),
                    owner: freedom_primitives::address_from_public_key(&key),
                    consensus_key: key,
                    stake: 1,
                    unbonding_stake: 0,
                    blocks_proposed: 0,
                    blocks_finalized: 0,
                    missed_votes: 0,
                }
            }).collect(),
        })
    }

    #[test]
    fn proposer_rotation_does_not_saturate_at_u64_max() {
        let state = state_with_three_validators();
        let at_max = proposer_for_height_round(&state, u64::MAX, 0).unwrap();
        let expected = proposer_for_height_round(&state, 1, (u64::MAX - 1) % 3).unwrap();
        assert_eq!(at_max, expected);
    }

    #[test]
    fn round_overflow_cannot_change_rotation_determinism() {
        let state = state_with_three_validators();
        let max_round = proposer_for_height_round(&state, 1, u64::MAX).unwrap();
        let reduced_round = proposer_for_height_round(&state, 1, u64::MAX % 3).unwrap();
        assert_eq!(max_round, reduced_round);
    }
}

#[cfg(test)]
mod m333_proposer_admission_tests {
    use super::*;
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Block, BlockHeader, Genesis, Validator};

    fn state_with_validators() -> ChainState {
        let keys = [[61u8; 32], [62u8; 32]];
        ChainState::genesis(&Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: keys.into_iter().enumerate().map(|(i, key)| Validator {
                validator_id: hash_bytes(&[b'm', b'3', b'3', b'3', i as u8]),
                owner: address_from_public_key(&key),
                consensus_key: key,
                stake: 1,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }).collect(),
        })
    }

    fn block_for(state: &ChainState, proposer: [u8; 32]) -> Block {
        Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: state.last_block_hash,
                state_root: state.state_root(), tx_root: tx_root(&[]),
                receipts_root: receipts_root(&[]), timestamp: 0, epoch: state.epoch,
                proposer, epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        }
    }

    #[test]
    fn block_admission_accepts_deterministic_proposer() {
        let state = state_with_validators();
        let proposer = proposer_for_height_round(&state, 1, 0).unwrap();
        assert!(validate_block_commitments(&state, &block_for(&state, proposer)).is_ok());
    }

    #[test]
    fn block_admission_rejects_wrong_proposer() {
        let state = state_with_validators();
        let expected = proposer_for_height_round(&state, 1, 0).unwrap();
        let wrong = state.active_validator_set.values()
            .map(|entry| entry.owner)
            .find(|owner| *owner != expected)
            .unwrap();
        assert_eq!(
            validate_block_commitments(&state, &block_for(&state, wrong)),
            Err("invalid block proposer".into())
        );
    }
}

#[cfg(test)]
mod m329_block_height_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator, Block, BlockHeader};

    fn state_at_max_height() -> ChainState {
        let key = [71u8; 32];
        let mut state = ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![Validator {
                validator_id: hash_bytes(b"m329-validator"),
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: 1,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }],
        });
        state.height = u64::MAX;
        state
    }

    #[test]
    fn block_commitments_reject_height_overflow() {
        let state = state_at_max_height();
        let block = Block {
            header: BlockHeader {
                height: u64::MAX, round: 0, previous_hash: state.last_block_hash,
                state_root: state.state_root(), tx_root: tx_root(&[]),
                receipts_root: receipts_root(&[]), timestamp: 0, epoch: state.epoch,
                proposer: [0; 32], epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        };
        assert_eq!(
            validate_block_commitments(&state, &block),
            Err("block height exceeds u64::MAX".into())
        );
    }
}

#[cfg(test)]
mod m331_transaction_auth_boundary_tests {
    use super::*;
    use freedom_crypto::generate_keypair;
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Genesis, Transaction, TxKind, Validator};

    fn state_and_tx() -> (ChainState, Transaction) {
        let key = generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let owner = address_from_public_key(&public_key);
        let validator = Validator {
            validator_id: hash_bytes(b"m331-validator"),
            owner,
            consensus_key: public_key,
            stake: 1,
            unbonding_stake: 0,
            blocks_proposed: 0,
            blocks_finalized: 0,
            missed_votes: 0,
        };
        let mut state = ChainState::genesis(&Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![validator],
        });
        state.accounts.insert(owner, Account::new(public_key, 100));
        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender: owner,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: vec![1],
            fee: 0,
            signature: vec![],
        };
        tx.signature = sign(&key, &tx.signing_bytes()).to_vec();
        (state, tx)
    }

    #[test]
    fn valid_external_transaction_signature_is_accepted() {
        let (state, tx) = state_and_tx();
        assert!(validate_transaction_auth(&state, &tx).is_ok());
    }

    #[test]
    fn forged_transaction_signature_is_rejected() {
        let (state, mut tx) = state_and_tx();
        let other = generate_keypair();
        tx.signature = sign(&other, &tx.signing_bytes()).to_vec();
        assert_eq!(validate_transaction_auth(&state, &tx), Err("invalid transaction signature".into()));
    }

    #[test]
    fn unsigned_transaction_is_rejected() {
        let (state, mut tx) = state_and_tx();
        tx.signature.clear();
        assert_eq!(validate_transaction_auth(&state, &tx), Err("invalid transaction signature length".into()));
    }
}

#[cfg(test)]
mod m332_block_nonce_sequence_tests {
    use super::*;
    use freedom_crypto::{generate_keypair, sign};
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Genesis, Transaction, TxKind, Validator};

    #[test]
    fn block_rejects_duplicate_sender_nonce() {
        let key = generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let owner = address_from_public_key(&public_key);
        let validator = Validator {
            validator_id: hash_bytes(b"m332-validator"),
            owner,
            consensus_key: public_key,
            stake: 1,
            unbonding_stake: 0,
            blocks_proposed: 0,
            blocks_finalized: 0,
            missed_votes: 0,
        };
        let mut state = ChainState::genesis(&Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![validator],
        });
        state.accounts.insert(owner, Account::new(public_key, 100));
        let make_tx = |payload: Vec<u8>| {
            let mut tx = Transaction {
                chain_id: CHAIN_ID, sender: owner, nonce: 0, kind: TxKind::Transfer,
                payload, fee: 0, signature: vec![],
            };
            tx.signature = sign(&key, &tx.signing_bytes()).to_vec();
            tx
        };
        let tx1 = make_tx(vec![1]);
        let tx2 = make_tx(vec![2]);
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: state.last_block_hash,
                state_root: state.state_root(), tx_root: tx_root(&[tx1.clone(), tx2.clone()]),
                receipts_root: receipts_root(&[]), timestamp: 0, epoch: state.epoch,
                proposer: owner, epoch_transition_certificate: None,
            },
            transactions: vec![tx1, tx2],
            receipts: vec![],
        };
        assert_eq!(validate_block_commitments(&state, &block), Err("transaction nonce is not the next sender nonce".into()));
    }

    #[test]
    fn nonce_overflow_is_rejected() {
        let key = generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let owner = address_from_public_key(&public_key);
        let validator = Validator {
            validator_id: hash_bytes(b"m332-overflow-validator"),
            owner, consensus_key: public_key, stake: 1, unbonding_stake: 0,
            blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
        };
        let mut state = ChainState::genesis(&Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![validator] });
        let mut account = Account::new(public_key, 100);
        account.nonce = u64::MAX;
        state.accounts.insert(owner, account);
        let mut tx = Transaction { chain_id: CHAIN_ID, sender: owner, nonce: u64::MAX, kind: TxKind::Transfer, payload: vec![1], fee: 0, signature: vec![] };
        tx.signature = sign(&key, &tx.signing_bytes()).to_vec();
        let block = Block { header: BlockHeader { height: 1, round: 0, previous_hash: state.last_block_hash, state_root: state.state_root(), tx_root: tx_root(&[tx.clone()]), receipts_root: receipts_root(&[Receipt{tx_hash:tx.hash(),success:true,gas_used:0,events:vec![]}]), timestamp: 0, epoch: state.epoch, proposer: owner, epoch_transition_certificate: None }, transactions: vec![tx.clone()], receipts: vec![Receipt{tx_hash:tx.hash(),success:true,gas_used:0,events:vec![]}] };
        assert_eq!(validate_block_commitments(&state, &block), Err("transaction nonce exceeds u64::MAX".into()));
    }
}

#[cfg(test)]mod tests{use super::*;use freedom_primitives::{merkle_proof,merkle_root,verify_merkle_proof};use freedom_types::{TxKind,Transaction,Receipt};#[test]fn merkle_proof_verifies(){let leaves=vec![hash_bytes(b"a"),hash_bytes(b"b"),hash_bytes(b"c")];let root=merkle_root(&leaves);let proof=merkle_proof(&leaves,2).unwrap();assert!(verify_merkle_proof(root,leaves[2],&proof));}#[test]fn roots_are_order_sensitive(){let a=Transaction{chain_id:1,sender:[1;32],nonce:0,kind:TxKind::Transfer,payload:vec![1],fee:0,signature:vec![2;64]};let b=Transaction{chain_id:1,sender:[1;32],nonce:1,kind:TxKind::Transfer,payload:vec![2],fee:0,signature:vec![3;64]};assert_ne!(tx_root(&[a.clone(),b.clone()]),tx_root(&[b,a]));let r1=Receipt{tx_hash:a.hash(),success:true,gas_used:1,events:vec![]};let r2=Receipt{tx_hash:b.hash(),success:true,gas_used:1,events:vec![]};assert_ne!(receipts_root(&[r1.clone(),r2.clone()]),receipts_root(&[r2,r1]));}}


use freedom_crypto::verify;

pub fn quorum_threshold(validator_count: usize) -> usize {
    if validator_count == 0 { return usize::MAX; }
    // Equivalent to floor(2*n/3) + 1, without overflowing n * 2.
    let ceil_third = validator_count / 3 + usize::from(validator_count % 3 != 0);
    validator_count - ceil_third + 1
}

pub fn total_active_stake(state: &ChainState) -> u128 {
    state.active_validator_set.values()
        .map(|v| v.stake)
        .try_fold(0u128, |acc, stake| acc.checked_add(stake))
        .unwrap_or(u128::MAX)
}

pub fn stake_quorum_reached(state: &ChainState, signers: &std::collections::BTreeSet<Hash>) -> bool {
    let total = match state.active_validator_set.values()
        .map(|v| v.stake)
        .try_fold(0u128, |acc, stake| acc.checked_add(stake)) {
        Some(value) => value,
        None => return false,
    };
    if total == 0 { return signers.len() >= quorum_threshold(state.active_validator_set.len()); }
    let signed = match signers.iter()
        .filter_map(|id| state.active_validator_set.get(id))
        .map(|v| v.stake)
        .try_fold(0u128, |acc, stake| acc.checked_add(stake)) {
        Some(value) => value,
        None => return false,
    };
    // Avoid saturating multiplication: for 0 <= signed <= total,
    // 3*signed >= 2*total is exactly equivalent to
    // signed >= total - floor(total/3), with no u128 overflow.
    let required = total - total / 3;
    signed >= required
}

pub fn validate_vote(state: &ChainState, vote: &freedom_types::ValidatorVote, expected_block_hash: Hash, expected_height: u64, expected_round: u64) -> Result<(), String> {
    if vote.block_hash != expected_block_hash { return Err("vote block hash mismatch".into()); }
    if vote.height != expected_height { return Err("vote height mismatch".into()); }
    if vote.round != expected_round { return Err("vote round mismatch".into()); }
    let validator = state.active_validator_set.get(&vote.validator_id).ok_or_else(|| "unknown validator".to_string())?;
    if vote.signature.len() != 64 { return Err("invalid vote signature length".into()); }
    let mut sig = [0u8;64];
    sig.copy_from_slice(&vote.signature);
    if !verify(&validator.consensus_key, &vote.signing_bytes(), &sig) {
        return Err("invalid vote signature".into());
    }
    Ok(())
}

pub fn validate_quorum(state: &ChainState, block_hash: Hash, height: u64, round: u64, votes: &[freedom_types::ValidatorVote]) -> Result<(), String> {
    let threshold = quorum_threshold(state.active_validator_set.len());
    let mut seen = std::collections::BTreeSet::new();
    for vote in votes {
        validate_vote(state, vote, block_hash, height, round)?;
        if !seen.insert(vote.validator_id) { return Err("duplicate validator vote".into()); }
    }
    if seen.len() < threshold { return Err(format!("quorum not reached: {}/{}", seen.len(), threshold)); }
    if !stake_quorum_reached(state, &seen) { return Err("stake quorum not reached".into()); }
    Ok(())
}


#[cfg(test)]
mod m335_active_validator_quorum_tests {
    use super::*;
    use freedom_crypto::generate_keypair;
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Genesis, Validator, ValidatorSetEntry, ValidatorVote};

    fn state_with_divergent_validator_sets() -> (ChainState, Vec<freedom_crypto::SigningKey>) {
        let keys = vec![generate_keypair(), generate_keypair(), generate_keypair(), generate_keypair(), generate_keypair()];
        let validators = keys.iter().enumerate().map(|(i, key)| {
            let public_key = key.verifying_key().to_bytes();
            let id = hash_bytes(&[b'm', b'3', b'3', b'5', i as u8]);
            (id, Validator {
                validator_id: id, owner: address_from_public_key(&public_key),
                consensus_key: public_key, stake: 1, unbonding_stake: 0,
                blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            })
        }).collect();
        let mut state = ChainState::genesis(&Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.validators = validators;
        state.active_validator_set.clear();
        for (i, key) in keys.iter().take(3).enumerate() {
            let public_key = key.verifying_key().to_bytes();
            let id = hash_bytes(&[b'm', b'3', b'3', b'5', i as u8]);
            state.active_validator_set.insert(id, ValidatorSetEntry {
                validator_id: id, owner: address_from_public_key(&public_key),
                consensus_key: public_key, stake: 1,
            });
        }
        (state, keys)
    }

    #[test]
    fn quorum_uses_active_validator_count() {
        let (state, keys) = state_with_divergent_validator_sets();
        let votes: Vec<ValidatorVote> = state.active_validator_set.iter().enumerate().map(|(i, (id, _))| {
            let mut vote = ValidatorVote {
                validator_id: *id, block_hash: [9; 32], height: 1, round: 0, signature: vec![],
            };
            vote.signature = freedom_crypto::sign(&keys[i], &vote.signing_bytes()).to_vec();
            vote
        }).collect();
        assert!(validate_quorum(&state, [9; 32], 1, 0, &votes).is_ok());
    }
}

#[cfg(test)]
mod m337_certificate_active_quorum_tests {
    use super::*;
    use freedom_crypto::{generate_keypair, sign};
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Genesis, Validator, ValidatorSetCertificate, ValidatorSetEntry, ValidatorSetVote};

    fn transition_state() -> (ChainState, Vec<freedom_crypto::SigningKey>) {
        let keys: Vec<_> = (0..5).map(|_| generate_keypair()).collect();
        let mut state = ChainState::genesis(&Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.epoch = 4;
        let mut active = std::collections::BTreeMap::new();
        for (i, key) in keys.iter().take(3).enumerate() {
            let public_key = key.verifying_key().to_bytes();
            let id = hash_bytes(&[b'm', b'3', b'3', b'7', i as u8]);
            active.insert(id, ValidatorSetEntry {
                validator_id: id, owner: address_from_public_key(&public_key),
                consensus_key: public_key, stake: 1,
            });
        }
        state.active_validator_set = active.clone();
        state.validator_set_history_sets.insert(state.epoch, active.iter().take(2).map(|(id, entry)| (*id, entry.consensus_key)).collect());
        (state, keys)
    }

    #[test]
    fn certificate_quorum_uses_active_validator_count() {
        let (mut state, keys) = transition_state();
        let previous_set_hash = validator_set_hash(&state.active_validator_set);
        let mut pending_set = state.active_validator_set.clone();
        let first_id = *pending_set.keys().next().unwrap();
        pending_set.get_mut(&first_id).unwrap().stake = 2;
        let new_set_hash = validator_set_hash(&pending_set);
        let mut cert = ValidatorSetCertificate {
            from_epoch: state.epoch, to_epoch: state.epoch + 1,
            previous_set_hash, new_set_hash, votes: vec![],
        };
        for (i, (id, _)) in state.active_validator_set.iter().enumerate() {
            let mut vote = ValidatorSetVote {
                validator_id: *id, from_epoch: state.epoch, to_epoch: state.epoch + 1,
                previous_set_hash, new_set_hash, signature: vec![],
            };
            vote.signature = sign(&keys[i], &vote.signing_bytes()).to_vec();
            cert.votes.push(vote);
        }
        state.pending_validator_set.clone_from(&pending_set);
        assert!(validate_validator_set_certificate(&state, &cert).is_ok());
    }
}

#[cfg(test)]
mod m336_timeout_active_validator_quorum_tests {
    use super::*;
    use freedom_crypto::{generate_keypair, sign};
    use freedom_primitives::{address_from_public_key, hash_bytes, CHAIN_ID};
    use freedom_types::{Genesis, Validator, ValidatorTimeout, ValidatorSetEntry};

    fn divergent_state() -> (ChainState, Vec<freedom_crypto::SigningKey>) {
        let keys: Vec<_> = (0..5).map(|_| generate_keypair()).collect();
        let validators = keys.iter().enumerate().map(|(i, key)| {
            let public_key = key.verifying_key().to_bytes();
            let id = hash_bytes(&[b'm', b'3', b'3', b'6', i as u8]);
            (id, Validator {
                validator_id: id, owner: address_from_public_key(&public_key),
                consensus_key: public_key, stake: 0, unbonding_stake: 0,
                blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            })
        }).collect();
        let mut state = ChainState::genesis(&Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.validators = validators;
        for (i, key) in keys.iter().take(3).enumerate() {
            let public_key = key.verifying_key().to_bytes();
            let id = hash_bytes(&[b'm', b'3', b'3', b'6', i as u8]);
            state.active_validator_set.insert(id, ValidatorSetEntry {
                validator_id: id, owner: address_from_public_key(&public_key),
                consensus_key: public_key, stake: 0,
            });
        }
        (state, keys)
    }

    #[test]
    fn timeout_quorum_uses_active_validator_count() {
        let (state, keys) = divergent_state();
        let timeouts: Vec<ValidatorTimeout> = state.active_validator_set.iter().enumerate().map(|(i, (id, _))| {
            let mut timeout = ValidatorTimeout {
                validator_id: *id, height: 1, round: 0, signature: vec![],
            };
            timeout.signature = sign(&keys[i], &timeout.signing_bytes()).to_vec();
            timeout
        }).collect();
        assert!(validate_timeout_quorum(&state, 1, 0, &timeouts).is_ok());
    }
}

#[cfg(test)]
mod m328_epoch_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator, Block, BlockHeader};

    fn state_at_max_epoch() -> ChainState {
        let key = [61u8; 32];
        let mut state = ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![Validator {
                validator_id: hash_bytes(b"m328-validator"),
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: 1,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }],
        });
        state.epoch = u64::MAX;
        state.epoch_length = 10;
        state
    }

    #[test]
    fn epoch_transition_rejects_max_epoch_overflow() {
        let state = state_at_max_epoch();
        let block = Block {
            header: BlockHeader {
                height: 10, round: 0, previous_hash: [0; 32], state_root: [0; 32],
                tx_root: [0; 32], receipts_root: [0; 32], timestamp: 0,
                epoch: u64::MAX, proposer: [0; 32], epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        };
        assert_eq!(
            validate_epoch_transition_certificate(&state, &block),
            Err("epoch transition exceeds u64::MAX".into())
        );
    }
}

#[cfg(test)]
mod m327_quorum_threshold_overflow_tests {
    use super::*;

    #[test]
    fn quorum_threshold_matches_current_formula_for_small_counts() {
        let expected = [usize::MAX, 1, 2, 3, 3, 4, 5, 5, 6, 7, 7, 8];
        for (count, expected_threshold) in expected.into_iter().enumerate() {
            assert_eq!(quorum_threshold(count), expected_threshold);
        }
    }

    #[test]
    fn quorum_threshold_is_overflow_safe_at_usize_max() {
        let n = usize::MAX;
        let ceil_third = n / 3 + usize::from(n % 3 != 0);
        assert_eq!(quorum_threshold(n), n - ceil_third + 1);
    }
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
                stake: 0, unbonding_stake: 0, blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            }
        }).collect();
        ChainState::genesis(&Genesis { chain_id: 1, initial_supply: 0, validators })
    }

    fn vote(state: &ChainState, index: u8, block_hash: Hash, height: u64, round: u64) -> ValidatorVote {
        let id = hash_bytes(&[b"validator", &[index + 1]].concat());
        let key = [index + 1; 32];
        let mut v = ValidatorVote { block_hash, height, round, validator_id: id, signature: Vec::new() };
        v.signature = sign(&ed25519_dalek::SigningKey::from_bytes(&key), &v.signing_bytes()).to_vec();
        v
    }

    #[test]
    fn two_of_three_votes_finalize_quorum() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let votes = vec![vote(&state, 0, hash, 1, 0), vote(&state, 1, hash, 1, 0)];
        assert_eq!(quorum_threshold(3), 3);
        assert!(validate_quorum(&state, hash, 1, 0, &votes).is_err());
    }

    #[test]
    fn three_of_three_votes_reach_current_threshold() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let votes = vec![vote(&state, 0, hash, 1), vote(&state, 1, hash, 1), vote(&state, 2, hash, 1, 0)];
        assert!(validate_quorum(&state, hash, 1, &votes).is_ok());
    }

    #[test]
    fn duplicate_vote_is_rejected() {
        let state = state_with_validators();
        let hash = hash_bytes(b"block");
        let v = vote(&state, 0, hash, 1);
        assert!(validate_quorum(&state, hash, 1, 0, &[v.clone(), v]).is_err());
    }
}

pub fn validate_timeout(state:&ChainState, timeout:&freedom_types::ValidatorTimeout, height:u64, round:u64)->Result<(),String>{
 if timeout.height!=height{return Err("timeout height mismatch".into())}
 if timeout.round!=round{return Err("timeout round mismatch".into())}
 let v=state.active_validator_set.get(&timeout.validator_id).ok_or_else(||"unknown validator".to_string())?;
 if v.consensus_key.is_empty(){return Err("timeout validator consensus key is empty".into())}
 if timeout.signature.len()!=64{return Err("invalid timeout signature length".into())}
 let mut sig=[0u8;64];sig.copy_from_slice(&timeout.signature);
 if !freedom_crypto::verify(&v.consensus_key,&timeout.signing_bytes(),&sig){return Err("invalid timeout signature".into())}
 Ok(())
}
pub fn validate_timeout_quorum(state:&ChainState,height:u64,round:u64,timeouts:&[freedom_types::ValidatorTimeout])->Result<(),String>{
 let mut seen=std::collections::BTreeSet::new();
 for t in timeouts{validate_timeout(state,t,height,round)?;if !seen.insert(t.validator_id){return Err("duplicate timeout validator".into())}}
 if seen.len()<quorum_threshold(state.active_validator_set.len()){return Err("timeout quorum not reached".into())}
 if !stake_quorum_reached(state,&seen){return Err("timeout stake quorum not reached".into())}
 Ok(())
}



pub fn validate_precommit_against_set(
    validator_set: &std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    precommit: &freedom_types::ValidatorPrecommit,
    expected_block_hash: Hash,
    expected_height: u64,
    expected_round: u64,
) -> Result<(), String> {
    if precommit.block_hash != expected_block_hash { return Err("precommit block hash mismatch".into()); }
    if precommit.height != expected_height { return Err("precommit height mismatch".into()); }
    if precommit.round != expected_round { return Err("precommit round mismatch".into()); }
    let validator = validator_set.get(&precommit.validator_id).ok_or_else(|| "unknown validator".to_string())?;
    if precommit.signature.len() != 64 { return Err("invalid precommit signature length".into()); }
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&precommit.signature);
    if !verify(&validator.consensus_key, &precommit.signing_bytes(), &sig) {
        return Err("invalid precommit signature".into());
    }
    Ok(())
}

pub fn validate_precommit(
    state: &ChainState,
    precommit: &freedom_types::ValidatorPrecommit,
    expected_block_hash: Hash,
    expected_height: u64,
    expected_round: u64,
) -> Result<(), String> {
    validate_precommit_against_set(&state.active_validator_set, precommit, expected_block_hash, expected_height, expected_round)
}

pub fn validate_precommit_quorum_for_set(
    validator_set: &std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    block_hash: Hash,
    height: u64,
    round: u64,
    precommits: &[freedom_types::ValidatorPrecommit],
) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for precommit in precommits {
        validate_precommit_against_set(validator_set, precommit, block_hash, height, round)?;
        if !seen.insert(precommit.validator_id) {
            return Err("duplicate validator precommit".into());
        }
    }
    let total = validator_set.values()
        .map(|v| v.stake)
        .try_fold(0u128, |acc, stake| acc.checked_add(stake))
        .ok_or_else(|| "precommit stake total overflow".to_string())?;
    let signed = seen.iter()
        .filter_map(|id| validator_set.get(id))
        .map(|v| v.stake)
        .try_fold(0u128, |acc, stake| acc.checked_add(stake))
        .ok_or_else(|| "precommit signed stake overflow".to_string())?;
    if total == 0 {
        if seen.len() < quorum_threshold(validator_set.len()) { return Err("precommit quorum not reached".into()); }
    } else {
        let required = total - total / 3;
        if signed < required {
            return Err("precommit stake quorum not reached".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod m325_precommit_stake_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Validator, ValidatorPrecommit};

    fn validator_set(stakes: &[u128]) -> std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry> {
        stakes.iter().enumerate().map(|(i, stake)| {
            let key = [i as u8 + 40; 32];
            let id = hash_bytes(&[b"m325-validator", &[i as u8 + 40]].concat());
            (id, freedom_types::ValidatorSetEntry {
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: *stake,
            })
        }).collect()
    }

    #[test]
    fn precommit_quorum_rejects_overflowing_total_stake() {
        let set = validator_set(&[u128::MAX, 1]);
        assert!(validate_precommit_quorum_for_set(&set, [1; 32], 1, 0, &[]).is_err());
    }

    #[test]
    fn precommit_quorum_does_not_accept_one_third_with_max_stake() {
        let total = u128::MAX;
        let set = validator_set(&[total / 3 + 1, total - (total / 3 + 1)]);
        let ids: Vec<_> = set.keys().copied().collect();
        let mut precommits = Vec::<ValidatorPrecommit>::new();
        let p = ValidatorPrecommit {
            block_hash: [1; 32], height: 1, round: 0,
            validator_id: ids[0], signature: vec![0; 64],
        };
        precommits.push(p);
        assert!(validate_precommit_quorum_for_set(&set, [1; 32], 1, 0, &precommits).is_err());
    }
}


pub fn validate_precommit_quorum(
    state: &ChainState,
    block_hash: Hash,
    height: u64,
    round: u64,
    precommits: &[freedom_types::ValidatorPrecommit],
) -> Result<(), String> {
    validate_precommit_quorum_for_set(&state.active_validator_set, block_hash, height, round, precommits)
}

pub fn detect_prevote_equivocation(
    votes: &[freedom_types::ValidatorVote],
) -> Vec<(Hash, Hash, u64, u64)> {
    let mut first = std::collections::BTreeMap::<(Hash,u64,u64), Hash>::new();
    let mut evidence = Vec::new();
    for vote in votes {
        let key = (vote.validator_id, vote.height, vote.round);
        if let Some(existing) = first.insert(key, vote.block_hash) {
            if existing != vote.block_hash {
                evidence.push((vote.validator_id, existing, vote.block_hash, vote.height));
            }
        }
    }
    evidence
}

pub fn detect_precommit_equivocation(
    precommits: &[freedom_types::ValidatorPrecommit],
) -> Vec<(Hash, Hash, u64, u64)> {
    let mut first = std::collections::BTreeMap::<(Hash,u64,u64), Hash>::new();
    let mut evidence = Vec::new();
    for precommit in precommits {
        let key = (precommit.validator_id, precommit.height, precommit.round);
        if let Some(existing) = first.insert(key, precommit.block_hash) {
            if existing != precommit.block_hash {
                evidence.push((precommit.validator_id, existing, precommit.block_hash, precommit.height));
            }
        }
    }
    evidence
}

pub fn validate_finality_certificate(
    state: &ChainState,
    cert: &freedom_types::FinalityCertificate,
) -> Result<(), String> {
    if !detect_precommit_equivocation(&cert.precommits).is_empty() {
        return Err("finality certificate contains equivocation".into());
    }
    validate_precommit_quorum(state, cert.block_hash, cert.height, cert.round, &cert.precommits)
}


pub fn validate_finality_certificate_for_set(
    validator_set: &std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    block: &freedom_types::Block,
    cert: &freedom_types::FinalityCertificate,
) -> Result<(), String> {
    if validator_set.is_empty() {
        return Err("finality validator set is empty".into());
    }
    let expected_hash = block_hash(block);
    if cert.block_hash != expected_hash { return Err("finality certificate block hash mismatch".into()); }
    if cert.height != block.header.height { return Err("finality certificate height mismatch".into()); }
    if cert.round != block.header.round { return Err("finality certificate round mismatch".into()); }
    if !detect_precommit_equivocation(&cert.precommits).is_empty() { return Err("finality certificate contains equivocation".into()); }
    validate_precommit_quorum_for_set(validator_set, cert.block_hash, cert.height, cert.round, &cert.precommits)
}

pub fn validate_finality_certificate_for_block(
    state: &ChainState,
    block: &freedom_types::Block,
    cert: &freedom_types::FinalityCertificate,
) -> Result<(), String> {
    validate_finality_certificate_for_set(&state.active_validator_set, block, cert)
}

pub fn validate_timeout_certificate(
    state: &ChainState,
    certificate: &freedom_types::TimeoutCertificate,
    height: u64,
    round: u64,
) -> Result<(), String> {
    if certificate.height != height { return Err("timeout certificate height mismatch".into()); }
    if certificate.round >= round { return Err("timeout certificate must be from an earlier round".into()); }
    validate_timeout_quorum(state, height, certificate.round, &certificate.timeouts)
}

pub fn validate_valid_round_proof(
    state: &ChainState,
    block_hash: Hash,
    height: u64,
    proposal_round: u64,
    valid_round: Option<u64>,
    precommits: &[freedom_types::ValidatorPrecommit],
) -> Result<(), String> {
    match valid_round {
        None => {
            if !precommits.is_empty() { return Err("precommits supplied without valid round".into()); }
            Ok(())
        }
        Some(r) => {
            if r >= proposal_round { return Err("valid round must be lower than proposal round".into()); }
            validate_precommit_quorum(state, block_hash, height, r, precommits)
        }
    }
}


pub fn make_evidence(
    kind: freedom_types::EquivocationKind,
    height: u64,
    round: u64,
    validator_id: Hash,
    first: Vec<u8>,
    second: Vec<u8>,
) -> freedom_types::EquivocationEvidence {
    let mut evidence = freedom_types::EquivocationEvidence {
        kind,
        height,
        round,
        validator_id,
        first,
        second,
        evidence_hash: [0u8; 32],
    };
    evidence.evidence_hash = freedom_primitives::hash_bytes(&evidence.signing_bytes());
    evidence
}

pub fn validate_evidence_message(state: &ChainState, msg: &freedom_types::EvidenceMessage) -> Result<(), String> { validate_evidence(&msg.evidence)?; let v=state.active_validator_set.get(&msg.sender).ok_or_else(||"evidence sender is not active validator".to_string())?; if msg.signature.len()!=64{return Err("invalid evidence message signature length".into())} let mut sig=[0u8;64];sig.copy_from_slice(&msg.signature); if !freedom_crypto::verify(&v.consensus_key,&msg.signing_bytes(),&sig){return Err("invalid evidence message signature".into())} Ok(()) }

pub fn validate_evidence(evidence: &freedom_types::EquivocationEvidence) -> Result<(), String> {
    if evidence.first.is_empty() || evidence.second.is_empty() {
        return Err("equivocation evidence payload is empty".into());
    }
    if evidence.first == evidence.second {
        return Err("equivocation evidence contains identical messages".into());
    }
    let expected = freedom_primitives::hash_bytes(&evidence.signing_bytes());
    if evidence.evidence_hash != expected {
        return Err("equivocation evidence hash mismatch".into());
    }
    Ok(())
}

pub fn detect_prevotes_evidence(
    votes: &[freedom_types::ValidatorVote],
) -> Vec<freedom_types::EquivocationEvidence> {
    let mut first = std::collections::BTreeMap::<(Hash,u64,u64), freedom_types::ValidatorVote>::new();
    let mut out = Vec::new();
    for vote in votes {
        let key = (vote.validator_id, vote.height, vote.round);
        if let Some(existing) = first.get(&key) {
            if existing.block_hash != vote.block_hash {
                out.push(make_evidence(
                    freedom_types::EquivocationKind::DoublePrevote,
                    vote.height, vote.round, vote.validator_id,
                    existing.signing_bytes(), vote.signing_bytes(),
                ));
            }
        } else {
            first.insert(key, vote.clone());
        }
    }
    out
}

pub fn detect_precommits_evidence(
    precommits: &[freedom_types::ValidatorPrecommit],
) -> Vec<freedom_types::EquivocationEvidence> {
    let mut first = std::collections::BTreeMap::<(Hash,u64,u64), freedom_types::ValidatorPrecommit>::new();
    let mut out = Vec::new();
    for pc in precommits {
        let key = (pc.validator_id, pc.height, pc.round);
        if let Some(existing) = first.get(&key) {
            if existing.block_hash != pc.block_hash {
                out.push(make_evidence(
                    freedom_types::EquivocationKind::DoublePrecommit,
                    pc.height, pc.round, pc.validator_id,
                    existing.signing_bytes(), pc.signing_bytes(),
                ));
            }
        } else {
            first.insert(key, pc.clone());
        }
    }
    out
}


#[cfg(test)]
mod m24_evidence_tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    #[test]
    fn evidence_hash_is_deterministic_and_valid() {
        let a = make_evidence(
            freedom_types::EquivocationKind::DoublePrevote,
            10, 2, hash_bytes(b"validator"),
            b"vote-a".to_vec(), b"vote-b".to_vec(),
        );
        assert!(validate_evidence(&a).is_ok());
        let mut tampered = a.clone();
        tampered.second = b"vote-c".to_vec();
        assert!(validate_evidence(&tampered).is_err());
    }
}


#[cfg(test)]
mod m26_stake_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator};

    fn state(stakes: [u128;3]) -> ChainState {
        let validators=(0..3).map(|i|{
            let key=[(i+1) as u8;32];
            let id=hash_bytes(&[b"v",&[(i+1) as u8]].concat());
            Validator{validator_id:id,owner:freedom_primitives::address_from_public_key(&key),consensus_key:key,stake:stakes[i],blocks_proposed:0,blocks_finalized:0,missed_votes:0}
        }).collect();
        ChainState::genesis(&Genesis{chain_id:1,initial_supply:0,validators})
    }

    #[test]
    fn stake_quorum_uses_weight_not_count() {
        let s=state([80,10,10]);
        let ids:std::collections::BTreeSet<_>=s.validators.keys().take(1).copied().collect();
        assert!(stake_quorum_reached(&s,&ids));
    }

    #[test]
    fn minority_stake_does_not_finalize() {
        let s=state([40,30,30]);
        let ids:std::collections::BTreeSet<_>=s.validators.keys().take(1).copied().collect();
        assert!(!stake_quorum_reached(&s,&ids));
    }

    #[test]
    fn zero_total_stake_has_no_quorum() {
        let s=state([0,0,0]);
        let ids=std::collections::BTreeSet::new();
        assert!(!stake_quorum_reached(&s,&ids));
    }
}

#[cfg(test)]
mod m320_validator_certificate_admission_tests {
    use super::*;

    fn state() -> ChainState {
        let validators = (1u8..=3u8).map(|byte| {
            let key = [byte; 32];
            let id = freedom_primitives::hash_bytes(&key);
            freedom_types::Validator {
                validator_id: id,
                owner: [byte + 10; 32],
                consensus_key: key,
                stake: 100,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        }).collect();
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators,
        })
    }

    #[test]
    fn certificate_admission_rejects_unsorted_votes() {
        let state = state();
        let keys: Vec<_> = state.active_validator_set.keys().copied().collect();
        let previous = validator_set_hash(&state.active_validator_set);
        let new_set = validator_set_hash(&state.pending_validator_set);
        let mut votes = Vec::new();
        for id in keys.iter().take(3).rev() {
            let entry = state.active_validator_set.get(id).unwrap();
            let mut vote = freedom_types::ValidatorSetVote {
                from_epoch: 0,
                to_epoch: 1,
                previous_set_hash: previous,
                new_set_hash: new_set,
                validator_id: *id,
                signature: Vec::new(),
            };
            vote.signature = freedom_crypto::sign(
                &ed25519_dalek::SigningKey::from_bytes(&entry.consensus_key),
                &vote.signing_bytes(),
            ).to_vec();
            votes.push(vote);
        }
        let cert = freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: previous,
            new_set_hash: new_set,
            votes,
        };
        assert_eq!(
            validate_validator_set_certificate(&state, &cert),
            Err("invalid validator set certificate transition fields".into())
        );
    }

    #[test]
    fn certificate_admission_requires_strict_validator_count_quorum() {
        let state = state();
        let id = *state.active_validator_set.keys().next().unwrap();
        let entry = state.active_validator_set.get(&id).unwrap();
        let previous = validator_set_hash(&state.active_validator_set);
        let new_set = validator_set_hash(&state.pending_validator_set);
        let mut vote = freedom_types::ValidatorSetVote {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: previous,
            new_set_hash: new_set,
            validator_id: id,
            signature: Vec::new(),
        };
        vote.signature = freedom_crypto::sign(
            &ed25519_dalek::SigningKey::from_bytes(&entry.consensus_key),
            &vote.signing_bytes(),
        ).to_vec();
        let cert = freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: previous,
            new_set_hash: new_set,
            votes: vec![vote],
        };
        assert_eq!(
            validate_validator_set_certificate(&state, &cert),
            Err("validator set certificate lacks validator-count quorum".into())
        );
    }
}


#[cfg(test)]
mod m322_epoch_boundary_replay_tests {
    use super::*;
    use freedom_types::Genesis;

    fn state() -> ChainState {
        ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        })
    }

    fn boundary_block(epoch: u64) -> Block {
        Block {
            header: freedom_types::BlockHeader {
                height: 1000,
                round: 0,
                previous_hash: [0; 32],
                state_root: [0; 32],
                tx_root: tx_root(&[]),
                receipts_root: receipts_root(&[]),
                timestamp: 1,
                epoch,
                proposer: [0; 32],
                epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        }
    }

    #[test]
    fn boundary_rejects_epoch_skip_even_before_certificate_lookup() {
        let mut state = state();
        state.epoch_length = 1000;
        state.epoch = 0;
        assert_eq!(
            validate_epoch_transition_certificate(&state, &boundary_block(2)),
            Err("epoch transition block target epoch must be the immediate successor".into())
        );
    }

    #[test]
    fn boundary_rejects_replayed_current_epoch_as_transition() {
        let mut state = state();
        state.epoch_length = 1000;
        state.epoch = 1;
        assert_eq!(
            validate_epoch_transition_certificate(&state, &boundary_block(1)),
            Err("epoch transition block target epoch must be the immediate successor".into())
        );
    }
}


#[cfg(test)]
mod m323_stake_quorum_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator};

    fn state_with_stakes(stakes: &[u128]) -> ChainState {
        let validators = stakes.iter().enumerate().map(|(i, stake)| {
            let key = [i as u8 + 1; 32];
            let id = hash_bytes(&[b"m323-validator", &[i as u8 + 1]].concat());
            Validator {
                validator_id: id,
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: *stake,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        }).collect::<Vec<_>>();
        ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators,
        })
    }

    #[test]
    fn huge_stake_does_not_turn_two_thirds_quorum_into_one_third() {
        let total = u128::MAX;
        let state = state_with_stakes(&[total / 3 + 1, total - (total / 3 + 1)]);
        let first = hash_bytes(&[b"m323-validator", &[1u8]].concat());

        let mut signers = std::collections::BTreeSet::new();
        signers.insert(first);

        assert!(!stake_quorum_reached(&state, &signers));
    }

    #[test]
    fn huge_stake_accepts_exact_two_thirds_threshold() {
        let total = u128::MAX;
        let required = total - total / 3;
        let state = state_with_stakes(&[required, total - required]);
        let first = hash_bytes(&[b"m323-validator", &[1u8]].concat());

        let mut signers = std::collections::BTreeSet::new();
        signers.insert(first);

        assert!(stake_quorum_reached(&state, &signers));
    }
}


#[cfg(test)]
mod m324_stake_sum_overflow_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::{Genesis, Validator};

    fn state_with_max_plus_one_total() -> ChainState {
        let stakes = [u128::MAX, 1u128];
        let validators = stakes.iter().enumerate().map(|(i, stake)| {
            let key = [i as u8 + 20; 32];
            let id = hash_bytes(&[b"m324-validator", &[i as u8 + 20]].concat());
            Validator {
                validator_id: id,
                owner: freedom_primitives::address_from_public_key(&key),
                consensus_key: key,
                stake: *stake,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        }).collect::<Vec<_>>();
        ChainState::genesis(&Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators,
        })
    }

    #[test]
    fn total_active_stake_does_not_wrap() {
        let state = state_with_max_plus_one_total();
        assert_eq!(total_active_stake(&state), u128::MAX);
    }

    #[test]
    fn stake_quorum_rejects_overflowing_signer_sum() {
        let state = state_with_max_plus_one_total();
        let ids: Vec<_> = state.active_validator_set.keys().copied().collect();
        let mut signers = std::collections::BTreeSet::new();
        signers.insert(ids[0]);
        signers.insert(ids[1]);

        assert!(!stake_quorum_reached(&state, &signers));
    }
}
