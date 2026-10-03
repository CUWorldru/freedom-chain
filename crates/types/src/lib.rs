use freedom_primitives::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub height: u64,
    pub previous_hash: Hash,
    pub state_root: Hash,
    pub tx_root: Hash,
    pub receipts_root: Hash,
    pub timestamp: u64,
    pub epoch: u64,
    pub proposer: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub chain_id: u32,
    pub sender: Address,
    pub nonce: u64,
    pub kind: TxKind,
    pub payload: Vec<u8>,
    pub fee: u128,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxKind {
    Transfer,
    RegisterNode,
    SubmitContribution,
    ClaimReward,
    Stake,
    Unstake,
    CreateTicket,
    Burn,
    Governance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub tx_hash: Hash,
    pub success: bool,
    pub gas_used: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    pub validator_id: Hash,
    pub owner: Address,
    pub consensus_key: [u8; 32],
    pub stake: u128,
    pub blocks_proposed: u64,
    pub blocks_finalized: u64,
    pub missed_votes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    pub chain_id: u32,
    pub initial_supply: u128,
    pub validators: Vec<Validator>,
}
