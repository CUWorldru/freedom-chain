use freedom_primitives::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub height: u64, pub previous_hash: Hash, pub state_root: Hash, pub tx_root: Hash,
    pub receipts_root: Hash, pub timestamp: u64, pub epoch: u64, pub proposer: Address,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block { pub header: BlockHeader, pub transactions: Vec<Transaction> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub chain_id: u32, pub sender: Address, pub nonce: u64, pub kind: TxKind,
    pub payload: Vec<u8>, pub fee: u128, pub signature: Vec<u8>,
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxKind {
    Transfer = 1, RegisterNode = 2, SubmitContribution = 3, ClaimReward = 4, Purchase = 10,
    Stake = 5, Unstake = 6, CreateTicket = 7, Burn = 8, Governance = 9,
}
impl TxKind { pub fn code(self) -> u8 { self as u8 } }
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
pub struct TransferPayload { pub to: Address, pub amount: u128 }
impl TransferPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(48); out.extend_from_slice(&self.to); out.extend_from_slice(&self.amount.to_le_bytes()); out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != 48 { return Err("transfer payload must be exactly 48 bytes".into()); }
        let mut to=[0u8;32]; to.copy_from_slice(&bytes[..32]);
        let mut amount=[0u8;16]; amount.copy_from_slice(&bytes[32..]);
        Ok(Self{to,amount:u128::from_le_bytes(amount)})
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterNodePayload { pub node_id: Hash, pub capabilities: u64 }
impl RegisterNodePayload {
    pub fn encode(&self)->Vec<u8>{let mut o=Vec::with_capacity(40);o.extend_from_slice(&self.node_id);o.extend_from_slice(&self.capabilities.to_le_bytes());o}
    pub fn decode(b:&[u8])->Result<Self,String>{if b.len()!=40{return Err("register node payload must be exactly 40 bytes".into())}let mut n=[0u8;32];n.copy_from_slice(&b[..32]);let mut c=[0u8;8];c.copy_from_slice(&b[32..]);Ok(Self{node_id:n,capabilities:u64::from_le_bytes(c)})}
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitContributionPayload {
    pub contribution_id: Hash, pub node_id: Hash, pub task_id: Hash,
    pub work_units: u128, pub quality_score: u32, pub result_hash: Hash,
}
impl SubmitContributionPayload {
    pub fn encode(&self)->Vec<u8>{
        let mut o=Vec::with_capacity(148);o.extend_from_slice(&self.contribution_id);o.extend_from_slice(&self.node_id);o.extend_from_slice(&self.task_id);o.extend_from_slice(&self.work_units.to_le_bytes());o.extend_from_slice(&self.quality_score.to_le_bytes());o.extend_from_slice(&self.result_hash);o
    }
    pub fn decode(b:&[u8])->Result<Self,String>{
        if b.len()!=148{return Err("submit contribution payload must be exactly 148 bytes".into())}
        let mut a=[0u8;32];let mut n=[0u8;32];let mut t=[0u8;32];let mut w=[0u8;16];let mut q=[0u8;4];let mut r=[0u8;32];
        a.copy_from_slice(&b[0..32]);n.copy_from_slice(&b[32..64]);t.copy_from_slice(&b[64..96]);w.copy_from_slice(&b[96..112]);q.copy_from_slice(&b[112..116]);r.copy_from_slice(&b[116..148]);
        Ok(Self{contribution_id:a,node_id:n,task_id:t,work_units:u128::from_le_bytes(w),quality_score:u32::from_le_bytes(q),result_hash:r})
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event { FdmTransferred { from: Address, to: Address, amount: u128 }, TicketCreated { ticket_id: Hash, draw_id: Hash, owner: Address }, DrawFundAllocated { draw_id: Hash, prize_pool: u128, cu_fund: u128, annual_reserve: u128 }, DrawExecuted { draw_id: Hash, randomness_seed: Hash, selection_proof: Hash }, PrizePaid { draw_id: Hash, ticket_id: Hash, winner: Address, amount: u128 } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt { pub tx_hash: Hash, pub success: bool, pub gas_used: u64, pub events: Vec<Event> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    pub validator_id: Hash, pub owner: Address, pub consensus_key: [u8;32], pub stake:u128,
    pub blocks_proposed:u64, pub blocks_finalized:u64, pub missed_votes:u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis { pub chain_id:u32, pub initial_supply:u128, pub validators:Vec<Validator> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimRewardPayload { pub contribution_id: Hash }
impl ClaimRewardPayload { pub fn encode(&self)->Vec<u8>{self.contribution_id.to_vec()} pub fn decode(b:&[u8])->Result<Self,String>{if b.len()!=32{return Err("claim reward payload must be exactly 32 bytes".into())}let mut id=[0u8;32];id.copy_from_slice(b);Ok(Self{contribution_id:id})} }


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProductId { AiBotConnect = 1 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BillingStatus { Quoted = 1, Reserved = 2, Captured = 3, Refunded = 4, Cancelled = 5 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PurchasePayload {
    pub product_id: u8,
    pub merchant: Address,
    pub price: u128,
    pub reference: Hash,
}
impl PurchasePayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut out=Vec::with_capacity(81);
        out.push(self.product_id);
        out.extend_from_slice(&self.merchant);
        out.extend_from_slice(&self.price.to_le_bytes());
        out.extend_from_slice(&self.reference);
        out
    }
    pub fn decode(b:&[u8])->Result<Self,String>{
        if b.len()!=81{return Err("purchase payload must be exactly 81 bytes".into())}
        let mut m=[0u8;32];let mut p=[0u8;16];let mut r=[0u8;32];
        m.copy_from_slice(&b[1..33]);p.copy_from_slice(&b[33..49]);r.copy_from_slice(&b[49..81]);
        Ok(Self{product_id:b[0],merchant:m,price:u128::from_le_bytes(p),reference:r})
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillingPayload {
    pub billing_id: Hash,
    pub product_id: u8,
    pub merchant: Address,
    pub amount: u128,
    pub reference: Hash,
}
impl BillingPayload {
    pub fn encode(&self)->Vec<u8>{
        let mut o=Vec::with_capacity(113);o.extend_from_slice(&self.billing_id);o.push(self.product_id);
        o.extend_from_slice(&self.merchant);o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.reference);o
    }
    pub fn decode(b:&[u8])->Result<Self,String>{
        if b.len()!=113{return Err("billing payload must be exactly 113 bytes".into())}
        let mut id=[0u8;32];let mut m=[0u8;32];let mut a=[0u8;16];let mut r=[0u8;32];
        id.copy_from_slice(&b[0..32]);m.copy_from_slice(&b[33..65]);a.copy_from_slice(&b[65..81]);r.copy_from_slice(&b[81..113]);
        Ok(Self{billing_id:id,product_id:b[32],merchant:m,amount:u128::from_le_bytes(a),reference:r})
    }
}


#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DrawType { Daily=1, Weekly=2, Monthly=3, Quarterly=4, Annual=5, Special=6 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTicketPayload { pub draw_id: Hash, pub quantity: u64 }
impl CreateTicketPayload {
    pub fn encode(&self)->Vec<u8>{ let mut o=Vec::with_capacity(40); o.extend_from_slice(&self.draw_id); o.extend_from_slice(&self.quantity.to_le_bytes()); o }
    pub fn decode(b:&[u8])->Result<Self,String>{ if b.len()!=40{return Err("create ticket payload must be exactly 40 bytes".into())} let mut d=[0u8;32]; let mut q=[0u8;8]; d.copy_from_slice(&b[..32]); q.copy_from_slice(&b[32..]); Ok(Self{draw_id:d,quantity:u64::from_le_bytes(q)}) }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawConfig { pub draw_id: Hash, pub draw_type: DrawType, pub ticket_price_mfdm: u128, pub winner_count: u32, pub starts_at: u64, pub ends_at: u64, pub seed_block: u64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawResult { pub draw_id: Hash, pub total_tickets: u64, pub prize_pool_mfdm: u128, pub cu_fund_mfdm: u128, pub annual_reserve_mfdm: u128, pub winners: Vec<Hash>, pub randomness_seed: Hash, pub selection_proof: Hash }
