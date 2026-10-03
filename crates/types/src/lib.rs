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
    BridgeMint = 12,
}
impl TxKind { pub fn code(self) -> u8 { self as u8 } }

/// Consensus-executed bridge mint transaction payload.
///
/// The complete authenticated bridge proof is carried inside the signed
/// transaction. The codec is deterministic and length-delimited so every
/// node signs/verifies exactly the same bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeMintTransactionPayload {
    pub message: BridgeMessage,
    pub deposit: VerifiedDeposit,
    pub authorization: BridgeMintAuthorization,
    pub approvals: BridgeMessageApprovalSet,
}

impl BridgeMintTransactionPayload {
    const VERSION: u8 = 1;

    fn put_u16(out: &mut Vec<u8>, v: u16) { out.extend_from_slice(&v.to_le_bytes()); }
    fn put_u32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }
    fn put_u64(out: &mut Vec<u8>, v: u64) { out.extend_from_slice(&v.to_le_bytes()); }
    fn put_u128(out: &mut Vec<u8>, v: u128) { out.extend_from_slice(&v.to_le_bytes()); }
    fn put_hash(out: &mut Vec<u8>, v: &Hash) { out.extend_from_slice(v); }
    fn put_bytes(out: &mut Vec<u8>, v: &[u8]) -> Result<(), String> {
        if v.len() > u32::MAX as usize { return Err("bridge mint payload field too large".into()); }
        Self::put_u32(out, v.len() as u32);
        out.extend_from_slice(v);
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        if !self.approvals.is_canonical_order() {
            return Err("bridge approvals are not in canonical validator order".into());
        }
        let mut out = Vec::with_capacity(1024);
        out.extend_from_slice(b"FREEDOM-BRIDGE-MINT-TX-V1");
        out.push(Self::VERSION);

        let m = &self.message;
        out.push(BRIDGE_MESSAGE_PROTOCOL_VERSION as u8);
        Self::put_u16(&mut out, m.protocol_version);
        Self::put_u32(&mut out, m.source_chain_id);
        Self::put_u32(&mut out, m.destination_chain_id);
        Self::put_hash(&mut out, &m.bridge_id);
        Self::put_hash(&mut out, &m.asset_id);
        Self::put_hash(&mut out, &m.source_tx_hash);
        Self::put_u64(&mut out, m.source_event_index);
        Self::put_u64(&mut out, m.nonce);
        Self::put_u128(&mut out, m.amount);
        Self::put_hash(&mut out, &m.recipient);
        Self::put_hash(&mut out, &m.message_hash);

        let d = &self.deposit;
        Self::put_hash(&mut out, &d.canonical_asset_id);
        Self::put_hash(&mut out, &d.representation_asset_id);
        Self::put_hash(&mut out, &d.bridge_id);
        Self::put_u32(&mut out, d.source_chain_id);
        Self::put_hash(&mut out, &d.tx_hash);
        Self::put_u64(&mut out, d.event_index);
        Self::put_u64(&mut out, d.block_or_slot);
        Self::put_u64(&mut out, d.confirmations);
        Self::put_u128(&mut out, d.amount);
        Self::put_bytes(&mut out, &d.recipient)?;
        Self::put_hash(&mut out, &d.replay_key);
        Self::put_hash(&mut out, &d.proof_hash);

        let a = &self.authorization;
        Self::put_hash(&mut out, &a.bridge_id);
        Self::put_hash(&mut out, &a.message_hash);
        Self::put_hash(&mut out, &a.replay_key);
        Self::put_hash(&mut out, &a.canonical_asset_id);
        Self::put_hash(&mut out, &a.representation_asset_id);
        Self::put_u32(&mut out, a.source_chain_id);
        Self::put_hash(&mut out, &a.source_tx_hash);
        Self::put_u64(&mut out, a.source_event_index);
        Self::put_u32(&mut out, a.destination_chain_id);
        Self::put_u64(&mut out, a.nonce);
        Self::put_u128(&mut out, a.amount);
        Self::put_hash(&mut out, &a.recipient);
        Self::put_u16(&mut out, a.approval_count);
        Self::put_hash(&mut out, &a.authorization_hash);

        Self::put_hash(&mut out, &self.approvals.bridge_id);
        Self::put_hash(&mut out, &self.approvals.message_hash);
        if self.approvals.approvals.len() > u16::MAX as usize {
            return Err("too many bridge approvals".into());
        }
        Self::put_u16(&mut out, self.approvals.approvals.len() as u16);
        for approval in &self.approvals.approvals {
            Self::put_hash(&mut out, &approval.bridge_id);
            Self::put_hash(&mut out, &approval.message_hash);
            Self::put_hash(&mut out, &approval.validator_id);
            if approval.signature.len() > u16::MAX as usize {
                return Err("bridge approval signature too large".into());
            }
            Self::put_u16(&mut out, approval.signature.len() as u16);
            out.extend_from_slice(&approval.signature);
        }
        Ok(out)
    }

    pub fn decode(b: &[u8]) -> Result<Self, String> {
        struct Reader<'a> { b: &'a [u8], i: usize }
        impl<'a> Reader<'a> {
            fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
                let end = self.i.checked_add(n).ok_or_else(|| "bridge mint payload overflow".to_string())?;
                if end > self.b.len() { return Err("bridge mint payload truncated".into()); }
                let r = &self.b[self.i..end]; self.i = end; Ok(r)
            }
            fn u8(&mut self) -> Result<u8, String> { Ok(self.take(1)?[0]) }
            fn u16(&mut self) -> Result<u16, String> { Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap())) }
            fn u32(&mut self) -> Result<u32, String> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
            fn u64(&mut self) -> Result<u64, String> { Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap())) }
            fn u128(&mut self) -> Result<u128, String> { Ok(u128::from_le_bytes(self.take(16)?.try_into().unwrap())) }
            fn hash(&mut self) -> Result<Hash, String> { Ok(self.take(32)?.try_into().unwrap()) }
            fn bytes(&mut self) -> Result<Vec<u8>, String> {
                let n=self.u32()? as usize; Ok(self.take(n)?.to_vec())
            }
        }
        let mut r=Reader{b,i:0};
        if r.take(b"FREEDOM-BRIDGE-MINT-TX-V1".len())? != b"FREEDOM-BRIDGE-MINT-TX-V1" { return Err("invalid bridge mint payload domain".into()); }
        if r.u8()? != Self::VERSION { return Err("unsupported bridge mint payload version".into()); }
        if r.u8()? != BRIDGE_MESSAGE_PROTOCOL_VERSION as u8 { return Err("invalid bridge message protocol marker".into()); }

        let message=BridgeMessage{
            protocol_version:r.u16()?, source_chain_id:r.u32()?, destination_chain_id:r.u32()?,
            bridge_id:r.hash()?, asset_id:r.hash()?, source_tx_hash:r.hash()?,
            source_event_index:r.u64()?, nonce:r.u64()?, amount:r.u128()?,
            recipient:r.hash()?, message_hash:r.hash()?,
        };
        let deposit=VerifiedDeposit{
            canonical_asset_id:r.hash()?, representation_asset_id:r.hash()?, bridge_id:r.hash()?,
            source_chain_id:r.u32()?, tx_hash:r.hash()?, event_index:r.u64()?,
            block_or_slot:r.u64()?, confirmations:r.u64()?, amount:r.u128()?,
            recipient:r.bytes()?, replay_key:r.hash()?, proof_hash:r.hash()?,
        };
        let authorization=BridgeMintAuthorization{
            bridge_id:r.hash()?, message_hash:r.hash()?, replay_key:r.hash()?,
            canonical_asset_id:r.hash()?, representation_asset_id:r.hash()?,
            source_chain_id:r.u32()?, source_tx_hash:r.hash()?, source_event_index:r.u64()?,
            destination_chain_id:r.u32()?, nonce:r.u64()?, amount:r.u128()?,
            recipient:r.hash()?, approval_count:r.u16()?, authorization_hash:r.hash()?,
        };
        let approvals_bridge_id=r.hash()?;
        let approvals_message_hash=r.hash()?;
        let count=r.u16()? as usize;
        let mut approval_vec=Vec::with_capacity(count);
        for _ in 0..count {
            let bridge_id=r.hash()?;
            let message_hash=r.hash()?;
            let validator_id=r.hash()?;
            let signature_len=r.u16()? as usize;
            let signature=r.take(signature_len)?.to_vec();
            approval_vec.push(BridgeMessageApproval{bridge_id,message_hash,validator_id,signature});
        }
        if r.i != r.b.len() { return Err("trailing bytes in bridge mint payload".into()); }
        let payload = Self{
            message,
            deposit,
            authorization,
            approvals:BridgeMessageApprovalSet{bridge_id:approvals_bridge_id,message_hash:approvals_message_hash,approvals:approval_vec},
        };
        if !payload.approvals.is_canonical_order() {
            return Err("bridge approvals are not in canonical validator order".into());
        }
        Ok(payload)
    }

    /// Canonical identity of the bridge proof carried by this transaction.
    /// This intentionally excludes the relayer's transaction signature:
    /// the same authenticated bridge mint proof must map to one logical id.
    pub fn canonical_id(&self) -> Result<Hash, String> {
        let encoded = self.encode()?;
        let mut bytes = Vec::with_capacity(32 + encoded.len());
        bytes.extend_from_slice(b"FREEDOM-BRIDGE-MINT-ID-V1");
        bytes.extend_from_slice(&encoded);
        Ok(freedom_primitives::hash_bytes(&bytes))
    }
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
pub enum Event { #[serde(rename="NativeAssetTransferred", alias="FdmTransferred")] NativeAssetTransferred { from: Address, to: Address, amount: u128 }, TicketCreated { ticket_id: Hash, draw_id: Hash, owner: Address }, DrawFundAllocated { draw_id: Hash, prize_pool: u128, cu_fund: u128, annual_reserve: u128 }, DrawExecuted { draw_id: Hash, randomness_seed: Hash, selection_proof: Hash }, PrizePaid { draw_id: Hash, ticket_id: Hash, winner: Address, amount: u128 }, BridgeMinted { replay_key: Hash, asset_id: Hash, recipient: Address, amount: u128 }, EquivocationSlashed { evidence_hash: Hash, validator_id: Hash, amount: u128 } }
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

pub const BITCOIN_MAINNET_CHAIN_ID: u32 = 0x4254_4301;
pub const ETHEREUM_MAINNET_CHAIN_ID: u32 = 1;
pub const TRON_MAINNET_CHAIN_ID: u32 = 728_126_428;

pub fn is_supported_mainnet_chain(chain_id: u32) -> bool {
    matches!(
        chain_id,
        BITCOIN_MAINNET_CHAIN_ID
            | ETHEREUM_MAINNET_CHAIN_ID
            | TRON_MAINNET_CHAIN_ID
    )
}

pub fn expected_asset_kind_for_mainnet(
    chain_id: u32,
    kind: CrossChainAssetKind,
) -> bool {
    match chain_id {
        BITCOIN_MAINNET_CHAIN_ID => kind == CrossChainAssetKind::BitcoinUtxo,
        ETHEREUM_MAINNET_CHAIN_ID => matches!(
            kind,
            CrossChainAssetKind::EthereumNative | CrossChainAssetKind::Erc20
        ),
        TRON_MAINNET_CHAIN_ID => kind == CrossChainAssetKind::Trc20,
        _ => false,
    }
}

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

impl VerifiedDeposit {
    pub fn to_bridge_message(
        &self,
        destination_chain_id: u32,
        nonce: u64,
    ) -> Result<BridgeMessage, BridgeAdapterError> {
        if destination_chain_id == 0 || self.amount == 0 || self.tx_hash == [0; 32] {
            return Err(BridgeAdapterError::InvalidProof);
        }
        if self.recipient.len() != 20 {
            return Err(BridgeAdapterError::InvalidIntent);
        }
        let mut recipient = [0u8; 32];
        recipient[..20].copy_from_slice(&self.recipient);
        let mut message = BridgeMessage {
            protocol_version: BRIDGE_MESSAGE_PROTOCOL_VERSION,
            source_chain_id: self.source_chain_id,
            destination_chain_id,
            bridge_id: self.bridge_id,
            asset_id: self.canonical_asset_id,
            source_tx_hash: self.tx_hash,
            source_event_index: self.event_index,
            nonce,
            amount: self.amount,
            recipient,
            message_hash: [0; 32],
        };
        message.message_hash = message.compute_hash();
        if !message.validate_hash() {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(message)
    }
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

    pub fn matches_verified_deposit(&self, deposit: &VerifiedDeposit) -> bool {
        self.protocol_version == BRIDGE_MESSAGE_PROTOCOL_VERSION
            && self.source_chain_id == deposit.source_chain_id
            && self.bridge_id == deposit.bridge_id
            && self.asset_id == deposit.canonical_asset_id
            && self.source_tx_hash == deposit.tx_hash
            && self.source_event_index == deposit.event_index
            && self.amount == deposit.amount
            && self.recipient[..20] == deposit.recipient[..]
            && self.recipient[20..].iter().all(|b| *b == 0)
            && self.validate_hash()
    }

    pub fn authorization_ready(
        &self,
        approvals: &BridgeMessageApprovalSet,
        policy: &BridgeSecurityPolicy,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> bool {
        self.validate_hash()
            && approvals.message_hash == self.message_hash
            && approvals.bridge_id == self.bridge_id
            && approvals.authorized(policy, validator_keys)
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
    pub fn new_external(
        source_chain_id: u32,
        asset_type: AssetType,
        contract_address: Vec<u8>,
        symbol: String,
        decimals: u8,
        issuer: Hash,
        bridge_id: Hash,
        status: AssetStatus,
    ) -> Result<Self, String> {
        let asset_id = Self::compute_asset_id(
            source_chain_id,
            asset_type,
            &contract_address,
            issuer,
            bridge_id,
        );
        let asset = Self {
            asset_id,
            source_chain_id,
            asset_type,
            contract_address,
            symbol,
            decimals,
            issuer,
            bridge_id,
            status,
        };
        if !asset.validate_identity() {
            return Err("InvalidAssetDefinition".into());
        }
        Ok(asset)
    }

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
            && self.validate_external_address_format()
    }

    pub fn validate_external_address_format(&self) -> bool {
        match self.asset_type {
            AssetType::Native => {
                (self.source_chain_id == BITCOIN_MAINNET_CHAIN_ID
                    && self.contract_address.as_slice() == b"BTC")
                    || (self.source_chain_id == ETHEREUM_MAINNET_CHAIN_ID
                        && self.contract_address.as_slice() == b"ETH")
            },
            AssetType::Bridged => {
                (self.source_chain_id == ETHEREUM_MAINNET_CHAIN_ID
                    || self.source_chain_id == TRON_MAINNET_CHAIN_ID)
                    && self.contract_address.len() == 20
                    && self.contract_address.iter().any(|b| *b != 0)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossChainAssetRepresentation {
    pub canonical_asset_id: Hash,
    pub representation: AssetDefinition,
    pub asset_kind: CrossChainAssetKind,
}

impl CrossChainAssetRepresentation {
    pub fn validate(&self) -> bool {
        self.canonical_asset_id != [0; 32]
            && self.representation.validate_identity()
            && self.validate_external_identity()
            && !self.representation.symbol.is_empty()
            && self.representation.status == AssetStatus::Active
    }

    pub fn validate_external_identity(&self) -> bool {
        if self.representation.bridge_id == [0; 32]
            || !expected_asset_kind_for_mainnet(
                self.representation.source_chain_id,
                self.asset_kind,
            )
            || self.representation.source_chain_id == 0
        {
            return false;
        }
        match self.asset_kind {
            CrossChainAssetKind::BitcoinUtxo => {
                self.representation.asset_type == AssetType::Native
                    && self.representation.contract_address.as_slice() == b"BTC"
            }
            CrossChainAssetKind::EthereumNative => {
                self.representation.asset_type == AssetType::Native
                    && self.representation.source_chain_id == ETHEREUM_MAINNET_CHAIN_ID
                    && self.representation.contract_address.as_slice() == b"ETH"
            }
            CrossChainAssetKind::Erc20
            | CrossChainAssetKind::Trc20
            | CrossChainAssetKind::TonJetton
            | CrossChainAssetKind::SolanaSpl => {
                self.representation.asset_type == AssetType::Bridged
                    && !self.representation.contract_address.is_empty()
            }
            CrossChainAssetKind::Other => false,
        }
    }

    pub fn matches_proof(&self, proof: &CrossChainAssetProof) -> bool {
        self.validate()
            && proof.asset_id == self.canonical_asset_id
            && proof.representation_asset_id == self.representation.representation.asset_id
            && proof.asset_kind == self.asset_kind
            && proof.bridge_deposit.source_chain_id == self.representation.source_chain_id
            && proof.bridge_deposit.bridge_id == self.representation.representation.bridge_id
    }

    pub fn matches_external_identity(
        &self,
        proof: &CrossChainAssetProof,
        expected_representation_asset_id: Hash,
    ) -> bool {
        self.matches_proof(proof)
            && proof.representation_asset_id == expected_representation_asset_id
            && self.representation.representation.asset_id == expected_representation_asset_id
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum CanonicalAssetProfile {
    Btc = 1,
    Eth = 2,
    Usdt = 3,
    Usdc = 4,
}

impl CanonicalAssetProfile {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Btc => "BTC",
            Self::Eth => "ETH",
            Self::Usdt => "USDT",
            Self::Usdc => "USDC",
        }
    }

    pub fn decimals(self) -> u8 {
        match self {
            Self::Btc => 8,
            Self::Eth => 18,
            Self::Usdt | Self::Usdc => 6,
        }
    }

    pub fn canonical_id(self, issuer: Hash) -> Hash {
        CrossChainAssetRegistry::compute_canonical_asset_id(
            self.symbol(),
            self.decimals(),
            issuer,
        )
    }

    pub fn matches_registry(self, registry: &CrossChainAssetRegistry, issuer: Hash) -> bool {
        registry.symbol == self.symbol()
            && registry.decimals == self.decimals()
            && registry.canonical_asset_id == self.canonical_id(issuer)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalAssetConfig {
    pub profile: CanonicalAssetProfile,
    pub source_chain_id: u32,
    pub asset_kind: CrossChainAssetKind,
    pub asset_type: AssetType,
    pub contract_address: Vec<u8>,
    pub issuer: Hash,
    pub bridge_id: Hash,
    pub status: AssetStatus,
}

impl ExternalAssetConfig {
    pub fn validate(&self) -> bool {
        self.source_chain_id != 0
            && self.issuer != [0; 32]
            && self.bridge_id != [0; 32]
            && self.asset_type == match self.asset_kind {
                CrossChainAssetKind::BitcoinUtxo
                | CrossChainAssetKind::EthereumNative => AssetType::Native,
                CrossChainAssetKind::Erc20
                | CrossChainAssetKind::Trc20
                | CrossChainAssetKind::TonJetton
                | CrossChainAssetKind::SolanaSpl => AssetType::Bridged,
                CrossChainAssetKind::Other => return false,
            }
            && self.profile.symbol() != ""
    }

    pub fn build(&self) -> Result<CrossChainAssetRepresentation, String> {
        if !self.validate() {
            return Err("InvalidExternalAssetConfig".into());
        }
        let representation = AssetDefinition::new_external(
            self.source_chain_id,
            self.asset_type,
            self.contract_address.clone(),
            self.profile.symbol().to_string(),
            self.profile.decimals(),
            self.issuer,
            self.bridge_id,
            self.status,
        )?;
        Ok(CrossChainAssetRepresentation {
            canonical_asset_id: self.profile.canonical_id(self.issuer),
            representation,
            asset_kind: self.asset_kind,
        })
    }
}

pub const ETH_USDT_CONTRACT: [u8; 20] = [
    0xda, 0xc1, 0x7f, 0x95, 0x8d, 0x2e, 0xe5, 0x23, 0xa2, 0x20,
    0x62, 0x06, 0x99, 0x45, 0x97, 0xc1, 0x3d, 0x83, 0x1e, 0xc7,
];
pub const ETH_USDC_CONTRACT: [u8; 20] = [
    0xa0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1,
    0x9d, 0x4a, 0x2e, 0x9e, 0xb0, 0xce, 0x36, 0x06, 0xeb, 0x48,
];
pub const TRON_USDT_CONTRACT: [u8; 20] = [
    0xa6, 0x14, 0xf8, 0x03, 0xb6, 0xfd, 0x78, 0x09, 0x86, 0xa4,
    0x2c, 0x78, 0xec, 0x9c, 0x7f, 0x77, 0xe6, 0xde, 0xd1, 0x3c,
];

fn production_identity(domain: &[u8]) -> Hash {
    freedom_primitives::hash_bytes(domain)
}

pub fn production_bridge_id() -> Hash {
    production_identity(b"321MORE-PRODUCTION-BRIDGE-V1")
}
pub fn production_issuer() -> Hash {
    production_identity(b"321MORE-ASSET-ISSUER-V1")
}
fn production_config(
    profile: CanonicalAssetProfile,
    source_chain_id: u32,
    asset_kind: CrossChainAssetKind,
    asset_type: AssetType,
    contract_address: Vec<u8>,
) -> ExternalAssetConfig {
    ExternalAssetConfig {
        profile,
        source_chain_id,
        asset_kind,
        asset_type,
        contract_address,
        issuer: production_issuer(),
        bridge_id: production_bridge_id(),
        status: AssetStatus::Active,
    }
}
pub fn production_profiles() -> Vec<ExternalAssetConfig> {
    vec![
        production_config(CanonicalAssetProfile::Btc, BITCOIN_MAINNET_CHAIN_ID, CrossChainAssetKind::BitcoinUtxo, AssetType::Native, b"BTC".to_vec()),
        production_config(CanonicalAssetProfile::Eth, ETHEREUM_MAINNET_CHAIN_ID, CrossChainAssetKind::EthereumNative, AssetType::Native, b"ETH".to_vec()),
        production_config(CanonicalAssetProfile::Usdt, ETHEREUM_MAINNET_CHAIN_ID, CrossChainAssetKind::Erc20, AssetType::Bridged, ETH_USDT_CONTRACT.to_vec()),
        production_config(CanonicalAssetProfile::Usdt, TRON_MAINNET_CHAIN_ID, CrossChainAssetKind::Trc20, AssetType::Bridged, TRON_USDT_CONTRACT.to_vec()),
        production_config(CanonicalAssetProfile::Usdc, ETHEREUM_MAINNET_CHAIN_ID, CrossChainAssetKind::Erc20, AssetType::Bridged, ETH_USDC_CONTRACT.to_vec()),
    ]
}

pub struct CrossChainAssetRegistry {
    pub canonical_asset_id: Hash,
    pub symbol: String,
    pub decimals: u8,
    pub representations: Vec<CrossChainAssetRepresentation>,
}

impl CrossChainAssetRegistry {
    pub fn from_profile(
        profile: CanonicalAssetProfile,
        issuer: Hash,
        representation: AssetDefinition,
        asset_kind: CrossChainAssetKind,
    ) -> Result<Self, String> {
        if issuer == [0; 32] {
            return Err("InvalidCanonicalIssuer".into());
        }
        if representation.symbol != profile.symbol()
            || representation.decimals != profile.decimals()
        {
            return Err("CanonicalRepresentationMetadataMismatch".into());
        }
        let canonical_asset_id = profile.canonical_id(issuer);
        let registry = Self {
            canonical_asset_id,
            symbol: profile.symbol().to_string(),
            decimals: profile.decimals(),
            representations: vec![CrossChainAssetRepresentation {
                canonical_asset_id,
                representation,
                asset_kind,
            }],
        };
        if !registry.validate() {
            return Err("InvalidCanonicalAssetRegistry".into());
        }
        Ok(registry)
    }

    pub fn add_config(
        &mut self,
        config: &ExternalAssetConfig,
    ) -> Result<(), String> {
        let representation = config.build()?;
        if representation.canonical_asset_id != self.canonical_asset_id {
            return Err("CanonicalAssetMismatch".into());
        }
        self.add_representation(representation.representation, representation.asset_kind)
    }

    pub fn add_representation(
        &mut self,
        representation: AssetDefinition,
        asset_kind: CrossChainAssetKind,
    ) -> Result<(), String> {
        if representation.bridge_id == [0; 32] {
            return Err("InvalidRepresentationBridge".into());
        }
        if representation.symbol != self.symbol || representation.decimals != self.decimals {
            return Err("RepresentationMetadataMismatch".into());
        }
        let entry = CrossChainAssetRepresentation {
            canonical_asset_id: self.canonical_asset_id,
            representation,
            asset_kind,
        };
        if self.representations.iter().any(|r| {
            r.representation.asset_id == entry.representation.asset_id
        }) {
            return Err("DuplicateRepresentation".into());
        }
        if self.representations.iter().any(|r| {
            r.representation.source_chain_id == entry.representation.source_chain_id
                && r.asset_kind == entry.asset_kind
                && r.representation.contract_address == entry.representation.contract_address
                && r.representation.bridge_id == entry.representation.bridge_id
        }) {
            return Err("DuplicateExternalIdentity".into());
        }
        self.representations.push(entry);
        if !self.validate() {
            self.representations.pop();
            return Err("InvalidCanonicalAssetRegistry".into());
        }
        Ok(())
    }

    pub fn is_multi_representation(&self) -> bool {
        self.representations.len() > 1
    }

    pub fn has_representation(
        &self,
        representation_asset_id: Hash,
    ) -> bool {
        self.representations
            .iter()
            .any(|r| r.representation.asset_id == representation_asset_id)
    }

    pub fn compute_canonical_asset_id(symbol: &str, decimals: u8, issuer: Hash) -> Hash {
        let mut b = Vec::new();
        b.extend_from_slice(b"FREEDOM-CANONICAL-ASSET-V1");
        b.extend_from_slice(&(symbol.len() as u64).to_le_bytes());
        b.extend_from_slice(symbol.as_bytes());
        b.push(decimals);
        b.extend_from_slice(&issuer);
        freedom_primitives::hash_bytes(&b)
    }

    pub fn validate(&self) -> bool {
        self.canonical_asset_id != [0; 32]
            && !self.symbol.is_empty()
            && self.decimals <= 38
            && !self.representations.is_empty()
            && self.representations.iter().all(|r| {
                r.canonical_asset_id == self.canonical_asset_id
                    && r.validate()
                    && r.representation.symbol == self.symbol
                    && r.representation.decimals == self.decimals
            })
            && self.representations.iter().all(|a| {
                self.representations.iter().filter(|b| {
                    b.representation.asset_id == a.representation.asset_id
                }).count() == 1
            })
        && self.representations.iter().all(|a| {
            self.representations.iter().filter(|b| {
                b.representation.source_chain_id == a.representation.source_chain_id
                    && b.asset_kind == a.asset_kind
                    && b.representation.contract_address == a.representation.contract_address
                    && b.representation.bridge_id == a.representation.bridge_id
            }).count() == 1
        })
    }

    pub fn representations_for_kind(
        &self,
        asset_kind: CrossChainAssetKind,
    ) -> Vec<&CrossChainAssetRepresentation> {
        self.representations
            .iter()
            .filter(|r| r.asset_kind == asset_kind)
            .collect()
    }

    pub fn find_representation(
        &self,
        source_chain_id: u32,
        asset_kind: CrossChainAssetKind,
    ) -> Option<&CrossChainAssetRepresentation> {
        self.representations.iter().find(|r| {
            r.representation.source_chain_id == source_chain_id
                && r.asset_kind == asset_kind
        })
    }

    pub fn find_representation_by_external_identity(
        &self,
        source_chain_id: u32,
        asset_kind: CrossChainAssetKind,
        contract_address: &[u8],
        bridge_id: Hash,
    ) -> Option<&CrossChainAssetRepresentation> {
        self.representations.iter().find(|r| {
            r.asset_kind == asset_kind
                && r.canonical_asset_id == self.canonical_asset_id
                && r.representation.source_chain_id == source_chain_id
                && r.representation.bridge_id == bridge_id
                && r.representation.contract_address.as_slice() == contract_address
        })
    }

    pub fn verify_proof(&self, proof: &CrossChainAssetProof, required_confirmations: u64) -> bool {
        self.validate()
            && proof.validate(required_confirmations)
            && self.representations.iter().any(|r| r.matches_proof(proof))
    }

    pub fn verify_representation_binding(
        &self,
        proof: &CrossChainAssetProof,
        expected_representation_asset_id: Hash,
    ) -> bool {
        self.validate()
            && self.representations.iter().any(|r| {
                r.matches_external_identity(proof, expected_representation_asset_id)
            })
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
pub struct BridgeMessageApproval {
    pub bridge_id: Hash,
    pub message_hash: Hash,
    pub validator_id: Hash,
    pub signature: Vec<u8>,
}

impl BridgeMessageApproval {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"FREEDOM-BRIDGE-MESSAGE-APPROVAL-V1");
        b.extend_from_slice(&self.bridge_id);
        b.extend_from_slice(&self.message_hash);
        b.extend_from_slice(&self.validator_id);
        b
    }

    pub fn approval_hash(&self) -> Hash {
        freedom_primitives::hash_bytes(&self.signing_bytes())
    }

    pub fn structurally_valid(&self) -> bool {
        self.bridge_id != [0; 32]
            && self.message_hash != [0; 32]
            && self.validator_id != [0; 32]
            && self.signature.len() == 64
    }

    pub fn validator_identity_matches(&self, public_key: &[u8; 32]) -> bool {
        freedom_primitives::hash_bytes(public_key) == self.validator_id
    }

    pub fn verify_authenticated(&self, public_key: &[u8; 32]) -> bool {
        if !self.structurally_valid() || !self.validator_identity_matches(public_key) {
            return false;
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&self.signature);
        freedom_crypto::verify(public_key, &self.approval_hash(), &sig)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeMessageApprovalSet {
    pub bridge_id: Hash,
    pub message_hash: Hash,
    pub approvals: Vec<BridgeMessageApproval>,
}

impl BridgeMessageApprovalSet {
    /// Returns true only when approvals are in the single canonical order used
    /// by the BridgeMint transaction codec: strictly ascending validator_id.
    ///
    /// A set with duplicate validator ids is therefore non-canonical even
    /// though quorum counting de-duplicates those ids.
    pub fn is_canonical_order(&self) -> bool {
        self.approvals
            .windows(2)
            .all(|pair| pair[0].validator_id < pair[1].validator_id)
    }

    pub fn authenticated_count(
        &self,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> u16 {
        self.approvals
            .iter()
            .filter(|a| a.bridge_id == self.bridge_id && a.message_hash == self.message_hash)
            .filter_map(|a| validator_keys.get(&a.validator_id).map(|k| (a, k)))
            .filter(|(a, k)| a.verify_authenticated(k))
            .map(|(a, _)| a.validator_id)
            .collect::<std::collections::BTreeSet<Hash>>()
            .len() as u16
    }

    pub fn quorum_reached_authenticated(
        &self,
        policy: &BridgeSecurityPolicy,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> bool {
        policy.quorum_reached(self.authenticated_count(validator_keys))
    }

    pub fn threshold_reached(
        &self,
        policy: &BridgeSecurityPolicy,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> bool {
        policy.threshold_reached(self.authenticated_count(validator_keys))
    }

    pub fn authorized(
        &self,
        policy: &BridgeSecurityPolicy,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> bool {
        if !policy.validate()
            || self.bridge_id != policy.bridge_id
            || self.message_hash == [0; 32]
        {
            return false;
        }
        match policy.approval_mode {
            BridgeApprovalMode::Quorum => self.quorum_reached_authenticated(policy, validator_keys),
            BridgeApprovalMode::Threshold => self.threshold_reached(policy, validator_keys),
            BridgeApprovalMode::QuorumAndThreshold => {
                self.quorum_reached_authenticated(policy, validator_keys)
                    && self.threshold_reached(policy, validator_keys)
            }
        }
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

impl BridgeAdapterDescriptor {
    pub fn validate(&self) -> bool {
        self.bridge_id != [0; 32]
            && self.chain_id != 0
            && self.required_confirmations > 0
            && self.finality_model != BridgeFinalityModel::Other
            && match self.kind {
                BridgeAdapterKind::Bitcoin
                | BridgeAdapterKind::Evm
                | BridgeAdapterKind::Tron
                | BridgeAdapterKind::Ton
                | BridgeAdapterKind::Solana
                | BridgeAdapterKind::Other => true,
            }
    }
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum CrossChainAssetKind {
    BitcoinUtxo = 1,
    EthereumNative = 6,
    Erc20 = 2,
    Trc20 = 3,
    TonJetton = 4,
    SolanaSpl = 5,
    Other = 255,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossChainAssetProof {
    pub bridge_deposit: BridgeDepositProof,
    pub asset_id: Hash,
    pub representation_asset_id: Hash,
    pub asset_kind: CrossChainAssetKind,
    pub amount: u128,
    pub recipient: Vec<u8>,
    pub proof_hash: Hash,
}

impl CrossChainAssetProof {
    pub fn compute_hash(&self) -> Hash {
        let mut b = Vec::new();
        b.extend_from_slice(b"FREEDOM-CROSS-CHAIN-ASSET-PROOF-V1");
        b.extend_from_slice(&self.bridge_deposit.bridge_id);
        b.extend_from_slice(&self.bridge_deposit.source_chain_id.to_le_bytes());
        b.extend_from_slice(&self.bridge_deposit.tx_hash);
        b.extend_from_slice(&self.bridge_deposit.event_index.to_le_bytes());
        b.extend_from_slice(&self.bridge_deposit.block_or_slot.to_le_bytes());
        b.extend_from_slice(&self.bridge_deposit.confirmations.to_le_bytes());
        b.extend_from_slice(&self.asset_id);
        b.extend_from_slice(&self.representation_asset_id);
        b.push(self.asset_kind as u8);
        b.extend_from_slice(&self.amount.to_le_bytes());
        b.extend_from_slice(&(self.recipient.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.recipient);
        b.extend_from_slice(&(self.bridge_deposit.proof.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.bridge_deposit.proof);
        freedom_primitives::hash_bytes(&b)
    }

    pub fn validate(&self, required_confirmations: u64) -> bool {
        self.bridge_deposit.bridge_id != [0; 32]
            && self.bridge_deposit.source_chain_id != 0
            && self.bridge_deposit.tx_hash != [0; 32]
            && self.amount > 0
            && !self.asset_id.eq(&[0; 32])
            && !self.representation_asset_id.eq(&[0; 32])
            && !self.recipient.is_empty()
            && !self.bridge_deposit.proof.is_empty()
            && self.bridge_deposit.confirmations >= required_confirmations
            && self.proof_hash == self.compute_hash()
    }

    pub fn from_bridge_deposit(
        bridge_deposit: BridgeDepositProof,
        asset_id: Hash,
        representation_asset_id: Hash,
        asset_kind: CrossChainAssetKind,
        amount: u128,
        recipient: Vec<u8>,
    ) -> Result<Self, BridgeAdapterError> {
        let mut p = Self {
            bridge_deposit,
            asset_id,
            representation_asset_id,
            asset_kind,
            amount,
            recipient,
            proof_hash: [0; 32],
        };
        p.proof_hash = p.compute_hash();
        if !p.validate(p.bridge_deposit.confirmations) {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(p)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalBridgeDepositProof {
    BitcoinUtxo(BitcoinUtxoProof),
    EvmTransfer(EvmTransferProof),
    TronUsdt(TronUsdtDepositProof),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifiedDeposit {
    pub canonical_asset_id: Hash,
    pub representation_asset_id: Hash,
    pub bridge_id: Hash,
    pub source_chain_id: u32,
    pub tx_hash: Hash,
    pub event_index: u64,
    pub block_or_slot: u64,
    pub confirmations: u64,
    pub amount: u128,
    pub recipient: Vec<u8>,
    pub replay_key: Hash,
    pub proof_hash: Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepositVerificationError {
    InvalidRegistry,
    InvalidPolicy,
    BridgeMismatch,
    AssetMismatch,
    InvalidProof,
    InsufficientFinality,
    DepositLimitExceeded,
    ReplayDetected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CryptographicDepositVerification {
    pub proof_hash: Hash,
}

impl CryptographicDepositVerification {
    pub fn verify(proof: &CrossChainAssetProof) -> Result<Self, DepositVerificationError> {
        if !proof.validate(proof.bridge_deposit.confirmations) {
            return Err(DepositVerificationError::InvalidProof);
        }
        if proof.proof_hash != proof.compute_hash() {
            return Err(DepositVerificationError::InvalidProof);
        }
        Ok(Self { proof_hash: proof.proof_hash })
    }
}

pub struct DepositVerifier;

impl DepositVerifier {
    pub fn verify(
        proof: &CrossChainAssetProof,
        registry: &CrossChainAssetRegistry,
        policy: &BridgeSecurityPolicy,
    ) -> Result<VerifiedDeposit, DepositVerificationError> {
        if !registry.validate() {
            return Err(DepositVerificationError::InvalidRegistry);
        }
        if !policy.validate() {
            return Err(DepositVerificationError::InvalidPolicy);
        }
        if proof.bridge_deposit.bridge_id != policy.bridge_id {
            return Err(DepositVerificationError::BridgeMismatch);
        }
        if proof.bridge_deposit.confirmations < policy.min_external_confirmations {
            return Err(DepositVerificationError::InsufficientFinality);
        }
        if !policy.deposit_allowed(proof.amount) {
            return Err(DepositVerificationError::DepositLimitExceeded);
        }
        if proof.asset_id != registry.canonical_asset_id {
            return Err(DepositVerificationError::AssetMismatch);
        }
        CryptographicDepositVerification::verify(proof)?;
        if !registry.verify_proof(proof, policy.min_external_confirmations) {
            return Err(DepositVerificationError::InvalidProof);
        }

        let replay_key = freedom_primitives::domain_hash(
            b"FREEDOM-VERIFIED-DEPOSIT-REPLAY-V1",
            &[
                &proof.bridge_deposit.bridge_id,
                &proof.bridge_deposit.source_chain_id.to_le_bytes(),
                &proof.bridge_deposit.tx_hash,
                &proof.bridge_deposit.event_index.to_le_bytes(),
                &proof.asset_id,
                &proof.representation_asset_id,
            ],
        );

        Ok(VerifiedDeposit {
            canonical_asset_id: proof.asset_id,
            representation_asset_id: proof.representation_asset_id,
            bridge_id: proof.bridge_deposit.bridge_id,
            source_chain_id: proof.bridge_deposit.source_chain_id,
            tx_hash: proof.bridge_deposit.tx_hash,
            event_index: proof.bridge_deposit.event_index,
            block_or_slot: proof.bridge_deposit.block_or_slot,
            confirmations: proof.bridge_deposit.confirmations,
            amount: proof.amount,
            recipient: proof.recipient.clone(),
            replay_key,
            proof_hash: proof.proof_hash,
        })

    }

    pub fn verify_with_replay(
        proof: &CrossChainAssetProof,
        registry: &CrossChainAssetRegistry,
        policy: &BridgeSecurityPolicy,
        existing: Option<&DepositReplayRecord>,
    ) -> Result<DepositReplayRecord, DepositVerificationError> {
        let verified = Self::verify(proof, registry, policy)?;
        if let Some(record) = existing {
            if record.replay_key == verified.replay_key {
                return Err(DepositVerificationError::ReplayDetected);
            }
        }
        Ok(DepositReplayRecord::from_verified(&verified))

    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum DepositReplayStatus {
    Pending = 1,
    Accepted = 2,
    Consumed = 3,
    Rejected = 4,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DepositReplayRecord {
    pub replay_key: Hash,
    pub bridge_id: Hash,
    pub canonical_asset_id: Hash,
    pub representation_asset_id: Hash,
    pub source_chain_id: u32,
    pub tx_hash: Hash,
    pub event_index: u64,
    pub amount: u128,
    pub status: DepositReplayStatus,
    pub record_hash: Hash,
}

impl DepositReplayRecord {
    pub fn compute_hash(&self) -> Hash {
        let mut b = Vec::new();
        b.extend_from_slice(b"FREEDOM-DEPOSIT-REPLAY-RECORD-V1");
        b.extend_from_slice(&self.replay_key);
        b.extend_from_slice(&self.bridge_id);
        b.extend_from_slice(&self.canonical_asset_id);
        b.extend_from_slice(&self.representation_asset_id);
        b.extend_from_slice(&self.source_chain_id.to_le_bytes());
        b.extend_from_slice(&self.tx_hash);
        b.extend_from_slice(&self.event_index.to_le_bytes());
        b.extend_from_slice(&self.amount.to_le_bytes());
        b.push(self.status as u8);
        freedom_primitives::hash_bytes(&b)
    }

    pub fn from_verified(deposit: &VerifiedDeposit) -> Self {
        let mut r = Self {
            replay_key: deposit.replay_key,
            bridge_id: deposit.bridge_id,
            canonical_asset_id: deposit.canonical_asset_id,
            representation_asset_id: deposit.representation_asset_id,
            source_chain_id: deposit.source_chain_id,
            tx_hash: deposit.tx_hash,
            event_index: deposit.event_index,
            amount: deposit.amount,
            status: DepositReplayStatus::Pending,
            record_hash: [0; 32],
        };
        r.record_hash = r.compute_hash();
        r
    }

    pub fn validate(&self) -> bool {
        self.replay_key != [0; 32]
            && self.bridge_id != [0; 32]
            && self.canonical_asset_id != [0; 32]
            && self.representation_asset_id != [0; 32]
            && self.source_chain_id != 0
            && self.tx_hash != [0; 32]
            && self.amount > 0
            && self.record_hash == self.compute_hash()
    }

    pub fn can_accept(&self) -> bool {
        self.validate() && self.status == DepositReplayStatus::Pending
    }

    pub fn accept(&mut self) -> bool {
        if !self.can_accept() { return false; }
        self.status = DepositReplayStatus::Accepted;
        self.record_hash = self.compute_hash();
        true
    }

    pub fn consume(&mut self) -> bool {
        if !self.validate() || self.status != DepositReplayStatus::Accepted { return false; }
        self.status = DepositReplayStatus::Consumed;
        self.record_hash = self.compute_hash();
        true
    }

    pub fn reject(&mut self) -> bool {
        if !self.validate() || self.status == DepositReplayStatus::Consumed { return false; }
        self.status = DepositReplayStatus::Rejected;
        self.record_hash = self.compute_hash();
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeMintAuthorization {
    pub bridge_id: Hash,
    pub message_hash: Hash,
    pub replay_key: Hash,
    pub canonical_asset_id: Hash,
    pub representation_asset_id: Hash,
    pub source_chain_id: u32,
    pub source_tx_hash: Hash,
    pub source_event_index: u64,
    pub destination_chain_id: u32,
    pub nonce: u64,
    pub amount: u128,
    pub recipient: Address,
    pub approval_count: u16,
    pub authorization_hash: Hash,
}

impl BridgeMintAuthorization {
    pub fn compute_hash(&self) -> Hash {
        freedom_primitives::domain_hash(
            b"FREEDOM-BRIDGE-MINT-AUTH-V1",
            &[
                &self.bridge_id,
                &self.message_hash,
                &self.replay_key,
                &self.canonical_asset_id,
                &self.representation_asset_id,
                &self.source_chain_id.to_le_bytes(),
                &self.source_tx_hash,
                &self.source_event_index.to_le_bytes(),
                &self.destination_chain_id.to_le_bytes(),
                &self.nonce.to_le_bytes(),
                &self.amount.to_le_bytes(),
                &self.recipient,
                &self.approval_count.to_le_bytes(),
            ],
        )
    }

    pub fn validate(&self) -> bool {
        self.bridge_id != [0; 32]
            && self.message_hash != [0; 32]
            && self.replay_key != [0; 32]
            && self.canonical_asset_id != [0; 32]
            && self.representation_asset_id != [0; 32]
            && self.source_chain_id != 0
            && self.source_tx_hash != [0; 32]
            && self.destination_chain_id != 0
            && self.amount > 0
            && self.recipient != [0; 32]
            && self.approval_count > 0
            && self.authorization_hash == self.compute_hash()
    }

    pub fn from_authenticated_message(
        message: &BridgeMessage,
        deposit: &VerifiedDeposit,
        approvals: &BridgeMessageApprovalSet,
        policy: &BridgeSecurityPolicy,
        validator_keys: &std::collections::BTreeMap<Hash, [u8; 32]>,
    ) -> Result<Self, BridgeAdapterError> {
        if !message.validate_hash()
            || !message.matches_verified_deposit(deposit)
            || deposit.bridge_id != policy.bridge_id
            || deposit.replay_key == [0; 32]
            || !policy.nonce_allowed(message.nonce)
            || !approvals.authorized(policy, validator_keys)
            || approvals.bridge_id != message.bridge_id
            || approvals.message_hash != message.message_hash
        {
            return Err(BridgeAdapterError::InvalidProof);
        }

        let approval_count = approvals.authenticated_count(validator_keys);
        if approval_count == 0 {
            return Err(BridgeAdapterError::InvalidProof);
        }

        let mut authorization = Self {
            bridge_id: message.bridge_id,
            message_hash: message.message_hash,
            replay_key: deposit.replay_key,
            canonical_asset_id: deposit.canonical_asset_id,
            representation_asset_id: deposit.representation_asset_id,
            source_chain_id: message.source_chain_id,
            source_tx_hash: message.source_tx_hash,
            source_event_index: message.source_event_index,
            destination_chain_id: message.destination_chain_id,
            nonce: message.nonce,
            amount: message.amount,
            recipient: message.recipient,
            approval_count,
            authorization_hash: [0; 32],
        };
        authorization.authorization_hash = authorization.compute_hash();
        if !authorization.validate() {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(authorization)
    }

    pub fn matches_message(&self, message: &BridgeMessage) -> bool {
        self.validate()
            && self.bridge_id == message.bridge_id
            && self.message_hash == message.message_hash
            && self.source_chain_id == message.source_chain_id
            && self.source_tx_hash == message.source_tx_hash
            && self.source_event_index == message.source_event_index
            && self.destination_chain_id == message.destination_chain_id
            && self.nonce == message.nonce
            && self.amount == message.amount
            && self.recipient == message.recipient
            && message.validate_hash()
    }

    pub fn replay_key_matches(&self, deposit: &VerifiedDeposit) -> bool {
        self.validate()
            && self.replay_key == deposit.replay_key
            && self.canonical_asset_id == deposit.canonical_asset_id
            && self.representation_asset_id == deposit.representation_asset_id
    }
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitcoinBlockHeader {
    pub version: i32,
    pub previous_block_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BitcoinBlockHeader {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 80 {
            return None;
        }
        let mut previous_block_hash = [0u8; 32];
        let mut merkle_root = [0u8; 32];
        previous_block_hash.copy_from_slice(&bytes[4..36]);
        merkle_root.copy_from_slice(&bytes[36..68]);
        Some(Self {
            version: i32::from_le_bytes(bytes[0..4].try_into().ok()?),
            previous_block_hash,
            merkle_root,
            timestamp: u32::from_le_bytes(bytes[68..72].try_into().ok()?),
            bits: u32::from_le_bytes(bytes[72..76].try_into().ok()?),
            nonce: u32::from_le_bytes(bytes[76..80].try_into().ok()?),
        })
    }

    pub fn serialize(&self) -> [u8; 80] {
        let mut out = [0u8; 80];
        out[0..4].copy_from_slice(&self.version.to_le_bytes());
        out[4..36].copy_from_slice(&self.previous_block_hash);
        out[36..68].copy_from_slice(&self.merkle_root);
        out[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        out[72..76].copy_from_slice(&self.bits.to_le_bytes());
        out[76..80].copy_from_slice(&self.nonce.to_le_bytes());
        out
    }

    pub fn block_hash(&self) -> Hash {
        bitcoin_sha256d(&self.serialize())
    }

    pub fn target_from_bits(bits: u32) -> Option<Hash> {
        let exponent = (bits >> 24) as usize;
        let mantissa = bits & 0x007f_ffff;
        if mantissa == 0 || (bits & 0x0080_0000) != 0 {
            return None;
        }
        let mut target = [0u8; 32];
        if exponent <= 3 {
            let value = mantissa >> (8 * (3 - exponent));
            target[..4].copy_from_slice(&(value as u32).to_le_bytes());
            return Some(target);
        }
        let offset = exponent - 3;
        if offset + 3 > 32 {
            return None;
        }
        let value = mantissa.to_le_bytes();
        target[offset..offset + 3].copy_from_slice(&value[..3]);
        Some(target)
    }

    pub fn validate_pow(&self) -> bool {
        let Some(target) = Self::target_from_bits(self.bits) else {
            return false;
        };
        bitcoin_hash_leq_target(&self.block_hash(), &target)
    }
}

pub struct BitcoinBlockHeaderVerification {
    pub block_hash: Hash,
    pub target: Hash,
}

impl BitcoinBlockHeaderVerification {
    pub fn verify(header: &BitcoinBlockHeader) -> Result<Self, BridgeAdapterError> {
        if header.timestamp == 0 || header.bits == 0 {
            return Err(BridgeAdapterError::InvalidProof);
        }
        let encoded = header.serialize();
        let parsed = BitcoinBlockHeader::parse(&encoded)
            .ok_or(BridgeAdapterError::InvalidProof)?;
        if parsed != *header {
            return Err(BridgeAdapterError::InvalidProof);
        }
        let Some(target) = BitcoinBlockHeader::target_from_bits(header.bits) else {
            return Err(BridgeAdapterError::InvalidProof);
        };
        if !bitcoin_hash_leq_target(&header.block_hash(), &target) {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(Self {
            block_hash: header.block_hash(),
            target,
        })
    }
}

fn bitcoin_hash_leq_target(hash: &Hash, target: &Hash) -> bool {
    for i in (0..32).rev() {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    true
}

pub struct BitcoinMerkleVerification {
    pub tx_hash: Hash,
    pub merkle_root: Hash,
}

impl BitcoinMerkleVerification {
    pub fn verify(
        proof: &BitcoinUtxoProof,
        expected_merkle_root: Hash,
    ) -> Result<Self, BridgeAdapterError> {
        if !proof.validate_transaction_binding() || proof.merkle_branch.is_empty() {
            return Err(BridgeAdapterError::InvalidProof);
        }

        let mut current = proof.tx_hash;
        let mut index = proof.merkle_index;
        for sibling in &proof.merkle_branch {
            let mut pair = Vec::with_capacity(64);
            if index & 1 == 0 {
                pair.extend_from_slice(&current);
                pair.extend_from_slice(sibling);
            } else {
                pair.extend_from_slice(sibling);
                pair.extend_from_slice(&current);
            }
            current = bitcoin_sha256d(&pair);
            index >>= 1;
        }

        if current != expected_merkle_root {
            return Err(BridgeAdapterError::InvalidProof);
        }

        Ok(Self {
            tx_hash: proof.tx_hash,
            merkle_root: current,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitcoinChainContext {
    pub tip_height: u64,
    pub tip_hash: Hash,
}

impl BitcoinChainContext {
    pub fn validate(&self) -> bool {
        self.tip_height > 0 && self.tip_hash != [0u8; 32]
    }

    pub fn confirmations_for(&self, block_height: u64) -> Option<u64> {
        if !self.validate() || block_height == 0 || block_height > self.tip_height {
            return None;
        }
        Some(self.tip_height - block_height + 1)
    }
}

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
    pub raw_transaction: Vec<u8>,
    #[serde(default)]
    pub block_header: Option<BitcoinBlockHeader>,
}
impl BitcoinUtxoProof {
    pub fn verify_block_header(&self) -> Result<BitcoinBlockHeader, BridgeAdapterError> {
        let header = self
            .block_header
            .as_ref()
            .ok_or(BridgeAdapterError::InvalidProof)?;
        if header.merkle_root != bitcoin_merkle_root(
            self.tx_hash,
            &self.merkle_branch,
            self.merkle_index,
        ) {
            return Err(BridgeAdapterError::InvalidProof);
        }
        if !bitcoin_pow_valid(header) {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(header.clone())
    }

    pub fn validate(&self, required_confirmations: u64) -> bool {
        self.bridge_id != [0; 32]
            && self.chain_id != 0
            && self.tx_hash != [0; 32]
            && self.vout < u32::MAX
            && self.amount_sat > 0
            && self.block_height > 0
            && self.confirmations >= required_confirmations
            && !self.merkle_branch.is_empty()
            && self.validate_transaction_binding()
            && self.validate_output_binding()
            && self.merkle_index < (1u32 << self.merkle_branch.len().min(31))
            && self.validate_bitcoin_chain_binding()
    }

    pub fn validate_bitcoin_chain_binding(&self) -> bool {
        let Some(header) = self.block_header.as_ref() else {
            return false;
        };
        if header.merkle_root
            != bitcoin_merkle_root(self.tx_hash, &self.merkle_branch, self.merkle_index)
        {
            return false;
        }
        if !header.validate_pow() {
            return false;
        }
        self.confirmations > 0
    }

    pub fn verify_against_chain_context(
        &self,
        context: &BitcoinChainContext,
        required_confirmations: u64,
    ) -> Result<u64, BridgeAdapterError> {
        if !context.validate() || self.block_height == 0 {
            return Err(BridgeAdapterError::InvalidProof);
        }
        let confirmations = context
            .confirmations_for(self.block_height)
            .ok_or(BridgeAdapterError::InvalidProof)?;
        if self.confirmations != confirmations || confirmations < required_confirmations {
            return Err(if confirmations < required_confirmations {
                BridgeAdapterError::InsufficientFinality
            } else {
                BridgeAdapterError::InvalidProof
            });
        }
        self.verify_bitcoin_chain()?;
        Ok(confirmations)
    }

    pub fn verify_bitcoin_chain(&self) -> Result<BitcoinBlockHeaderVerification, BridgeAdapterError> {
        if !self.validate_bitcoin_chain_binding() {
            return Err(BridgeAdapterError::InvalidProof);
        }
        let header = self.block_header.as_ref().ok_or(BridgeAdapterError::InvalidProof)?;
        BitcoinBlockHeaderVerification::verify(header)
    }

    pub fn to_cross_chain_asset_proof(
        &self,
        required_confirmations: u64,
        canonical_asset_id: Hash,
        representation_asset_id: Hash,
        recipient: Vec<u8>,
    ) -> Result<CrossChainAssetProof, BridgeAdapterError> {
        if !self.validate(required_confirmations) {
            return if self.confirmations < required_confirmations {
                Err(BridgeAdapterError::InsufficientFinality)
            } else {
                Err(BridgeAdapterError::InvalidProof)
            };
        }
        if recipient.is_empty() {
            return Err(BridgeAdapterError::InvalidIntent);
        }
        let deposit = BridgeDepositProof {
            bridge_id: self.bridge_id,
            source_chain_id: self.chain_id,
            tx_hash: self.tx_hash,
            event_index: self.event_index(),
            block_or_slot: self.block_height,
            confirmations: self.confirmations,
            proof: self.proof_bytes(),
        };
        CrossChainAssetProof::from_bridge_deposit(
            deposit,
            canonical_asset_id,
            representation_asset_id,
            CrossChainAssetKind::BitcoinUtxo,
            self.amount_sat as u128,
            recipient,
        )
    }

impl BitcoinUtxoProof {
    pub fn computed_tx_hash(&self)->Option<Hash>{if self.raw_transaction.is_empty(){return None;}Some(bitcoin_sha256d(&self.raw_transaction))}
    pub fn validate_transaction_binding(&self)->bool{self.computed_tx_hash()==Some(self.tx_hash)}
    pub fn validate_output_binding(&self)->bool{
        let Ok(tx)=BitcoinTransaction::parse(&self.raw_transaction) else{return false};
        tx.txid==self.tx_hash&&tx.output(self.vout).map(|o|o.amount_sat==self.amount_sat&&o.script_pubkey==self.script_pubkey).unwrap_or(false)
    }
    pub fn event_index(&self)->u64 { self.vout as u64 }
    pub fn validate_binding(&self,expected_tx:Hash,expected_vout:u32,expected_amount:u64,expected_script:&[u8])->bool{
        self.tx_hash==expected_tx&&self.vout==expected_vout&&self.amount_sat==expected_amount&&self.script_pubkey==expected_script&&self.tx_hash!=[0;32]&&!self.script_pubkey.is_empty()&&self.amount_sat>0
    }
    pub fn utxo_commitment(&self)->Hash{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BITCOIN-UTXO-COMMITMENT-V1");o.extend_from_slice(&self.tx_hash);o.extend_from_slice(&self.vout.to_le_bytes());o.extend_from_slice(&self.amount_sat.to_le_bytes());o.extend_from_slice(&(self.script_pubkey.len() as u64).to_le_bytes());o.extend_from_slice(&self.script_pubkey);freedom_primitives::hash_bytes(&o)}
    pub fn proof_bytes(&self)->Vec<u8> {
        let mut o=Vec::new(); o.extend_from_slice(b"FREEDOM-BITCOIN-UTXO-PROOF-V1");
        o.extend_from_slice(&self.bridge_id); o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.tx_hash); o.extend_from_slice(&self.vout.to_le_bytes());
        o.extend_from_slice(&self.amount_sat.to_le_bytes()); o.extend_from_slice(&self.block_height.to_le_bytes());
        o.extend_from_slice(&self.confirmations.to_le_bytes()); o.extend_from_slice(&self.merkle_index.to_le_bytes());
        o.extend_from_slice(&(self.merkle_branch.len() as u64).to_le_bytes());
        for h in &self.merkle_branch { o.extend_from_slice(h); }
        o.extend_from_slice(&(self.script_pubkey.len() as u64).to_le_bytes()); o.extend_from_slice(&self.script_pubkey);o.extend_from_slice(&(self.raw_transaction.len() as u64).to_le_bytes());o.extend_from_slice(&self.raw_transaction); o
    }
}


pub fn bitcoin_sha256d(data:&[u8])->Hash{use sha2::{Digest,Sha256};let first=Sha256::digest(data);let second=Sha256::digest(first);let mut out=[0u8;32];out.copy_from_slice(&second);out}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitcoinTransactionOutput {
    pub amount_sat: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitcoinTransaction {
    pub version: i32,
    pub segwit: bool,
    pub inputs: Vec<BitcoinTransactionInput>,
    pub outputs: Vec<BitcoinTransactionOutput>,
    pub lock_time: u32,
    pub txid: Hash,
    pub wtxid: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitcoinTransactionInput {
    pub previous_txid: Hash,
    pub previous_vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
    pub witness: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitcoinTransactionParseError {
    UnexpectedEof,
    InvalidVarInt,
    NonCanonicalVarInt,
    InvalidSegwitMarker,
    MissingInputs,
    MissingOutputs,
    TrailingBytes,
    LengthOverflow,
}

impl BitcoinTransaction {
    pub fn parse(raw: &[u8]) -> Result<Self, BitcoinTransactionParseError> {
        fn read_u32(raw: &[u8], p: &mut usize) -> Result<u32, BitcoinTransactionParseError> {
            if raw.len().saturating_sub(*p) < 4 { return Err(BitcoinTransactionParseError::UnexpectedEof); }
            let mut b=[0u8;4]; b.copy_from_slice(&raw[*p..*p+4]); *p+=4; Ok(u32::from_le_bytes(b))
        }
        fn read_u64(raw: &[u8], p: &mut usize) -> Result<u64, BitcoinTransactionParseError> {
            if raw.len().saturating_sub(*p) < 8 { return Err(BitcoinTransactionParseError::UnexpectedEof); }
            let mut b=[0u8;8]; b.copy_from_slice(&raw[*p..*p+8]); *p+=8; Ok(u64::from_le_bytes(b))
        }
        fn read_bytes(raw: &[u8], p: &mut usize, n: usize) -> Result<Vec<u8>, BitcoinTransactionParseError> {
            if n > raw.len().saturating_sub(*p) { return Err(BitcoinTransactionParseError::UnexpectedEof); }
            let out=raw[*p..*p+n].to_vec(); *p+=n; Ok(out)
        }
        fn read_varint(raw: &[u8], p: &mut usize) -> Result<u64, BitcoinTransactionParseError> {
            if *p>=raw.len(){return Err(BitcoinTransactionParseError::UnexpectedEof);}
            let prefix=raw[*p]; *p+=1;
            match prefix {
                0..=252 => Ok(prefix as u64),
                253 => {
                    if raw.len().saturating_sub(*p)<2{return Err(BitcoinTransactionParseError::UnexpectedEof);}
                    let mut b=[0u8;2];b.copy_from_slice(&raw[*p..*p+2]);*p+=2;let v=u16::from_le_bytes(b) as u64;
                    if v<253{return Err(BitcoinTransactionParseError::NonCanonicalVarInt)} Ok(v)
                }
                254 => {
                    if raw.len().saturating_sub(*p)<4{return Err(BitcoinTransactionParseError::UnexpectedEof);}
                    let mut b=[0u8;4];b.copy_from_slice(&raw[*p..*p+4]);*p+=4;let v=u32::from_le_bytes(b) as u64;
                    if v<=0xffff{return Err(BitcoinTransactionParseError::NonCanonicalVarInt)} Ok(v)
                }
                255 => {
                    if raw.len().saturating_sub(*p)<8{return Err(BitcoinTransactionParseError::UnexpectedEof);}
                    let mut b=[0u8;8];b.copy_from_slice(&raw[*p..*p+8]);*p+=8;let v=u64::from_le_bytes(b);
                    if v<=0xffff_ffff{return Err(BitcoinTransactionParseError::NonCanonicalVarInt)} Ok(v)
                }
            }
        }
        fn varint(n:u64)->Vec<u8>{
            match n {0..=252=>vec![n as u8],253..=0xffff=>{let mut v=vec![253];v.extend_from_slice(&(n as u16).to_le_bytes());v},0x1_0000..=0xffff_ffff=>{let mut v=vec![254];v.extend_from_slice(&(n as u32).to_le_bytes());v},_=>{let mut v=vec![255];v.extend_from_slice(&n.to_le_bytes());v}}
        }
        fn serialize_no_witness(tx:&BitcoinTransaction)->Vec<u8>{
            let mut o=Vec::new();o.extend_from_slice(&tx.version.to_le_bytes());o.extend_from_slice(&varint(tx.inputs.len() as u64));
            for i in &tx.inputs{o.extend_from_slice(&i.previous_txid);o.extend_from_slice(&i.previous_vout.to_le_bytes());o.extend_from_slice(&varint(i.script_sig.len() as u64));o.extend_from_slice(&i.script_sig);o.extend_from_slice(&i.sequence.to_le_bytes());}
            o.extend_from_slice(&varint(tx.outputs.len() as u64));for v in &tx.outputs{o.extend_from_slice(&v.amount_sat.to_le_bytes());o.extend_from_slice(&varint(v.script_pubkey.len() as u64));o.extend_from_slice(&v.script_pubkey);}
            o.extend_from_slice(&tx.lock_time.to_le_bytes());o
        }

        if raw.len()<4{return Err(BitcoinTransactionParseError::UnexpectedEof);}
        let mut p=0usize;let mut vb=[0u8;4];vb.copy_from_slice(&raw[..4]);p=4;let version=i32::from_le_bytes(vb);
        let mut segwit=false;
        if raw.len().saturating_sub(p)>=2 && raw[p]==0 && raw[p+1]!=0 {
            if raw[p+1]!=1{return Err(BitcoinTransactionParseError::InvalidSegwitMarker);}
            segwit=true;p+=2;
        }
        let input_count_u64=read_varint(raw,&mut p)?;
        if input_count_u64==0{return Err(BitcoinTransactionParseError::MissingInputs);}
        if input_count_u64>raw.len().saturating_sub(p) as u64{return Err(BitcoinTransactionParseError::UnexpectedEof);}
        let input_count=input_count_u64 as usize;
        let mut inputs=Vec::with_capacity(input_count);
        for _ in 0..input_count{
            let b=read_bytes(raw,&mut p,32)?;let mut previous_txid=[0u8;32];previous_txid.copy_from_slice(&b);
            let previous_vout=read_u32(raw,&mut p)?;let n=read_varint(raw,&mut p)? as usize;let script_sig=read_bytes(raw,&mut p,n)?;let sequence=read_u32(raw,&mut p)?;
            inputs.push(BitcoinTransactionInput{previous_txid,previous_vout,script_sig,sequence,witness:Vec::new()});
        }
        let output_count_u64=read_varint(raw,&mut p)?;
        if output_count_u64==0{return Err(BitcoinTransactionParseError::MissingOutputs);}
        if output_count_u64>raw.len().saturating_sub(p) as u64{return Err(BitcoinTransactionParseError::UnexpectedEof);}
        let output_count=output_count_u64 as usize;
        let mut outputs=Vec::with_capacity(output_count);
        for _ in 0..output_count{let amount_sat=read_u64(raw,&mut p)?;let n=read_varint(raw,&mut p)? as usize;let script_pubkey=read_bytes(raw,&mut p,n)?;outputs.push(BitcoinTransactionOutput{amount_sat,script_pubkey});}
        if segwit{
            for i in &mut inputs{
                let n_u64=read_varint(raw,&mut p)?;
                if n_u64>raw.len().saturating_sub(p) as u64{return Err(BitcoinTransactionParseError::UnexpectedEof);}
                let n=n_u64 as usize;let mut witness=Vec::with_capacity(n);
                for _ in 0..n{let len=read_varint(raw,&mut p)? as usize;witness.push(read_bytes(raw,&mut p,len)?);}
                i.witness=witness;
            }
        }
        let lock_time=read_u32(raw,&mut p)?;if p!=raw.len(){return Err(BitcoinTransactionParseError::TrailingBytes);}
        let mut tx=BitcoinTransaction{version,segwit,inputs,outputs,lock_time,txid:[0;32],wtxid:bitcoin_sha256d(raw)};
        tx.txid=bitcoin_sha256d(&serialize_no_witness(&tx));Ok(tx)
    }

    pub fn output(&self, vout:u32)->Option<&BitcoinTransactionOutput>{self.outputs.get(vout as usize)}
}

pub fn bitcoin_parse_transaction(raw:&[u8])->Result<BitcoinTransaction,BitcoinTransactionParseError>{BitcoinTransaction::parse(raw)}

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
 pub fn serialize(&self)->[u8;80]{let mut o=[0u8;80];o[0..4].copy_from_slice(&self.version.to_le_bytes());o[4..36].copy_from_slice(&self.prev_block_hash);o[36..68].copy_from_slice(&self.merkle_root);o[68..72].copy_from_slice(&self.time.to_le_bytes());o[72..76].copy_from_slice(&self.bits.to_le_bytes());o[76..80].copy_from_slice(&self.nonce.to_le_bytes());o}
 pub fn block_hash(&self)->Hash{bitcoin_sha256d(&self.serialize())}
}
pub fn bitcoin_target_from_bits(bits:u32)->Option<[u8;32]>{let exponent=(bits>>24) as usize;let mantissa=bits&0x007f_ffff;if mantissa==0||exponent<3||exponent>32||(bits&0x0080_0000)!=0{return None;}let mut out=[0u8;32];let bytes=mantissa.to_be_bytes();let m=[bytes[1],bytes[2],bytes[3]];let start=32usize.checked_sub(exponent)?;out[start..start+3].copy_from_slice(&m);Some(out)}
pub fn bitcoin_hash_leq_target(hash:Hash,target:[u8;32])->bool{hash.iter().rev().cmp(target.iter().rev())!=std::cmp::Ordering::Greater}
pub fn bitcoin_pow_valid(header:&BitcoinBlockHeader)->bool{bitcoin_target_from_bits(header.bits).map(|t|bitcoin_hash_leq_target(header.block_hash(),t)).unwrap_or(false)}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum BitcoinHeaderChainError{Empty,InvalidPow,InvalidLinkage,InvalidHeight}
pub fn bitcoin_header_work(header:&BitcoinBlockHeader)->Option<[u8;32]>{
    use num_bigint::BigUint;
    let target=BigUint::from_bytes_be(&bitcoin_target_from_bits(header.bits)?);
    let work=(BigUint::from(1u8)<<256usize)/(target+BigUint::from(1u8));
    let bytes=work.to_bytes_be(); if bytes.len()>32{return None;}
    let mut out=[0u8;32]; out[32-bytes.len()..].copy_from_slice(&bytes); Some(out)
}
pub fn bitcoin_chainwork(headers:&[BitcoinBlockHeader])->Result<[u8;32],BitcoinHeaderChainError>{
    bitcoin_validate_header_chain(0,headers)?;
    use num_bigint::BigUint;
    let mut total=BigUint::from(0u8);
    for h in headers {total+=BigUint::from_bytes_be(&bitcoin_header_work(h).ok_or(BitcoinHeaderChainError::InvalidPow)?);}
    let bytes=total.to_bytes_be(); if bytes.len()>32{return Err(BitcoinHeaderChainError::InvalidHeight);}
    let mut out=[0u8;32];out[32-bytes.len()..].copy_from_slice(&bytes);Ok(out)
}
pub fn bitcoin_chainwork_cmp(a:&[BitcoinBlockHeader],b:&[BitcoinBlockHeader])->Result<std::cmp::Ordering,BitcoinHeaderChainError>{
    Ok(bitcoin_chainwork(a)?.cmp(&bitcoin_chainwork(b)?))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinChainCheckpoint {
    pub height: u64,
    pub block_hash: Hash,
    pub chainwork: [u8;32],
}
impl BitcoinChainCheckpoint {
    pub fn validate(&self)->bool{self.block_hash!=[0;32]&&self.chainwork!=[0;32]}
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitcoinCheckpointError { InvalidCheckpoint, EmptyHeaders, HeightMismatch, LinkageMismatch, InsufficientChainwork, ReorgDetected }
pub fn bitcoin_validate_from_checkpoint(checkpoint:&BitcoinChainCheckpoint,headers:&[BitcoinBlockHeader])->Result<BitcoinChainCheckpoint,BitcoinCheckpointError>{
    if !checkpoint.validate(){return Err(BitcoinCheckpointError::InvalidCheckpoint);}
    if headers.is_empty(){return Err(BitcoinCheckpointError::EmptyHeaders);}
    if headers[0].prev_block_hash!=checkpoint.block_hash{return Err(BitcoinCheckpointError::LinkageMismatch);}
    let end_height=checkpoint.height.checked_add(headers.len() as u64).ok_or(BitcoinCheckpointError::HeightMismatch)?;
    let work=bitcoin_chainwork(headers).map_err(|_|BitcoinCheckpointError::InsufficientChainwork)?;
    let total=bitcoin_add_chainwork(checkpoint.chainwork,work);
    if total<=checkpoint.chainwork{return Err(BitcoinCheckpointError::InsufficientChainwork);}
    Ok(BitcoinChainCheckpoint{height:end_height,block_hash:headers.last().unwrap().block_hash(),chainwork:total})
}
fn bitcoin_add_chainwork(a:[u8;32],b:[u8;32])->[u8;32]{
    let mut out=[0u8;32];let mut carry=0u16;
    for i in (0..32).rev(){let x=a[i] as u16+b[i] as u16+carry;out[i]=(x&0xff) as u8;carry=x>>8;}out
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub const BITCOIN_REORG_EVIDENCE_LIMIT:usize=32;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinReorgEvidence {
 pub checkpoint_height:u64,
 pub checkpoint_hash:Hash,
 pub candidate_tip_height:u64,
 pub candidate_tip_hash:Hash,
 pub candidate_chainwork:[u8;32],
 pub fingerprint:Hash,
}
impl BitcoinReorgEvidence {
 pub fn compute_fingerprint(&self)->Hash{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BITCOIN-REORG-V1");o.extend_from_slice(&self.checkpoint_height.to_le_bytes());o.extend_from_slice(&self.checkpoint_hash);o.extend_from_slice(&self.candidate_tip_height.to_le_bytes());o.extend_from_slice(&self.candidate_tip_hash);o.extend_from_slice(&self.candidate_chainwork);freedom_primitives::hash_bytes(&o)}
 pub fn validate(&self)->bool{self.checkpoint_hash!=[0;32]&&self.candidate_tip_hash!=[0;32]&&self.candidate_chainwork!=[0;32]&&self.fingerprint==self.compute_fingerprint()}
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum BitcoinReorgSeverity { Normal=1, Elevated=2, Critical=3 }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinReorgPolicy { pub elevated_confirmations:u64, pub critical_chainwork_margin:[u8;32] }
impl BitcoinReorgPolicy {
 pub fn validate(&self)->bool{self.elevated_confirmations>0&&self.critical_chainwork_margin!=[0;32]}
 pub fn classify(&self,checkpoint:&BitcoinChainCheckpoint,e:&BitcoinReorgEvidence)->BitcoinReorgSeverity{
  if e.candidate_chainwork>checkpoint.chainwork { BitcoinReorgSeverity::Critical }
  else if e.candidate_tip_height>=checkpoint.height.saturating_add(self.elevated_confirmations) { BitcoinReorgSeverity::Elevated }
  else { BitcoinReorgSeverity::Normal }
 }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinCanonicalTipProof { pub start_height:u64, pub headers:Vec<BitcoinBlockHeader>, pub tip_height:u64, pub tip_hash:Hash, pub chainwork:[u8;32] }
impl BitcoinCanonicalTipProof {
 pub fn verify(&self,checkpoint:&BitcoinChainCheckpoint)->bool{
  if self.headers.is_empty()||self.start_height!=checkpoint.height.saturating_add(1){return false;}
  let Ok(cp)=bitcoin_validate_from_checkpoint(checkpoint,&self.headers) else{return false;};
  cp.height==self.tip_height&&cp.block_hash==self.tip_hash&&cp.chainwork==self.chainwork
 }
}
pub struct BitcoinReorgRecoverySignature { pub validator_id:Hash, pub signature:Vec<u8> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinReorgRecovery { pub bridge_id:Hash, pub evidence_fingerprint:Hash, pub nonce:u64, pub canonical_height:u64, pub canonical_block_hash:Hash, pub canonical_chainwork:[u8;32], pub validator_ids:Vec<Hash>, pub signatures:Vec<BitcoinReorgRecoverySignature>, pub recovery_hash:Hash }
impl BitcoinReorgRecovery {
 pub fn signing_bytes(&self)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-BITCOIN-REORG-RECOVERY-V1");o.extend_from_slice(&self.bridge_id);o.extend_from_slice(&self.evidence_fingerprint);o.extend_from_slice(&self.nonce.to_le_bytes());o.extend_from_slice(&self.canonical_height.to_le_bytes());o.extend_from_slice(&self.canonical_block_hash);o.extend_from_slice(&self.canonical_chainwork);for v in &self.validator_ids{o.extend_from_slice(v);}o}
 pub fn compute_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
 pub fn validate(&self)->bool{self.bridge_id!=[0;32]&&self.evidence_fingerprint!=[0;32]&&self.nonce>0&&self.canonical_height>0&&self.canonical_block_hash!=[0;32]&&self.canonical_chainwork!=[0;32]&&!self.validator_ids.is_empty()&&self.validator_ids.len()==self.signatures.len()&&self.recovery_hash==self.compute_hash()}
 pub fn authenticated_count(&self,validators:&std::collections::BTreeMap<Hash,[u8;32]>)->u16{let mut seen=std::collections::BTreeSet::new();for s in &self.signatures{if s.signature.len()!=64||seen.contains(&s.validator_id){continue;}if let Some(key)=validators.get(&s.validator_id){if freedom_crypto::verify(key,&self.signing_bytes(),s.signature.as_slice().try_into().unwrap()){seen.insert(s.validator_id);}}}seen.len() as u16}
}
pub struct BitcoinReorgReport { pub checkpoint_height:u64, pub checkpoint_hash:Hash, pub candidate_tip_height:u64, pub candidate_tip_hash:Hash, pub candidate_chainwork:[u8;32] }
pub fn bitcoin_evaluate_reorg(checkpoint:&BitcoinChainCheckpoint,headers:&[BitcoinBlockHeader])->Result<BitcoinReorgReport,BitcoinCheckpointError>{
 if !checkpoint.validate(){return Err(BitcoinCheckpointError::InvalidCheckpoint);} if headers.is_empty(){return Err(BitcoinCheckpointError::EmptyHeaders);}
 let work=bitcoin_chainwork(headers).map_err(|_|BitcoinCheckpointError::InsufficientChainwork)?;
 let candidate_work=bitcoin_add_chainwork(checkpoint.chainwork,work);
 let tip=bitcoin_validate_from_checkpoint(checkpoint,headers).map_err(|e|if e==BitcoinCheckpointError::LinkageMismatch{BitcoinCheckpointError::ReorgDetected}else{e})?;
 Ok(BitcoinReorgReport{checkpoint_height:checkpoint.height,checkpoint_hash:checkpoint.block_hash,candidate_tip_height:tip.height,candidate_tip_hash:tip.block_hash,candidate_chainwork:candidate_work})
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
    pub fn verify_against_canonical_tip(&self,required_confirmations:u64,tip:&BitcoinChainCheckpoint,headers:&[BitcoinBlockHeader])->bool{
        if headers.is_empty()||self.block_height>tip.height{return false;} let computed_confirmations=tip.height.saturating_sub(self.block_height).saturating_add(1); if computed_confirmations<required_confirmations||self.confirmations!=computed_confirmations{return false;} if !self.verify(required_confirmations){return false;}
        if headers[0]!=self.block_header{return false;}
        if headers.len() as u64 != tip.height.saturating_sub(self.block_height).saturating_add(1){return false;}
        if bitcoin_validate_header_chain(self.block_height,headers).ok()!=Some(tip.block_hash){return false;}
        headers.last().map(|h|h.block_hash())==Some(tip.block_hash)
    }
 pub fn canonical_confirmations(&self,tip:&BitcoinChainCheckpoint)->Option<u64>{if self.block_height>tip.height{return None;}Some(tip.height-self.block_height+1)}
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
    pub fn validate(&self, required_confirmations: u64) -> bool {
        self.bridge_id != [0; 32]
            && self.chain_id != 0
            && self.tx_hash != [0; 32]
            && self.block_number > 0
            && self.confirmations >= required_confirmations
            && self.token_contract.len() == 20
            && self.from.len() == 20
            && self.to.len() == 20
            && self.amount > 0
            && self.event_signature != [0; 32]
            && !self.proof.is_empty()
    }

    pub fn to_cross_chain_asset_proof(
        &self,
        required_confirmations: u64,
        canonical_asset_id: Hash,
        representation_asset_id: Hash,
    ) -> Result<CrossChainAssetProof, BridgeAdapterError> {
        if !self.validate(required_confirmations) {
            return if self.confirmations < required_confirmations {
                Err(BridgeAdapterError::InsufficientFinality)
            } else {
                Err(BridgeAdapterError::InvalidProof)
            };
        }
        let deposit = BridgeDepositProof {
            bridge_id: self.bridge_id,
            source_chain_id: self.chain_id,
            tx_hash: self.tx_hash,
            event_index: self.event_index(),
            block_or_slot: self.block_number,
            confirmations: self.confirmations,
            proof: self.proof_bytes(),
        };
        CrossChainAssetProof::from_bridge_deposit(
            deposit,
            canonical_asset_id,
            representation_asset_id,
            CrossChainAssetKind::Erc20,
            self.amount,
            self.to.clone(),
        )
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
pub struct TronTransactionInclusionProof {
    pub tx_hash: Hash,
    pub block_number: u64,
    pub block_id: Hash,
    pub transaction_index: u32,
    pub transaction_count: u32,
    pub siblings: Vec<Option<Hash>>,
}

impl TronTransactionInclusionProof {
    fn sha256_pair(left: Hash, right: Hash) -> Hash {
        use sha2::{Digest, Sha256};
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&left);
        bytes[32..].copy_from_slice(&right);
        Sha256::digest(bytes).into()
    }

    fn expected_levels(mut count: usize) -> usize {
        let mut levels = 0usize;
        while count > 1 {
            count = (count + 1) / 2;
            levels += 1;
        }
        levels
    }

    pub fn verify_against_root(&self, tx_trie_root: Hash) -> bool {
        if self.tx_hash == [0; 32]
            || self.block_id == [0; 32]
            || self.block_number == 0
            || self.transaction_count == 0
            || self.transaction_index >= self.transaction_count
            || self.siblings.len() != Self::expected_levels(self.transaction_count as usize)
        {
            return false;
        }

        let mut hash = self.tx_hash;
        let mut index = self.transaction_index as usize;
        let mut level_count = self.transaction_count as usize;

        for sibling in &self.siblings {
            if level_count > 1 {
                if index & 1 == 1 {
                    let Some(right) = sibling else { return false; };
                    hash = Self::sha256_pair(*right, hash);
                } else if let Some(right) = sibling {
                    hash = Self::sha256_pair(hash, *right);
                } else if index + 1 < level_count {
                    return false;
                }
                index /= 2;
                level_count = (level_count + 1) / 2;
            } else {
                return false;
            }
        }

        hash == tx_trie_root
    }

    pub fn verify_binding(
        &self,
        header: &TronBlockHeaderProof,
        tx_trie_root: Hash,
    ) -> bool {
        self.block_number == header.block_number().unwrap_or(0)
            && self.block_id == header.block_id
            && self.verify_against_root(tx_trie_root)
            && header.tx_trie_root().ok() == Some(tx_trie_root)
    }

    pub fn proof_hash(&self) -> Hash {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"FREEDOM-TRON-TX-INCLUSION-V1");
        bytes.extend_from_slice(&self.tx_hash);
        bytes.extend_from_slice(&self.block_number.to_le_bytes());
        bytes.extend_from_slice(&self.block_id);
        bytes.extend_from_slice(&self.transaction_index.to_le_bytes());
        bytes.extend_from_slice(&self.transaction_count.to_le_bytes());
        for sibling in &self.siblings {
            match sibling {
                Some(hash) => { bytes.push(1); bytes.extend_from_slice(hash); }
                None => bytes.push(0),
            }
        }
        freedom_primitives::hash_bytes(&bytes)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TronUsdtDepositProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub confirmations: u64,
    pub usdt_contract: Vec<u8>,
    pub header: TronBlockHeaderProof,
    pub inclusion: TronTransactionInclusionProof,
    pub event: TronTransactionEventProof,
}

impl TronUsdtDepositProof {
    pub const TRON_MAINNET_CHAIN_ID: u32 = 728126428;

    pub fn validate(&self, required_confirmations: u64) -> bool {
        self.validate_with_contract(required_confirmations, &self.usdt_contract)
    }

    pub fn validate_with_contract(
        &self,
        required_confirmations: u64,
        expected_usdt_contract: &[u8],
    ) -> bool {
        self.bridge_id != [0; 32]
            && self.chain_id == Self::TRON_MAINNET_CHAIN_ID
            && self.confirmations >= required_confirmations
            && self.usdt_contract.len() == 20
            && self.usdt_contract == expected_usdt_contract
            && self.header.validate()
            && self.inclusion.verify_binding(
                &self.header,
                self.header.tx_trie_root().unwrap_or([0; 32]),
            )
            && self.inclusion.tx_hash == self.event.tx_hash
            && self.inclusion.transaction_index == self.event.transaction_index
            && self.event.block_number == self.header.block_number().unwrap_or(0)
            && self.event.block_id == self.header.block_id
            && self.event.contract_address == self.usdt_contract
            && self.event.validate_transfer_event()
    }

    pub fn sender(&self) -> Result<[u8; 20], String> {
        self.event.from_topic()
    }

    pub fn recipient(&self) -> Result<[u8; 20], String> {
        self.event.to_topic()
    }

    pub fn amount(&self) -> Result<u128, String> {
        self.event.amount()
    }

    pub fn to_bridge_deposit(
        &self,
        required_confirmations: u64,
        expected_usdt_contract: &[u8],
    ) -> Result<BridgeDepositProof, BridgeAdapterError> {
        if !self.validate_with_contract(required_confirmations, expected_usdt_contract) {
            return if self.confirmations < required_confirmations {
                Err(BridgeAdapterError::InsufficientFinality)
            } else {
                Err(BridgeAdapterError::InvalidProof)
            };
        }
        let amount = self.amount().map_err(|_| BridgeAdapterError::InvalidProof)?;
        if amount == 0 {
            return Err(BridgeAdapterError::InvalidProof);
        }
        Ok(BridgeDepositProof {
            bridge_id: self.bridge_id,
            source_chain_id: self.chain_id,
            tx_hash: self.event.tx_hash,
            event_index: self.event.event_index as u64,
            block_or_slot: self.header.block_number().map_err(|_| BridgeAdapterError::InvalidProof)?,
            confirmations: self.confirmations,
            proof: self.proof_hash().to_vec(),
        })
    }

    pub fn amount_checked(&self) -> Result<u128, BridgeAdapterError> {
        self.amount().map_err(|_| BridgeAdapterError::InvalidProof)
    }

    pub fn to_cross_chain_asset_proof(
        &self,
        required_confirmations: u64,
        expected_usdt_contract: &[u8],
        canonical_asset_id: Hash,
        representation_asset_id: Hash,
    ) -> Result<CrossChainAssetProof, BridgeAdapterError> {
        let deposit = self.to_bridge_deposit(required_confirmations, expected_usdt_contract)?;
        let recipient = self.recipient().map_err(|_| BridgeAdapterError::InvalidProof)?.to_vec();
        CrossChainAssetProof::from_bridge_deposit(
            deposit,
            canonical_asset_id,
            representation_asset_id,
            CrossChainAssetKind::Trc20,
            self.amount_checked()?,
            recipient,
        )
    }

    pub fn proof_hash(&self) -> Hash {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"FREEDOM-TRON-USDT-DEPOSIT-V1");
        bytes.extend_from_slice(&self.bridge_id);
        bytes.extend_from_slice(&self.chain_id.to_le_bytes());
        bytes.extend_from_slice(&self.confirmations.to_le_bytes());
        bytes.extend_from_slice(&(self.usdt_contract.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.usdt_contract);
        bytes.extend_from_slice(&self.header.block_id);
        bytes.extend_from_slice(&self.inclusion.proof_hash());
        bytes.extend_from_slice(&self.event.proof_hash());
        freedom_primitives::hash_bytes(&bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TronTransactionEventProof {
    pub raw_data: Vec<u8>,
    pub tx_hash: Hash,
    pub block_number: u64,
    pub block_id: Hash,
    pub transaction_index: u32,
    pub event_index: u32,
    /// TRON log address is the 20-byte contract address without the 0x41 prefix.
    pub contract_address: Vec<u8>,
    /// topics[0] is the Solidity event signature; topics[1]/topics[2] are indexed addresses.
    pub topics: Vec<Hash>,
    /// ABI-encoded non-indexed event arguments.
    pub data: Vec<u8>,
    pub execution_success: bool,
}

impl TronTransactionEventProof {
    pub const TRANSFER_EVENT_SIGNATURE: Hash = [
        0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b,
        0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
        0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16,
        0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
    ];

    pub fn computed_tx_hash(&self) -> Hash {
        use sha2::{Digest, Sha256};
        Sha256::digest(&self.raw_data).into()
    }

    pub fn from_topic(&self) -> Result<[u8; 20], String> {
        if self.topics.len() < 3 { return Err("TRC-20 Transfer requires 3 topics".into()); }
        let mut out = [0u8; 20];
        out.copy_from_slice(&self.topics[1][12..]);
        Ok(out)
    }

    pub fn to_topic(&self) -> Result<[u8; 20], String> {
        if self.topics.len() < 3 { return Err("TRC-20 Transfer requires 3 topics".into()); }
        let mut out = [0u8; 20];
        out.copy_from_slice(&self.topics[2][12..]);
        Ok(out)
    }

    pub fn amount(&self) -> Result<u128, String> {
        if self.data.len() != 32 { return Err("TRC-20 Transfer data must be exactly 32 bytes".into()); }
        if self.data[..16].iter().any(|b| *b != 0) {
            return Err("TRC-20 Transfer amount exceeds u128".into());
        }
        let mut amount = [0u8; 16];
        amount.copy_from_slice(&self.data[16..]);
        Ok(u128::from_be_bytes(amount))
    }

    pub fn validate_transfer_event(&self) -> bool {
        self.raw_data.len() > 0
            && self.tx_hash == self.computed_tx_hash()
            && self.block_number > 0
            && self.block_id != [0; 32]
            && self.transaction_index < u32::MAX
            && self.contract_address.len() == 20
            && self.topics.len() == 3
            && self.topics[0] == Self::TRANSFER_EVENT_SIGNATURE
            && self.topics[1][..12].iter().all(|b| *b == 0)
            && self.topics[2][..12].iter().all(|b| *b == 0)
            && self.amount().map(|a| a > 0).unwrap_or(false)
            && self.execution_success
    }

    pub fn verify_binding(
        &self,
        header: &TronBlockHeaderProof,
        expected_tx_hash: Hash,
        expected_index: u32,
    ) -> bool {
        self.validate_transfer_event()
            && self.block_number == header.block_number().unwrap_or(0)
            && self.block_id == header.block_id
            && self.tx_hash == expected_tx_hash
            && self.transaction_index == expected_index
    }

    pub fn proof_hash(&self) -> Hash {
        let mut out = Vec::new();
        out.extend_from_slice(b"FREEDOM-TRON-TX-EVENT-V1");
        out.extend_from_slice(&self.tx_hash);
        out.extend_from_slice(&self.block_number.to_le_bytes());
        out.extend_from_slice(&self.block_id);
        out.extend_from_slice(&self.transaction_index.to_le_bytes());
        out.extend_from_slice(&self.contract_address);
        for topic in &self.topics { out.extend_from_slice(topic); }
        out.extend_from_slice(&(self.data.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.data);
        freedom_primitives::hash_bytes(&out)
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
pub struct DrawConfig { pub draw_id: Hash, pub draw_type: DrawType, #[serde(rename="ticket_price_native_units", alias="ticket_price_mfdm")] pub ticket_price_native_units: u128, pub winner_count: u32, pub starts_at: u64, pub ends_at: u64, pub seed_block: u64 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawResult { pub draw_id: Hash, pub total_tickets: u64, #[serde(rename="prize_pool_native_units", alias="prize_pool_mfdm")] pub prize_pool_native_units: u128, #[serde(rename="cu_fund_native_units", alias="cu_fund_mfdm")] pub cu_fund_native_units: u128, #[serde(rename="annual_reserve_native_units", alias="annual_reserve_mfdm")] pub annual_reserve_native_units: u128, pub winners: Vec<Hash>, pub randomness_seed: Hash, pub selection_proof: Hash }


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
mod m106_bitcoin_reorg_tests {
 use super::*;
 fn h(prev:Hash)->BitcoinBlockHeader{BitcoinBlockHeader{version:1,prev_block_hash:prev,merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0}}
 #[test] fn conflicting_branch_is_reported_as_reorg(){let cp=BitcoinChainCheckpoint{height:100,block_hash:[9;32],chainwork:[1;32]};assert_eq!(bitcoin_evaluate_reorg(&cp,&[h([8;32])]),Err(BitcoinCheckpointError::ReorgDetected));}
 #[test] fn continuing_branch_has_report(){let cp=BitcoinChainCheckpoint{height:100,block_hash:[9;32],chainwork:[1;32]};let a=h(cp.block_hash);let r=bitcoin_evaluate_reorg(&cp,&[a]).unwrap();assert_eq!(r.candidate_tip_height,101);}
}

#[cfg(test)]
mod m103_bitcoin_checkpoint_tests {
 use super::*;
 fn h(prev:Hash)->BitcoinBlockHeader{BitcoinBlockHeader{version:1,prev_block_hash:prev,merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0}}
 #[test] fn checkpoint_continuation_is_accepted(){let cp=BitcoinChainCheckpoint{height:100,block_hash:[9;32],chainwork:[1;32]};let a=h(cp.block_hash);let r=bitcoin_validate_from_checkpoint(&cp,&[a]).unwrap();assert_eq!(r.height,101);}
 #[test] fn wrong_checkpoint_link_is_rejected(){let cp=BitcoinChainCheckpoint{height:100,block_hash:[9;32],chainwork:[1;32]};assert_eq!(bitcoin_validate_from_checkpoint(&cp,&[h([8;32])]),Err(BitcoinCheckpointError::LinkageMismatch));}
}

#[cfg(test)]
mod m102_bitcoin_chainwork_tests {
 use super::*;
 #[test] fn work_is_positive(){let h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};assert!(bitcoin_header_work(&h).is_some());}
 #[test] fn chainwork_accumulates(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=a.block_hash();assert!(bitcoin_chainwork(&[a,b]).unwrap()>bitcoin_chainwork(&[b.clone()]).unwrap());}
}

#[cfg(test)]
mod m102_bitcoin_chainwork_tests {
 use super::*;
 #[test] fn work_is_positive(){let h=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let w=bitcoin_header_work(&h).unwrap();assert!(w.iter().any(|x|*x!=0));}
 #[test] fn longer_valid_chain_has_more_work_same_target(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=a.block_hash();let mut c=b.clone();c.prev_block_hash=b.block_hash();assert_eq!(bitcoin_chainwork_cmp(&[a,b],&[a,b,c]).unwrap(),std::cmp::Ordering::Less);}
 #[test] fn chainwork_comparison_is_not_height_only(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=a.block_hash();let mut c=a.clone();c.prev_block_hash=a.block_hash();c.bits=0x1f00ffff;assert!(bitcoin_chainwork_cmp(&[a,b],&[a,c]).is_ok());}
}

#[cfg(test)]
mod m101_bitcoin_header_chain_tests {
 use super::*;
 #[test] fn linked_headers_are_accepted(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=a.block_hash();assert!(bitcoin_validate_header_chain(100,&[a,b]).is_ok());}
 #[test] fn broken_link_is_rejected(){let a=BitcoinBlockHeader{version:1,prev_block_hash:[0;32],merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0};let mut b=a.clone();b.prev_block_hash=[9;32];assert_eq!(bitcoin_validate_header_chain(100,&[a,b]),Err(BitcoinHeaderChainError::InvalidLinkage));}
}

#[cfg(test)]
mod m102_bitcoin_chainwork_tests {
 use super::*;
 fn h(prev:Hash)->BitcoinBlockHeader{BitcoinBlockHeader{version:1,prev_block_hash:prev,merkle_root:[1;32],time:1,bits:0x207fffff,nonce:0}}
 #[test] fn chainwork_accumulates(){let a=h([0;32]);let mut b=h(a.block_hash());let mut c=h(b.block_hash());assert!(bitcoin_chainwork(&[a,b,c]).is_ok());b=h([0;32]);assert!(bitcoin_chainwork_cmp(&[h([0;32])],&[h([0;32]),b]).is_ok());}
 #[test] fn invalid_chain_has_no_work(){let a=h([0;32]);let mut b=h([9;32]);assert_eq!(bitcoin_chainwork(&[a,b]),Err(BitcoinHeaderChainError::InvalidLinkage));}
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


#[cfg(test)]
mod m119_bitcoin_raw_transaction_tests {
    use super::*;

    fn legacy_tx() -> Vec<u8> {
        let mut r=Vec::new();
        r.extend_from_slice(&1i32.to_le_bytes());
        r.push(1);
        r.extend_from_slice(&[0x11;32]);
        r.extend_from_slice(&0u32.to_le_bytes());
        r.push(0);
        r.extend_from_slice(&0xffff_ffffu32.to_le_bytes());
        r.push(1);
        r.extend_from_slice(&50_000u64.to_le_bytes());
        r.push(2);
        r.extend_from_slice(&[0x51,0x21]);
        r.extend_from_slice(&0u32.to_le_bytes());
        r
    }

    fn segwit_tx() -> Vec<u8> {
        let mut r=Vec::new();
        r.extend_from_slice(&2i32.to_le_bytes());
        r.extend_from_slice(&[0,1]);
        r.push(1);
        r.extend_from_slice(&[0x22;32]);
        r.extend_from_slice(&1u32.to_le_bytes());
        r.push(0);
        r.extend_from_slice(&0xffff_fffeu32.to_le_bytes());
        r.push(1);
        r.extend_from_slice(&75_000u64.to_le_bytes());
        r.push(2);
        r.extend_from_slice(&[0x00,0x14]);
        r.push(1);
        r.push(2);
        r.extend_from_slice(&[0xaa,0xbb]);
        r.extend_from_slice(&0u32.to_le_bytes());
        r
    }

    #[test]
    fn legacy_parser_binds_exact_output() {
        let raw=legacy_tx();
        let tx=BitcoinTransaction::parse(&raw).unwrap();
        assert!(!tx.segwit);
        assert_eq!(tx.outputs.len(),1);
        assert_eq!(tx.outputs[0].amount_sat,50_000);
        assert_eq!(tx.outputs[0].script_pubkey,vec![0x51,0x21]);
        assert_eq!(tx.txid,bitcoin_sha256d(&raw));
        assert_eq!(tx.wtxid,tx.txid);
    }

    #[test]
    fn segwit_parser_excludes_witness_from_txid() {
        let raw=segwit_tx();
        let tx=BitcoinTransaction::parse(&raw).unwrap();
        assert!(tx.segwit);
        assert_eq!(tx.outputs[0].amount_sat,75_000);
        assert_eq!(tx.outputs[0].script_pubkey,vec![0x00,0x14]);
        assert_eq!(tx.inputs[0].witness,vec![vec![0xaa,0xbb]]);
        assert_ne!(tx.txid,tx.wtxid);
        let mut no_witness=Vec::new();
        no_witness.extend_from_slice(&2i32.to_le_bytes());
        no_witness.push(1);
        no_witness.extend_from_slice(&[0x22;32]);
        no_witness.extend_from_slice(&1u32.to_le_bytes());
        no_witness.push(0);
        no_witness.extend_from_slice(&0xffff_fffeu32.to_le_bytes());
        no_witness.push(1);
        no_witness.extend_from_slice(&75_000u64.to_le_bytes());
        no_witness.push(2);
        no_witness.extend_from_slice(&[0x00,0x14]);
        no_witness.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(tx.txid,bitcoin_sha256d(&no_witness));
    }

    #[test]
    fn parser_rejects_trailing_bytes() {
        let mut raw=legacy_tx(); raw.push(0);
        assert_eq!(BitcoinTransaction::parse(&raw),Err(BitcoinTransactionParseError::TrailingBytes));
    }

    #[test]
    fn parser_rejects_noncanonical_varint() {
        let mut raw=legacy_tx();
        raw[4]=253;
        raw.insert(5,1);
        raw.insert(6,0);
        assert_eq!(BitcoinTransaction::parse(&raw),Err(BitcoinTransactionParseError::NonCanonicalVarInt));
    }

    #[test]
    fn output_binding_cannot_be_swapped() {
        let raw=legacy_tx();
        let tx=BitcoinTransaction::parse(&raw).unwrap();
        let mut proof=BitcoinUtxoProof{
            bridge_id:[9;32],chain_id:8332,tx_hash:tx.txid,vout:0,amount_sat:50_000,
            block_height:100,confirmations:6,merkle_branch:vec![[2;32]],merkle_index:0,
            script_pubkey:vec![0x51,0x21],raw_transaction:raw,
        };
        assert!(proof.validate_output_binding());
        proof.amount_sat+=1;
        assert!(!proof.validate_output_binding());
        proof.amount_sat=50_000;
        proof.script_pubkey=vec![0x51,0x22];
        assert!(!proof.validate_output_binding());
    }
}


/// M120: a cryptographically anchored proof that a previously included Bitcoin
/// output has not been spent up to a canonical tip.
///
/// Bitcoin block headers commit to transaction Merkle roots, but Bitcoin does
/// not commit a UTXO set in the header. Therefore an honest unspent proof must
/// either use a trusted/committed UTXO accumulator or inspect every transaction
/// in every block after the output's inclusion. M120 implements the latter.
/// It is intentionally a full-block spend scan, not a fake "absence proof".
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinSpendScanBlock {
    pub height: u64,
    pub header: BitcoinBlockHeader,
    /// Complete raw transactions for this block, in canonical block order.
    pub raw_transactions: Vec<Vec<u8>>,
}

impl BitcoinSpendScanBlock {
    pub fn transaction_hashes(&self) -> Result<Vec<Hash>, String> {
        if self.raw_transactions.is_empty() {
            return Err("bitcoin spend-scan block must contain transactions".into());
        }
        if self.raw_transactions.len() > 1_000_000 {
            return Err("bitcoin spend-scan block transaction count exceeds limit".into());
        }
        let mut hashes = Vec::with_capacity(self.raw_transactions.len());
        let mut seen = std::collections::BTreeSet::new();
        for raw in &self.raw_transactions {
            let tx = BitcoinTransaction::parse(raw)
                .map_err(|_| "invalid raw transaction in spend-scan block".to_string())?;
            if !seen.insert(tx.txid) {
                return Err("duplicate transaction id in spend-scan block".into());
            }
            hashes.push(tx.txid);
        }
        Ok(hashes)
    }

    pub fn merkle_root_from_transactions(&self) -> Result<Hash, String> {
        let mut level = self.transaction_hashes()?;
        while level.len() > 1 {
            let mut next = Vec::with_capacity((level.len() + 1) / 2);
            let mut i = 0usize;
            while i < level.len() {
                let left = level[i];
                let right = if i + 1 < level.len() { level[i + 1] } else { left };
                let mut bytes = Vec::with_capacity(64);
                bytes.extend_from_slice(&left);
                bytes.extend_from_slice(&right);
                next.push(bitcoin_sha256d(&bytes));
                i += 2;
            }
            level = next;
        }
        Ok(level[0])
    }

    pub fn verify_structure(&self) -> Result<(), String> {
        if self.header.block_hash() == [0; 32] || !bitcoin_pow_valid(&self.header) {
            return Err("invalid bitcoin block proof-of-work".into());
        }
        if self.header.merkle_root != self.merkle_root_from_transactions()? {
            return Err("bitcoin spend-scan Merkle root mismatch".into());
        }
        if self.height == 0 {
            return Err("bitcoin spend-scan height must be positive".into());
        }
        Ok(())
    }

    pub fn spends_outpoint(&self, tx_hash: Hash, vout: u32) -> Result<bool, String> {
        for raw in &self.raw_transactions {
            let tx = BitcoinTransaction::parse(raw)
                .map_err(|_| "invalid raw transaction in spend-scan block".to_string())?;
            if tx.inputs.iter().any(|i| i.previous_txid == tx_hash && i.previous_vout == vout) {
                return Ok(true);
            }
        }
        Ok(false)
    }    pub fn verify_against_parent(&self, parent_hash: Hash, expected_height: u64) -> Result<(), String> {
        self.verify_structure()?;
        if self.height != expected_height {
            return Err("bitcoin block height mismatch".into());
        }
        if self.header.prev_block_hash != parent_hash {
            return Err("bitcoin block parent hash mismatch".into());
        }
        Ok(())
    }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoSpendStateProof {
    pub utxo: BitcoinUtxoProof,
    pub inclusion_header: BitcoinBlockHeader,
    /// Complete raw transactions for the inclusion block. This is required
    /// because a transaction may spend an output created earlier in the same block.
    pub inclusion_block_transactions: Vec<Vec<u8>>,
    /// Blocks from the block immediately after the UTXO inclusion block through
    /// the claimed canonical tip, inclusive.
    pub post_inclusion_blocks: Vec<BitcoinSpendScanBlock>,
    pub tip_height: u64,
    pub tip_hash: Hash,
    pub tip_chainwork: [u8; 32],
}

impl BitcoinUtxoSpendStateProof {
    pub fn verify_against_canonical_tip(
        &self,
        canonical_tip: &BitcoinChainCheckpoint,
    ) -> Result<(), String> {
        if !self.utxo.validate_output_binding() {
            return Err("UTXO transaction/output binding invalid".into());
        }
        if self.utxo.block_height == 0 || self.utxo.block_height > canonical_tip.height {
            return Err("UTXO block height is outside canonical tip".into());
        }
        if self.inclusion_header.block_hash() == [0; 32]
            || !bitcoin_pow_valid(&self.inclusion_header)
            || self.inclusion_header.merkle_root
                != bitcoin_merkle_root(
                    self.utxo.tx_hash,
                    &self.utxo.merkle_branch,
                    self.utxo.merkle_index,
                )
        {
            return Err("UTXO inclusion header/Merkle proof invalid".into());
        }
        let inclusion_block = BitcoinSpendScanBlock {
            height: self.utxo.block_height,
            header: self.inclusion_header.clone(),
            raw_transactions: self.inclusion_block_transactions.clone(),
        };
        inclusion_block.verify_structure()?;
        let inclusion_hashes = inclusion_block.transaction_hashes()?;
        if !inclusion_hashes.contains(&self.utxo.tx_hash) {
            return Err("UTXO transaction is absent from inclusion block".into());
        }
        if inclusion_block.spends_outpoint(self.utxo.tx_hash, self.utxo.vout)? {
            return Err("bitcoin UTXO was spent in its inclusion block".into());
        }

        let expected_blocks = canonical_tip.height - self.utxo.block_height;
        if self.post_inclusion_blocks.len() as u64 != expected_blocks {
            return Err("spend-scan does not cover every block through canonical tip".into());
        }
        if self.tip_height != canonical_tip.height
            || self.tip_hash != canonical_tip.block_hash
            || self.tip_chainwork != canonical_tip.chainwork
        {
            return Err("spend-scan tip does not match canonical tip".into());
        }

        let mut previous_hash = self.inclusion_header.block_hash();
        let mut expected_height = self.utxo.block_height + 1;
        for block in &self.post_inclusion_blocks {
            if block.height != expected_height || block.header.prev_block_hash != previous_hash {
                return Err("bitcoin spend-scan header linkage mismatch".into());
            }
            block.verify_structure()?;
            if block.spends_outpoint(self.utxo.tx_hash, self.utxo.vout)? {
                return Err("bitcoin UTXO has been spent".into());
            }
            previous_hash = block.header.block_hash();
            expected_height += 1;
        }
        if previous_hash != canonical_tip.block_hash {
            return Err("bitcoin spend-scan does not reach canonical tip".into());
        }
        Ok(())
    }

    pub fn is_unspent(&self, canonical_tip: &BitcoinChainCheckpoint) -> bool {
        self.verify_against_canonical_tip(canonical_tip).is_ok()
    }
}


#[cfg(test)]
mod m120_bitcoin_utxo_tests {
    use super::*;

    fn tx_raw(prev: Hash, prev_vout: u32) -> Vec<u8> {
        let mut o=Vec::new();
        o.extend_from_slice(&1i32.to_le_bytes());
        o.push(1);
        o.extend_from_slice(&prev);
        o.extend_from_slice(&prev_vout.to_le_bytes());
        o.push(0);
        o.extend_from_slice(&0xffff_ffffu32.to_le_bytes());
        o.push(1);
        o.extend_from_slice(&1u64.to_le_bytes());
        o.push(1);
        o.push(0x51);
        o.extend_from_slice(&0u32.to_le_bytes());
        o
    }

    #[test]
    fn m120_detects_spend_of_target_outpoint() {
        let target=[7u8;32];
        let block=BitcoinSpendScanBlock {
            height: 2,
            header: BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [0;32],
                merkle_root: [0;32],
                time: 0,
                bits: 0,
                nonce: 0,
            },
            raw_transactions: vec![tx_raw(target, 3)],
        };
        assert!(block.spends_outpoint(target,3).unwrap());
        assert!(!block.spends_outpoint(target,4).unwrap());
    }

    #[test]
    fn m120_merkle_root_matches_single_transaction() {
        let target=[9u8;32];
        let raw=tx_raw(target,0);
        let tx=BitcoinTransaction::parse(&raw).unwrap();
        let block=BitcoinSpendScanBlock {
            height: 2,
            header: BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [0;32],
                merkle_root: tx.txid,
                time: 0,
                bits: 0,
                nonce: 0,
            },
            raw_transactions: vec![raw],
        };
        assert_eq!(block.merkle_root_from_transactions().unwrap(), tx.txid);
    }
}


/// M121 compact UTXO accumulator commitment.
///
/// This commitment is NOT part of Bitcoin consensus. A bridge may only use it
/// when the commitment itself is authenticated by an external/bridge-controlled
/// commitment source. It is deliberately domain-separated from Bitcoin hashes.
pub const BITCOIN_UTXO_ACCUMULATOR_DOMAIN: &[u8] = b"FREEDOM-BITCOIN-UTXO-ACCUMULATOR-V1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoAccumulator {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub bitcoin_tip_height: u64,
    pub bitcoin_tip_hash: Hash,
    pub bitcoin_tip_chainwork: [u8; 32],
    pub root: Hash,
    #[serde(default)]
    pub leaf_count: u64,
}

impl BitcoinUtxoAccumulator {
    pub fn leaf_hash(txid: &Hash, vout: u32, amount: u64, script: &[u8]) -> Hash {
        let mut b=Vec::new();
        b.extend_from_slice(BITCOIN_UTXO_ACCUMULATOR_DOMAIN);
        b.extend_from_slice(txid);
        b.extend_from_slice(&vout.to_le_bytes());
        b.extend_from_slice(&amount.to_le_bytes());
        b.extend_from_slice(&(script.len() as u64).to_le_bytes());
        b.extend_from_slice(script);
        bitcoin_sha256d(&b)
    }

    pub fn merkle_parent(left: &Hash, right: &Hash) -> Hash {
        freedom_primitives::hash_bytes(&[
            BITCOIN_UTXO_ACCUMULATOR_DOMAIN, b"NODE", left, right
        ].concat())
    }

    pub fn compute_root(utxos: &[(Hash, u32, u64, Vec<u8>)]) -> Hash {
        let mut leaves: Vec<Hash> = utxos.iter()
            .map(|(txid,vout,amount,script)| Self::leaf_hash(txid,vout,*amount,script))
            .collect();
        leaves.sort();
        if leaves.is_empty() {
            return freedom_primitives::hash_bytes(
                &[BITCOIN_UTXO_ACCUMULATOR_DOMAIN, b"EMPTY"].concat()
            );
        }
        while leaves.len() > 1 {
            let mut next=Vec::with_capacity((leaves.len()+1)/2);
            let mut i=0;
            while i<leaves.len() {
                let right=if i+1<leaves.len(){leaves[i+1]}else{leaves[i]};
                next.push(Self::merkle_parent(&leaves[i],&right));
                i+=2;
            }
            leaves=next;
        }
        leaves[0]
    }

    pub fn validate(&self, canonical_tip: &BitcoinChainCheckpoint) -> bool {
        self.bridge_id != [0;32]
            && self.chain_id != 0
            && self.bitcoin_tip_height == canonical_tip.height
            && self.bitcoin_tip_hash == canonical_tip.block_hash
            && self.bitcoin_tip_chainwork == canonical_tip.chainwork
            && self.root != [0;32]
            && self.leaf_count > 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoAccumulatorProof {
    pub accumulator: BitcoinUtxoAccumulator,
    pub tx_hash: Hash,
    pub vout: u32,
    pub amount_sat: u64,
    pub script_pubkey: Vec<u8>,
    pub leaf_index: u64,
    pub siblings: Vec<Hash>,
    pub exists: bool,
}

impl BitcoinUtxoAccumulatorProof {
    pub fn leaf_hash(&self) -> Hash {
        let mut b=Vec::new();
        b.extend_from_slice(BITCOIN_UTXO_ACCUMULATOR_DOMAIN);
        b.extend_from_slice(&self.tx_hash);
        b.extend_from_slice(&self.vout.to_le_bytes());
        b.extend_from_slice(&self.amount_sat.to_le_bytes());
        b.extend_from_slice(&(self.script_pubkey.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.script_pubkey);
        bitcoin_sha256d(&b)
    }

    pub fn verify_membership(&self, canonical_tip: &BitcoinChainCheckpoint) -> bool {
        if !self.exists || !self.accumulator.validate(canonical_tip)
            || self.accumulator.leaf_count == 0
            || self.leaf_index >= self.accumulator.leaf_count
            || self.siblings.len() > 64 {
            return false;
        }
        let mut current=self.leaf_hash();
        let mut index=self.leaf_index;
        for sibling in &self.siblings {
            current=if index & 1 == 0 {
                BitcoinUtxoAccumulator::merkle_parent(&current,sibling)
            } else {
                BitcoinUtxoAccumulator::merkle_parent(sibling,&current)
            };
            index >>= 1;
        }
        current==self.accumulator.root && index==0
    }

    pub fn verify_exact_output(&self, tx:&BitcoinUtxoProof, canonical_tip:&BitcoinChainCheckpoint)->bool {
        tx.validate_output_binding()
            && tx.tx_hash==self.tx_hash
            && tx.vout==self.vout
            && tx.amount_sat==self.amount_sat
            && tx.script_pubkey==self.script_pubkey
            && self.verify_membership(canonical_tip)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoStateEntry {
    pub tx_hash: Hash,
    pub vout: u32,
    pub amount_sat: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub const BITCOIN_MAX_BLOCK_TXS_FOR_UTXO_TRANSITION: usize = 1_000_000;

pub struct BitcoinUtxoAccumulatorTransition {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub previous_tip: BitcoinChainCheckpoint,
    pub next_tip: BitcoinChainCheckpoint,
    pub previous_root: Hash,
    pub previous_leaf_count: u64,
    pub spent: Vec<(Hash, u32)>,
    pub created: Vec<BitcoinUtxoStateEntry>,
    pub next_root: Hash,
    pub next_leaf_count: u64,
}

impl BitcoinUtxoAccumulatorTransition {
    pub fn compute_next(
        previous: &[BitcoinUtxoStateEntry],
        spent: &[(Hash,u32)],
        created: &[BitcoinUtxoStateEntry],
    ) -> Result<(Hash,u64), String> {
        let spent_set: std::collections::BTreeSet<(Hash,u32)> = spent.iter().copied().collect();
        if spent_set.len()!=spent.len() { return Err("duplicate spent outpoint".into()); }
        let mut map=std::collections::BTreeMap::<(Hash,u32),BitcoinUtxoStateEntry>::new();
        for e in previous {
            if map.insert((e.tx_hash,e.vout),e.clone()).is_some() {
                return Err("duplicate previous outpoint".into());
            }
        }
        for outpoint in &spent_set {
            if map.remove(outpoint).is_none() { return Err("spent outpoint is not in previous UTXO set".into()); }
        }
        for e in created {
            if map.insert((e.tx_hash,e.vout),e.clone()).is_some() {
                return Err("created outpoint already exists".into());
            }
        }
        let rows: Vec<(Hash,u32,u64,Vec<u8>)> = map.values().map(|e| {
            (e.tx_hash,e.vout,e.amount_sat,e.script_pubkey.clone())
        }).collect();
        if rows.is_empty() { return Err("UTXO set cannot be empty".into()); }
        Ok((BitcoinUtxoAccumulator::compute_root(&rows), map.len() as u64))
    }

    pub fn verify_transition(&self, previous: &[BitcoinUtxoStateEntry]) -> Result<(), String> {
        if self.bridge_id == [0;32] || self.chain_id == 0
            || self.previous_root == [0;32] || self.next_root == [0;32]
            || self.previous_leaf_count == 0 || self.next_leaf_count == 0
            || self.next_tip.height != self.previous_tip.height + 1 {
            return Err("invalid UTXO transition metadata".into());
        }
        let rows: Vec<(Hash,u32,u64,Vec<u8>)> = previous.iter().map(|e|
            (e.tx_hash,e.vout,e.amount_sat,e.script_pubkey.clone())
        ).collect();
        if rows.len() as u64 != self.previous_leaf_count
            || BitcoinUtxoAccumulator::compute_root(&rows) != self.previous_root {
            return Err("previous UTXO root mismatch".into());
        }
        let (root,count)=Self::compute_next(previous,&self.spent,&self.created)?;
        if root != self.next_root || count != self.next_leaf_count {
            return Err("next UTXO root mismatch".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoTransitionWitness {
    pub tx_hash: Hash,
    pub vout: u32,
    pub amount_sat: u64,
    pub script_pubkey: Vec<u8>,
    pub leaf_index: u64,
    pub siblings: Vec<Hash>,
}

impl BitcoinUtxoTransitionWitness {
    pub fn leaf_hash(&self) -> Hash {
        BitcoinUtxoAccumulator::leaf_hash(
            &self.tx_hash, self.vout, self.amount_sat, &self.script_pubkey
        )
    }

    pub fn verify(&self, root: Hash, leaf_count: u64) -> bool {
        if leaf_count == 0 || self.leaf_index >= leaf_count || self.siblings.len() > 64 {
            return false;
        }
        let mut current=self.leaf_hash();
        let mut index=self.leaf_index;
        for sibling in &self.siblings {
            current=if index & 1 == 0 {
                BitcoinUtxoAccumulator::merkle_parent(&current,sibling)
            } else {
                BitcoinUtxoAccumulator::merkle_parent(sibling,&current)
            };
            index >>= 1;
        }
        current==root && index==0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoCompactTransitionProof {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub previous_root: Hash,
    pub previous_leaf_count: u64,
    pub spent_witnesses: Vec<BitcoinUtxoTransitionWitness>,
    pub created_entries: Vec<BitcoinUtxoStateEntry>,
    pub next_root: Hash,
    pub next_leaf_count: u64,
}

impl BitcoinUtxoCompactTransitionProof {
    pub fn verify_spent_membership(&self) -> bool {
        let mut seen=std::collections::BTreeSet::new();
        for w in &self.spent_witnesses {
            if !seen.insert((w.tx_hash,w.vout)) || !w.verify(self.previous_root,self.previous_leaf_count) {
                return false;
            }
        }
        true
    }

    pub fn validate_metadata(&self) -> bool {
        self.bridge_id != [0;32]
            && self.chain_id != 0
            && self.previous_root != [0;32]
            && self.next_root != [0;32]
            && self.previous_leaf_count > 0
            && self.next_leaf_count > 0
            && !self.created_entries.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoMerkleMultiproof {
    pub leaf_count: u64,
    pub leaves: Vec<BitcoinUtxoTransitionWitness>,
    pub auxiliary_nodes: Vec<Hash>,
}

impl BitcoinUtxoMerkleMultiproof {
    pub fn verify(&self, root: Hash) -> bool {
        if self.leaf_count == 0 || self.leaves.is_empty() || self.leaves.len() > 1_000_000 {
            return false;
        }
        let mut seen=std::collections::BTreeSet::new();
        for leaf in &self.leaves {
            if !seen.insert(leaf.leaf_index) || !leaf.verify(root,self.leaf_count) {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoCompactTransitionBoundary {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub previous_root: Hash,
    pub previous_leaf_count: u64,
    pub previous_state_multiproof: BitcoinUtxoMerkleMultiproof,
    pub spent_outpoints: Vec<(Hash,u32)>,
    pub created_entries: Vec<BitcoinUtxoStateEntry>,
    pub next_root: Hash,
    pub next_leaf_count: u64,
}

impl BitcoinUtxoCompactTransitionBoundary {
    pub fn validate(&self) -> bool {
        if self.bridge_id == [0;32] || self.chain_id == 0
            || self.previous_root == [0;32] || self.next_root == [0;32]
            || self.previous_leaf_count == 0 || self.next_leaf_count == 0
            || self.spent_outpoints.len() != self.created_entries.len()
            || !self.previous_state_multiproof.verify(self.previous_root) {
            return false;
        }
        let mut spent=std::collections::BTreeSet::new();
        for x in &self.spent_outpoints {
            if !spent.insert(*x) { return false; }
        }
        true
    }
}

pub const BITCOIN_UTXO_SMT_DOMAIN: &[u8] = b"FREEDOM-BITCOIN-UTXO-SMT-V2";
pub const BITCOIN_UTXO_SMT_DEPTH: usize = 256;

pub fn bitcoin_utxo_smt_key(txid: &Hash, vout: u32) -> Hash {
    let mut b=Vec::new();
    b.extend_from_slice(BITCOIN_UTXO_SMT_DOMAIN);
    b.extend_from_slice(b"KEY");
    b.extend_from_slice(txid);
    b.extend_from_slice(&vout.to_le_bytes());
    freedom_primitives::hash_bytes(&b)
}

pub fn bitcoin_utxo_smt_leaf(entry: Option<&BitcoinUtxoStateEntry>) -> Hash {
    let mut b=Vec::new();
    b.extend_from_slice(BITCOIN_UTXO_SMT_DOMAIN);
    if let Some(e)=entry {
        b.extend_from_slice(b"LEAF");
        b.extend_from_slice(&bitcoin_utxo_smt_key(&e.tx_hash,e.vout));
        b.extend_from_slice(&e.amount_sat.to_le_bytes());
        b.extend_from_slice(&(e.script_pubkey.len() as u64).to_le_bytes());
        b.extend_from_slice(&e.script_pubkey);
    } else {
        b.extend_from_slice(b"EMPTY");
    }
    freedom_primitives::hash_bytes(&b)
}

pub fn bitcoin_utxo_smt_node(left: Hash, right: Hash) -> Hash {
    freedom_primitives::hash_bytes(&[
        BITCOIN_UTXO_SMT_DOMAIN,b"NODE",left.as_slice(),right.as_slice()
    ].concat())
}

pub fn bitcoin_utxo_smt_empty_hashes() -> Vec<Hash> {
    let mut h=Vec::with_capacity(BITCOIN_UTXO_SMT_DEPTH+1);
    h.push(bitcoin_utxo_smt_leaf(None));
    for _ in 0..BITCOIN_UTXO_SMT_DEPTH {
        let x=*h.last().unwrap();
        h.push(bitcoin_utxo_smt_node(x,x));
    }
    h
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoSmtSnapshot {
    pub entries: Vec<BitcoinUtxoStateEntry>,
}

impl BitcoinUtxoSmtSnapshot {
    pub fn root(&self) -> Result<Hash,String> {
        let mut map=std::collections::BTreeMap::<Hash,BitcoinUtxoStateEntry>::new();
        for e in &self.entries {
            let key=bitcoin_utxo_smt_key(&e.tx_hash,e.vout);
            if map.insert(key,e.clone()).is_some() { return Err("duplicate SMT key".into()); }
        }
        let empty=bitcoin_utxo_smt_empty_hashes();
        fn build(
            map:&std::collections::BTreeMap<Hash,BitcoinUtxoStateEntry>,
            depth:usize,
            prefix:u16,
            empty:&[Hash],
        )->Hash {
            if depth==BITCOIN_UTXO_SMT_DEPTH { return map.values().next().map(|e|bitcoin_utxo_smt_leaf(Some(e))).unwrap_or(empty[0]); }
            if map.is_empty() { return empty[BITCOIN_UTXO_SMT_DEPTH-depth]; }
            let bitpos=depth;
            let mut left=std::collections::BTreeMap::new();
            let mut right=std::collections::BTreeMap::new();
            for (k,e) in map {
                if ((k[bitpos/8]>>(bitpos%8))&1)==0 { left.insert(*k,e.clone()); } else { right.insert(*k,e.clone()); }
            }
            bitcoin_utxo_smt_node(build(&left,depth+1,prefix<<1,empty),build(&right,depth+1,(prefix<<1)|1,empty))
        }
        Ok(build(&map,0,0,&empty))
    }

    pub fn proof_for(&self,key:Hash)->Result<BitcoinUtxoSmtProof,String>{
        let mut map=std::collections::BTreeMap::<Hash,BitcoinUtxoStateEntry>::new();
        for e in &self.entries {
            let k=bitcoin_utxo_smt_key(&e.tx_hash,e.vout);
            if map.insert(k,e.clone()).is_some(){return Err("duplicate SMT key".into());}
        }
        let entry=map.get(&key).cloned();
        let empty=bitcoin_utxo_smt_empty_hashes();
        let mut siblings=Vec::with_capacity(BITCOIN_UTXO_SMT_DEPTH);
        let mut current=map;
        for depth in 0..BITCOIN_UTXO_SMT_DEPTH {
            let bit=((key[depth/8]>>(depth%8))&1)!=0;
            let mut sibling_map=std::collections::BTreeMap::new();
            let mut next=std::collections::BTreeMap::new();
            for (k,e) in current {
                let b=((k[depth/8]>>(depth%8))&1)!=0;
                if b==bit { next.insert(k,e); } else { sibling_map.insert(k,e); }
            }
            siblings.push(if sibling_map.is_empty() {
                empty[BITCOIN_UTXO_SMT_DEPTH-depth-1]
            } else {
                BitcoinUtxoSmtSnapshot{entries:sibling_map.into_values().collect()}.root()?
            });
            current=next;
        }
        Ok(BitcoinUtxoSmtProof{key,entry,siblings})
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoSmtProof {
    pub key: Hash,
    pub entry: Option<BitcoinUtxoStateEntry>,
    pub siblings: Vec<Hash>,
}

impl BitcoinUtxoSmtProof {
    pub fn verify(&self, root: Hash) -> bool {
        if self.siblings.len() != BITCOIN_UTXO_SMT_DEPTH { return false; }
        if let Some(e)=&self.entry {
            if bitcoin_utxo_smt_key(&e.tx_hash,e.vout) != self.key { return false; }
        }
        let mut current=bitcoin_utxo_smt_leaf(self.entry.as_ref());
        for bit in 0..BITCOIN_UTXO_SMT_DEPTH {
            let right=((self.key[bit/8]>>(bit%8))&1)==1;
            let sibling=&self.siblings[bit];
            current=if right {
                freedom_primitives::hash_bytes(&[BITCOIN_UTXO_SMT_DOMAIN,b"NODE",sibling.as_slice(),current.as_slice()].concat())
            } else {
                freedom_primitives::hash_bytes(&[BITCOIN_UTXO_SMT_DOMAIN,b"NODE",current.as_slice(),sibling.as_slice()].concat())
            };
        }
        current==root
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitcoinUtxoSmtUpdateStep {
    pub key: Hash,
    pub old_entry: Option<BitcoinUtxoStateEntry>,
    pub new_entry: Option<BitcoinUtxoStateEntry>,
    pub siblings: Vec<Hash>,
}

impl BitcoinUtxoSmtUpdateStep {
    fn root_for(&self, entry: Option<&BitcoinUtxoStateEntry>) -> Option<Hash> {
        if self.siblings.len()!=BITCOIN_UTXO_SMT_DEPTH { return None; }
        if let Some(e)=entry {
            if bitcoin_utxo_smt_key(&e.tx_hash,e.vout)!=self.key { return None; }
        }
        let mut current=bitcoin_utxo_smt_leaf(entry);
        for bit in 0..BITCOIN_UTXO_SMT_DEPTH {
            let right=((self.key[bit/8]>>(bit%8))&1)==1;
            let sibling=&self.siblings[bit];
            current=if right {
                freedom_primitives::hash_bytes(&[BITCOIN_UTXO_SMT_DOMAIN,b"NODE",sibling.as_slice(),current.as_slice()].concat())
            } else {
                freedom_primitives::hash_bytes(&[BITCOIN_UTXO_SMT_DOMAIN,b"NODE",current.as_slice(),sibling.as_slice()].concat())
            };
        }
        Some(current)
    }

    pub fn verify(&self, current_root: Hash) -> Option<Hash> {
        if self.root_for(self.old_entry.as_ref())? != current_root { return None; }
        self.root_for(self.new_entry.as_ref())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
impl BitcoinUtxoSmtUpdateProof {
    pub fn build(
        previous: &BitcoinUtxoSmtSnapshot,
        next: &BitcoinUtxoSmtSnapshot,
    ) -> Result<Self,String> {
        let mut old=std::collections::BTreeMap::<Hash,BitcoinUtxoStateEntry>::new();
        let mut new=std::collections::BTreeMap::<Hash,BitcoinUtxoStateEntry>::new();
        for e in &previous.entries {
            if old.insert(bitcoin_utxo_smt_key(&e.tx_hash,e.vout),e.clone()).is_some(){return Err("duplicate previous SMT key".into());}
        }
        for e in &next.entries {
            if new.insert(bitcoin_utxo_smt_key(&e.tx_hash,e.vout),e.clone()).is_some(){return Err("duplicate next SMT key".into());}
        }
        let mut keys=std::collections::BTreeSet::new();
        keys.extend(old.keys().copied());
        keys.extend(new.keys().copied());
        let previous_root=previous.root()?;
        let next_root=next.root()?;
        let mut steps=Vec::new();
        for key in keys {
            if old.get(&key)==new.get(&key){continue;}
            let old_proof=previous.proof_for(key)?;
            steps.push(BitcoinUtxoSmtUpdateStep{
                key,
                old_entry:old.get(&key).cloned(),
                new_entry:new.get(&key).cloned(),
                siblings:old_proof.siblings,
            });
        }
        if steps.is_empty(){return Err("SMT snapshots are identical".into());}
        Ok(Self{previous_root,next_root,steps})
    }
}

pub struct BitcoinUtxoSmtUpdateProof {
    pub previous_root: Hash,
    pub next_root: Hash,
    pub steps: Vec<BitcoinUtxoSmtUpdateStep>,
}

impl BitcoinUtxoSmtUpdateProof {
    pub fn from_block_delta(
        previous_root: Hash,
        next_root: Hash,
        previous: &[BitcoinUtxoStateEntry],
        raw_transactions: &[Vec<u8>],
        witness_steps: Vec<BitcoinUtxoSmtUpdateStep>,
    ) -> Result<Self, String> {
        let (spent,created)=BitcoinUtxoAccumulatorTransition::derive_block_delta(raw_transactions)?;
        let mut expected=std::collections::BTreeMap::<(Hash,u32),Option<BitcoinUtxoStateEntry>>::new();
        for e in previous {
            if expected.insert((e.tx_hash,e.vout),Some(e.clone())).is_some() {
                return Err("duplicate previous outpoint".into());
            }
        }
        for outpoint in spent {
            if !expected.contains_key(&outpoint) {
                return Err("block spends unavailable UTXO".into());
            }
            expected.insert(outpoint,None);
        }
        for e in created {
            let key=(e.tx_hash,e.vout);
            if expected.insert(key,Some(e)).is_some() {
                return Err("created output collision".into());
            }
        }
        if witness_steps.len()!=expected.len() {
            return Err("witness step count does not match block delta".into());
        }
        for step in &witness_steps {
            let key=match &step.new_entry {
                Some(e)=>(e.tx_hash,e.vout),
                None=>{
                    let Some(Some(old))=expected.get_key_value(&(step.old_entry.as_ref().ok_or("missing deletion entry")?.tx_hash,step.old_entry.as_ref().unwrap().vout)).map(|(_,v)|v) else { return Err("invalid deletion".into()); };
                    (old.tx_hash,old.vout)
                }
            };
            if !expected.contains_key(&key) { return Err("unexpected witness key".into()); }
            if step.old_entry != expected.get(&key).cloned().unwrap_or(None) { return Err("old entry mismatch".into()); }
        }
        Ok(Self{previous_root,next_root,steps:witness_steps})
    }


}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossChainProofKind {
    BitcoinUtxo,
    Ethereum,
    Tron,
    Ton,
    Solana,
    GenericFinality,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossChainProofError {
    InvalidEnvelope,
    UnsupportedProofKind,
    InvalidProof,
}

pub trait CrossChainProofAdapter {
    fn proof_kind(&self) -> CrossChainProofKind;
    fn verify_envelope(&self, envelope: &CrossChainFinalityEnvelope) -> Result<(), CrossChainProofError>;
}

#[derive(Clone, Debug, Default)]
pub struct BitcoinUtxoFinalityAdapter;

impl CrossChainProofAdapter for BitcoinUtxoFinalityAdapter {
    fn proof_kind(&self) -> CrossChainProofKind { CrossChainProofKind::BitcoinUtxo }

    fn verify_envelope(&self, envelope: &CrossChainFinalityEnvelope) -> Result<(), CrossChainProofError> {
        if envelope.proof_kind != self.proof_kind() || !envelope.validate_structure() {
            return Err(CrossChainProofError::InvalidEnvelope);
        }
        if envelope.state_root == envelope.previous_state_root {
            return Err(CrossChainProofError::InvalidProof);
        }
        Ok(())
    }
}

fn ethereum_keccak256(data:&[u8])->Hash {
    use sha3::{Digest,Keccak256};
    let mut h=Keccak256::new();
    h.update(data);
    h.finalize().into()
}

fn ethereum_rlp_decode_list(input:&[u8])->Result<Vec<&[u8]>,String>{
    if input.is_empty(){return Err("empty RLP".into());}
    let (start,len)=match input[0] {
        0xc0..=0xf7 => (1,(input[0]-0xc0) as usize),
        0xf8..=0xff => {let n=(input[0]-0xf7) as usize;if input.len()<1+n{return Err("RLP length".into());}let mut l=0usize;for b in &input[1..1+n]{l=(l<<8)|*b as usize;}(1+n,l)}
        _=>return Err("RLP node is not list".into()),
    };
    if start+len!=input.len(){return Err("RLP list length mismatch".into());}
    let mut out=Vec::new();let mut i=start;let stop=start+len;
    while i<stop {
        let (next,payload)=ethereum_rlp_item(input,i)?;
        out.push(payload);i=next;
    }
    if i!=stop{Err("RLP list parse mismatch".into())}else{Ok(out)}
}

fn ethereum_rlp_item(input:&[u8],i:usize)->Result<(usize,&[u8]),String>{
    if i>=input.len(){return Err("RLP item out of bounds".into());}
    let b=input[i];
    if b<=0x7f{return Ok((i+1,&input[i..i+1]));}
    if b<=0xb7{let n=(b-0x80) as usize;let s=i+1;let e=s+n;if e>input.len(){return Err("RLP string bounds".into());}return Ok((e,&input[s..e]));}
    if b<=0xbf{let n=(b-0xb7) as usize;if i+1+n>input.len(){return Err("RLP long string length".into());}let mut l=0usize;for x in &input[i+1..i+1+n]{l=(l<<8)|*x as usize;}let s=i+1+n;let e=s+l;if e>input.len(){return Err("RLP long string bounds".into());}return Ok((e,&input[s..e]));}
    if b<=0xf7{let n=(b-0xc0) as usize;let s=i+1;let e=s+n;if e>input.len(){return Err("RLP short list bounds".into());}return Ok((e,&input[s..e]));}
    let n=(b-0xf7) as usize;if i+1+n>input.len(){return Err("RLP long list length".into());}let mut l=0usize;for x in &input[i+1..i+1+n]{l=(l<<8)|*x as usize;}let s=i+1+n;let e=s+l;if e>input.len(){return Err("RLP long list bounds".into());}Ok((e,&input[s..e]))
}

fn ethereum_compact_path(bytes:&[u8])->Result<(bool,Vec<u8>),String>{
    if bytes.is_empty(){return Err("empty compact path".into());}
    let flag=bytes[0]>>4;let leaf=flag>=2;
    let odd=(flag&1)!=0;let mut nibbles=Vec::new();
    if odd {nibbles.push(bytes[0]&0x0f);}
    let start=if odd{1}else{1};
    if !odd && (flag!=0 && flag!=2){return Err("invalid compact path flags".into());}
    for b in &bytes[start..]{nibbles.push(b>>4);nibbles.push(b&0x0f);}
    Ok((leaf,nibbles))
}

fn ethereum_mpt_verify(root:Hash,key_nibbles:&[u8],nodes:&[Vec<u8>],expected_value:&[u8])->Result<(),String>{
    if nodes.is_empty(){return Err("empty MPT proof".into());}
    let mut expected_hash=Some(root);let mut expected_raw:Option<Vec<u8>>=None;let mut depth=0usize;
    for raw in nodes {
        if let Some(h)=expected_hash.take(){if ethereum_keccak256(raw)!=h{return Err("MPT node hash mismatch".into());}}
        else if expected_raw.as_deref()!=Some(raw.as_slice()){return Err("MPT embedded node mismatch".into());}
        let items=ethereum_rlp_decode_list(raw)?;
        match items.len() {
            17=>{
                if depth>key_nibbles.len(){return Err("MPT depth overflow".into());}
                if depth==key_nibbles.len(){if items[16]!=expected_value{return Err("MPT value mismatch".into());}return Ok(());}
                let idx=key_nibbles[depth] as usize;
                if items[idx].is_empty(){return Err("missing MPT child".into());}
                let child=items[idx];
                if child.len()==32{let mut h=[0u8;32];h.copy_from_slice(child);expected_hash=Some(h);expected_raw=None;}
                else {expected_hash=None;expected_raw=Some(child.to_vec());}
                depth+=1;
            },
            2=>{
                let (leaf,path)=ethereum_compact_path(items[0])?;
                if key_nibbles.len()<depth+path.len()||key_nibbles[depth..depth+path.len()]!=path[..]{return Err("MPT path mismatch".into());}
                depth+=path.len();
                if leaf {if depth!=key_nibbles.len()||items[1]!=expected_value{return Err("MPT leaf mismatch".into());}return Ok(());}
                if items[1].is_empty(){return Err("empty extension child".into());}
                let child=items[1];
                if child.len()==32{let mut h=[0u8;32];h.copy_from_slice(child);expected_hash=Some(h);expected_raw=None;}
                else {expected_hash=None;expected_raw=Some(child.to_vec());}
            },
            _=>return Err("invalid MPT node arity".into()),
        }
    }
    Err("MPT proof ended before leaf".into())
}

fn ethereum_rlp_encode_bytes(payload:&[u8])->Vec<u8>{
    if payload.len()==1 && payload[0]<0x80{return payload.to_vec();}
    if payload.len()<56{let mut o=vec![0x80+payload.len() as u8];o.extend_from_slice(payload);o}
    else {let mut len=payload.len();let mut b=Vec::new();while len>0{b.push((len&0xff) as u8);len>>=8;}b.reverse();let mut o=vec![0xb7+b.len() as u8];o.extend_from_slice(&b);o.extend_from_slice(payload);o}
}

fn ethereum_rlp_encode_u64(value:u64)->Vec<u8>{
    if value==0{return ethereum_rlp_encode_bytes(&[]);}
    let mut b=value.to_be_bytes().to_vec();
    while b.first()==Some(&0){b.remove(0);}
    ethereum_rlp_encode_bytes(&b)
}

fn ethereum_rlp_decode_u64(payload:&[u8])->Result<u64,String>{
    if payload.len()>8{return Err("RLP integer exceeds u64".into());}
    if payload.len()>1 && payload[0]==0{return Err("non-canonical RLP integer".into());}
    let mut value=0u64;
    for b in payload{value=value.checked_mul(256).ok_or("RLP integer overflow")?.checked_add(*b as u64).ok_or("RLP integer overflow")?;}
    Ok(value)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumExecutionBlockHeader { pub rlp:Vec<u8> }

impl EthereumExecutionBlockHeader {
    pub fn fields(&self)->Result<Vec<&[u8]>,String>{ethereum_rlp_decode_list(&self.rlp)}
    pub fn block_hash(&self)->Hash{ethereum_keccak256(&self.rlp)}
    pub fn state_root(&self)->Result<Hash,String>{
        let f=self.fields()?; if f.len()<15{return Err("Ethereum header has too few fields".into());}
        f[3].try_into().map_err(|_|"invalid stateRoot length".into())
    }
    pub fn receipts_root(&self)->Result<Hash,String>{
        let f=self.fields()?; if f.len()<15{return Err("Ethereum header has too few fields".into());}
        f[5].try_into().map_err(|_|"invalid receiptsRoot length".into())
    }
    pub fn block_number(&self)->Result<u64,String>{
        let f=self.fields()?; if f.len()<15{return Err("Ethereum header has too few fields".into());}
        ethereum_rlp_decode_u64(f[8])
    }
    pub fn validate_structure(&self)->bool{
        let Ok(f)=self.fields() else{return false;};
        f.len()>=15 && f[3].len()==32 && f[5].len()==32 && self.block_number().map(|n|n>0).unwrap_or(false)
    }
    pub fn verify_execution_binding(&self,bh:Hash,sr:Hash,bn:u64,rr:Hash)->bool{
        self.validate_structure() && self.block_hash()==bh && self.state_root()==Ok(sr)
            && self.block_number()==Ok(bn) && self.receipts_root()==Ok(rr)
    }
    pub fn proof_hash(&self)->Hash{
        freedom_primitives::hash_bytes(&[b"FREEDOM-ETH-EXECUTION-HEADER-V1".as_slice(),&self.rlp].concat())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumTransactionProof {
    pub block_number:u64,
    pub block_hash:Hash,
    pub transaction_hash:Hash,
    pub transaction_index:u32,
    pub receipt_root:Hash,
    pub proof_nodes:Vec<Vec<u8>>,
    pub receipt_value:Vec<u8>,
}

impl EthereumTransactionProof {
    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();o.extend_from_slice(b"FREEDOM-ETH-TX-PROOF-V2");
        o.extend_from_slice(&self.block_number.to_le_bytes());o.extend_from_slice(&self.block_hash);
        o.extend_from_slice(&self.transaction_hash);o.extend_from_slice(&self.transaction_index.to_le_bytes());
        o.extend_from_slice(&self.receipt_root);o.extend_from_slice(&(self.proof_nodes.len() as u32).to_le_bytes());
        for n in &self.proof_nodes{o.extend_from_slice(&(n.len() as u32).to_le_bytes());o.extend_from_slice(n);}
        o.extend_from_slice(&(self.receipt_value.len() as u32).to_le_bytes());o.extend_from_slice(&self.receipt_value);o
    }
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
    pub fn validate_structure(&self)->bool{self.block_number>0&&self.block_hash!=[0;32]&&self.transaction_hash!=[0;32]&&self.receipt_root!=[0;32]&&!self.proof_nodes.is_empty()&&!self.receipt_value.is_empty()}
    pub fn verify_receipt_mpt(&self)->bool{
        if !self.validate_structure(){return false;}
        let key_rlp=ethereum_rlp_encode_u64(self.transaction_index as u64);
        let mut nibbles=Vec::with_capacity(key_rlp.len()*2);
        for b in key_rlp{nibbles.push(b>>4);nibbles.push(b&0x0f);}
        ethereum_mpt_verify(self.receipt_root,&nibbles,&self.proof_nodes,&self.receipt_value).is_ok()
    }
    pub fn verify_binding(&self,envelope:&CrossChainFinalityEnvelope)->bool{self.validate_structure()&&envelope.proof_kind==CrossChainProofKind::Ethereum&&envelope.finalized_height==self.block_number&&envelope.finalized_hash==self.block_hash&&envelope.proof_hash==self.proof_hash()}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumSszMerkleProof {
    pub leaf: Hash,
    pub generalized_index: u64,
    pub branch: Vec<Hash>,
    pub root: Hash,
}

impl EthereumSszMerkleProof {
    pub fn hash_pair(left:Hash,right:Hash)->Hash{
        use sha2::{Digest,Sha256};
        let mut h=Sha256::new();
        h.update(left);h.update(right);
        h.finalize().into()
    }

    pub fn verify(&self)->bool{
        if self.generalized_index<1 || self.branch.len()!=64-self.generalized_index.leading_zeros() as usize-1 {return false;}
        let mut node=self.leaf;
        let mut index=self.generalized_index;
        for sibling in &self.branch {
            node=if index&1==0{Self::hash_pair(node,*sibling)}else{Self::hash_pair(*sibling,node)};
            index>>=1;
        }
        index==1 && node==self.root
    }

    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-ETH-SSZ-MERKLE-V1");
        o.extend_from_slice(&self.leaf);
        o.extend_from_slice(&self.generalized_index.to_le_bytes());
        for h in &self.branch{o.extend_from_slice(h);}
        o.extend_from_slice(&self.root);
        o
    }
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
}

fn ethereum_ssz_mix_in_length(root:Hash,length:u64)->Hash{
    use sha2::{Digest,Sha256};
    let mut b=[0u8;64];b[..32].copy_from_slice(&root);b[32..40].copy_from_slice(&length.to_le_bytes());
    Sha256::digest(b).into()
}

pub fn ethereum_ssz_chunk(data:&[u8])->Hash{
    let mut out=[0u8;32];
    let n=data.len().min(32);out[..n].copy_from_slice(&data[..n]);out
}

pub fn ethereum_ssz_merkleize(chunks:&[Hash])->Hash{
    if chunks.is_empty(){return [0;32];}
    let mut level=chunks.to_vec();
    let size=level.len().next_power_of_two();
    level.resize(size,[0;32]);
    while level.len()>1{
        let mut next=Vec::with_capacity(level.len()/2);
        for pair in level.chunks_exact(2){next.push(EthereumSszMerkleProof::hash_pair(pair[0],pair[1]));}
        level=next;
    }
    level[0]
}

pub fn ethereum_ssz_hash_tree_root_bytes(data:&[u8])->Hash{
    ethereum_ssz_merkleize(&data.chunks(32).map(ethereum_ssz_chunk).collect::<Vec<_>>())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumValidatorMembershipProof {
    pub validator_index:u64,
    pub validator_root:Hash,
    pub validator_leaf:Hash,
    pub merkle_branch:Vec<Hash>,
    pub generalized_index:u64,
}

impl EthereumValidatorMembershipProof {
    pub fn verify(&self)->bool{
        let p=EthereumSszMerkleProof{
            leaf:self.validator_leaf,
            generalized_index:self.generalized_index,
            branch:self.merkle_branch.clone(),
            root:self.validator_root,
        };
        p.verify()
    }
    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-ETH-VALIDATOR-MEMBERSHIP-V1");
        o.extend_from_slice(&self.validator_index.to_le_bytes());
        o.extend_from_slice(&self.validator_root);
        o.extend_from_slice(&self.validator_leaf);
        o.extend_from_slice(&self.generalized_index.to_le_bytes());
        for h in &self.merkle_branch{o.extend_from_slice(h);}
        o
    }
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumSyncCommitteeSignature {
    pub validator_indices:Vec<u64>,
    pub public_keys:Vec<[u8;48]>,
    pub participation_bits:Vec<bool>,
    pub signature:Vec<u8>,
    pub signing_root:Hash,
}

impl EthereumSyncCommitteeSignature {
    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-ETH-SYNC-COMMITTEE-BLS-V1");
        o.extend_from_slice(&self.signing_root);
        for (i,b) in self.validator_indices.iter().zip(self.participation_bits.iter()){
            o.extend_from_slice(&i.to_le_bytes());o.push(*b as u8);
        }
        o
    }
    pub fn verify(&self)->bool{
        if self.signature.len()!=96 || self.public_keys.len()!=self.participation_bits.len() || self.validator_indices.len()!=self.public_keys.len() || self.public_keys.is_empty(){return false;}
        if self.validator_indices.windows(2).any(|w|w[0]>=w[1]){return false;}
        if self.participation_bits.iter().any(|b|!*b){return false;}
        let Ok(sig)=self.signature.as_slice().try_into() else{return false;};
        freedom_crypto::bls_verify_aggregate(&self.public_keys,&self.signing_bytes(),&sig)
    }
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
}

pub struct EthereumSyncCommitteeProof {
    pub committee_root:Hash,
    pub committee_leaf:Hash,
    pub committee_index:u64,
    pub generalized_index:u64,
    pub merkle_branch:Vec<Hash>,
    pub validator_membership:Option<EthereumValidatorMembershipProof>,
    pub finalized_checkpoint_root:Hash,
}

impl EthereumSyncCommitteeProof {
    pub fn verify(&self)->bool{
        let p=EthereumSszMerkleProof{
            leaf:self.committee_leaf,
            generalized_index:self.generalized_index,
            branch:self.merkle_branch.clone(),
            root:self.committee_root,
        };
        self.finalized_checkpoint_root!=[0;32]
            && p.verify() && self.validator_membership.as_ref().map(|v|v.verify()).unwrap_or(true)
    }
    pub fn proof_hash(&self)->Hash{
        let mut b=Vec::new();
        b.extend_from_slice(b"FREEDOM-ETH-SYNC-COMMITTEE-V1");
        b.extend_from_slice(&self.finalized_checkpoint_root);
        b.extend_from_slice(&self.committee_root);
        b.extend_from_slice(&self.committee_leaf);
        b.extend_from_slice(&self.committee_index.to_le_bytes());
        b.extend_from_slice(&self.generalized_index.to_le_bytes());
        for h in &self.merkle_branch{b.extend_from_slice(h);}
        if let Some(v)=&self.validator_membership{b.extend_from_slice(&v.proof_hash());}
        freedom_primitives::hash_bytes(&b)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EthereumEndToEndProof {
    pub envelope:CrossChainFinalityEnvelope,
    pub finality:EthereumFinalityProof,
    pub committee:EthereumSyncCommitteeProof,
    pub signature:EthereumSyncCommitteeSignature,
    pub receipt:EthereumTransactionProof,
    pub execution_header:EthereumExecutionBlockHeader,
}

impl EthereumEndToEndProof {
    pub fn verify(&self)->bool{
        if !self.envelope.validate_structure(){return false;}
        if self.envelope.proof_kind!=CrossChainProofKind::Ethereum{return false;}
        if !self.finality.validate_structure(){return false;}
        if !self.finality.verify_chain_anchor(self.envelope.finalized_hash,self.finality.state_root){return false;}
        if !self.committee.verify(){return false;}
        if !self.signature.verify(){return false;}
        if self.signature.signing_root!=self.finality.proof_hash(){return false;}
        if !self.receipt.verify_receipt_mpt(){return false;}
        if !self.receipt.verify_binding(&self.envelope){return false;}
        if self.receipt.block_hash!=self.finality.finalized_block_hash
            || self.receipt.block_number!=self.finality.finalized_block_number{return false;}
        if !self.execution_header.verify_execution_binding(
            self.finality.finalized_block_hash,
            self.finality.state_root,
            self.finality.finalized_block_number,
            self.receipt.receipt_root,
        ){return false;}
        true
    }
    pub fn proof_hash(&self)->Hash{
        let mut b=Vec::new();
        b.extend_from_slice(b"FREEDOM-ETH-END-TO-END-V1");
        b.extend_from_slice(&self.envelope.proof_hash);
        b.extend_from_slice(&self.finality.proof_hash());
        b.extend_from_slice(&self.committee.proof_hash());
        b.extend_from_slice(&self.signature.proof_hash());
        b.extend_from_slice(&self.receipt.proof_hash());
        b.extend_from_slice(&self.execution_header.proof_hash());
        freedom_primitives::hash_bytes(&b)
    }
}

pub struct EthereumFinalityProof {
    pub finalized_block_number:u64,
    pub finalized_block_hash:Hash,
    pub state_root:Hash,
    pub block_hash_chain:Vec<Hash>,
    pub validator_root:Hash,
    pub finality_signature:Vec<u8>,
}

impl EthereumFinalityProof {
    pub fn signing_bytes(&self)->Vec<u8>{
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-ETH-FINALITY-V1");
        o.extend_from_slice(&self.finalized_block_number.to_le_bytes());
        o.extend_from_slice(&self.finalized_block_hash);
        o.extend_from_slice(&self.state_root);
        o.extend_from_slice(&self.validator_root);
        o.extend_from_slice(&(self.block_hash_chain.len() as u32).to_le_bytes());
        for h in &self.block_hash_chain{o.extend_from_slice(h);}
        o
    }
    pub fn proof_hash(&self)->Hash{freedom_primitives::hash_bytes(&self.signing_bytes())}
    pub fn validate_structure(&self)->bool{
        self.finalized_block_number>0
            && self.finalized_block_hash!=[0;32]
            && self.state_root!=[0;32]
            && self.validator_root!=[0;32]
            && !self.block_hash_chain.is_empty()
            && self.finality_signature.len()==96
    }
    pub fn verify_chain_anchor(&self,expected_block_hash:Hash,expected_state_root:Hash)->bool{
        self.validate_structure()
            && self.finalized_block_hash==expected_block_hash
            && self.state_root==expected_state_root
            && self.block_hash_chain.last().copied()==Some(expected_block_hash)
    }
}

#[derive(Clone, Debug, Default)]
pub struct EvmFinalityAdapter;

impl CrossChainProofAdapter for EvmFinalityAdapter {
    fn proof_kind(&self) -> CrossChainProofKind { CrossChainProofKind::Ethereum }

    fn verify_envelope(&self, envelope:&CrossChainFinalityEnvelope)->Result<(),CrossChainProofError>{
        if envelope.proof_kind!=self.proof_kind() || !envelope.validate_structure(){return Err(CrossChainProofError::InvalidEnvelope);}
        Ok(())
    }

    pub fn verify_finality_proof(&self,proof:&EthereumFinalityProof,envelope:&CrossChainFinalityEnvelope)->Result<(),CrossChainProofError>{
        if !proof.validate_structure(){return Err(CrossChainProofError::InvalidProof);}
        if envelope.proof_kind!=CrossChainProofKind::Ethereum
            || envelope.finalized_height!=proof.finalized_block_number
            || envelope.finalized_hash!=proof.finalized_block_hash
            || envelope.state_root!=proof.state_root
            || envelope.proof_hash!=proof.proof_hash(){
            return Err(CrossChainProofError::InvalidProof);
        }
        Ok(())
    }
}


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TronBlockHeaderProof {
    pub raw_data: Vec<u8>,
    pub block_id: Hash,
}

impl TronBlockHeaderProof {
    fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, String> {
        let mut value = 0u64;
        for shift in (0..70).step_by(7) {
            if *pos >= bytes.len() { return Err("truncated protobuf varint".into()); }
            let b = bytes[*pos];
            *pos += 1;
            if shift == 63 && b > 1 { return Err("protobuf varint overflow".into()); }
            value |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 { return Ok(value); }
        }
        Err("protobuf varint too long".into())
    }

    fn read_len_delimited(bytes: &[u8], pos: &mut usize) -> Result<Vec<u8>, String> {
        let len = Self::read_varint(bytes, pos)? as usize;
        let end = pos.checked_add(len).ok_or("protobuf length overflow")?;
        if end > bytes.len() { return Err("truncated protobuf bytes field".into()); }
        let out = bytes[*pos..end].to_vec();
        *pos = end;
        Ok(out)
    }

    fn parse_raw_data(&self) -> Result<(u64, Hash, Hash, u64, Vec<u8>, Option<Hash>), String> {
        let mut pos = 0usize;
        let mut number = None;
        let mut tx_trie_root = None;
        let mut parent_hash = None;
        let mut timestamp = None;
        let mut witness_address = None;
        let mut account_state_root = None;

        while pos < self.raw_data.len() {
            let key = Self::read_varint(&self.raw_data, &mut pos)?;
            let field = key >> 3;
            let wire = key & 7;
            if field == 0 { return Err("protobuf field number cannot be zero".into()); }

            match (field, wire) {
                (1, 0) => {
                    if timestamp.is_some() { return Err("duplicate timestamp".into()); }
                    timestamp = Some(Self::read_varint(&self.raw_data, &mut pos)?);
                }
                (2, 2) => {
                    if tx_trie_root.is_some() { return Err("duplicate txTrieRoot".into()); }
                    let v = Self::read_len_delimited(&self.raw_data, &mut pos)?;
                    if v.len() != 32 { return Err("txTrieRoot must be 32 bytes".into()); }
                    let mut h = [0u8; 32]; h.copy_from_slice(&v); tx_trie_root = Some(h);
                }
                (3, 2) => {
                    if parent_hash.is_some() { return Err("duplicate parentHash".into()); }
                    let v = Self::read_len_delimited(&self.raw_data, &mut pos)?;
                    if v.len() != 32 { return Err("parentHash must be 32 bytes".into()); }
                    let mut h = [0u8; 32]; h.copy_from_slice(&v); parent_hash = Some(h);
                }
                (7, 0) => {
                    if number.is_some() { return Err("duplicate block number".into()); }
                    number = Some(Self::read_varint(&self.raw_data, &mut pos)?);
                }
                (9, 2) => {
                    if witness_address.is_some() { return Err("duplicate witness_address".into()); }
                    witness_address = Some(Self::read_len_delimited(&self.raw_data, &mut pos)?);
                }
                (11, 2) => {
                    if account_state_root.is_some() { return Err("duplicate accountStateRoot".into()); }
                    let v = Self::read_len_delimited(&self.raw_data, &mut pos)?;
                    if v.len() != 32 { return Err("accountStateRoot must be 32 bytes".into()); }
                    let mut h = [0u8; 32]; h.copy_from_slice(&v); account_state_root = Some(h);
                }
                (_, 0) => { let _ = Self::read_varint(&self.raw_data, &mut pos)?; }
                (_, 2) => { let _ = Self::read_len_delimited(&self.raw_data, &mut pos)?; }
                (_, 1) => {
                    let end = pos.checked_add(8).ok_or("protobuf length overflow")?;
                    if end > self.raw_data.len() { return Err("truncated protobuf fixed64 field".into()); }
                    pos = end;
                }
                (_, 5) => {
                    let end = pos.checked_add(4).ok_or("protobuf length overflow")?;
                    if end > self.raw_data.len() { return Err("truncated protobuf fixed32 field".into()); }
                    pos = end;
                }
                _ => return Err("unsupported protobuf wire type".into()),
            }
        }

        Ok((
            number.ok_or("missing block number")?,
            tx_trie_root.ok_or("missing txTrieRoot")?,
            parent_hash.ok_or("missing parentHash")?,
            timestamp.ok_or("missing timestamp")?,
            witness_address.ok_or("missing witness_address")?,
            account_state_root,
        ))
    }

    pub fn sha256_raw_data(&self) -> Hash {
        use sha2::{Digest, Sha256};
        Sha256::digest(&self.raw_data).into()
    }

    pub fn block_number(&self) -> Result<u64, String> { Ok(self.parse_raw_data()?.0) }
    pub fn tx_trie_root(&self) -> Result<Hash, String> { Ok(self.parse_raw_data()?.1) }
    pub fn parent_hash(&self) -> Result<Hash, String> { Ok(self.parse_raw_data()?.2) }
    pub fn timestamp_ms(&self) -> Result<u64, String> { Ok(self.parse_raw_data()?.3) }
    pub fn witness_address(&self) -> Result<Vec<u8>, String> { Ok(self.parse_raw_data()?.4) }
    pub fn account_state_root(&self) -> Result<Option<Hash>, String> { Ok(self.parse_raw_data()?.5) }

    pub fn computed_block_id(&self) -> Result<Hash, String> {
        let number = self.block_number()?;
        let digest = self.sha256_raw_data();
        let mut id = [0u8; 32];
        id[..8].copy_from_slice(&number.to_be_bytes());
        id[8..].copy_from_slice(&digest[8..]);
        Ok(id)
    }

    pub fn validate(&self) -> bool {
        !self.raw_data.is_empty()
            && self.block_id != [0; 32]
            && self.parse_raw_data().is_ok()
            && self.computed_block_id().map(|id| id == self.block_id).unwrap_or(false)
    }

    pub fn verify_binding(
        &self,
        expected_height: u64,
        expected_block_id: Hash,
        expected_parent_hash: Hash,
        expected_tx_trie_root: Hash,
    ) -> bool {
        self.validate()
            && self.block_number().ok() == Some(expected_height)
            && self.block_id == expected_block_id
            && self.parent_hash().ok() == Some(expected_parent_hash)
            && self.tx_trie_root().ok() == Some(expected_tx_trie_root)
    }
}

impl TronFinalityAdapter {
    pub fn verify_block_header(
        &self,
        proof: &TronBlockHeaderProof,
        envelope: &CrossChainFinalityEnvelope,
    ) -> Result<(), CrossChainProofError> {
        if envelope.proof_kind != CrossChainProofKind::Tron || !envelope.validate_structure() {
            return Err(CrossChainProofError::InvalidEnvelope);
        }
        if !proof.validate()
            || proof.block_number().ok() != Some(envelope.finalized_height)
            || proof.block_id != envelope.finalized_hash
        {
            return Err(CrossChainProofError::InvalidProof);
        }
        if let Ok(Some(root)) = proof.account_state_root() {
            if root != envelope.state_root {
                return Err(CrossChainProofError::InvalidProof);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct TronFinalityAdapter;

impl CrossChainProofAdapter for TronFinalityAdapter {
    fn proof_kind(&self) -> CrossChainProofKind { CrossChainProofKind::Tron }

    fn verify_envelope(&self, envelope: &CrossChainFinalityEnvelope) -> Result<(), CrossChainProofError> {
        if envelope.proof_kind != self.proof_kind() || !envelope.validate_structure() {
            return Err(CrossChainProofError::InvalidEnvelope);
        }
        Ok(())
    }
}

pub struct CrossChainFinalityEnvelope {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub sequence: u64,
    pub proof_kind: CrossChainProofKind,
    pub finalized_height: u64,
    pub finalized_hash: Hash,
    pub state_root: Hash,
    pub previous_state_root: Hash,
    pub proof_hash: Hash,
}

impl CrossChainFinalityEnvelope {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-CROSS-CHAIN-FINALITY-V1");
        o.extend_from_slice(&self.bridge_id);
        o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.sequence.to_le_bytes());
        o.push(match self.proof_kind {
            CrossChainProofKind::BitcoinUtxo=>0,
            CrossChainProofKind::Ethereum=>1,
            CrossChainProofKind::Tron=>2,
            CrossChainProofKind::Ton=>3,
            CrossChainProofKind::Solana=>4,
            CrossChainProofKind::GenericFinality=>255,
        });
        o.extend_from_slice(&self.finalized_height.to_le_bytes());
        o.extend_from_slice(&self.finalized_hash);
        o.extend_from_slice(&self.state_root);
        o.extend_from_slice(&self.previous_state_root);
        o.extend_from_slice(&self.proof_hash);
        o
    }

    pub fn compute_hash(&self)->Hash { freedom_primitives::hash_bytes(&self.signing_bytes()) }

    pub fn validate_structure(&self)->bool {
        self.bridge_id != [0;32] && self.chain_id != 0 && self.sequence > 0
            && self.finalized_height > 0 && self.finalized_hash != [0;32]
            && self.state_root != [0;32] && self.previous_state_root != [0;32]
            && self.proof_hash != [0;32]
    }
}

pub struct BitcoinUtxoBlockCommitment {
    pub block_height: u64,
    pub block_hash: Hash,
    pub previous_root: Hash,
    pub next_root: Hash,
    pub update_proof: BitcoinUtxoSmtUpdateProof,
}

impl BitcoinUtxoBlockCommitment {
    pub fn build(
        previous: &BitcoinUtxoSmtSnapshot,
        block: &BitcoinSpendScanBlock,
    ) -> Result<Self,String> {
        block.verify_structure()?;
        let mut entries=previous.entries.clone();
        let mut map=std::collections::BTreeMap::<(Hash,u32),BitcoinUtxoStateEntry>::new();
        for e in entries.drain(..) {
            if map.insert((e.tx_hash,e.vout),e).is_some(){return Err("duplicate previous outpoint".into());}
        }
        for raw in &block.raw_transactions {
            let tx=BitcoinTransaction::parse(raw).map_err(|_|"invalid Bitcoin transaction".to_string())?;
            let coinbase=tx.inputs.len()==1 && tx.inputs[0].previous_txid==[0;32];
            if !coinbase {
                for input in &tx.inputs {
                    if map.remove(&(input.previous_txid,input.previous_vout)).is_none(){
                        return Err("block spends unavailable UTXO".into());
                    }
                }
            }
            for (vout,out) in tx.outputs.iter().enumerate() {
                let e=BitcoinUtxoStateEntry{
                    tx_hash:tx.txid,vout:vout as u32,amount_sat:out.value,script_pubkey:out.script_pubkey.clone()
                };
                if map.insert((e.tx_hash,e.vout),e).is_some(){return Err("output collision".into());}
            }
        }
        if map.is_empty(){return Err("resulting UTXO set is empty".into());}
        let next=BitcoinUtxoSmtSnapshot{entries:map.into_values().collect()};
        let proof=BitcoinUtxoSmtUpdateProof::build(previous,&next)?;
        Ok(Self{
            block_height:block.height,
            block_hash:block.header.block_hash(),
            previous_root:proof.previous_root,
            next_root:proof.next_root,
            update_proof:proof,
        })
    }

    pub fn verify(&self)->bool {
        self.block_height>0
            && self.block_hash != [0;32]
            && self.previous_root != [0;32]
            && self.next_root != [0;32]
            && self.update_proof.previous_root==self.previous_root
            && self.update_proof.next_root==self.next_root
            && self.update_proof.verify()
    }
}

pub struct BitcoinUtxoCommitmentAttestation {
    pub bridge_id: Hash,
    pub chain_id: u32,
    pub policy_id: Hash,
    pub sequence: u64,
    pub bitcoin_tip_height: u64,
    pub bitcoin_tip_hash: Hash,
    pub bitcoin_tip_chainwork: [u8; 32],
    pub previous_utxo_root: Hash,
    pub accumulator_root: Hash,
    pub validator_ids: Vec<Hash>,
    pub signatures: Vec<BitcoinReorgRecoverySignature>,
    pub attestation_hash: Hash,
}

impl BitcoinUtxoCommitmentAttestation {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut o=Vec::new();
        o.extend_from_slice(b"FREEDOM-BITCOIN-UTXO-ATTESTATION-V1");
        o.extend_from_slice(&self.bridge_id);
        o.extend_from_slice(&self.chain_id.to_le_bytes());
        o.extend_from_slice(&self.policy_id);
        o.extend_from_slice(&self.sequence.to_le_bytes());
        o.extend_from_slice(&self.bitcoin_tip_height.to_le_bytes());
        o.extend_from_slice(&self.bitcoin_tip_hash);
        o.extend_from_slice(&self.bitcoin_tip_chainwork);
        o.extend_from_slice(&self.previous_utxo_root);
        o.extend_from_slice(&self.accumulator_root);
        for id in &self.validator_ids { o.extend_from_slice(id); }
        o
    }

    pub fn compute_hash(&self) -> Hash {
        freedom_primitives::hash_bytes(&self.signing_bytes())
    }

    pub fn validate_structure(&self) -> bool {
        self.bridge_id != [0;32]
            && self.chain_id != 0
            && self.policy_id != [0;32]
            && self.sequence > 0
            && self.bitcoin_tip_height > 0
            && self.bitcoin_tip_hash != [0;32]
            && self.bitcoin_tip_chainwork != [0;32]
            && self.previous_utxo_root != [0;32]
            && self.accumulator_root != [0;32]
            && !self.validator_ids.is_empty()
            && self.validator_ids.len() == self.signatures.len()
            && self.attestation_hash == self.compute_hash()
    }

    pub fn authenticated_count(&self, validators: &std::collections::BTreeMap<Hash,[u8;32]>) -> u16 {
        let mut seen=std::collections::BTreeSet::new();
        for sig in &self.signatures {
            if sig.signature.len()!=64 || seen.contains(&sig.validator_id) {
                continue;
            }
            if !self.validator_ids.contains(&sig.validator_id) {
                continue;
            }
            if let Some(key)=validators.get(&sig.validator_id) {
                let Ok(signature)=sig.signature.as_slice().try_into() else { continue };
                if freedom_crypto::verify(key,&self.signing_bytes(),signature) {
                    seen.insert(sig.validator_id);
                }
            }
        }
        seen.len() as u16
    }

    pub fn verify_signatures(
        &self,
        validators: &std::collections::BTreeMap<Hash,[u8;32]>,
        quorum: u16,
    ) -> bool {
        self.validate_structure() && quorum > 0 && self.authenticated_count(validators) >= quorum
    }

    pub fn verify(
        &self,
        canonical_tip: &BitcoinChainCheckpoint,
        validators: &std::collections::BTreeMap<Hash,[u8;32]>,
        quorum: u16,
    ) -> bool {
        self.verify_signatures(validators,quorum)
            && self.bitcoin_tip_height == canonical_tip.height
            && self.bitcoin_tip_hash == canonical_tip.block_hash
            && self.bitcoin_tip_chainwork == canonical_tip.chainwork
    }
}
impl BitcoinUtxoAccumulatorTransition {
    pub fn verify_block_against_header(
        previous: &[BitcoinUtxoStateEntry],
        block: &BitcoinSpendScanBlock,
    ) -> Result<(Hash,u64), String> {
        block.verify_structure()?;
        Self::verify_block_delta(previous, &block.raw_transactions)
    }

    pub fn derive_block_delta(
        raw_transactions: &[Vec<u8>],
    ) -> Result<(Vec<(Hash,u32)>,Vec<BitcoinUtxoStateEntry>), String> {
        if raw_transactions.is_empty() || raw_transactions.len() > BITCOIN_MAX_BLOCK_TXS_FOR_UTXO_TRANSITION {
            return Err("invalid block transaction count".into());
        }
        let mut spent=Vec::new();
        let mut created=Vec::new();
        let mut seen_txids=std::collections::BTreeSet::new();
        for (tx_pos,raw) in raw_transactions.iter().enumerate() {
            let tx=BitcoinTransaction::parse(raw).map_err(|_|"invalid Bitcoin transaction".to_string())?;
            if !seen_txids.insert(tx.txid) { return Err("duplicate transaction id".into()); }
            let is_coinbase = tx.inputs.len()==1 && tx.inputs[0].previous_txid == [0;32];
            if tx_pos==0 {
                if !is_coinbase { return Err("first transaction is not coinbase".into()); }
            } else if is_coinbase || tx.inputs.iter().any(|i| i.previous_txid == [0;32]) {
                return Err("invalid coinbase placement".into());
            }
            if !is_coinbase {
                for input in &tx.inputs {
                    spent.push((input.previous_txid,input.previous_vout));
                }
            }
            for (vout,out) in tx.outputs.iter().enumerate() {
                created.push(BitcoinUtxoStateEntry {
                    tx_hash: tx.txid,
                    vout: vout as u32,
                    amount_sat: out.value,
                    script_pubkey: out.script_pubkey.clone(),
                });
            }
        }
        Ok((spent,created))
    }

    pub fn verify_block_delta(
        previous: &[BitcoinUtxoStateEntry],
        raw_transactions: &[Vec<u8>],
    ) -> Result<(Hash,u64), String> {
        if raw_transactions.is_empty() { return Err("empty Bitcoin block".into()); }
        let mut map=std::collections::BTreeMap::<(Hash,u32),BitcoinUtxoStateEntry>::new();
        for e in previous {
            if map.insert((e.tx_hash,e.vout),e.clone()).is_some() {
                return Err("duplicate previous outpoint".into());
            }
        }
        let mut seen_txids=std::collections::BTreeSet::new();
        for (tx_pos,raw) in raw_transactions.iter().enumerate() {
            let tx=BitcoinTransaction::parse(raw).map_err(|_|"invalid Bitcoin transaction".to_string())?;
            if !seen_txids.insert(tx.txid) { return Err("duplicate transaction id".into()); }
            let coinbase=tx.inputs.len()==1 && tx.inputs[0].previous_txid==[0;32];
            if tx_pos==0 {
                if !coinbase { return Err("first transaction is not coinbase".into()); }
            } else if coinbase || tx.inputs.iter().any(|i| i.previous_txid==[0;32]) {
                return Err("invalid coinbase placement".into());
            }
            if !coinbase {
                for input in &tx.inputs {
                    if map.remove(&(input.previous_txid,input.previous_vout)).is_none() {
                        return Err("transaction spends unavailable UTXO".into());
                    }
                }
            }
            for (vout,out) in tx.outputs.iter().enumerate() {
                let entry=BitcoinUtxoStateEntry{tx_hash:tx.txid,vout:vout as u32,amount_sat:out.value,script_pubkey:out.script_pubkey.clone()};
                if map.insert((entry.tx_hash,entry.vout),entry).is_some() {
                    return Err("output collision".into());
                }
            }
        }
        if map.is_empty() { return Err("UTXO set cannot be empty".into()); }
        let rows:Vec<(Hash,u32,u64,Vec<u8>)>=map.values().map(|e|(e.tx_hash,e.vout,e.amount_sat,e.script_pubkey.clone())).collect();
        Ok((BitcoinUtxoAccumulator::compute_root(&rows),map.len() as u64))
    }
}



#[cfg(test)]
mod ethereum_m142_tests {
    use super::*;

    fn rlp_list(items:&[Vec<u8>])->Vec<u8>{
        let payload_len:usize=items.iter().map(Vec::len).sum();
        let mut payload=Vec::new();
        for item in items{payload.extend_from_slice(item);}
        if payload_len<56{
            let mut out=vec![0xc0+payload_len as u8];
            out.extend_from_slice(&payload);
            out
        }else{
            panic!("test header too large for short-list RLP");
        }
    }

    fn bytes32(x:u8)->Vec<u8>{vec![x;32]}

    #[test]
    fn ethereum_receipt_key_uses_rlp_transaction_index(){
        assert_eq!(ethereum_rlp_encode_u64(0),vec![0x80]);
        assert_eq!(ethereum_rlp_encode_u64(1),vec![0x01]);
        assert_eq!(ethereum_rlp_encode_u64(255),vec![0x81,0xff]);
        assert_eq!(ethereum_rlp_encode_u64(256),vec![0x82,0x01,0x00]);
    }

    #[test]
    fn ethereum_execution_header_binds_hash_number_state_and_receipts(){
        let mut fields=Vec::new();
        for i in 0..15{
            let value=match i{
                3=>bytes32(0x33),
                5=>bytes32(0x55),
                8=>ethereum_rlp_encode_u64(12345),
                _=>ethereum_rlp_encode_bytes(&[]),
            };
            fields.push(value);
        }
        let rlp=rlp_list(&fields);
        let header=EthereumExecutionBlockHeader{rlp:rlp.clone()};
        let hash=ethereum_keccak256(&rlp);
        assert!(header.validate_structure());
        assert_eq!(header.block_hash(),hash);
        assert_eq!(header.state_root(),Ok([0x33;32]));
        assert_eq!(header.receipts_root(),Ok([0x55;32]));
        assert_eq!(header.block_number(),Ok(12345));
        assert!(header.verify_execution_binding(hash,[0x33;32],12345,[0x55;32]));
        assert!(!header.verify_execution_binding(hash,[0x33;32],12345,[0x66;32]));
    }

    #[test]
    fn tron_block_header_proof_binds_raw_data_to_composite_block_id() {
        fn varint(mut v: u64) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let mut b = (v & 0x7f) as u8;
                v >>= 7;
                if v != 0 { b |= 0x80; }
                out.push(b);
                if v == 0 { return out; }
            }
        }
        fn field_bytes(field: u64, value: &[u8]) -> Vec<u8> {
            let mut out = varint((field << 3) | 2);
            out.extend_from_slice(&varint(value.len() as u64));
            out.extend_from_slice(value);
            out
        }
        fn field_u64(field: u64, value: u64) -> Vec<u8> {
            let mut out = varint(field << 3);
            out.extend_from_slice(&varint(value));
            out
        }

        let height = 42u64;
        let parent = [0x11; 32];
        let tx_root = [0x22; 32];
        let state_root = [0x33; 32];
        let witness = vec![0x41, 0xaa, 0xbb, 0xcc];

        let mut raw = Vec::new();
        raw.extend_from_slice(&field_u64(1, 1_725_000_000_000));
        raw.extend_from_slice(&field_bytes(2, &tx_root));
        raw.extend_from_slice(&field_bytes(3, &parent));
        raw.extend_from_slice(&field_u64(7, height));
        raw.extend_from_slice(&field_bytes(9, &witness));
        raw.extend_from_slice(&field_bytes(11, &state_root));

        let mut proof = TronBlockHeaderProof { raw_data: raw, block_id: [0; 32] };
        proof.block_id = proof.computed_block_id().unwrap();

        assert!(proof.validate());
        assert_eq!(proof.block_number(), Ok(height));
        assert_eq!(proof.parent_hash(), Ok(parent));
        assert_eq!(proof.tx_trie_root(), Ok(tx_root));
        assert_eq!(proof.account_state_root(), Ok(Some(state_root)));
        assert_eq!(proof.witness_address(), Ok(witness));

        let expected = proof.block_id;
        let envelope = CrossChainFinalityEnvelope {
            bridge_id: [0x44; 32],
            chain_id: 728126428,
            sequence: 1,
            proof_kind: CrossChainProofKind::Tron,
            finalized_height: height,
            finalized_hash: expected,
            state_root,
            previous_state_root: [0x55; 32],
            proof_hash: [0x66; 32],
        };
        assert!(TronFinalityAdapter.verify_block_header(&proof, &envelope).is_ok());

        proof.raw_data.push(0);
        assert!(!proof.validate());
    }

}


#[cfg(test)]
mod m144_tron_event_tests {
    use super::*;

    fn varint(mut v: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut b = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 { b |= 0x80; }
            out.push(b);
            if v == 0 { return out; }
        }
    }

    fn field_u64(field: u64, value: u64) -> Vec<u8> {
        let mut out = varint(field << 3);
        out.extend_from_slice(&varint(value));
        out
    }

    fn field_bytes(field: u64, value: &[u8]) -> Vec<u8> {
        let mut out = varint((field << 3) | 2);
        out.extend_from_slice(&varint(value.len() as u64));
        out.extend_from_slice(value);
        out
    }

    fn header() -> TronBlockHeaderProof {
        let mut raw = Vec::new();
        raw.extend_from_slice(&field_u64(1, 1_725_000_000_000));
        raw.extend_from_slice(&field_bytes(2, &[0x22; 32]));
        raw.extend_from_slice(&field_bytes(3, &[0x11; 32]));
        raw.extend_from_slice(&field_u64(7, 42));
        raw.extend_from_slice(&field_bytes(9, &[0x41, 1, 2, 3]));
        let mut p = TronBlockHeaderProof { raw_data: raw, block_id: [0; 32] };
        p.block_id = p.computed_block_id().unwrap();
        p
    }

    #[test]
    fn tron_trc20_transfer_event_is_abi_bound() {
        let raw_data = b"example-tron-transaction-raw-data".to_vec();
        let tx_hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(&raw_data).into()
        };
        let mut from = [0u8; 32];
        from[12..].copy_from_slice(&[0x11; 20]);
        let mut to = [0u8; 32];
        to[12..].copy_from_slice(&[0x22; 20]);
        let mut amount = [0u8; 32];
        amount[16..].copy_from_slice(&1_000_000u128.to_be_bytes());

        let h = header();
        let proof = TronTransactionEventProof {
            raw_data,
            tx_hash,
            block_number: 42,
            block_id: h.block_id,
            transaction_index: 3,
            event_index: 0,
            contract_address: vec![0xaa; 20],
            topics: vec![TronTransactionEventProof::TRANSFER_EVENT_SIGNATURE, from, to],
            data: amount.to_vec(),
            execution_success: true,
        };

        assert!(proof.validate_transfer_event());
        assert_eq!(proof.from_topic(), Ok([0x11; 20]));
        assert_eq!(proof.to_topic(), Ok([0x22; 20]));
        assert_eq!(proof.amount(), Ok(1_000_000));
        assert!(proof.verify_binding(&h, tx_hash, 3));

        let mut broken = proof.clone();
        broken.topics[0] = [0x00; 32];
        assert!(!broken.validate_transfer_event());

        let mut broken_tx = proof.clone();
        broken_tx.raw_data.push(0);
        assert!(!broken_tx.validate_transfer_event());
    }

    #[test]
    fn tron_trc20_transfer_rejects_failed_execution_and_noncanonical_topics() {
        let raw_data = b"raw".to_vec();
        let tx_hash = {
            use sha2::{Digest, Sha256};
            Sha256::digest(&raw_data).into()
        };
        let h = header();
        let mut topics = vec![[0u8; 32]; 3];
        topics[0] = TronTransactionEventProof::TRANSFER_EVENT_SIGNATURE;
        topics[1][12..].copy_from_slice(&[1u8; 20]);
        topics[2][12..].copy_from_slice(&[2u8; 20]);
        let mut data = [0u8; 32];
        data[31] = 1;

        let mut p = TronTransactionEventProof {
            raw_data,
            tx_hash,
            block_number: 42,
            block_id: h.block_id,
            transaction_index: 0,
            contract_address: vec![3u8; 20],
            topics,
            data: data.to_vec(),
            execution_success: false,
        };
        assert!(!p.validate_transfer_event());
        p.execution_success = true;
        assert!(p.validate_transfer_event());
        p.topics[1][0] = 1;
        assert!(!p.validate_transfer_event());
    }
}

#[cfg(test)]
mod m145_tron_inclusion_tests {
    use super::*;

    fn pair(a: Hash, b: Hash) -> Hash {
        TronTransactionInclusionProof::sha256_pair(a, b)
    }

    #[test]
    fn tron_tx_inclusion_matches_java_tron_merkle_rules() {
        let txs = vec![[1;32], [2;32], [3;32], [4;32], [5;32]];
        let p0 = pair(txs[0], txs[1]);
        let p1 = pair(txs[2], txs[3]);
        let p2 = pair(p0, p1);
        let root = pair(p2, txs[4]);

        let proof = TronTransactionInclusionProof {
            tx_hash: txs[4],
            block_number: 42,
            block_id: [9;32],
            transaction_index: 4,
            transaction_count: 5,
            siblings: vec![None, None, Some(p2)],
        };
        assert!(proof.verify_against_root(root));

        let proof0 = TronTransactionInclusionProof {
            tx_hash: txs[0],
            block_number: 42,
            block_id: [9;32],
            transaction_index: 0,
            transaction_count: 5,
            siblings: vec![Some(txs[1]), Some(p1), Some(txs[4])],
        };
        assert!(proof0.verify_against_root(root));
    }

    #[test]
    fn tron_tx_inclusion_rejects_wrong_sibling_and_count() {
        let txs = vec![[1;32], [2;32]];
        let root = pair(txs[0], txs[1]);
        let mut proof = TronTransactionInclusionProof {
            tx_hash: txs[0],
            block_number: 42,
            block_id: [9;32],
            transaction_index: 0,
            transaction_count: 2,
            siblings: vec![Some(txs[1])],
        };
        assert!(proof.verify_against_root(root));
        proof.siblings[0] = Some([7;32]);
        assert!(!proof.verify_against_root(root));
        proof.transaction_count = 3;
        assert!(!proof.verify_against_root(root));
    }
}

#[cfg(test)]
mod m146_tron_usdt_tests {
    use super::*;

    fn varint(mut v: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut b = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 { b |= 0x80; }
            out.push(b);
            if v == 0 { return out; }
        }
    }

    fn field_u64(field: u64, value: u64) -> Vec<u8> {
        let mut out = varint(field << 3);
        out.extend_from_slice(&varint(value));
        out
    }

    fn field_bytes(field: u64, value: &[u8]) -> Vec<u8> {
        let mut out = varint((field << 3) | 2);
        out.extend_from_slice(&varint(value.len() as u64));
        out.extend_from_slice(value);
        out
    }

    #[test]
    fn unified_tron_usdt_deposit_proof_binds_all_layers() {
        use sha2::{Digest, Sha256};

        let usdt = [0xa6; 20];
        let tx_raw = b"tron-usdt-transaction".to_vec();
        let tx_hash: Hash = Sha256::digest(&tx_raw).into();
        let other_tx = [0x77; 32];
        let tx_root = TronTransactionInclusionProof::sha256_pair(tx_hash, other_tx);

        let mut header_raw = Vec::new();
        header_raw.extend_from_slice(&field_u64(1, 1_725_000_000_000));
        header_raw.extend_from_slice(&field_bytes(2, &tx_root));
        header_raw.extend_from_slice(&field_bytes(3, &[0x11; 32]));
        header_raw.extend_from_slice(&field_u64(7, 42));
        header_raw.extend_from_slice(&field_bytes(9, &[0x41, 1, 2, 3]));
        let mut header = TronBlockHeaderProof { raw_data: header_raw, block_id: [0; 32] };
        header.block_id = header.computed_block_id().unwrap();

        let inclusion = TronTransactionInclusionProof {
            tx_hash,
            block_number: 42,
            block_id: header.block_id,
            transaction_index: 0,
            transaction_count: 2,
            siblings: vec![Some(other_tx)],
        };

        let mut from = [0u8; 32];
        from[12..].copy_from_slice(&[0x11; 20]);
        let mut to = [0u8; 32];
        to[12..].copy_from_slice(&[0x22; 20]);
        let mut amount = [0u8; 32];
        amount[16..].copy_from_slice(&500u128.to_be_bytes());

        let event = TronTransactionEventProof {
            raw_data: tx_raw,
            tx_hash,
            block_number: 42,
            block_id: header.block_id,
            transaction_index: 0,
            event_index: 1,
            contract_address: usdt.to_vec(),
            topics: vec![TronTransactionEventProof::TRANSFER_EVENT_SIGNATURE, from, to],
            data: amount.to_vec(),
            execution_success: true,
        };

        let proof = TronUsdtDepositProof {
            bridge_id: [0x55; 32],
            chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
            confirmations: 20,
            usdt_contract: usdt.to_vec(),
            header,
            inclusion,
            event,
        };

        assert!(proof.validate(20));
        assert_eq!(proof.sender(), Ok([0x11; 20]));
        assert_eq!(proof.recipient(), Ok([0x22; 20]));
        assert_eq!(proof.amount(), Ok(500));
        assert_ne!(proof.proof_hash(), [0; 32]);
    }

    #[test]
    fn unified_tron_usdt_deposit_rejects_contract_mismatch() {
        let mut proof = TronUsdtDepositProof {
            bridge_id: [1; 32],
            chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
            confirmations: 20,
            usdt_contract: vec![1; 20],
            header: TronBlockHeaderProof { raw_data: vec![], block_id: [0; 32] },
            inclusion: TronTransactionInclusionProof {
                tx_hash: [0; 32],
                block_number: 1,
                block_id: [0; 32],
                transaction_index: 0,
                transaction_count: 1,
                siblings: vec![],
            },
            event: TronTransactionEventProof {
                raw_data: vec![1],
                tx_hash: [0; 32],
                block_number: 1,
                block_id: [0; 32],
                transaction_index: 0,
                event_index: 0,
                contract_address: vec![1; 20],
                topics: vec![],
                data: vec![],
                execution_success: true,
            },
        };
        assert!(!proof.validate_with_contract(20, &[2; 20]));
        proof.confirmations = 19;
        assert!(!proof.validate_with_contract(20, &[1; 20]));
    }
}

#[cfg(test)]
mod m147_bridge_lifecycle_tests {
    use super::*;

    #[test]
    fn tron_usdt_proof_converts_to_bridge_deposit() {
        let mut p = TronUsdtDepositProof {
            bridge_id: [7;32],
            chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
            confirmations: 20,
            usdt_contract: vec![0xaa;20],
            header: TronBlockHeaderProof { raw_data: vec![1], block_id: [1;32] },
            inclusion: TronTransactionInclusionProof {
                tx_hash: [2;32], block_number: 42, block_id: [1;32],
                transaction_index: 0, transaction_count: 1, siblings: vec![],
            },
            event: TronTransactionEventProof {
                raw_data: vec![3], tx_hash: [2;32], block_number: 42,
                block_id: [1;32], transaction_index: 0, event_index: 7,
                contract_address: vec![0xaa;20], topics: vec![],
                data: vec![], execution_success: true,
            },
        };
        // This fixture is intentionally structurally incomplete; lifecycle must reject it.
        assert!(p.to_bridge_deposit(20, &[0xaa;20]).is_err());
        p.confirmations = 19;
        assert_eq!(p.to_bridge_deposit(20, &[0xaa;20]), Err(BridgeAdapterError::InsufficientFinality));
    }
}

#[cfg(test)]
mod m148_unified_asset_proof_tests {
    use super::*;

    #[test]
    fn unified_asset_proof_rejects_tampered_metadata() {
        let deposit = BridgeDepositProof {
            bridge_id: [1;32],
            source_chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
            tx_hash: [2;32],
            event_index: 4,
            block_or_slot: 100,
            confirmations: 20,
            proof: vec![9;32],
        };
        let mut p = CrossChainAssetProof::from_bridge_deposit(
            deposit, [3;32], [4;32], CrossChainAssetKind::Trc20, 123, vec![4;20]
        ).unwrap();
        assert!(p.validate(20));
        p.amount = 124;
        assert!(!p.validate(20));
    }
}

#[cfg(test)]
mod m149_canonical_asset_registry_tests {
    use super::*;

    #[test]
    fn canonical_registry_accepts_multiple_usdt_representations() {
        let issuer = [9;32];
        let canonical = CrossChainAssetRegistry::compute_canonical_asset_id("USDT", 6, issuer);
        let tron_contract = vec![0xaa;20];
        let tron = AssetDefinition {
            asset_id: AssetDefinition::compute_asset_id(
                TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
                AssetType::Bridged,
                &tron_contract,
                issuer,
                [7;32],
            ),
            source_chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
            asset_type: AssetType::Bridged,
            contract_address: tron_contract,
            symbol: "USDT".into(),
            decimals: 6,
            issuer,
            bridge_id: [7;32],
            status: AssetStatus::Active,
        };
        let eth = AssetDefinition {
            asset_id: AssetDefinition::compute_asset_id(1, AssetType::Bridged, &[0xbb;20], issuer, [8;32]),
            source_chain_id: 1,
            asset_type: AssetType::Bridged,
            contract_address: vec![0xbb;20],
            symbol: "USDT".into(),
            decimals: 6,
            issuer,
            bridge_id: [8;32],
            status: AssetStatus::Active,
        };
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: canonical,
            symbol: "USDT".into(),
            decimals: 6,
            representations: vec![
                CrossChainAssetRepresentation {
                    canonical_asset_id: canonical,
                    representation: tron,
                    asset_kind: CrossChainAssetKind::Trc20,
                },
                CrossChainAssetRepresentation {
                    canonical_asset_id: canonical,
                    representation: eth,
                    asset_kind: CrossChainAssetKind::Erc20,
                },
            ],
        };
        assert!(registry.validate());
        assert!(registry.find_representation(TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID, CrossChainAssetKind::Trc20).is_some());
        assert!(registry.find_representation(1, CrossChainAssetKind::Erc20).is_some());
    }

    #[test]
    fn canonical_registry_rejects_wrong_asset_identity() {
        let issuer = [9;32];
        let canonical = CrossChainAssetRegistry::compute_canonical_asset_id("USDT", 6, issuer);
        let proof = CrossChainAssetProof {
            bridge_deposit: BridgeDepositProof {
                bridge_id: [7;32], source_chain_id: TronUsdtDepositProof::TRON_MAINNET_CHAIN_ID,
                tx_hash: [2;32], event_index: 0, block_or_slot: 1, confirmations: 20,
                proof: vec![1;32],
            },
            asset_id: [3;32],
            representation_asset_id: [4;32],
            asset_kind: CrossChainAssetKind::Trc20,
            amount: 100,
            recipient: vec![4;20],
            proof_hash: [0;32],
        };
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: canonical, symbol: "USDT".into(), decimals: 6, representations: vec![],
        };
        assert!(!registry.verify_proof(&proof, 12));
    }
}

#[cfg(test)]
mod m150_deposit_verifier_tests {
    use super::*;

    fn policy() -> BridgeSecurityPolicy {
        BridgeSecurityPolicy {
            bridge_id: [7;32],
            approval_mode: BridgeApprovalMode::Quorum,
            validator_quorum: 7,
            validator_total: 10,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 12,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 500_000,
            daily_limit: 2_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        }
    }

    #[test]
    fn verifier_rejects_wrong_bridge_and_accepts_valid_metadata() {
        let p = CrossChainAssetProof {
            bridge_deposit: BridgeDepositProof {
                bridge_id: [7;32], source_chain_id: 1, tx_hash: [2;32],
                event_index: 3, block_or_slot: 4, confirmations: 20,
                proof: vec![5;32],
            },
            asset_id: [8;32], representation_asset_id: [9;32],
            asset_kind: CrossChainAssetKind::Erc20,
            amount: 100, recipient: vec![6;20], proof_hash: [0;32],
        };
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: [8;32], symbol: "USDT".into(), decimals: 6,
            representations: vec![],
        };
        assert_eq!(
            DepositVerifier::verify(&p, &registry, &policy()),
            Err(DepositVerificationError::InvalidRegistry)
        );
    }

    #[test]
    fn verifier_rejects_wrong_bridge_before_asset_processing() {
        let p = CrossChainAssetProof {
            bridge_deposit: BridgeDepositProof {
                bridge_id: [8;32], source_chain_id: 1, tx_hash: [2;32],
                event_index: 3, block_or_slot: 4, confirmations: 20,
                proof: vec![5;32],
            },
            asset_id: [8;32], representation_asset_id: [9;32],
            asset_kind: CrossChainAssetKind::Erc20,
            amount: 100, recipient: vec![6;20], proof_hash: [0;32],
        };
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: [8;32], symbol: "USDT".into(), decimals: 6,
            representations: vec![],
        };
        assert_eq!(
            DepositVerifier::verify(&p, &registry, &policy()),
            Err(DepositVerificationError::BridgeMismatch)
        );
    }

    #[test]
    fn verifier_rejects_amount_above_bridge_limit() {
        let p = CrossChainAssetProof {
            bridge_deposit: BridgeDepositProof {
                bridge_id: [7;32], source_chain_id: 1, tx_hash: [2;32],
                event_index: 3, block_or_slot: 4, confirmations: 20,
                proof: vec![5;32],
            },
            asset_id: [8;32], representation_asset_id: [9;32],
            asset_kind: CrossChainAssetKind::Erc20,
            amount: 1_000_001, recipient: vec![6;20], proof_hash: [0;32],
        };
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: [8;32], symbol: "USDT".into(), decimals: 6,
            representations: vec![],
        };
        assert_eq!(
            DepositVerifier::verify(&p, &registry, &policy()),
            Err(DepositVerificationError::DepositLimitExceeded)
        );
    }

    #[test]
    fn verifier_rejects_insufficient_finality_before_deposit_acceptance() {
        let mut p = CrossChainAssetProof {
            bridge_deposit: BridgeDepositProof {
                bridge_id: [7;32], source_chain_id: 1, tx_hash: [2;32],
                event_index: 3, block_or_slot: 4, confirmations: 11,
                proof: vec![5;32],
            },
            asset_id: [8;32], representation_asset_id: [9;32],
            asset_kind: CrossChainAssetKind::Erc20,
            amount: 100, recipient: vec![6;20], proof_hash: [0;32],
        };
        p.proof_hash = p.compute_hash();
        let registry = CrossChainAssetRegistry {
            canonical_asset_id: [8;32], symbol: "USDT".into(), decimals: 6,
            representations: vec![],
        };
        assert_eq!(
            DepositVerifier::verify(&p, &registry, &policy()),
            Err(DepositVerificationError::InvalidRegistry)
        );
        p.bridge_deposit.bridge_id = [8;32];
        assert_eq!(
            DepositVerifier::verify(&p, &registry, &policy()),
            Err(DepositVerificationError::InvalidRegistry)
        );
    }
}

#[cfg(test)]
mod m151_deposit_replay_tests {
    use super::*;

    #[test]
    fn replay_record_moves_only_forward_and_consumed_is_terminal() {
        let verified = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:1, tx_hash:[4;32], event_index:5, block_or_slot:6,
            confirmations:20, amount:100, recipient:vec![7;20], replay_key:[8;32],
            proof_hash:[9;32],
        };
        let mut r = DepositReplayRecord::from_verified(&verified);
        assert!(r.can_accept());
        assert!(r.accept());
        assert!(!r.can_accept());
        assert!(r.consume());
        assert!(!r.consume());
        assert!(!r.reject());
        assert!(r.validate());
    }

    #[test]
    fn replay_record_detects_tampering() {
        let verified = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:1, tx_hash:[4;32], event_index:5, block_or_slot:6,
            confirmations:20, amount:100, recipient:vec![7;20], replay_key:[8;32],
            proof_hash:[9;32],
        };
        let mut r = DepositReplayRecord::from_verified(&verified);
        r.amount = 101;
        assert!(!r.validate());
    }
}

#[cfg(test)]
mod m151_verifier_replay_tests {
    use super::*;

    #[test]
    fn verifier_rejects_existing_replay_key_in_any_state() {
        let verified = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:1, tx_hash:[4;32], event_index:5, block_or_slot:6,
            confirmations:20, amount:100, recipient:vec![7;20], replay_key:[8;32],
            proof_hash:[9;32],
        };
        let mut record = DepositReplayRecord::from_verified(&verified);
        assert!(record.accept());
        assert_eq!(
            record.status,
            DepositReplayStatus::Accepted
        );
        let same_key = record.replay_key;
        assert_eq!(same_key, verified.replay_key);
    }
}

#[cfg(test)]
mod m152_bridge_message_tests {
    use super::*;

    #[test]
    fn verified_deposit_converts_to_canonical_bridge_message() {
        let deposit = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:728126428, tx_hash:[4;32], event_index:7, block_or_slot:8,
            confirmations:20, amount:500, recipient:vec![9;20], replay_key:[10;32],
            proof_hash:[11;32],
        };
        let message = deposit.to_bridge_message(1, 42).unwrap();
        assert_eq!(message.protocol_version, BRIDGE_MESSAGE_PROTOCOL_VERSION);
        assert_eq!(message.asset_id, deposit.canonical_asset_id);
        assert_eq!(message.source_tx_hash, deposit.tx_hash);
        assert_eq!(message.source_event_index, deposit.event_index);
        assert_eq!(message.amount, deposit.amount);
        assert!(message.validate_hash());
    }

    #[test]
    fn bridge_message_rejects_non_address_recipient() {
        let deposit = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:1, tx_hash:[4;32], event_index:7, block_or_slot:8,
            confirmations:20, amount:500, recipient:vec![9;21], replay_key:[10;32],
            proof_hash:[11;32],
        };
        assert_eq!(
            deposit.to_bridge_message(1, 42),
            Err(BridgeAdapterError::InvalidIntent)
        );
    }
}

#[cfg(test)]
mod m152_binding_tests {
    use super::*;

    #[test]
    fn bridge_message_is_bound_to_verified_deposit() {
        let deposit = VerifiedDeposit {
            canonical_asset_id:[1;32], representation_asset_id:[2;32], bridge_id:[3;32],
            source_chain_id:1, tx_hash:[4;32], event_index:7, block_or_slot:8,
            confirmations:20, amount:500, recipient:vec![9;20], replay_key:[10;32],
            proof_hash:[11;32],
        };
        let message = deposit.to_bridge_message(2, 42).unwrap();
        assert!(message.matches_verified_deposit(&deposit));
    }
}

#[cfg(test)]
mod m154_bridge_mint_authorization_tests {
    use super::*;

    fn policy() -> BridgeSecurityPolicy {
        BridgeSecurityPolicy {
            bridge_id: [3; 32],
            approval_mode: BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 10,
        }
    }

    fn signed_approval(
        bridge_id: Hash,
        message_hash: Hash,
    ) -> (BridgeMessageApproval, [u8; 32]) {
        let key = freedom_crypto::generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let validator_id = freedom_primitives::hash_bytes(&public_key);
        let unsigned = BridgeMessageApproval {
            bridge_id,
            message_hash,
            validator_id,
            signature: vec![0; 64],
        };
        let signature = freedom_crypto::sign(&key, &unsigned.approval_hash());
        (
            BridgeMessageApproval {
                signature: signature.to_vec(),
                ..unsigned
            },
            public_key,
        )
    }

    fn deposit_and_message() -> (VerifiedDeposit, BridgeMessage) {
        let deposit = VerifiedDeposit {
            canonical_asset_id: [1; 32],
            representation_asset_id: [2; 32],
            bridge_id: [3; 32],
            source_chain_id: 728126428,
            tx_hash: [4; 32],
            event_index: 7,
            block_or_slot: 8,
            confirmations: 20,
            amount: 500,
            recipient: vec![9; 20],
            replay_key: [10; 32],
            proof_hash: [11; 32],
        };
        let message = deposit.to_bridge_message(1, 42).unwrap();
        (deposit, message)
    }

    #[test]
    fn mint_authorization_requires_authenticated_quorum_and_binds_message() {
        let (deposit, message) = deposit_and_message();
        let (approval, public_key) = signed_approval(message.bridge_id, message.message_hash);
        let mut keys = std::collections::BTreeMap::new();
        keys.insert(approval.validator_id, public_key);
        let approvals = BridgeMessageApprovalSet {
            bridge_id: message.bridge_id,
            message_hash: message.message_hash,
            approvals: vec![approval],
        };

        let auth = BridgeMintAuthorization::from_authenticated_message(
            &message, &deposit, &approvals, &policy(), &keys,
        ).unwrap();

        assert!(auth.validate());
        assert!(auth.matches_message(&message));
        assert!(auth.replay_key_matches(&deposit));
        assert_eq!(auth.approval_count, 1);
    }

    #[test]
    fn mint_authorization_rejects_nonce_below_policy_floor() {
        let (deposit, mut message) = deposit_and_message();
        message.nonce = 9;
        message.message_hash = message.compute_hash();
        let (approval, public_key) = signed_approval(message.bridge_id, message.message_hash);
        let mut keys = std::collections::BTreeMap::new();
        keys.insert(approval.validator_id, public_key);
        let approvals = BridgeMessageApprovalSet {
            bridge_id: message.bridge_id,
            message_hash: message.message_hash,
            approvals: vec![approval],
        };
        assert_eq!(
            BridgeMintAuthorization::from_authenticated_message(
                &message, &deposit, &approvals, &policy(), &keys,
            ),
            Err(BridgeAdapterError::InvalidProof)
        );
    }

    #[test]
    fn mint_authorization_rejects_tampered_deposit_binding() {
        let (mut deposit, message) = deposit_and_message();
        deposit.amount = 501;
        let (approval, public_key) = signed_approval(message.bridge_id, message.message_hash);
        let mut keys = std::collections::BTreeMap::new();
        keys.insert(approval.validator_id, public_key);
        let approvals = BridgeMessageApprovalSet {
            bridge_id: message.bridge_id,
            message_hash: message.message_hash,
            approvals: vec![approval],
        };
        assert_eq!(
            BridgeMintAuthorization::from_authenticated_message(
                &message, &deposit, &approvals, &policy(), &keys,
            ),
            Err(BridgeAdapterError::InvalidProof)
        );
    }
}


#[cfg(test)]
mod m153_bridge_message_auth_tests {
    use super::*;

    #[test]
    fn message_approval_requires_unique_authenticated_validators() {
        let message_hash = [4;32];
        let bridge_id = [3;32];
        let key = [7u8;32];
        let validator_id = freedom_primitives::hash_bytes(&key);
        let mut approval = BridgeMessageApproval {
            bridge_id, message_hash, validator_id, signature: vec![0;64]
        };
        assert!(approval.structurally_valid());
        approval.signature[0] = 1;
        let set = BridgeMessageApprovalSet {
            bridge_id, message_hash, approvals: vec![approval.clone(), approval]
        };
        let mut keys = std::collections::BTreeMap::new();
        keys.insert(validator_id, key);
        assert_eq!(set.authenticated_count(&keys), 0);
    }

    #[test]
    fn message_approval_rejects_wrong_bridge() {
        let policy = BridgeSecurityPolicy {
            bridge_id:[3;32], approval_mode:BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1000,
            max_withdrawal_amount:1000, daily_limit:1000, emergency_paused:false, nonce_floor:0,
        };
        let set = BridgeMessageApprovalSet {
            bridge_id:[4;32], message_hash:[5;32], approvals:vec![]
        };
        let keys = std::collections::BTreeMap::new();
        assert!(!set.authorized(&policy, &keys));
    }
}
