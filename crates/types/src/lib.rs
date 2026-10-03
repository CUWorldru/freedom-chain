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

#[repr(u8)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxKind {
    Transfer = 1,
    RegisterNode = 2,
    SubmitContribution = 3,
    ClaimReward = 4,
    Stake = 5,
    Unstake = 6,
    CreateTicket = 7,
    Burn = 8,
    Governance = 9,
}

impl TxKind {
    pub fn code(self) -> u8 { self as u8 }
}

impl Transaction {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-TX-V1");
        out.extend_from_slice(&self.chain_id.to_le_bytes());
        out.extend_from_slice(&self.sender);
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out.push(self.kind.code());
        out.extend_from_slice(&(self.payload.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&self.fee.to_le_bytes());
        out
    }

    pub fn hash(&self) -> Hash {
        let mut bytes = self.signing_bytes();
        bytes.extend_from_slice(&(self.signature.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.signature);
        freedom_primitives::hash_bytes(&bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferPayload {
    pub to: Address,
    pub amount: u128,
}

impl TransferPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(48);
        out.extend_from_slice(&self.to);
        out.extend_from_slice(&self.amount.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 48 {
            return Err("transfer payload must be exactly 48 bytes".into());
        }
        let mut to = [0u8; 32];
        to.copy_from_slice(&bytes[..32]);
        let mut amount_bytes = [0u8; 16];
        amount_bytes.copy_from_slice(&bytes[32..]);
        Ok(Self { to, amount: u128::from_le_bytes(amount_bytes) })
    }
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
