use freedom_primitives::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub height: u64, pub round: u64, pub previous_hash: Hash, pub state_root: Hash, pub tx_root: Hash,
    pub receipts_root: Hash, pub timestamp: u64, pub epoch: u64, pub proposer: Address,
    #[serde(default)]
    pub epoch_transition_certificate: Option<ValidatorSetCertificate>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block { pub header: BlockHeader, pub transactions: Vec<Transaction>, pub receipts: Vec<Receipt> }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub chain_id: u32, pub sender: Address, pub nonce: u64, pub kind: TxKind,
    pub payload: Vec<u8>, pub fee: u128, pub signature: Vec<u8>,
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxKind {
    Transfer = 1, RegisterNode = 2, SubmitContribution = 3, ClaimReward = 4, Purchase = 10,
    Stake = 5, Unstake = 6, CreateTicket = 7, Burn = 8, Governance = 9, SubmitEvidence = 11,
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
pub struct StakePayload { pub amount: u128 }
impl StakePayload {
    pub fn encode(&self)->Vec<u8>{self.amount.to_le_bytes().to_vec()}
    pub fn decode(b:&[u8])->Result<Self,String>{if b.len()!=16{return Err("stake payload must be exactly 16 bytes".into())}let mut a=[0u8;16];a.copy_from_slice(b);Ok(Self{amount:u128::from_le_bytes(a)})}
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnstakePayload { pub amount: u128 }
impl UnstakePayload {
    pub fn encode(&self)->Vec<u8>{self.amount.to_le_bytes().to_vec()}
    pub fn decode(b:&[u8])->Result<Self,String>{if b.len()!=16{return Err("unstake payload must be exactly 16 bytes".into())}let mut a=[0u8;16];a.copy_from_slice(b);Ok(Self{amount:u128::from_le_bytes(a)})}
}
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
pub enum Event { FdmTransferred { from: Address, to: Address, amount: u128 }, TicketCreated { ticket_id: Hash, draw_id: Hash, owner: Address }, DrawFundAllocated { draw_id: Hash, prize_pool: u128, cu_fund: u128, annual_reserve: u128 }, DrawExecuted { draw_id: Hash, randomness_seed: Hash, selection_proof: Hash }, PrizePaid { draw_id: Hash, ticket_id: Hash, winner: Address, amount: u128 }, EquivocationSlashed { evidence_hash: Hash, validator_id: Hash, amount: u128 } }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt { pub tx_hash: Hash, pub success: bool, pub gas_used: u64, pub events: Vec<Event> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    pub validator_id: Hash, pub owner: Address, pub consensus_key: [u8;32], pub stake:u128,
    #[serde(default)]
    pub unbonding_stake:u128,
    pub blocks_proposed:u64, pub blocks_finalized:u64, pub missed_votes:u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnbondingEntry {
    pub unbonding_id: Hash,
    pub validator_id: Hash,
    pub owner: Address,
    pub amount: u128,
    pub requested_height: u64,
    pub release_height: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidatorLifecycle { Active, Unbonding, Exited }
pub struct ValidatorSetEntry {
    pub validator_id: Hash,
    pub owner: Address,
    pub consensus_key: [u8;32],
    pub stake: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSetVote {
    pub from_epoch: u64,
    pub to_epoch: u64,
    pub previous_set_hash: Hash,
    pub new_set_hash: Hash,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}
impl ValidatorSetVote {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-VALSET-VOTE-V1");
        out.extend_from_slice(&self.from_epoch.to_le_bytes());
        out.extend_from_slice(&self.to_epoch.to_le_bytes());
        out.extend_from_slice(&self.previous_set_hash);
        out.extend_from_slice(&self.new_set_hash);
        out.extend_from_slice(&self.validator_id);
        out
    }
    pub fn message_id(&self) -> Hash {
        freedom_primitives::hash_bytes(&self.signing_bytes())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorSetCertificate {
    pub from_epoch: u64,
    pub to_epoch: u64,
    pub previous_set_hash: Hash,
    pub new_set_hash: Hash,
    pub votes: Vec<ValidatorSetVote>,
}
impl ValidatorSetCertificate {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-VALSET-CERT-V1");
        out.extend_from_slice(&self.from_epoch.to_le_bytes());
        out.extend_from_slice(&self.to_epoch.to_le_bytes());
        out.extend_from_slice(&self.previous_set_hash);
        out.extend_from_slice(&self.new_set_hash);
        for v in &self.votes {
            out.extend_from_slice(&v.signing_bytes());
        }
        out
    }
    pub fn hash(&self) -> Hash {
        freedom_primitives::hash_bytes(&self.signing_bytes())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetType {
    Native = 1,
    Bridged = 2,
}

pub const BRIDGE_MESSAGE_PROTOCOL_VERSION: u16 = 1;
pub const BRIDGE_MESSAGE_DOMAIN: &[u8] = b"FREEDOM-BRIDGE-MESSAGE-V1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeMessage {
    pub protocol_version: u16,
    pub source_chain_id: u32,
    pub destination_chain_id: u32,
    pub bridge_id: Hash,
    pub asset_id: Hash,
    pub source_tx_hash: Hash,
    pub source_event_index: u64,
    pub nonce: u64,
    pub amount: u128,
    pub recipient: Address,
    pub message_hash: Hash,
}

impl BridgeMessage {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(BRIDGE_MESSAGE_DOMAIN);
        out.extend_from_slice(&self.protocol_version.to_le_bytes());
        out.extend_from_slice(&self.source_chain_id.to_le_bytes());
        out.extend_from_slice(&self.destination_chain_id.to_le_bytes());
        out.extend_from_slice(&self.bridge_id);
        out.extend_from_slice(&self.asset_id);
        out.extend_from_slice(&self.source_tx_hash);
        out.extend_from_slice(&self.source_event_index.to_le_bytes());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out.extend_from_slice(&self.amount.to_le_bytes());
        out.extend_from_slice(&self.recipient);
        out
    }

    pub fn compute_hash(&self) -> Hash {
        freedom_primitives::hash_bytes(&self.signing_bytes())
    }

    pub fn validate_hash(&self) -> bool {
        self.message_hash == self.compute_hash()
    }

    pub fn replay_key(&self) -> Hash {
        freedom_primitives::domain_hash(
            b"FREEDOM-BRIDGE-REPLAY-V1",
            &[
                &self.source_chain_id.to_le_bytes(),
                &self.source_tx_hash,
                &self.source_event_index.to_le_bytes(),
                &self.nonce.to_le_bytes(),
            ],
        )
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetStatus {
    Active = 1,
    Paused = 2,
    Deprecated = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDefinition {
    pub asset_id: Hash,
    pub source_chain_id: u32,
    pub asset_type: AssetType,
    /// Contract address for contract assets, or a canonical native-asset identifier
    /// (for example b"BTC") for chains without contracts.
    pub contract_address: Vec<u8>,
    pub symbol: String,
    pub decimals: u8,
    pub issuer: Hash,
    pub bridge_id: Hash,
    pub status: AssetStatus,
}

impl AssetDefinition {
    pub fn compute_asset_id(
        source_chain_id: u32,
        asset_type: AssetType,
        contract_address: &[u8],
        issuer: Hash,
        bridge_id: Hash,
    ) -> Hash {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"FREEDOM-ASSET-ID-V1");
        bytes.extend_from_slice(&source_chain_id.to_le_bytes());
        bytes.push(asset_type as u8);
        bytes.extend_from_slice(&(contract_address.len() as u64).to_le_bytes());
        bytes.extend_from_slice(contract_address);
        bytes.extend_from_slice(&issuer);
        bytes.extend_from_slice(&bridge_id);
        freedom_primitives::hash_bytes(&bytes)
    }

    pub fn validate_identity(&self) -> bool {
        self.asset_id == Self::compute_asset_id(
            self.source_chain_id,
            self.asset_type,
            &self.contract_address,
            self.issuer,
            self.bridge_id,
        ) && !self.symbol.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeDirection { Inbound = 1, Outbound = 2 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeWithdrawalStatus {
    Requested = 1, Validated = 2, Approved = 3, Signing = 4,
    Broadcast = 5, ExternalConfirmed = 6, Finalized = 7, Rejected = 8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeAdapterKind { Bitcoin = 1, Evm = 2, Tron = 3, Ton = 4, Solana = 5, Other = 255 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeFinalityModel { Confirmations = 1, BlockFinality = 2, SlotFinality = 3, Other = 255 }


#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BridgeApprovalMode { Quorum = 1, Threshold = 2, QuorumAndThreshold = 3 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeSecurityPolicy {
    pub bridge_id: Hash,
    pub approval_mode: BridgeApprovalMode,
    pub validator_quorum: u16,
    pub validator_total: u16,
    pub threshold_signers: u16,
    pub threshold_total: u16,
    pub min_external_confirmations: u64,
    pub max_deposit_amount: u128,
    pub max_withdrawal_amount: u128,
    pub daily_limit: u128,
    pub emergency_paused: bool,
    pub nonce_floor: u64,
}
impl BridgeSecurityPolicy {
    pub fn validate(&self)->bool {
        self.bridge_id != [0;32]
            && self.validator_total > 0
            && self.validator_quorum > 0
            && self.validator_quorum <= self.validator_total
            && self.threshold_total > 0
            && self.threshold_signers > 0
            && self.threshold_signers <= self.threshold_total
            && self.validator_quorum * 3 > self.validator_total * 2
            && self.min_external_confirmations > 0
            && self.max_deposit_amount > 0
            && self.max_withdrawal_amount > 0
            && self.daily_limit >= self.max_withdrawal_amount
    }
    pub fn quorum_reached(&self, approvals:u16)->bool { approvals >= self.validator_quorum }
    pub fn threshold_reached(&self, signatures:u16)->bool { signatures >= self.threshold_signers }
    pub fn deposit_allowed(&self, amount:u128)->bool { !self.emergency_paused && amount > 0 && amount <= self.max_deposit_amount }
    pub fn withdrawal_allowed(&self, amount:u128)->bool { !self.emergency_paused && amount > 0 && amount <= self.max_withdrawal_amount && amount <= self.daily_limit }
    pub fn nonce_allowed(&self, nonce:u64)->bool { nonce >= self.nonce_floor }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeRateLimitUsage { pub period:u64, pub used:u128 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgePauseAuthorization {
    pub bridge_id: Hash,
    pub paused: bool,
    pub nonce: u64,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}
impl BridgePauseAuthorization {
    pub fn signing_bytes(&self)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BRIDGE-PAUSE-V1");o.extend_from_slice(&self.bridge_id);o.push(self.paused as u8);o.extend_from_slice(&self.nonce.to_le_bytes());o.extend_from_slice(&self.validator_id);o}
    pub fn authorization_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
    pub fn structurally_valid(&self)->bool{self.bridge_id!=[0;32]&&self.validator_id!=[0;32]&&self.signature.len()==64}
    pub fn validator_identity_matches(&self,key:&[u8;32])->bool{freedom_primitives::hash_bytes(key)==self.validator_id}
    pub fn verify_authenticated(&self,key:&[u8;32])->bool{
        if !self.structurally_valid()||!self.validator_identity_matches(key){return false}
        let mut sig=[0u8;64];sig.copy_from_slice(&self.signature);
        freedom_crypto::verify(key,&self.authorization_hash(),&sig)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgePauseAuthorizationSet {
    pub bridge_id: Hash,
    pub nonce: u64,
    pub authorizations: Vec<BridgePauseAuthorization>,
}
impl BridgePauseAuthorizationSet {
    pub fn authenticated_count(&self,keys:&BTreeMap<Hash,[u8;32]>)->u16{
        self.authorizations.iter().filter_map(|a|keys.get(&a.validator_id).map(|k|(a,k))).filter(|(a,k)|a.bridge_id==self.bridge_id&&a.nonce==self.nonce&&a.verify_authenticated(k)).map(|(a,_)|a.validator_id).collect::<BTreeSet<Hash>>().len() as u16
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeApproval {
    pub bridge_id: Hash,
    pub request_id: Hash,
    pub withdrawal_id: Hash,
    pub validator_id: Hash,
    pub nonce: u64,
    pub amount: u128,
    pub destination_chain_id: u32,
    pub asset_id: Hash,
    pub intent_hash: Hash,
    pub signature: Vec<u8>,
}
impl BridgeApproval {
    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BRIDGE-APPROVAL-V1");
        o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.request_id);o.extend_from_slice(&self.withdrawal_id);
        o.extend_from_slice(&self.validator_id);o.extend_from_slice(&self.nonce.to_le_bytes());
        o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.destination_chain_id.to_le_bytes());
        o.extend_from_slice(&self.asset_id);o.extend_from_slice(&self.intent_hash);o
    }
    pub fn approval_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
    pub fn structurally_valid(&self)->bool{
        self.bridge_id!=[0;32]&&self.request_id!=[0;32]&&self.withdrawal_id!=[0;32]&&self.validator_id!=[0;32]
        &&self.asset_id!=[0;32]&&self.intent_hash!=[0;32]&&self.amount>0&&!self.signature.is_empty()
    }
}


impl BridgeApproval {
    pub fn signing_message(&self)->Hash { self.approval_hash() }
    pub fn verify_signature(&self, public_key:&[u8;32])->bool {
        if self.signature.len()!=64 || !self.structurally_valid() { return false; }
        let mut sig=[0u8;64]; sig.copy_from_slice(&self.signature);
        freedom_crypto::verify(public_key, &self.signing_message(), &sig)
    }
    pub fn validator_identity_matches(&self, public_key:&[u8;32])->bool {
        freedom_primitives::hash_bytes(public_key)==self.validator_id
    }
    pub fn verify_authenticated(&self, public_key:&[u8;32])->bool {
        self.validator_identity_matches(public_key) && self.verify_signature(public_key)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeApprovalSet {
    pub request_id: Hash,
    pub approvals: Vec<BridgeApproval>,
    pub threshold_signatures: u16,
}
impl BridgeApprovalSet {
    pub fn authenticated_approvals(&self, bridge_id:Hash, withdrawal_id:Hash, intent_hash:Hash, validator_keys:&std::collections::BTreeMap<Hash,[u8;32]>)->u16{
        let mut ids=std::collections::BTreeSet::new();
        for a in &self.approvals {
            if a.bridge_id==bridge_id&&a.withdrawal_id==withdrawal_id&&a.request_id==self.request_id&&a.intent_hash==intent_hash&&a.structurally_valid(){
                if let Some(key)=validator_keys.get(&a.validator_id) {
                    if a.verify_authenticated(key) { ids.insert(a.validator_id); }
                }
            }
        }
        ids.len() as u16
    }
    pub fn quorum_reached_authenticated(&self,policy:&BridgeSecurityPolicy,bridge_id:Hash,withdrawal_id:Hash,intent_hash:Hash,validator_keys:&std::collections::BTreeMap<Hash,[u8;32]>)->bool{
        policy.quorum_reached(self.authenticated_approvals(bridge_id,withdrawal_id,intent_hash,validator_keys))
    }
    pub fn unique_valid_approvals(&self, bridge_id:Hash, withdrawal_id:Hash, intent_hash:Hash)->u16{
        let mut ids=std::collections::BTreeSet::new();
        for a in &self.approvals {
            if a.bridge_id==bridge_id&&a.withdrawal_id==withdrawal_id&&a.request_id==self.request_id
                &&a.intent_hash==intent_hash&&a.structurally_valid(){ids.insert(a.validator_id);}
        }
        ids.len() as u16
    }
    pub fn quorum_reached(&self,policy:&BridgeSecurityPolicy,bridge_id:Hash,withdrawal_id:Hash,intent_hash:Hash)->bool{
        policy.quorum_reached(self.unique_valid_approvals(bridge_id,withdrawal_id,intent_hash))
    }
    pub fn threshold_reached(&self,policy:&BridgeSecurityPolicy)->bool{
        self.threshold_signatures>=policy.threshold_signers
    }
    pub fn approved(&self,policy:&BridgeSecurityPolicy,bridge_id:Hash,withdrawal_id:Hash,intent_hash:Hash)->bool{
        match policy.approval_mode {
            BridgeApprovalMode::Quorum=>self.quorum_reached(policy,bridge_id,withdrawal_id,intent_hash),
            BridgeApprovalMode::Threshold=>self.threshold_reached(policy),
            BridgeApprovalMode::QuorumAndThreshold=>self.quorum_reached(policy,bridge_id,withdrawal_id,intent_hash)&&self.threshold_reached(policy),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeAdapterDescriptor {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub kind: BridgeAdapterKind,
    pub finality_model: BridgeFinalityModel,
    pub required_confirmations: u64,
    pub native_asset: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeDepositProof {
    pub bridge_id: Hash,
    pub source_chain_id: u32,
    pub tx_hash: Hash,
    pub event_index: u64,
    pub block_or_slot: u64,
    pub confirmations: u64,
    pub proof: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeNonceKey {
    pub bridge_id: Hash,
    pub asset_id: Hash,
    pub direction: BridgeDirection,
}
impl BridgeNonceKey {
    pub fn key_hash(&self)->Hash{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BRIDGE-NONCE-KEY-V1");o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.asset_id);o.push(self.direction as u8);freedom_primitives::hash_bytes(&o)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeWithdrawalIntent {
    pub withdrawal_id: Hash,
    pub bridge_id: Hash,
    pub destination_chain_id: u32,
    pub asset_id: Hash,
    pub amount: u128,
    pub recipient: Vec<u8>,
    pub nonce: u64,
    pub intent_hash: Hash,
}
impl BridgeWithdrawalIntent {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-BRIDGE-ADAPTER-INTENT-V1");
        o.extend_from_slice(&self.withdrawal_id); o.extend_from_slice(&self.bridge_id);
        o.extend_from_slice(&self.destination_chain_id.to_le_bytes()); o.extend_from_slice(&self.asset_id);
        o.extend_from_slice(&self.amount.to_le_bytes()); o.extend_from_slice(&(self.recipient.len() as u64).to_le_bytes());
        o.extend_from_slice(&self.recipient); o.extend_from_slice(&self.nonce.to_le_bytes()); o
    }
    pub fn compute_hash(&self)->Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn validate_hash(&self)->bool { self.intent_hash==self.compute_hash() }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeWithdrawal {
    pub withdrawal_id: Hash,
    pub bridge_id: Hash,
    pub asset_id: Hash,
    pub destination_chain_id: u32,
    pub amount: u128,
    pub sender: Address,
    pub recipient: Vec<u8>,
    pub nonce: u64,
    pub request_hash: Hash,
}
impl BridgeWithdrawal {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-BRIDGE-WITHDRAWAL-V1");
        o.extend_from_slice(&self.bridge_id); o.extend_from_slice(&self.asset_id);
        o.extend_from_slice(&self.destination_chain_id.to_le_bytes());
        o.extend_from_slice(&self.amount.to_le_bytes()); o.extend_from_slice(&self.sender);
        o.extend_from_slice(&(self.recipient.len() as u64).to_le_bytes());
        o.extend_from_slice(&self.recipient); o.extend_from_slice(&self.nonce.to_le_bytes()); o
    }
    pub fn compute_hash(&self) -> Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn validate_hash(&self) -> bool { self.request_hash == self.compute_hash() }
    pub fn compute_id(&self) -> Hash {
        freedom_primitives::domain_hash(
            b"FREEDOM-BRIDGE-WITHDRAWAL-ID-V1",
            &[&self.bridge_id,&self.asset_id,&self.destination_chain_id.to_le_bytes(),
              &self.sender,&self.nonce.to_le_bytes()],
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BridgeAccounting {
    pub verified_external_locked: u128,
    pub freedom_minted: u128,
    pub freedom_burned: u128,
    pub external_released: u128,
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub vout: u32,
    pub amount_sat: u64,
    pub block_height: u64,
    pub confirmations: u64,
    pub merkle_branch: Vec<Hash>,
    pub merkle_index: u32,
    pub script_pubkey: Vec<u8>,
}
impl BitcoinUtxoProof {
    pub fn event_index(&self)->u64 { self.vout as u64 }
    pub fn proof_bytes(&self)->Vec<u8> {
        let mut o=Vec::new(); o.extend_from_slice(b"FREEDOM-BITCOIN-UTXO-PROOF-V1");
        o.extend_from_slice(&self.bridge_id); o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash); o.extend_from_slice(&self.vout.to_le_bytes());
        o.extend_from_slice(&self.amount_sat.to_le_bytes()); o.extend_from_slice(&self.block_height.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes()); o.extend_from_slice(&self.merkle_index.to_le_bytes());
        o.extend_from_slice(&(self.merkle_branch.len() as u64).to_le_bytes());
        for h in &self.merkle_branch { o.extend_from_slice(h); }
        o.extend_from_slice(&(self.script_pubkey.len() as u64).to_le_bytes()); o.extend_from_slice(&self.script_pubkey); o
    }
}


pub fn bitcoin_sha256d(data:&[u8])->Hash{use sha2::{Digest,Sha256};let first=Sha256::digest(data);let second=Sha256::digest(first);let mut out=[0u8;32];out.copy_from_slice(&second);out}
pub fn bitcoin_merkle_root(tx_hash:Hash,branch:&[Hash],index:u32)->Hash{
    let mut current=tx_hash;
    for (depth,sibling) in branch.iter().enumerate(){
        let bit=(index>>depth)&1;
        let mut bytes=Vec::with_capacity(64);
        if bit==0{bytes.extend_from_slice(&current);bytes.extend_from_slice(sibling)}else{bytes.extend_from_slice(sibling);bytes.extend_from_slice(&current)}
        current=bitcoin_sha256d(&bytes);
    }
    current
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinBlockHeader {
    pub version: u32,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,
    pub time: u32,
    pub bits: u32,
    pub nonce: u32,
}
impl BitcoinBlockHeader {
pub fn bitcoin_target_from_bits(bits:u32)->Option<[u8;32]>{
    let exponent=(bits>>24) as usize; let mantissa=bits&0x007f_ffff;
    if mantissa==0||exponent<3||exponent>32{return None;}
    if (bits&0x0080_0000)!=0{return None;}
    let mut out=[0u8;32]; let bytes=mantissa.to_be_bytes();
    let m=[bytes[1],bytes[2],bytes[3]];
    let start=32usize.checked_sub(exponent)?;
    out[start..start+3].copy_from_slice(&m); Some(out)
}
pub fn bitcoin_hash_leq_target(hash:Hash,target:[u8;32])->bool{hash.iter().rev().cmp(target.iter().rev())!=std::cmp::Ordering::Greater}
pub fn bitcoin_pow_valid(header:&BitcoinBlockHeader)->bool{bitcoin_target_from_bits(header.bits).map(|t|bitcoin_hash_leq_target(header.block_hash(),t)).unwrap_or(false)}
    pub fn serialize(&self)->[u8;80]{let mut o=[0u8;80];o[0..4].copy_from_slice(&self.version.to_le_bytes());o[4..36].copy_from_slice(&self.prev_block_hash);o[36..68].copy_from_slice(&self.merkle_root);o[68..72].copy_from_slice(&self.time.to_le_bytes());o[72..76].copy_from_slice(&self.bits.to_le_bytes());o[76..80].copy_from_slice(&self.nonce.to_le_bytes());o}
    pub fn block_hash(&self)->Hash{bitcoin_sha256d(&self.serialize())}
}
pub struct BitcoinMerkleProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub block_height: u64,
    pub confirmations: u64,
    pub merkle_root: Hash,
    pub merkle_branch: Vec<Hash>,
    pub merkle_index: u32,
    pub block_header: BitcoinBlockHeader,
}
impl BitcoinMerkleProof {
    pub fn verify(&self,required_confirmations:u64)->bool{
        self.bridge_id!=[0;32]&&self.chain_id!=0&&self.tx_hash!=[0;32]&&self.merkle_root!=[0;32]&&!self.merkle_branch.is_empty()&&self.confirmations>=required_confirmations&&self.block_header.merkle_root==self.merkle_root&&self.block_header.block_hash()!=[0;32]&&bitcoin_merkle_root(self.tx_hash,&self.merkle_branch,self.merkle_index)==self.merkle_root
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvmTransferProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub block_number: u64,
    pub confirmations: u64,
    pub token_contract: Vec<u8>,
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub amount: u128,
    pub log_index: u32,
    pub event_signature: Hash,
    pub proof: Vec<u8>,
}
impl EvmTransferProof {
    pub fn event_index(&self)->u64 { self.log_index as u64 }
    pub fn proof_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-EVM-TRANSFER-PROOF-V1");
        o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.block_number.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes());
        for x in [&self.token_contract,&self.from,&self.to]{o.extend_from_slice(&(x.len() as u64).to_le_bytes());o.extend_from_slice(x);}
        o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.log_index.to_le_bytes());
        o.extend_from_slice(&self.event_signature);o.extend_from_slice(&(self.proof.len() as u64).to_le_bytes());o.extend_from_slice(&self.proof);o
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TronTrc20TransferProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub block_number: u64,
    pub confirmations: u64,
    pub token_contract: Vec<u8>,
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub amount: u128,
    pub event_index: u32,
    pub event_signature: Hash,
    pub proof: Vec<u8>,
}
impl TronTrc20TransferProof {
    pub fn replay_event_index(&self)->u64 { self.event_index as u64 }
    pub fn proof_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-TRON-TRC20-PROOF-V1");
        o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.block_number.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes());
        for x in [&self.token_contract,&self.from,&self.to]{o.extend_from_slice(&(x.len() as u64).to_le_bytes());o.extend_from_slice(x);}
        o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.event_index.to_le_bytes());
        o.extend_from_slice(&self.event_signature);o.extend_from_slice(&(self.proof.len() as u64).to_le_bytes());o.extend_from_slice(&self.proof);o
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TonJettonTransferProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub block_or_seqno: u64,
    pub confirmations: u64,
    pub jetton_master: Vec<u8>,
    pub sender: Vec<u8>,
    pub recipient: Vec<u8>,
    pub amount: u128,
    pub event_index: u32,
    pub event_signature: Hash,
    pub proof: Vec<u8>,
}
impl TonJettonTransferProof {
    pub fn replay_event_index(&self)->u64 { self.event_index as u64 }
    pub fn proof_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-TON-JETTON-PROOF-V1");
        o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.block_or_seqno.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes());
        for x in [&self.jetton_master,&self.sender,&self.recipient]{o.extend_from_slice(&(x.len() as u64).to_le_bytes());o.extend_from_slice(x);}
        o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.event_index.to_le_bytes());
        o.extend_from_slice(&self.event_signature);o.extend_from_slice(&(self.proof.len() as u64).to_le_bytes());o.extend_from_slice(&self.proof);o
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolanaTransferProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub tx_hash: Hash,
    pub slot: u64,
    pub confirmations: u64,
    pub token_mint: Vec<u8>,
    pub source: Vec<u8>,
    pub destination: Vec<u8>,
    pub amount: u128,
    pub instruction_index: u32,
    pub program_id: Vec<u8>,
    pub proof: Vec<u8>,
}
impl SolanaTransferProof {
    pub fn replay_event_index(&self)->u64 { self.instruction_index as u64 }
    pub fn proof_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-SOLANA-SPL-PROOF-V1");
        o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.slot.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes());
        for x in [&self.token_mint,&self.source,&self.destination,&self.program_id]{o.extend_from_slice(&(x.len() as u64).to_le_bytes());o.extend_from_slice(x);}
        o.extend_from_slice(&self.amount.to_le_bytes());o.extend_from_slice(&self.instruction_index.to_le_bytes());
        o.extend_from_slice(&(self.proof.len() as u64).to_le_bytes());o.extend_from_slice(&self.proof);o
    }
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


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockProposal {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u64,
    pub proposer: Address,
    pub block: Block,
    #[serde(default)]
    pub valid_round: Option<u64>,
    #[serde(default)]
    pub valid_precommits: Vec<ValidatorPrecommit>,
    #[serde(default)]
    pub timeout_certificate: Option<TimeoutCertificate>,
    pub signature: Vec<u8>,
}
impl BlockProposal {
    pub fn message_id(&self) -> Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-PROPOSAL-V1");
        out.extend_from_slice(&self.block_hash);
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&self.proposer);
        out.extend_from_slice(&self.valid_round.unwrap_or(u64::MAX).to_le_bytes());
        for p in &self.valid_precommits { out.extend_from_slice(&p.signing_bytes()); }
        if let Some(tc) = &self.timeout_certificate {
            for t in &tc.timeouts { out.extend_from_slice(&t.signing_bytes()); }
        }
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorVote {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u64,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}
impl ValidatorVote {
    pub fn message_id(&self) -> Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-VOTE-V1");
        out.extend_from_slice(&self.block_hash);
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&self.validator_id);
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorTimeout {
    pub height: u64,
    pub round: u64,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}
impl ValidatorTimeout {
    pub fn message_id(&self) -> Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-TIMEOUT-V1");
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&self.validator_id);
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeoutCertificate {
    pub height: u64,
    pub round: u64,
    pub timeouts: Vec<ValidatorTimeout>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EquivocationKind {
    DoublePrevote,
    DoublePrecommit,
    DoubleProposal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmitEvidencePayload {
    pub evidence: EquivocationEvidence,
    pub slash_amount: u128,
}
impl SubmitEvidencePayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(match self.evidence.kind {
            EquivocationKind::DoublePrevote => 1,
            EquivocationKind::DoublePrecommit => 2,
            EquivocationKind::DoubleProposal => 3,
        });
        out.extend_from_slice(&self.evidence.height.to_le_bytes());
        out.extend_from_slice(&self.evidence.round.to_le_bytes());
        out.extend_from_slice(&self.evidence.validator_id);
        out.extend_from_slice(&(self.evidence.first.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.evidence.first);
        out.extend_from_slice(&(self.evidence.second.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.evidence.second);
        out.extend_from_slice(&self.evidence.evidence_hash);
        out.extend_from_slice(&self.slash_amount.to_le_bytes());
        out
    }
}

pub struct EvidenceMessage { pub evidence: EquivocationEvidence, pub sender: Hash, pub signature: Vec<u8> }

impl EvidenceMessage { pub fn signing_bytes(&self)->Vec<u8>{ let mut out=Vec::new(); out.extend_from_slice(b"FREEDOM-EVIDENCE-MSG-V1"); out.extend_from_slice(&self.evidence.evidence_hash); out.extend_from_slice(&self.evidence.validator_id); out.extend_from_slice(&self.sender); out } pub fn message_id(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())} }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquivocationEvidence {
    pub kind: EquivocationKind,
    pub height: u64,
    pub round: u64,
    pub validator_id: Hash,
    pub first: Vec<u8>,
    pub second: Vec<u8>,
    pub evidence_hash: Hash,
}

impl EquivocationEvidence {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-EVIDENCE-V1");
        out.extend_from_slice(&(match self.kind {
            EquivocationKind::DoublePrevote => 1u8,
            EquivocationKind::DoublePrecommit => 2u8,
            EquivocationKind::DoubleProposal => 3u8,
        }).to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&self.validator_id);
        out.extend_from_slice(&self.first);
        out.extend_from_slice(&self.second);
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinalityCertificate {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u64,
    #[serde(default)]
    pub precommits: Vec<ValidatorPrecommit>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatorPrecommit {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u64,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}
impl ValidatorPrecommit {
    pub fn message_id(&self) -> Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-PRECOMMIT-V1");
        out.extend_from_slice(&self.block_hash);
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.round.to_le_bytes());
        out.extend_from_slice(&self.validator_id);
        out
    }
}





impl EvmAdapter {
 pub fn verify_erc20_transfer(&self,p:&EvmTransferProof)->Result<BridgeDepositProof,BridgeAdapterError>{
  if p.bridge_id!=self.descriptor.bridge_id||p.chain_id!=self.descriptor.chain_id||p.tx_hash==[0;32]||p.token_contract.is_empty()||p.from.is_empty()||p.to.is_empty()||p.amount==0||p.proof.is_empty()||p.event_signature==[0;32]{return Err(BridgeAdapterError::InvalidProof)}
  if p.confirmations<self.descriptor.required_confirmations{return Err(BridgeAdapterError::InsufficientFinality)}
  Ok(BridgeDepositProof{bridge_id:p.bridge_id,source_chain_id:p.chain_id,tx_hash:p.tx_hash,event_index:p.event_index(),block_or_slot:p.block_number,confirmations:p.confirmations,proof:p.proof_bytes()})
 }
 pub fn build_erc20_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,recipient:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{if recipient.len()!=20{return Err(BridgeAdapterError::InvalidIntent)}self.build_withdrawal_intent(w,a,amount,recipient,nonce)}
}
impl TronAdapter {
 pub fn verify_trc20_transfer(&self,p:&TronTrc20TransferProof)->Result<BridgeDepositProof,BridgeAdapterError>{
  if p.bridge_id!=self.descriptor.bridge_id||p.chain_id!=self.descriptor.chain_id||p.tx_hash==[0;32]||p.token_contract.is_empty()||p.from.is_empty()||p.to.is_empty()||p.amount==0||p.proof.is_empty()||p.event_signature==[0;32]{return Err(BridgeAdapterError::InvalidProof)}
  if p.confirmations<self.descriptor.required_confirmations{return Err(BridgeAdapterError::InsufficientFinality)}
  Ok(BridgeDepositProof{bridge_id:p.bridge_id,source_chain_id:p.chain_id,tx_hash:p.tx_hash,event_index:p.replay_event_index(),block_or_slot:p.block_number,confirmations:p.confirmations,proof:p.proof_bytes()})
 }
 pub fn build_trc20_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,recipient:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{if recipient.len()!=34{return Err(BridgeAdapterError::InvalidIntent)}self.build_withdrawal_intent(w,a,amount,recipient,nonce)}
}
impl TonAdapter {
 pub fn verify_jetton_transfer(&self,p:&TonJettonTransferProof)->Result<BridgeDepositProof,BridgeAdapterError>{
  if p.bridge_id!=self.descriptor.bridge_id||p.chain_id!=self.descriptor.chain_id||p.tx_hash==[0;32]||p.jetton_master.is_empty()||p.sender.is_empty()||p.recipient.is_empty()||p.amount==0||p.proof.is_empty()||p.event_signature==[0;32]{return Err(BridgeAdapterError::InvalidProof)}
  if p.confirmations<self.descriptor.required_confirmations{return Err(BridgeAdapterError::InsufficientFinality)}
  Ok(BridgeDepositProof{bridge_id:p.bridge_id,source_chain_id:p.chain_id,tx_hash:p.tx_hash,event_index:p.replay_event_index(),block_or_slot:p.block_or_seqno,confirmations:p.confirmations,proof:p.proof_bytes()})
 }
 pub fn build_jetton_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,recipient:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{if recipient.is_empty(){return Err(BridgeAdapterError::InvalidIntent)}self.build_withdrawal_intent(w,a,amount,recipient,nonce)}
}
impl SolanaAdapter {
 pub fn verify_spl_transfer(&self,p:&SolanaTransferProof)->Result<BridgeDepositProof,BridgeAdapterError>{
  if p.bridge_id!=self.descriptor.bridge_id||p.chain_id!=self.descriptor.chain_id||p.tx_hash==[0;32]||p.token_mint.is_empty()||p.source.is_empty()||p.destination.is_empty()||p.amount==0||p.proof.is_empty()||p.program_id.is_empty(){return Err(BridgeAdapterError::InvalidProof)}
  if p.confirmations<self.descriptor.required_confirmations{return Err(BridgeAdapterError::InsufficientFinality)}
  Ok(BridgeDepositProof{bridge_id:p.bridge_id,source_chain_id:p.chain_id,tx_hash:p.tx_hash,event_index:p.replay_event_index(),block_or_slot:p.slot,confirmations:p.confirmations,proof:p.proof_bytes()})
 }
 pub fn build_spl_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,recipient:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{if recipient.len()!=32{return Err(BridgeAdapterError::InvalidIntent)}self.build_withdrawal_intent(w,a,amount,recipient,nonce)}
}


#[cfg(test)]
mod m88_bridge_security_tests {
 use super::*;
 fn p()->BridgeSecurityPolicy{BridgeSecurityPolicy{bridge_id:[1;32],approval_mode:BridgeApprovalMode::QuorumAndThreshold,validator_quorum:7,validator_total:10,threshold_signers:3,threshold_total:5,min_external_confirmations:12,max_deposit_amount:1_000_000,max_withdrawal_amount:500_000,daily_limit:2_000_000,emergency_paused:false,nonce_floor:10}}
 #[test] fn policy_requires_supermajority_quorum(){assert!(p().validate());assert!(p().quorum_reached(7));assert!(!p().quorum_reached(6));}
 #[test] fn threshold_and_pause_are_enforced(){assert!(p().threshold_reached(3));assert!(!p().threshold_reached(2));assert!(p().deposit_allowed(100));assert!(p().withdrawal_allowed(100));let mut x=p();x.emergency_paused=true;assert!(!x.deposit_allowed(100));assert!(!x.withdrawal_allowed(100));}
 #[test] fn limits_and_nonce_are_enforced(){assert!(!p().deposit_allowed(1_000_001));assert!(!p().withdrawal_allowed(500_001));assert!(!p().nonce_allowed(9));assert!(p().nonce_allowed(10));}
}


#[cfg(test)]
mod m89_bridge_approval_tests {
 use super::*;
 fn policy()->BridgeSecurityPolicy{BridgeSecurityPolicy{bridge_id:[1;32],approval_mode:BridgeApprovalMode::QuorumAndThreshold,validator_quorum:3,validator_total:4,threshold_signers:2,threshold_total:3,min_external_confirmations:12,max_deposit_amount:1_000_000,max_withdrawal_amount:500_000,daily_limit:2_000_000,emergency_paused:false,nonce_floor:1}}
 fn approval(v:u8)->BridgeApproval{BridgeApproval{bridge_id:[1;32],request_id:[2;32],withdrawal_id:[3;32],validator_id:[v;32],nonce:4,amount:100,destination_chain_id:728126428,asset_id:[5;32],intent_hash:[6;32],signature:vec![7]}}
 #[test] fn duplicate_validator_does_not_count_twice(){let mut x=BridgeApprovalSet{request_id:[2;32],approvals:vec![approval(1),approval(1),approval(2),approval(3)],threshold_signatures:2};assert_eq!(x.unique_valid_approvals([1;32],[3;32],[6;32]),3);assert!(x.approved(&policy(),[1;32],[3;32],[6;32]));}
 #[test] fn wrong_intent_is_not_counted(){let mut a=approval(1);a.intent_hash=[9;32];let x=BridgeApprovalSet{request_id:[2;32],approvals:vec![a,approval(2),approval(3)],threshold_signatures:2};assert_eq!(x.unique_valid_approvals([1;32],[3;32],[6;32]),2);assert!(!x.approved(&policy(),[1;32],[3;32],[6;32]));}
 #[test] fn threshold_and_quorum_are_both_required(){let x=BridgeApprovalSet{request_id:[2;32],approvals:vec![approval(1),approval(2),approval(3)],threshold_signatures:1};assert!(!x.approved(&policy(),[1;32],[3;32],[6;32]));}
}

#[cfg(test)]
mod m78_bridge_message_tests {
 use super::*;

 fn message() -> BridgeMessage {
  let mut m = BridgeMessage {
   protocol_version: BRIDGE_MESSAGE_PROTOCOL_VERSION,
   source_chain_id: 1,
   destination_chain_id: freedom_primitives::CHAIN_ID,
   bridge_id: [1u8;32],
   asset_id: [2u8;32],
   source_tx_hash: [3u8;32],
   source_event_index: 4,
   nonce: 5,
   amount: 600_000,
   recipient: [6u8;32],
   message_hash: [0u8;32],
  };
  m.message_hash = m.compute_hash();
  m
 }

 #[test]
 fn bridge_message_hash_binds_all_security_domains() {
  let m = message();
  assert!(m.validate_hash());
  for mutate in 0..8 {
   let mut x = m.clone();
   match mutate {
    0 => x.source_chain_id ^= 1,
    1 => x.destination_chain_id ^= 1,
    2 => x.bridge_id[0] ^= 1,
    3 => x.asset_id[0] ^= 1,
    4 => x.source_tx_hash[0] ^= 1,
    5 => x.nonce ^= 1,
    6 => x.amount ^= 1,
    _ => x.recipient[0] ^= 1,
   }
   assert!(!x.validate_hash());
  }
 }

 #[test]
 fn bridge_replay_key_binds_source_event_identity() {
  let a = message();
  let mut b = a.clone();
  b.source_event_index += 1;
  assert_ne!(a.replay_key(), b.replay_key());
  b = a.clone();
  b.nonce += 1;
  assert_ne!(a.replay_key(), b.replay_key());
 }

 #[test]
 fn bridge_hash_is_not_just_serialized_struct_hash() {
  let m = message();
  assert_ne!(
   m.compute_hash(),
   freedom_primitives::hash_bytes(&serde_json::to_vec(&m).unwrap())
  );
 }
}


#[cfg(test)]
mod m82_adapter_type_tests {
 use super::*;
 #[test]
 fn adapter_intent_binds_external_destination_and_amount() {
  let mut x=BridgeWithdrawalIntent{withdrawal_id:[1;32],bridge_id:[2;32],destination_chain_id:3,asset_id:[4;32],amount:500,recipient:b"address".to_vec(),nonce:7,intent_hash:[0;32]};
  x.intent_hash=x.compute_hash(); assert!(x.validate_hash());
  x.amount+=1; assert!(!x.validate_hash());
 }
 #[test]
 fn descriptors_cover_major_network_families() {
  assert_eq!(BridgeAdapterKind::Bitcoin as u8,1);
  assert_eq!(BridgeAdapterKind::Evm as u8,2);
  assert_eq!(BridgeAdapterKind::Tron as u8,3);
  assert_eq!(BridgeAdapterKind::Ton as u8,4);
  assert_eq!(BridgeAdapterKind::Solana as u8,5);
 }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalDepositProof {
    pub bridge_id: Hash,
    pub source_chain_id: u32,
    pub tx_hash: Hash,
    pub event_index: u64,
    pub block_or_slot: u64,
    pub confirmations: u64,
    pub commitment: Hash,
    pub proof: Vec<u8>,
}
impl ExternalDepositProof {
    pub fn signing_bytes(&self)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-EXTERNAL-DEPOSIT-PROOF-V1");o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.source_chain_id.to_le_bytes());o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.event_index.to_le_bytes());o.extend_from_slice(&self.block_or_slot.to_le_bytes());o.extend_from_slice(&self.confirmations.to_le_bytes());o.extend_from_slice(&self.commitment);o.extend_from_slice(&(self.proof.len() as u64).to_le_bytes());o.extend_from_slice(&self.proof);o}
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalProofError { InvalidBinding, InvalidCommitment, InvalidProof, InsufficientFinality }
pub trait ExternalProofVerifier {
    fn verify(&self, proof:&ExternalDepositProof, descriptor:&BridgeAdapterDescriptor)->Result<(),ExternalProofError>;
}
pub struct CommitmentProofVerifier;
impl ExternalProofVerifier for CommitmentProofVerifier {
    fn verify(&self,p:&ExternalDepositProof,d:&BridgeAdapterDescriptor)->Result<(),ExternalProofError>{
        if p.bridge_id!=d.bridge_id||p.source_chain_id!=d.chain_id||p.tx_hash==[0;32]||p.proof.is_empty(){return Err(ExternalProofError::InvalidBinding);}
        if p.confirmations<d.required_confirmations{return Err(ExternalProofError::InsufficientFinality);}
        if p.commitment==[0;32]||p.proof_hash()==[0;32]{return Err(ExternalProofError::InvalidCommitment);}
        let expected=freedom_primitives::hash_bytes(&[p.proof_hash().as_slice(),d.bridge_id.as_slice()].concat());
        if expected==[0;32]{return Err(ExternalProofError::InvalidProof);}
        Ok(())
    }
}

pub mod bridge_adapter {
 use super::*;
 #[derive(Clone,Debug,PartialEq,Eq)] pub enum BridgeAdapterError { InvalidDescriptor,InvalidProof,InsufficientFinality,InvalidIntent }
 pub trait BridgeAdapter { fn descriptor(&self)->&BridgeAdapterDescriptor; fn verify_deposit(&self,p:&BridgeDepositProof)->Result<(),BridgeAdapterError>; fn build_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,recipient:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>; }
 fn make_intent(d:&BridgeAdapterDescriptor,w:Hash,a:Hash,amount:u128,r:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{if amount==0||r.is_empty(){return Err(BridgeAdapterError::InvalidIntent)}let mut x=BridgeWithdrawalIntent{withdrawal_id:w,bridge_id:d.bridge_id,destination_chain_id:d.chain_id,asset_id:a,amount,recipient:r,nonce,intent_hash:[0;32]};x.intent_hash=x.compute_hash();Ok(x)}
 macro_rules! adapter {($name:ident,$kind:expr,$model:expr)=>{pub struct $name{descriptor:BridgeAdapterDescriptor} impl $name{pub fn new(d:BridgeAdapterDescriptor)->Result<Self,BridgeAdapterError>{if d.kind!=$kind||d.finality_model!=$model||d.bridge_id==[0;32]||d.chain_id==0||!d.enabled||d.required_confirmations==0{return Err(BridgeAdapterError::InvalidDescriptor)}Ok(Self{descriptor:d})}}impl BridgeAdapter for $name{fn descriptor(&self)->&BridgeAdapterDescriptor{&self.descriptor}fn verify_deposit(&self,p:&BridgeDepositProof)->Result<(),BridgeAdapterError>{if p.bridge_id!=self.descriptor.bridge_id||p.source_chain_id!=self.descriptor.chain_id||p.confirmations<self.descriptor.required_confirmations||p.tx_hash==[0;32]||p.proof.is_empty(){return Err(if p.confirmations<self.descriptor.required_confirmations{BridgeAdapterError::InsufficientFinality}else{BridgeAdapterError::InvalidProof})}Ok(())}fn build_withdrawal_intent(&self,w:Hash,a:Hash,amount:u128,r:Vec<u8>,nonce:u64)->Result<BridgeWithdrawalIntent,BridgeAdapterError>{make_intent(&self.descriptor,w,a,amount,r,nonce)}}}}}
 adapter!(BitcoinAdapter,BridgeAdapterKind::Bitcoin,BridgeFinalityModel::Confirmations);
 adapter!(EvmAdapter,BridgeAdapterKind::Evm,BridgeFinalityModel::BlockFinality);
 adapter!(TronAdapter,BridgeAdapterKind::Tron,BridgeFinalityModel::BlockFinality);
 adapter!(TonAdapter,BridgeAdapterKind::Ton,BridgeFinalityModel::BlockFinality);
 adapter!(SolanaAdapter,BridgeAdapterKind::Solana,BridgeFinalityModel::SlotFinality);
}


#[cfg(test)]
mod m97_external_proof_tests {
 use super::*;
 #[test] fn proof_binds_security_domain(){let p=ExternalDepositProof{bridge_id:[1;32],source_chain_id:2,tx_hash:[3;32],event_index:4,block_or_slot:5,confirmations:6,commitment:[7;32],proof:vec![8]};let mut q=p.clone();q.event_index=9;assert_ne!(p.proof_hash(),q.proof_hash());}
 #[test] fn verifier_rejects_wrong_bridge_and_finality(){let d=BridgeAdapterDescriptor{bridge_id:[1;32],chain_id:2,kind:BridgeAdapterKind::Evm,finality_model:BridgeFinalityModel::BlockFinality,required_confirmations:6,native_asset:false,enabled:true};let mut p=ExternalDepositProof{bridge_id:[1;32],source_chain_id:2,tx_hash:[3;32],event_index:4,block_or_slot:5,confirmations:5,commitment:[7;32],proof:vec![8]};assert_eq!(CommitmentProofVerifier.verify(&p,&d),Err(ExternalProofError::InsufficientFinality));p.bridge_id=[9;32];p.confirmations=6;assert_eq!(CommitmentProofVerifier.verify(&p,&d),Err(ExternalProofError::InvalidBinding));}
}

#[cfg(test)]
mod m82_adapter_tests { use super::*; use super::bridge_adapter::*;
 fn d(k:BridgeAdapterKind,m:BridgeFinalityModel)->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[9;32],chain_id:1,kind:k,finality_model:m,required_confirmations:2,native_asset:false,enabled:true}}
 #[test] fn major_adapters_are_distinct(){assert!(BitcoinAdapter::new(d(BridgeAdapterKind::Bitcoin,BridgeFinalityModel::Confirmations)).is_ok());assert!(EvmAdapter::new(d(BridgeAdapterKind::Evm,BridgeFinalityModel::BlockFinality)).is_ok());assert!(TronAdapter::new(d(BridgeAdapterKind::Tron,BridgeFinalityModel::BlockFinality)).is_ok());assert!(TonAdapter::new(d(BridgeAdapterKind::Ton,BridgeFinalityModel::BlockFinality)).is_ok());assert!(SolanaAdapter::new(d(BridgeAdapterKind::Solana,BridgeFinalityModel::SlotFinality)).is_ok());}
 #[test] fn bitcoin_requires_confirmations(){let a=BitcoinAdapter::new(d(BridgeAdapterKind::Bitcoin,BridgeFinalityModel::Confirmations)).unwrap();let p=BridgeDepositProof{bridge_id:[9;32],source_chain_id:1,tx_hash:[1;32],event_index:0,block_or_slot:10,confirmations:1,proof:vec![1]};assert_eq!(a.verify_deposit(&p),Err(BridgeAdapterError::InsufficientFinality));}
 #[test] fn withdrawal_intent_is_canonical(){let a=EvmAdapter::new(d(BridgeAdapterKind::Evm,BridgeFinalityModel::BlockFinality)).unwrap();let x=a.build_withdrawal_intent([1;32],[2;32],100,b"0xrecipient".to_vec(),7).unwrap();assert!(x.validate_hash());}
}

#[cfg(test)]
mod m83_bitcoin_tests { use super::*; use super::bridge_adapter::*;
 fn d()->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[9;32],chain_id:8332,kind:BridgeAdapterKind::Bitcoin,finality_model:BridgeFinalityModel::Confirmations,required_confirmations:6,native_asset:true,enabled:true}}
 fn p(c:u64)->BitcoinUtxoProof{BitcoinUtxoProof{bridge_id:[9;32],chain_id:8332,tx_hash:[1;32],vout:2,amount_sat:50_000,block_height:100,confirmations:c,merkle_branch:vec![[2;32]],merkle_index:0,script_pubkey:vec![0,20,1,2,3]}}
 #[test] fn bitcoin_utxo_requires_six_confirmations(){let a=BitcoinAdapter::new(d()).unwrap();assert_eq!(a.verify_utxo_deposit(&p(5)),Err(BridgeAdapterError::InsufficientFinality));assert!(a.verify_utxo_deposit(&p(6)).is_ok());}
 #[test] fn bitcoin_vout_is_replay_event_identity(){assert_eq!(p(6).event_index(),2);}
 #[test] fn bitcoin_withdrawal_uses_script_bytes(){let a=BitcoinAdapter::new(d()).unwrap();let x=a.build_utxo_withdrawal_intent([3;32],[4;32],1000,vec![0,20,9],8).unwrap();assert!(x.validate_hash());}
}

#[cfg(test)]
mod m100_bitcoin_pow_tests {
 use super::*;
 #[test] fn bits_to_target_rejects_negative_and_zero(){assert!(bitcoin_target_from_bits(0).is_none());assert!(bitcoin_target_from_bits(0x01800001).is_none());}
 #[test] fn pow_is_deterministic(){let h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};assert_eq!(bitcoin_pow_valid(&h),bitcoin_pow_valid(&h));}
 #[test] fn header_links_previous_block(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=a.block_hash();assert_eq!(b.prev_block_hash,a.block_hash());}
}

#[cfg(test)]
mod m98_bitcoin_merkle_tests {
 use super::*;
 #[test] fn merkle_root_changes_with_index(){let tx=[1;32];let a=[2;32];let b=[3;32];let x=bitcoin_merkle_root(tx,&[a,b],0);let y=bitcoin_merkle_root(tx,&[a,b],1);assert_ne!(x,y);}
 #[test] fn merkle_proof_verifies(){let tx=[1;32];let branch=vec![[2;32],[3;32]];let root=bitcoin_merkle_root(tx,&branch,0);let h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:root,time:1,bits:1,nonce:1};let p=BitcoinMerkleProof{bridge_id:[9;32],chain_id:8332,tx_hash:tx,block_height:100,confirmations:6,merkle_root:root,merkle_branch:branch,merkle_index:0,block_header:h};assert!(p.verify(6));}
 #[test] fn insufficient_confirmations_fail(){let tx=[1;32];let branch=vec![[2;32]];let root=bitcoin_merkle_root(tx,&branch,0);let h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:root,time:1,bits:1,nonce:1};let p=BitcoinMerkleProof{bridge_id:[9;32],chain_id:8332,tx_hash:tx,block_height:100,confirmations:5,merkle_root:root,merkle_branch:branch,merkle_index:0,block_header:h};assert!(!p.verify(6));}
 #[test] fn header_merkle_root_is_bound(){let tx=[1;32];let branch=vec![[2;32]];let root=bitcoin_merkle_root(tx,&branch,0);let mut h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:root,time:1,bits:1,nonce:1};let mut p=BitcoinMerkleProof{bridge_id:[9;32],chain_id:8332,tx_hash:tx,block_height:100,confirmations:6,merkle_root:root,merkle_branch:branch,merkle_index:0,block_header:h};assert!(p.verify(6));h.merkle_root=[8;32];p.block_header=h;assert!(!p.verify(6));}
}

#[cfg(test)]
mod m84_evm_tests { use super::*; use super::bridge_adapter::*;
 fn d()->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[7;32],chain_id:1,kind:BridgeAdapterKind::Evm,finality_model:BridgeFinalityModel::BlockFinality,required_confirmations:12,native_asset:false,enabled:true}}
 fn p(c:u64)->EvmTransferProof{EvmTransferProof{bridge_id:[7;32],chain_id:1,tx_hash:[1;32],block_number:200,confirmations:c,token_contract:vec![0xaa;20],from:vec![0xbb;20],to:vec![0xcc;20],amount:1_000_000,log_index:3,event_signature:[4;32],proof:vec![5]}}
 #[test] fn erc20_transfer_requires_finality(){let a=EvmAdapter::new(d()).unwrap();assert_eq!(a.verify_erc20_transfer(&p(11)),Err(BridgeAdapterError::InsufficientFinality));assert!(a.verify_erc20_transfer(&p(12)).is_ok());}
 #[test] fn log_index_is_event_identity(){assert_eq!(p(12).event_index(),3);}
 #[test] fn erc20_recipient_is_20_bytes(){let a=EvmAdapter::new(d()).unwrap();assert!(a.build_erc20_withdrawal_intent([1;32],[2;32],10,vec![3;20],1).is_ok());assert_eq!(a.build_erc20_withdrawal_intent([1;32],[2;32],10,vec![3;19],1),Err(BridgeAdapterError::InvalidIntent));}
}

#[cfg(test)]
mod m85_tron_tests { use super::*; use super::bridge_adapter::*;
 fn d()->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[8;32],chain_id:728126428,kind:BridgeAdapterKind::Tron,finality_model:BridgeFinalityModel::BlockFinality,required_confirmations:20,native_asset:false,enabled:true}}
 fn p(c:u64)->TronTrc20TransferProof{TronTrc20TransferProof{bridge_id:[8;32],chain_id:728126428,tx_hash:[1;32],block_number:500,confirmations:c,token_contract:vec![0xaa;34],from:vec![0xbb;34],to:vec![0xcc;34],amount:1_000_000,event_index:4,event_signature:[5;32],proof:vec![6]}}
 #[test] fn trc20_transfer_requires_finality(){let a=TronAdapter::new(d()).unwrap();assert_eq!(a.verify_trc20_transfer(&p(19)),Err(BridgeAdapterError::InsufficientFinality));assert!(a.verify_trc20_transfer(&p(20)).is_ok());}
 #[test] fn event_index_is_replay_identity(){assert_eq!(p(20).replay_event_index(),4);}
 #[test] fn trc20_recipient_is_canonical_length(){let a=TronAdapter::new(d()).unwrap();assert!(a.build_trc20_withdrawal_intent([1;32],[2;32],10,vec![3;34],1).is_ok());assert_eq!(a.build_trc20_withdrawal_intent([1;32],[2;32],10,vec![3;33],1),Err(BridgeAdapterError::InvalidIntent));}
}

#[cfg(test)]
mod m86_ton_tests { use super::*; use super::bridge_adapter::*;
 fn d()->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[10;32],chain_id:239,kind:BridgeAdapterKind::Ton,finality_model:BridgeFinalityModel::BlockFinality,required_confirmations:5,native_asset:false,enabled:true}}
 fn p(c:u64)->TonJettonTransferProof{TonJettonTransferProof{bridge_id:[10;32],chain_id:239,tx_hash:[1;32],block_or_seqno:900,confirmations:c,jetton_master:vec![0xaa;32],sender:vec![0xbb;36],recipient:vec![0xcc;36],amount:1000,event_index:2,event_signature:[4;32],proof:vec![5]}}
 #[test] fn jetton_transfer_requires_finality(){let a=TonAdapter::new(d()).unwrap();assert_eq!(a.verify_jetton_transfer(&p(4)),Err(BridgeAdapterError::InsufficientFinality));assert!(a.verify_jetton_transfer(&p(5)).is_ok());}
 #[test] fn event_index_is_replay_identity(){assert_eq!(p(5).replay_event_index(),2);}
 #[test] fn jetton_withdrawal_preserves_address_bytes(){let a=TonAdapter::new(d()).unwrap();let x=a.build_jetton_withdrawal_intent([1;32],[2;32],10,vec![3;36],1).unwrap();assert!(x.validate_hash());}
}

#[cfg(test)]
mod m87_solana_tests { use super::*; use super::bridge_adapter::*;
 fn d()->BridgeAdapterDescriptor{BridgeAdapterDescriptor{bridge_id:[11;32],chain_id:101,kind:BridgeAdapterKind::Solana,finality_model:BridgeFinalityModel::SlotFinality,required_confirmations:32,native_asset:false,enabled:true}}
 fn p(c:u64)->SolanaTransferProof{SolanaTransferProof{bridge_id:[11;32],chain_id:101,tx_hash:[1;32],slot:1000,confirmations:c,token_mint:vec![0xaa;32],source:vec![0xbb;32],destination:vec![0xcc;32],amount:1000,instruction_index:6,program_id:vec![0xdd;32],proof:vec![7]}}
 #[test] fn spl_transfer_requires_slot_finality(){let a=SolanaAdapter::new(d()).unwrap();assert_eq!(a.verify_spl_transfer(&p(31)),Err(BridgeAdapterError::InsufficientFinality));assert!(a.verify_spl_transfer(&p(32)).is_ok());}
 #[test] fn instruction_index_is_replay_identity(){assert_eq!(p(32).replay_event_index(),6);}
 #[test] fn spl_withdrawal_requires_32_byte_recipient(){let a=SolanaAdapter::new(d()).unwrap();assert!(a.build_spl_withdrawal_intent([1;32],[2;32],10,vec![3;32],1).is_ok());assert_eq!(a.build_spl_withdrawal_intent([1;32],[2;32],10,vec![3;31],1),Err(BridgeAdapterError::InvalidIntent));}
}

#[cfg(test)]
mod m90_bridge_signature_tests {
 use super::*;
 use ed25519_dalek::{Signer,SigningKey};
 use std::collections::BTreeMap;
 fn approval(key:&SigningKey)->BridgeApproval{
  let public=key.verifying_key().to_bytes();
  let mut a=BridgeApproval{bridge_id:[1;32],request_id:[2;32],withdrawal_id:[3;32],validator_id:freedom_primitives::hash_bytes(&public),nonce:4,amount:100,destination_chain_id:1,asset_id:[5;32],intent_hash:[6;32],signature:Vec::new()};
  a.signature=key.sign(&a.signing_message()).to_bytes().to_vec(); a
 }
 #[test] fn authenticated_approval_requires_matching_validator_identity(){let k=SigningKey::from_bytes(&[7;32]);let a=approval(&k);assert!(a.verify_authenticated(&k.verifying_key().to_bytes()));let other=SigningKey::from_bytes(&[8;32]);assert!(!a.verify_authenticated(&other.verifying_key().to_bytes()));}
 #[test] fn tampered_approval_signature_fails(){let k=SigningKey::from_bytes(&[7;32]);let mut a=approval(&k);a.amount+=1;assert!(!a.verify_authenticated(&k.verifying_key().to_bytes()));}
 #[test] fn quorum_counts_only_authenticated_signers(){let k1=SigningKey::from_bytes(&[1;32]);let k2=SigningKey::from_bytes(&[2;32]);let k3=SigningKey::from_bytes(&[3;32]);let a1=approval(&k1);let a2=approval(&k2);let mut bad=approval(&k3);bad.signature[0]^=1;let mut keys=BTreeMap::new();for k in [&k1,&k2,&k3]{let p=k.verifying_key().to_bytes();keys.insert(freedom_primitives::hash_bytes(&p),p);}let set=BridgeApprovalSet{request_id:[2;32],approvals:vec![a1,a2,bad],threshold_signatures:0};assert_eq!(set.authenticated_approvals([1;32],[3;32],[6;32],&keys),2);}
}
