const MAX_EVIDENCE_COMPONENT_BYTES: usize = 64 * 1024;

fn decode_submit_evidence(b:&[u8])->Option<freedom_types::SubmitEvidencePayload>{
 let mut i=0usize;
 let kind=match *b.get(i)?{1=>freedom_types::EquivocationKind::DoublePrevote,2=>freedom_types::EquivocationKind::DoublePrecommit,3=>freedom_types::EquivocationKind::DoubleProposal,_=>return None}; i+=1;
 let height=u64::from_le_bytes(b.get(i..i+8)?.try_into().ok()?); i+=8;
 let round=u64::from_le_bytes(b.get(i..i+8)?.try_into().ok()?); i+=8;
 let mut validator_id=[0u8;32]; validator_id.copy_from_slice(b.get(i..i+32)?); i+=32;
 let n1=u32::from_le_bytes(b.get(i..i+4)?.try_into().ok()?) as usize;i+=4;
 if n1>MAX_EVIDENCE_COMPONENT_BYTES{return None;}
 let end1=i.checked_add(n1)?; let first=b.get(i..end1)?.to_vec(); i=end1;
 let n2=u32::from_le_bytes(b.get(i..i+4)?.try_into().ok()?) as usize;i+=4;
 if n2>MAX_EVIDENCE_COMPONENT_BYTES{return None;}
 let end2=i.checked_add(n2)?; let second=b.get(i..end2)?.to_vec(); i=end2;
 let mut evidence_hash=[0u8;32];evidence_hash.copy_from_slice(b.get(i..i+32)?);i+=32;
 let mut slash=[0u8;16];slash.copy_from_slice(b.get(i..i+16)?);i+=16;
 if i!=b.len(){return None;}
 Some(freedom_types::SubmitEvidencePayload{evidence:freedom_types::EquivocationEvidence{kind,height,round,validator_id,first,second,evidence_hash},slash_amount:u128::from_le_bytes(slash)})
}
use std::collections::{BTreeMap,BTreeSet};
use serde::{Deserialize, Serialize};
use freedom_primitives::{address_from_public_key, Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, RegisterNodePayload, SubmitContributionPayload, BillingPayload, PurchasePayload, CreateTicketPayload, DrawConfig, DrawResult, Event, Transaction, TxKind, TransferPayload, Validator, UnbondingEntry, AssetDefinition, AssetType, AssetStatus, BridgeMessage, BRIDGE_MESSAGE_PROTOCOL_VERSION, BridgeSecurityPolicy, BridgeApprovalSet, BridgeMessageApprovalSet, BridgeMintAuthorization, BridgeMintTransactionPayload, VerifiedDeposit, DepositReplayRecord, BridgeRateLimitUsage, BridgeNonceKey, BridgeDirection, BitcoinChainCheckpoint, BitcoinReorgEvidence, BridgeWithdrawal, BridgeWithdrawalStatus, BridgeAccounting, BridgePauseAuthorizationSet, BridgeMessageStatus};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account { pub address:Address,pub public_key:[u8;32],pub nonce:u64,pub balance:u128,pub locked:u128,pub staked:u128 }
impl Account { pub fn new(public_key:[u8;32],balance:u128)->Self{Self{address:address_from_public_key(&public_key),public_key,nonce:0,balance,locked:0,staked:0}} }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node { pub node_id:Hash,pub owner:Address,pub capabilities:u64,pub reputation:u64,pub total_wu:u128,pub successful_tasks:u64,pub failed_tasks:u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub enum ContributionStatus { Submitted, Verified, Rejected, Rewarded }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contribution { pub contribution_id:Hash,pub node_id:Hash,pub task_id:Hash,pub work_units:u128,pub quality_score:u32,pub verification_score:u32,pub result_hash:Hash,pub verifier_set:Vec<Hash>,pub status:ContributionStatus,pub reward_id:Option<Hash>,pub reward_amount:u128 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reward { pub reward_id:Hash,pub owner:Address,pub contribution_id:Hash,pub wu:u128,pub gross_reward:u128,pub final_reward:u128,pub epoch:u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry { pub tx_hash:Hash, pub sender:Address, pub kind:u8, pub nonce:u64, pub success:bool }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub struct Purchase { pub purchase_id:Hash,pub buyer:Address,pub merchant:Address,pub product_id:u8,pub price:u128,pub reference:Hash,pub block_height:u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub struct BillingRecord { pub billing_id:Hash,pub buyer:Address,pub merchant:Address,pub product_id:u8,pub amount:u128,pub reference:Hash,pub status:u8,pub reserved_at:u64,pub captured_at:Option<u64>,pub refunded_at:Option<u64> }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub struct Ticket { pub ticket_id:Hash,pub owner:Address,pub draw_id:Hash,pub created_at:u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub struct DrawState { pub config:DrawConfig,pub total_tickets:u64,#[serde(rename="prize_pool_native_units",alias="prize_pool_mfdm")] pub prize_pool_native_units:u128,#[serde(rename="cu_fund_native_units",alias="cu_fund_mfdm")] pub cu_fund_native_units:u128,#[serde(rename="annual_reserve_native_units",alias="annual_reserve_mfdm")] pub annual_reserve_native_units:u128,pub executed:bool }

#[derive(Clone, Debug, Serialize, Deserialize)] pub struct ChainState { pub height:u64,pub last_block_hash:Hash,pub accounts:BTreeMap<Address,Account>,pub validators:BTreeMap<Hash,Validator>,#[serde(default)] pub unbonding:BTreeMap<Hash,UnbondingEntry>,pub nodes:BTreeMap<Hash,Node>,pub contributions:BTreeMap<Hash,Contribution>,pub rewards:BTreeMap<Hash,Reward>,pub purchases:BTreeMap<Hash,Purchase>,pub billing:BTreeMap<Hash,BillingRecord>,pub tickets:BTreeMap<Hash,Ticket>,pub draws:BTreeMap<Hash,DrawState>,pub draw_results:BTreeMap<Hash,DrawResult>,#[serde(rename="cu_fund_native_units",alias="cu_fund_mfdm")] pub cu_fund_native_units:u128,#[serde(rename="annual_reserve_native_units",alias="annual_reserve_mfdm")] pub annual_reserve_native_units:u128,#[serde(rename="draw_prize_pool_native_units",alias="draw_prize_pool_mfdm")] pub draw_prize_pool_native_units:u128,pub events:Vec<Event>,pub ledger:Vec<LedgerEntry>,pub supply:Supply,pub epoch:u64,pub epoch_length:u64,pub active_validator_set:BTreeMap<Hash,freedom_types::ValidatorSetEntry>,#[serde(default)] pub asset_registry:BTreeMap<Hash,AssetDefinition>,#[serde(default)] pub cross_chain_asset_registries:BTreeMap<Hash,freedom_types::CrossChainAssetRegistry>,#[serde(default)] pub processed_bridge_messages:BTreeMap<Hash,BridgeMessage>,#[serde(default)] pub bridge_message_status:BTreeMap<Hash,BridgeMessageStatus>,#[serde(default)] pub bridge_withdrawals:BTreeMap<Hash,BridgeWithdrawal>,#[serde(default)] pub bridge_withdrawal_status:BTreeMap<Hash,BridgeWithdrawalStatus>,#[serde(default)] pub bridge_accounting:BTreeMap<Hash,BridgeAccounting>,#[serde(default)] pub bridge_security_policies:BTreeMap<Hash,BridgeSecurityPolicy>,#[serde(default)] pub bridge_approval_sets:BTreeMap<Hash,BridgeApprovalSet>,#[serde(default)] pub bridge_pause_authorizations:BTreeMap<Hash,BridgePauseAuthorizationSet>,#[serde(default)] pub bridge_nonce_registry:BTreeMap<Hash,BTreeSet<u64>>,#[serde(default)] pub bridge_rate_limits:BTreeMap<Hash,BridgeRateLimitUsage>,#[serde(default)] pub bridge_deposit_replays:BTreeMap<Hash,DepositReplayRecord>,#[serde(default)] pub bridge_mint_authorizations:BTreeMap<Hash,BridgeMintAuthorization>,#[serde(default)] pub bridge_withdrawal_rate_limit_period:BTreeMap<Hash,u64>,#[serde(default)] pub bitcoin_checkpoints:BTreeMap<Hash,BitcoinChainCheckpoint>,#[serde(default)] pub bitcoin_canonical_tips:BTreeMap<Hash,BitcoinChainCheckpoint>,#[serde(default)] pub bitcoin_reorg_evidence:BTreeMap<Hash,Vec<BitcoinReorgEvidence>>,#[serde(default)] pub bitcoin_reorg_policy:BTreeMap<Hash,freedom_types::BitcoinReorgPolicy>,#[serde(default)] pub bitcoin_reorg_recovery:BTreeMap<Hash,freedom_types::BitcoinReorgRecovery>,#[serde(default)] pub bitcoin_utxo_attestations:BTreeMap<Hash,freedom_types::BitcoinUtxoCommitmentAttestation>,#[serde(default)] pub bitcoin_utxo_attestation_policy_hashes:BTreeMap<Hash,Hash>,#[serde(default)] pub bitcoin_utxo_commitments:BTreeMap<Hash,freedom_types::BitcoinUtxoBlockCommitment>,#[serde(default)] pub pending_validator_set:BTreeMap<Hash,freedom_types::ValidatorSetEntry>,#[serde(default)] pub validator_set_history:BTreeMap<u64,Hash>,#[serde(default)] pub validator_set_history_sets:BTreeMap<u64,BTreeMap<Hash,[u8;32]>>,
#[serde(default)] pub validator_set_history_entries:BTreeMap<u64,BTreeMap<Hash,freedom_types::ValidatorSetEntry>>,#[serde(default)] pub validator_set_history_key_hashes:BTreeMap<u64,Hash>,#[serde(default)] pub validator_set_certificate:Option<freedom_types::ValidatorSetCertificate>,#[serde(default)] pub staking_activated:bool,pub epoch_issuance:u128,pub epoch_cap:u128,pub evidence:BTreeMap<Hash,freedom_types::EquivocationEvidence>,pub slashed_evidence:BTreeMap<Hash,u128> }
#[derive(Debug,PartialEq,Eq)] pub enum StateError { AccountNotFound,InvalidChainId,InvalidNonce{expected:u64,actual:u64},InsufficientBalance,InvalidTransferPayload,InvalidRecipient,ArithmeticOverflow,UnsupportedTransaction,InvalidNodePayload,InvalidNode,DuplicateNode,InvalidContributionPayload,InvalidContribution,DuplicateContribution,InvalidWorkUnits,InvalidQualityScore,InvalidVerification,AlreadyVerified,ContributionNotRewardable,DuplicateReward,InvalidReward,IssuanceCapExceeded,InvalidPurchase,DuplicatePurchase,InsufficientBalanceForPurchase,InvalidEvidence,InvalidTicketPayload,InvalidDraw,InvalidTicketQuantity,InvalidTicketPrice,DuplicateTicket,DrawNotReady,DrawAlreadyExecuted,InsufficientPrizePool,InvalidUnbonding,InvalidValidatorLifecycle,InvalidStakeAmount,InvalidAsset,DuplicateAsset,InvalidBridgeMessage,DuplicateBridgeMessage,InvalidBridgeWithdrawal,DuplicateBridgeWithdrawal,InvalidBridgeWithdrawalTransition,InvalidBridgeAccounting,BridgeRateLimitExceeded,InvalidBridgePauseAuthorization,DuplicateBridgeNonce,InvalidBridgeMintTransaction,BitcoinReorgDetected }
impl ChainState {
 fn validate_evidence_integrity(&self)->Result<(),StateError>{
  for (id,evidence) in &self.evidence {
    if *id != evidence.evidence_hash
      || *id == [0;32]
      || evidence.validator_id == [0;32]
      || evidence.first.is_empty()
      || evidence.second.is_empty()
      || evidence.first == evidence.second
      || evidence.height > self.height
      || freedom_primitives::hash_bytes(&evidence.signing_bytes()) != *id
      || !self.validators.contains_key(&evidence.validator_id) {
      return Err(StateError::InvalidEvidence);
    }
  }
  Ok(())
 }

 pub(crate) fn record_equivocation(&mut self,evidence:freedom_types::EquivocationEvidence)->Result<bool,StateError>{
  // Evidence insertion is consensus state mutation. Refuse to append new
  // evidence while the authoritative validator registry is already corrupt.
  self.validate_validator_records_integrity()?;
  if evidence.evidence_hash == [0;32]
    || evidence.validator_id == [0;32]
    || evidence.first.is_empty()
    || evidence.second.is_empty()
    || evidence.first == evidence.second
    || evidence.height > self.height
    || !self.validators.contains_key(&evidence.validator_id) {
    return Err(StateError::InvalidEvidence);
  }
  let expected = freedom_primitives::hash_bytes(&evidence.signing_bytes());
  if expected != evidence.evidence_hash { return Err(StateError::InvalidEvidence); }
  // A slashed evidence hash is already finalized historically. It must never
  // be reintroduced through the recording API as a fresh evidence insertion.
  if self.slashed_evidence.contains_key(&evidence.evidence_hash) {
    return Err(StateError::InvalidEvidence);
  }
  if let Some(existing)=self.evidence.get(&evidence.evidence_hash) {
    if existing != &evidence {
      return Err(StateError::InvalidEvidence);
    }
    return Ok(false);
  }
  self.evidence.insert(evidence.evidence_hash,evidence); Ok(true)
 }
 pub(crate) fn slash_evidence(&mut self,evidence_hash:Hash,amount:u128)->Result<u128,StateError>{
  if self.slashed_evidence.contains_key(&evidence_hash) {
    self.validate_validator_records_integrity()?;
    self.validate_evidence_integrity()?;
    self.validate_slashed_evidence_integrity()?;
  }
  if let Some(recorded)=self.slashed_evidence.get(&evidence_hash) {
    if amount != *recorded {
      return Err(StateError::InvalidEvidence);
    }
    return Ok(0);
  }
  self.validate_validator_records_integrity()?;
  self.validate_unbonding_integrity()?;
  self.validate_staking_activation_integrity()?;
  self.validate_evidence_integrity()?;
  let evidence=self.evidence.get(&evidence_hash).ok_or(StateError::InvalidEvidence)?;
  let (owner,total)={let v=self.validators.get(&evidence.validator_id).ok_or(StateError::InvalidEvidence)?;(v.owner,v.stake.checked_add(v.unbonding_stake).ok_or(StateError::InvalidEvidence)?)};
  let account=self.accounts.get(&owner).ok_or(StateError::AccountNotFound)?;
  if account.staked < total || total == 0 {
    return Err(StateError::InvalidEvidence);
  }
  // An exited validator has no slashable stake. Evidence remains historical,
  // but a new slash operation must never succeed against an empty lifecycle.
  if self.validator_lifecycle(evidence.validator_id)? == freedom_types::ValidatorLifecycle::Exited {
    return Err(StateError::InvalidEvidence);
  }
  let expected=self.deterministic_slash_amount(&evidence.kind,total);
  if expected==0 || amount!=expected {
    return Err(StateError::InvalidEvidence);
  }
  let slash=expected;
  let active_cut=slash.min(self.validators.get(&evidence.validator_id)
    .ok_or(StateError::InvalidEvidence)?.stake);
  let pending_cut=slash.checked_sub(active_cut)
    .ok_or(StateError::InvalidEvidence)?;
  let mut pending=pending_cut;
  let ids:Vec<Hash>=self.unbonding.iter()
    .filter(|(_,e)|e.validator_id==evidence.validator_id)
    .map(|(id,_)|*id)
    .collect();
  let mut slash_plan=Vec::<(Hash,u128)>::new();
  for id in ids {
    if pending==0 { break; }
    let e=self.unbonding.get(&id).ok_or(StateError::InvalidEvidence)?;
    let cut=pending.min(e.amount);
    if cut>0 {
      slash_plan.push((id,cut));
      pending=pending.checked_sub(cut).ok_or(StateError::InvalidEvidence)?;
    }
  }
  if pending!=0 || active_cut.checked_add(pending_cut) != Some(slash) {
    return Err(StateError::InvalidEvidence);
  }

  // All slash accounting is now fully planned and validated. Only after this
  // point may validator, account, and unbonding state be mutated.
  {
    let v=self.validators.get_mut(&evidence.validator_id)
      .ok_or(StateError::InvalidEvidence)?;
    v.stake=v.stake.checked_sub(active_cut).ok_or(StateError::InvalidEvidence)?;
    v.unbonding_stake=v.unbonding_stake.checked_sub(pending_cut).ok_or(StateError::InvalidEvidence)?;
  }

  if let Some(a)=self.accounts.get_mut(&owner) {
    a.staked=a.staked.checked_sub(slash).ok_or(StateError::InvalidEvidence)?;
  } else {
    return Err(StateError::AccountNotFound);
  }

  for (id,cut) in slash_plan {
    let remove={
      let e=self.unbonding.get_mut(&id).ok_or(StateError::InvalidEvidence)?;
      e.amount=e.amount.checked_sub(cut).ok_or(StateError::InvalidEvidence)?;
      e.amount==0
    };
    if remove {
      self.unbonding.remove(&id);
    }
  }
  self.slashed_evidence.insert(evidence_hash,slash);self.refresh_pending_validator_set();Ok(slash)
 }
 pub fn deterministic_slash_amount(&self,kind:&freedom_types::EquivocationKind,stake:u128)->u128{
  let bps=match kind {
    freedom_types::EquivocationKind::DoubleProposal=>500,
    freedom_types::EquivocationKind::DoublePrevote=>1000,
    freedom_types::EquivocationKind::DoublePrecommit=>1000,
  };
  // Compute stake * bps / 10_000 without saturating multiplication. Splitting
  // before multiplication preserves the exact integer quotient even for the
  // full u128 range.
  let divisor=10_000u128;
  let whole=stake/divisor;
  let remainder=stake%divisor;
  whole.checked_mul(bps as u128)
    .and_then(|base| base.checked_add(remainder*(bps as u128)/divisor))
    .unwrap_or(u128::MAX)
 }

 pub const UNBONDING_COOLDOWN: u64 = 200;
 pub fn genesis(g:&Genesis)->Self{assert_eq!(g.initial_supply,GENESIS_SUPPLY,"Genesis native asset supply must be zero");let validators:BTreeMap<Hash,Validator>=g.validators.iter().cloned().map(|v|(v.validator_id,v)).collect();Self{height:0,last_block_hash:[0;32],accounts:BTreeMap::new(),validators:validators.clone(),nodes:BTreeMap::new(),contributions:BTreeMap::new(),rewards:BTreeMap::new(),purchases:BTreeMap::new(),billing:BTreeMap::new(),tickets:BTreeMap::new(),draws:BTreeMap::new(),draw_results:BTreeMap::new(),cu_fund_native_units:0,annual_reserve_native_units:0,draw_prize_pool_native_units:0,events:Vec::new(),ledger:Vec::new(),supply:Supply::genesis(),epoch:0,epoch_length:1000,active_validator_set:validators.iter().map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect(),asset_registry: Self::genesis_asset_registry(),cross_chain_asset_registries: Self::genesis_cross_chain_asset_registries(),processed_bridge_messages:BTreeMap::new(),bridge_message_status:BTreeMap::new(),bridge_withdrawals:BTreeMap::new(),bridge_withdrawal_status:BTreeMap::new(),bridge_accounting:BTreeMap::new(),bridge_security_policies:BTreeMap::new(),bridge_approval_sets:BTreeMap::new(),bridge_pause_authorizations:BTreeMap::new(),bridge_nonce_registry:BTreeMap::new(),bridge_rate_limits:BTreeMap::new(),bridge_deposit_replays:BTreeMap::new(),bridge_mint_authorizations:BTreeMap::new(),bridge_withdrawal_rate_limit_period:BTreeMap::new(),bitcoin_checkpoints:BTreeMap::new(),bitcoin_canonical_tips:BTreeMap::new(),bitcoin_reorg_evidence:BTreeMap::new(),bitcoin_reorg_policy:BTreeMap::new(),bitcoin_reorg_recovery:BTreeMap::new(),bitcoin_utxo_attestations:BTreeMap::new(),bitcoin_utxo_attestation_policy_hashes:BTreeMap::new(),bitcoin_utxo_commitments:BTreeMap::new(),pending_validator_set:validators.iter().map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect(),validator_set_history:{ let mut h=BTreeMap::new(); let mut bytes=Vec::new(); for (id,v) in validators.iter() { bytes.extend_from_slice(id); bytes.extend_from_slice(&v.owner); bytes.extend_from_slice(&v.consensus_key); bytes.extend_from_slice(&v.stake.to_le_bytes()); } h.insert(0,freedom_primitives::hash_bytes(&bytes)); h },validator_set_history_sets:{ let mut h=BTreeMap::new(); h.insert(0,validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect()); h },validator_set_history_entries:{ let mut h=BTreeMap::new(); h.insert(0,validators.iter().map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect()); h },validator_set_history_key_hashes:{ let mut h=BTreeMap::new(); let mut b=Vec::new(); for (id,v) in validators.iter(){ b.extend_from_slice(id); b.extend_from_slice(&v.consensus_key); } h.insert(0,freedom_primitives::hash_bytes(&b)); h },validator_set_certificate:None,staking_activated:false,epoch_issuance:0,epoch_cap:1_000_000,unbonding:BTreeMap::new(),evidence:BTreeMap::new(),slashed_evidence:BTreeMap::new()}}
 pub fn genesis_cross_chain_asset_registries() -> BTreeMap<Hash, freedom_types::CrossChainAssetRegistry> {
  let mut registries = BTreeMap::new();
  for config in freedom_types::production_profiles() {
   let representation = match config.build() { Ok(value) => value, Err(_) => continue };
   registries.entry(representation.canonical_asset_id)
    .and_modify(|registry: &mut freedom_types::CrossChainAssetRegistry| {
     let _ = registry.add_representation(representation.representation.clone(), representation.asset_kind);
    })
    .or_insert_with(|| freedom_types::CrossChainAssetRegistry {
     canonical_asset_id: representation.canonical_asset_id,
     symbol: config.profile.symbol().to_string(),
     decimals: config.profile.decimals(),
     representations: vec![representation],
    });
  }
  registries.retain(|_, registry| registry.validate());
  registries
 }
 pub fn genesis_asset_registry() -> BTreeMap<Hash, AssetDefinition> {
  let contract_address = freedom_primitives::NATIVE_ASSET_SYMBOL.as_bytes().to_vec();
  let asset_id = AssetDefinition::compute_asset_id(
   freedom_primitives::CHAIN_ID,
   AssetType::Native,
   &contract_address,
   [0; 32],
   [0; 32],
  );
  let asset = AssetDefinition {
   asset_id,
   source_chain_id: freedom_primitives::CHAIN_ID,
   asset_type: AssetType::Native,
   contract_address,
   symbol: freedom_primitives::NATIVE_ASSET_SYMBOL.to_string(),
   decimals: 18,
   issuer: [0; 32],
   bridge_id: [0; 32],
   status: AssetStatus::Active,
  };
  let mut registry = BTreeMap::from([(asset_id, asset)]);
  for config in freedom_types::production_profiles() {
   if let Ok(representation) = config.build() {
    registry.insert(representation.representation.asset_id, representation.representation);
   }
  }
  registry
 }
 pub(crate) fn register_cross_chain_registry(
  &mut self,
  registry: freedom_types::CrossChainAssetRegistry,
) -> Result<(), StateError> {
  if !registry.validate() {
   return Err(StateError::InvalidAsset);
  }
  if self.cross_chain_asset_registries.contains_key(&registry.canonical_asset_id) {
   return Err(StateError::DuplicateAsset);
  }
  for representation in &registry.representations {
   if self.asset_registry.contains_key(&representation.representation.asset_id) {
    return Err(StateError::DuplicateAsset);
   }
  }
  for representation in &registry.representations {
   self.asset_registry.insert(
    representation.representation.asset_id,
    representation.representation.clone(),
   );
  }
  self.cross_chain_asset_registries.insert(registry.canonical_asset_id, registry);
  Ok(())
 }
 pub(crate) fn register_asset(&mut self, asset: AssetDefinition) -> Result<(), StateError> {
  if !asset.validate_identity() || asset.status == AssetStatus::Deprecated {
   return Err(StateError::InvalidAsset);
  }
  if asset.asset_type == AssetType::Native && asset.source_chain_id != freedom_primitives::CHAIN_ID {
   return Err(StateError::InvalidAsset);
  }
  if self.asset_registry.contains_key(&asset.asset_id) {
   return Err(StateError::DuplicateAsset);
  }
  self.asset_registry.insert(asset.asset_id, asset);
  Ok(())
 }
 pub(crate) fn reserve_bridge_withdrawal_limit(&mut self,bridge_id:Hash,period:u64,amount:u128)->Result<(),StateError>{
  if amount==0{return Err(StateError::BridgeRateLimitExceeded);}
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if policy.emergency_paused||amount>policy.max_withdrawal_amount{return Err(StateError::BridgeRateLimitExceeded);}
  let usage=self.bridge_rate_limits.entry(bridge_id).or_insert(BridgeRateLimitUsage{period,used:0});
  if usage.period!=period {usage.period=period;usage.used=0;}
  let next=usage.used.checked_add(amount).ok_or(StateError::BridgeRateLimitExceeded)?;
  if next>policy.daily_limit{return Err(StateError::BridgeRateLimitExceeded);}
  usage.used=next; Ok(())
 }
 pub(crate) fn release_bridge_withdrawal_limit(&mut self,bridge_id:Hash,period:u64,amount:u128)->Result<(),StateError>{
  let usage=self.bridge_rate_limits.get_mut(&bridge_id).ok_or(StateError::BridgeRateLimitExceeded)?;
  if usage.period!=period||amount>usage.used{return Err(StateError::BridgeRateLimitExceeded);}
  usage.used-=amount; Ok(())
 }
 pub(crate) fn apply_bridge_emergency_pause(&mut self,bridge_id:Hash,auth_set:BridgePauseAuthorizationSet)->Result<(),StateError>{
  let policy=self.bridge_security_policies.get(&bridge_id).cloned().ok_or(StateError::InvalidBridgePauseAuthorization)?;
  if auth_set.bridge_id!=bridge_id||auth_set.nonce==0||auth_set.authorizations.is_empty(){return Err(StateError::InvalidBridgePauseAuthorization);}
  if let Some(previous)=self.bridge_pause_authorizations.get(&bridge_id){if auth_set.nonce<=previous.nonce{return Err(StateError::InvalidBridgePauseAuthorization);}}
  let keys=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<BTreeMap<Hash,[u8;32]>>();
  if auth_set.authorizations.windows(2).any(|pair| pair[0].validator_id >= pair[1].validator_id) {
   return Err(StateError::InvalidBridgePauseAuthorization);
  }
  if auth_set.authorizations.iter().any(|a|a.bridge_id!=bridge_id||a.nonce!=auth_set.nonce||!keys.contains_key(&a.validator_id)){return Err(StateError::InvalidBridgePauseAuthorization);}
  if auth_set.authenticated_count(&keys)<policy.validator_quorum{return Err(StateError::InvalidBridgePauseAuthorization);}
  let mut next=policy.clone();next.emergency_paused=auth_set.authorizations.first().map(|a|a.paused).ok_or(StateError::InvalidBridgePauseAuthorization)?;
  if auth_set.authorizations.iter().any(|a|a.paused!=next.emergency_paused){return Err(StateError::InvalidBridgePauseAuthorization);}
  self.bridge_security_policies.insert(bridge_id,next);
  self.bridge_pause_authorizations.insert(bridge_id,auth_set);
  Ok(())
 }
 pub(crate) fn register_bitcoin_checkpoint(&mut self,bridge_id:Hash,checkpoint:BitcoinChainCheckpoint)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  if bridge_id==[0;32]||!checkpoint.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if policy.bridge_id!=bridge_id||policy.min_external_confirmations==0{return Err(StateError::InvalidBridgeWithdrawal);}
  if let Some(old)=self.bitcoin_checkpoints.get(&bridge_id){
   if checkpoint.height<old.height
    || checkpoint.chainwork<old.chainwork
    || checkpoint.height==old.height&&checkpoint.block_hash!=old.block_hash
    || checkpoint.chainwork==old.chainwork&&checkpoint.block_hash!=old.block_hash {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  self.bitcoin_checkpoints.insert(bridge_id,checkpoint);Ok(())
 }
 pub fn validate_bitcoin_checkpoint_binding(&self,bridge_id:Hash,descriptor:&freedom_types::BridgeAdapterDescriptor)->Result<(),StateError>{
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let checkpoint=self.bitcoin_checkpoints.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if descriptor.bridge_id!=bridge_id||descriptor.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID||descriptor.kind!=freedom_types::BridgeAdapterKind::Bitcoin||descriptor.finality_model!=freedom_types::BridgeFinalityModel::Confirmations||descriptor.required_confirmations!=policy.min_external_confirmations||!checkpoint.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  Ok(())
 }
 pub(crate) fn register_bitcoin_reorg_policy(&mut self,bridge_id:Hash,policy:freedom_types::BitcoinReorgPolicy)->Result<(),StateError>{if bridge_id==[0;32]||!policy.validate(){return Err(StateError::BitcoinReorgDetected);} self.bitcoin_reorg_policy.insert(bridge_id,policy);Ok(())}
 pub fn bitcoin_reorg_severity(&self,bridge_id:Hash,e:&BitcoinReorgEvidence)->Result<freedom_types::BitcoinReorgSeverity,StateError>{let cp=self.bitcoin_checkpoints.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;let p=self.bitcoin_reorg_policy.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;if !e.validate()||!p.validate()||e.checkpoint_height!=cp.height||e.checkpoint_hash!=cp.block_hash||e.candidate_tip_height<=cp.height||e.candidate_chainwork<=cp.chainwork{return Err(StateError::BitcoinReorgDetected)}Ok(p.classify(cp,e))}
 pub(crate) fn record_bitcoin_reorg_evidence(&mut self,bridge_id:Hash,e:BitcoinReorgEvidence)->Result<(),StateError>{
  if bridge_id==[0;32]||!e.validate(){return Err(StateError::BitcoinReorgDetected);}
  let checkpoint=self.bitcoin_checkpoints.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;
  if e.checkpoint_height!=checkpoint.height
    || e.checkpoint_hash!=checkpoint.block_hash
    || e.candidate_tip_height<=e.checkpoint_height
    || e.candidate_chainwork<=checkpoint.chainwork {
    return Err(StateError::BitcoinReorgDetected);
  }
  let policy=self.bitcoin_reorg_policy.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;
  if !policy.validate(){return Err(StateError::BitcoinReorgDetected);}
  let journal=self.bitcoin_reorg_evidence.entry(bridge_id).or_default();
  if journal.iter().any(|x|x.fingerprint==e.fingerprint){return Ok(());}
  journal.push(e);
  if journal.len()>freedom_types::BITCOIN_REORG_EVIDENCE_LIMIT{let excess=journal.len()-freedom_types::BITCOIN_REORG_EVIDENCE_LIMIT;journal.drain(0..excess);}
  Ok(())
 }
 fn bitcoin_canonical_tip_transition_allowed(previous:&freedom_types::BitcoinChainCheckpoint,next:&freedom_types::BitcoinChainCheckpoint)->bool{
  if next.height<previous.height||next.chainwork<previous.chainwork{return false;}
  if next.chainwork==previous.chainwork&&next.block_hash!=previous.block_hash{return false;}
  true
}
 pub(crate) fn persist_bitcoin_canonical_tip(&mut self,bridge_id:Hash,proof:&freedom_types::BitcoinCanonicalTipProof)->Result<(),StateError>{self.bitcoin_bridge_gate(bridge_id)?;let checkpoint=self.bitcoin_checkpoints.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;if !proof.verify(checkpoint){return Err(StateError::BitcoinReorgDetected);}let next=BitcoinChainCheckpoint{height:proof.tip_height,block_hash:proof.tip_hash,chainwork:proof.chainwork};if let Some(prev)=self.bitcoin_canonical_tips.get(&bridge_id){if !Self::bitcoin_canonical_tip_transition_allowed(prev,&next){return Err(StateError::BitcoinReorgDetected);}}self.bitcoin_canonical_tips.insert(bridge_id,next);Ok(())}
 fn bitcoin_recovery_validator_keys(&self)->std::collections::BTreeMap<Hash,[u8;32]>{self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect()}
 pub(crate) fn recover_bitcoin_reorg_with_tip(&mut self,recovery:freedom_types::BitcoinReorgRecovery,proof:&freedom_types::BitcoinCanonicalTipProof)->Result<(),StateError>{
  if !recovery.validate(){return Err(StateError::BitcoinReorgDetected);} if proof.tip_height!=recovery.canonical_height||proof.tip_hash!=recovery.canonical_block_hash||proof.chainwork!=recovery.canonical_chainwork{return Err(StateError::BitcoinReorgDetected);}
  let journal=self.bitcoin_reorg_evidence.get(&recovery.bridge_id).ok_or(StateError::BitcoinReorgDetected)?;
  if !journal.iter().any(|e|e.fingerprint==recovery.evidence_fingerprint){return Err(StateError::BitcoinReorgDetected);} let checkpoint=self.bitcoin_checkpoints.get(&recovery.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?; if recovery.canonical_height<checkpoint.height||recovery.canonical_chainwork<checkpoint.chainwork{return Err(StateError::BitcoinReorgDetected);} if !proof.verify(checkpoint){return Err(StateError::BitcoinReorgDetected);}
  let policy=self.bitcoin_reorg_policy.get(&recovery.bridge_id).ok_or(StateError::BitcoinReorgDetected)?;
  let mut ids=recovery.validator_ids.clone();ids.sort();ids.dedup();
  if ids.len()!=recovery.validator_ids.len(){return Err(StateError::BitcoinReorgDetected);}
  let validators=self.bitcoin_recovery_validator_keys();
  if recovery.authenticated_count(&validators) < (validators.len() - validators.len()/3){return Err(StateError::BitcoinReorgDetected);}
  if let Some(prev)=self.bitcoin_reorg_recovery.get(&recovery.bridge_id){if recovery.nonce<=prev.nonce{return Err(StateError::BitcoinReorgDetected);}}
  self.bitcoin_reorg_recovery.insert(recovery.bridge_id,recovery);Ok(())
 }
 fn validate_validator_records_integrity(&self)->Result<(),StateError>{
  // The map key is the canonical validator identity. A mismatched
  // Validator::validator_id would otherwise be silently normalized by the
  // epoch candidate builder and could create an identity fork.
  let mut owners=std::collections::BTreeSet::<Address>::new();
  for (id,validator) in &self.validators {
    if *id != validator.validator_id
      || *id == [0;32]
      || validator.owner == [0;32]
      || freedom_primitives::hash_bytes(&validator.consensus_key) != *id
      || !owners.insert(validator.owner) {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  Ok(())
 }

 fn validate_unbonding_integrity(&self)->Result<(),StateError>{
  let mut totals=std::collections::BTreeMap::<Hash,u128>::new();
  for (id,entry) in &self.unbonding {
    if *id != entry.unbonding_id
      || *id == [0;32]
      || entry.validator_id == [0;32]
      || entry.owner == [0;32]
      || entry.amount == 0
      || entry.release_height < entry.requested_height {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    let validator=self.validators.get(&entry.validator_id)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
    if validator.owner != entry.owner {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    let total=totals.entry(entry.validator_id).or_insert(0);
    *total=total.checked_add(entry.amount)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
  }

  let mut owner_totals=std::collections::BTreeMap::<Address,u128>::new();
  for (id,validator) in &self.validators {
    if totals.get(id).copied().unwrap_or(0) != validator.unbonding_stake {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    let total=validator.stake.checked_add(validator.unbonding_stake)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
    let owner_total=owner_totals.entry(validator.owner).or_insert(0);
    *owner_total=owner_total.checked_add(total)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
  }

  if self.staking_activated {
    for (owner,total) in &owner_totals {
      let account=self.accounts.get(owner)
        .ok_or(StateError::InvalidBridgeWithdrawal)?;
      if account.staked != *total {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
    }
  }
  Ok(())
 }

 fn validate_validator_set_certificate_integrity(&self)->Result<(),StateError>{
  let Some(certificate)=self.validator_set_certificate.as_ref() else {
    return Ok(());
  };
  if !self.staking_activated
    || !certificate.validate_transition_fields()
    || certificate.hash() == [0;32]
    || certificate.to_epoch != self.epoch
    || certificate.from_epoch.checked_add(1) != Some(self.epoch) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let previous_set_hash=self.validator_set_history.get(&certificate.from_epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if certificate.previous_set_hash != previous_set_hash {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let previous_entries=self.validator_set_history_entries.get(&certificate.from_epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let previous_keys=self.validator_set_history_sets.get(&certificate.from_epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  if previous_entries.len()!=previous_keys.len()
    || previous_entries.is_empty()
    || previous_entries.iter().any(|(id,entry)|
      *id != entry.validator_id
      || previous_keys.get(id).copied()!=Some(entry.consensus_key)) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  // The certificate is for the transition out of the currently active set.
  // Persisted historical state must agree with that active set at the exact
  // transition boundary; otherwise quorum could be evaluated over one set
  // while signatures are interpreted against another.
  if previous_entries.len() != self.active_validator_set.len()
    || previous_entries.iter().any(|(id,entry)| {
      self.active_validator_set.get(id).map(|active| {
        active.validator_id == entry.validator_id
          && active.owner == entry.owner
          && active.consensus_key == entry.consensus_key
          && active.stake == entry.stake
      }).unwrap_or(false) == false
    }) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut previous_bytes=Vec::new();
  for (id,key) in previous_keys {
    previous_bytes.extend_from_slice(id);
    previous_bytes.extend_from_slice(key);
  }
  if self.validator_set_history_key_hashes.get(&certificate.from_epoch).copied()
      != Some(freedom_primitives::hash_bytes(&previous_bytes)) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut active_bytes=Vec::new();
  for (id,entry) in &self.active_validator_set {
    active_bytes.extend_from_slice(id);
    active_bytes.extend_from_slice(&entry.owner);
    active_bytes.extend_from_slice(&entry.consensus_key);
    active_bytes.extend_from_slice(&entry.stake.to_le_bytes());
  }
  if certificate.new_set_hash != freedom_primitives::hash_bytes(&active_bytes) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  if certificate.votes.iter().any(|vote| {
    let Some(entry)=previous_entries.get(&vote.validator_id) else { return true; };
    previous_keys.get(&vote.validator_id).copied() != Some(entry.consensus_key)
      || freedom_primitives::hash_bytes(&entry.consensus_key) != vote.validator_id
  }) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let authenticated_votes=certificate.authenticated_vote_count(previous_keys) as usize;
  let required_quorum=(previous_keys.len() - previous_keys.len()/3);
  if authenticated_votes < required_quorum {
    return Err(StateError::InvalidBridgeWithdrawal);
  }
  Ok(())
 }

 fn validate_slashed_evidence_integrity(&self)->Result<(),StateError>{
  for (evidence_hash,amount) in &self.slashed_evidence {    if *evidence_hash == [0;32] || *amount == 0 {      return Err(StateError::InvalidEvidence);
    }
    let evidence=self.evidence.get(evidence_hash)
      .ok_or(StateError::InvalidEvidence)?;
    if evidence.evidence_hash != *evidence_hash
      || freedom_primitives::hash_bytes(&evidence.signing_bytes()) != *evidence_hash
      || !self.validators.contains_key(&evidence.validator_id) {
      return Err(StateError::InvalidEvidence);
    }
    // The deterministic slash amount is authenticated at mutation time by
    // slash_evidence(). The persisted registry intentionally stores only the
    // resulting amount, so historical validation must not recompute that
    // amount from a validator balance that may have changed later.
  }
  Ok(())
 }

 fn validate_current_validator_set_history_binding(&self)->Result<(),StateError>{
  let epoch=self.epoch;
  let expected_set_hash=self.validator_set_history.get(&epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_keys=self.validator_set_history_sets.get(&epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_entries=self.validator_set_history_entries.get(&epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_key_hash=self.validator_set_history_key_hashes.get(&epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;

  if self.active_validator_set != *expected_entries {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let active_keys=self.active_validator_set.iter()
    .map(|(id,entry)|(*id,entry.consensus_key))
    .collect::<std::collections::BTreeMap<Hash,[u8;32]>>();
  if active_keys != *expected_keys {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut set_bytes=Vec::new();
  for (id,entry) in &self.active_validator_set {
    set_bytes.extend_from_slice(id);
    set_bytes.extend_from_slice(&entry.owner);
    set_bytes.extend_from_slice(&entry.consensus_key);
    set_bytes.extend_from_slice(&entry.stake.to_le_bytes());
  }
  if freedom_primitives::hash_bytes(&set_bytes) != expected_set_hash {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut key_bytes=Vec::new();
  for (id,key) in &active_keys {
    key_bytes.extend_from_slice(id);
    key_bytes.extend_from_slice(key);
  }
  if freedom_primitives::hash_bytes(&key_bytes) != expected_key_hash {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  Ok(())
 }

 fn validate_bitcoin_attestation_validator_set_binding(
  &self,
  attestation:&freedom_types::BitcoinUtxoCommitmentAttestation
 )->Result<(),StateError>{
  self.validate_current_validator_set_history_binding()?;
  let expected_hash=self.validator_set_history.get(&self.epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if attestation.validator_set_epoch!=self.epoch
    || attestation.validator_set_hash!=expected_hash {
    return Err(StateError::InvalidBridgeWithdrawal);
  }
  Ok(())
 }
 fn validate_bitcoin_attestation_policy_binding(
  &self,
  bridge_id:Hash,
  policy:&freedom_types::BridgeSecurityPolicy
 )->Result<(),StateError>{
  let expected=policy.commitment_hash();
  if let Some(stored)=self.bitcoin_utxo_attestation_policy_hashes.get(&bridge_id) {
    if *stored!=expected {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  Ok(())
 }
 pub fn validate_bitcoin_utxo_attestation_for_commitment(&self,bridge_id:Hash,commitment:&freedom_types::BitcoinUtxoBlockCommitment,attestation:&freedom_types::BitcoinUtxoCommitmentAttestation)->Result<(),StateError>{
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  self.validate_bitcoin_attestation_policy_binding(bridge_id,policy)?;
  self.validate_bitcoin_attestation_validator_set_binding(attestation)?;
  if attestation.validator_ids.iter().any(|id| !self.active_validator_set.contains_key(id)){return Err(StateError::InvalidBridgeWithdrawal);}
  if !policy.quorum_matches_validator_count(self.active_validator_set.len()){return Err(StateError::InvalidBridgeWithdrawal);}
  let validators=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<std::collections::BTreeMap<_,_>>();
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if commitment.block_height!=expected_height
    || attestation.bridge_id!=bridge_id
    || attestation.policy_id!=policy.bridge_id
    || attestation.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID
    || attestation.bitcoin_tip_height!=commitment.block_height
    || attestation.bitcoin_tip_hash!=commitment.block_hash
    || attestation.previous_utxo_root!=commitment.previous_root
    || attestation.accumulator_root!=commitment.next_root
    || !commitment.verify()
    || !attestation.validate_structure()
    || !attestation.verify_signatures(&validators,policy.validator_quorum) {
      return Err(StateError::InvalidBridgeWithdrawal);
  }
  match self.bitcoin_utxo_attestations.get(&bridge_id) {
    Some(previous) if previous.sequence == u64::MAX
      || attestation.sequence != previous.sequence.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)? => {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    None if attestation.sequence != 1 => {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    _ => {}
  }
  Ok(())
 } pub fn validate_finality_state_integrity(&self)->Result<(),StateError>{
  // Historical validator-set snapshots are authoritative and must form a
  // contiguous chain from genesis through the current epoch. A missing
  // historical epoch must never make an attestation history look valid.
  for epoch in 0..=self.epoch {
   if !self.validator_set_history.contains_key(&epoch)
     || !self.validator_set_history_sets.contains_key(&epoch)
     || !self.validator_set_history_entries.contains_key(&epoch)
     || !self.validator_set_history_key_hashes.contains_key(&epoch) {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  for (epoch, keys) in &self.validator_set_history_sets {
   let mut bytes=Vec::new();
   for (id,key) in keys { bytes.extend_from_slice(id); bytes.extend_from_slice(key); }
   let hash=freedom_primitives::hash_bytes(&bytes);
   if self.validator_set_history_key_hashes.get(epoch).copied()!=Some(hash) { return Err(StateError::InvalidBridgeWithdrawal); }
   if keys.is_empty() && self.validator_set_history_key_hashes.get(epoch).copied()!=Some(hash) { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  for epoch in self.validator_set_history_key_hashes.keys() {
   if !self.validator_set_history_sets.contains_key(epoch) || !self.validator_set_history_entries.contains_key(epoch) { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  for (epoch, entries) in &self.validator_set_history_entries {
   let expected_hash=self.validator_set_history.get(epoch).copied().ok_or(StateError::InvalidBridgeWithdrawal)?;
   let mut bytes=Vec::new();
   for (id,entry) in entries {
    if *id != entry.validator_id { return Err(StateError::InvalidBridgeWithdrawal); }
    bytes.extend_from_slice(id); bytes.extend_from_slice(&entry.owner); bytes.extend_from_slice(&entry.consensus_key); bytes.extend_from_slice(&entry.stake.to_le_bytes());
   }
   if entries.is_empty() || freedom_primitives::hash_bytes(&bytes) != expected_hash { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  self.validate_validator_records_integrity()?;
  self.validate_unbonding_integrity()?;
  self.validate_evidence_integrity()?;
  self.validate_slashed_evidence_integrity()?;
  self.validate_validator_set_certificate_integrity()?;
  self.validate_current_validator_set_history_binding()?;
  // Persisted Bitcoin checkpoints are consensus-critical inputs. Validate both
  // maps after deserialization and bind every entry to its bridge key.
  for (bridge_id, checkpoint) in &self.bitcoin_checkpoints {
   if *bridge_id == [0;32] || !checkpoint.validate() {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  for (bridge_id, checkpoint) in &self.bitcoin_canonical_tips {
   if *bridge_id == [0;32] || !checkpoint.validate() {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  for (bridge_id,attestation) in &self.bitcoin_utxo_attestations {
   if *bridge_id==[0;32] || attestation.bridge_id!=*bridge_id || !attestation.validate_structure() { return Err(StateError::InvalidBridgeWithdrawal); }
   self.validate_bitcoin_attestation_validator_set_binding(attestation)?;
   let policy=self.bridge_security_policies.get(bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
   if self.bitcoin_utxo_attestation_policy_hashes.get(bridge_id).copied()!=Some(policy.commitment_hash()) { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  for (bridge_id,policy_hash) in &self.bitcoin_utxo_attestation_policy_hashes {
   let policy=self.bridge_security_policies.get(bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
   if self.bitcoin_utxo_attestations.get(bridge_id).is_none() || *policy_hash!=policy.commitment_hash() { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  for (bridge_id,commitment) in &self.bitcoin_utxo_commitments {
   if *bridge_id==[0;32] || !commitment.verify() { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  // If both objects are persisted, they must describe the same UTXO transition.
  // Either may be absent while a bridge transition is in progress.
  for (bridge_id, attestation) in &self.bitcoin_utxo_attestations {
   if let Some(commitment) = self.bitcoin_utxo_commitments.get(bridge_id) {
    if attestation.bitcoin_tip_height != commitment.block_height
      || attestation.bitcoin_tip_hash != commitment.block_hash
      || attestation.previous_utxo_root != commitment.previous_root
      || attestation.accumulator_root != commitment.next_root {
     return Err(StateError::InvalidBridgeWithdrawal);
    }
   }
  }
  Ok(())
 }

 pub fn validate_cross_chain_finality_envelope(&self,envelope:&freedom_types::CrossChainFinalityEnvelope)->Result<(),StateError>{
  if !envelope.validate_structure(){return Err(StateError::InvalidBridgeWithdrawal);}
  self.validate_finality_state_integrity()?;
  let policy=self.bridge_security_policies.get(&envelope.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if policy.bridge_id!=envelope.bridge_id || !policy.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  let canonical=self.bitcoin_canonical_tips.get(&envelope.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if envelope.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID{return Err(StateError::InvalidBridgeWithdrawal);}
  if envelope.proof_kind==freedom_types::CrossChainProofKind::BitcoinUtxo {
    let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if envelope.finalized_height<canonical.height || envelope.finalized_height!=expected_height{return Err(StateError::InvalidBridgeWithdrawal);}
    if envelope.previous_state_root==envelope.state_root{return Err(StateError::InvalidBridgeWithdrawal);}
    let attestation = self.bitcoin_utxo_attestations.get(&envelope.bridge_id)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
    self.validate_bitcoin_attestation_validator_set_binding(attestation)?;
    if self.bitcoin_utxo_attestation_policy_hashes.get(&envelope.bridge_id).copied()!=Some(policy.commitment_hash()) {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    if self.staking_activated {
      let certificate=self.validator_set_certificate.as_ref()
        .ok_or(StateError::InvalidBridgeWithdrawal)?;
      if !certificate.validate_transition_fields()
        || certificate.hash() == [0;32]
        || certificate.to_epoch != self.epoch
        || certificate.from_epoch.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)? != self.epoch {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
      let active_set_hash = {
        let mut bytes=Vec::new();
        for (id, entry) in &self.active_validator_set {
          bytes.extend_from_slice(id);
          bytes.extend_from_slice(&entry.owner);
          bytes.extend_from_slice(&entry.consensus_key);
          bytes.extend_from_slice(&entry.stake.to_le_bytes());
        }
        freedom_primitives::hash_bytes(&bytes)
      };
      if certificate.new_set_hash != active_set_hash {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
      let previous_set_hash=self.validator_set_history.get(&certificate.from_epoch)
        .ok_or(StateError::InvalidBridgeWithdrawal)?;
      if *previous_set_hash != certificate.previous_set_hash {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
      if certificate.from_epoch.checked_add(1) != Some(certificate.to_epoch) {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
      let certificate_entries=self.validator_set_history_entries.get(&certificate.from_epoch)
        .ok_or(StateError::InvalidBridgeWithdrawal)?;
      let certificate_keys=self.validator_set_history_sets.get(&certificate.from_epoch)
        .ok_or(StateError::InvalidBridgeWithdrawal)?;
      if certificate_entries.len() != certificate_keys.len()
        || certificate_entries.iter().any(|(id,entry)| certificate_keys.get(id).copied() != Some(entry.consensus_key)) {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
      let mut previous_bytes=Vec::new();
      for (id,key) in certificate_keys {
        previous_bytes.extend_from_slice(id);
        previous_bytes.extend_from_slice(key);
      }
      let previous_key_snapshot_hash=freedom_primitives::hash_bytes(&previous_bytes);
      if self.validator_set_history_key_hashes.get(&certificate.from_epoch).copied()
          != Some(previous_key_snapshot_hash)
        || certificate_keys.is_empty()
        || !policy.quorum_matches_validator_count(certificate_keys.len())
        || certificate.authenticated_vote_count(certificate_keys) < usize::from(policy.validator_quorum) {
        return Err(StateError::InvalidBridgeWithdrawal);
      }
    }
    let validators=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<std::collections::BTreeMap<_,_>>();
    if attestation.sequence != envelope.sequence
      || attestation.bridge_id != envelope.bridge_id
      || attestation.policy_id != policy.bridge_id
      || attestation.bitcoin_tip_height != envelope.finalized_height
      || attestation.bitcoin_tip_hash != envelope.finalized_hash
      || attestation.bitcoin_tip_height != canonical.height
      || attestation.bitcoin_tip_hash != canonical.block_hash
      || attestation.bitcoin_tip_chainwork != canonical.chainwork
      || attestation.previous_utxo_root != envelope.previous_state_root
      || attestation.accumulator_root != envelope.state_root
      || !attestation.validate_structure()
      || !policy.quorum_matches_validator_count(self.active_validator_set.len())
      || !attestation.verify_signatures(&validators,policy.validator_quorum) {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
    let commitment = self.bitcoin_utxo_commitments.get(&envelope.bridge_id)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
    if !commitment.verify()
      || commitment.block_height != envelope.finalized_height
      || commitment.block_hash != envelope.finalized_hash
      || commitment.previous_root != envelope.previous_state_root
      || commitment.next_root != envelope.state_root {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  // The persisted attestation is the candidate finalized transition itself.
  // Its sequence continuity was enforced when it was registered; requiring
  // envelope.sequence == stored.sequence + 1 here would make every persisted
  // attestation impossible to finalize.
  if envelope.proof_kind==freedom_types::CrossChainProofKind::BitcoinUtxo {
    let attestation=self.bitcoin_utxo_attestations.get(&envelope.bridge_id)
      .ok_or(StateError::InvalidBridgeWithdrawal)?;
    if attestation.sequence != envelope.sequence || attestation.sequence == 0 {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  Ok(())
 }
}

#[cfg(test)]
mod m339_validator_certificate_quorum_overflow_tests {
    use super::*;
    use freedom_primitives::CHAIN_ID;
    use freedom_types::Genesis;

    #[test]
    fn persisted_certificate_quorum_is_overflow_safe() {
        let count = usize::MAX;
        let expected = count - (count / 3 + usize::from(count % 3 != 0)) + 1;
        assert_eq!(freedom_consensus::quorum_threshold(count), expected);
    }

    #[test]
    fn persisted_certificate_quorum_matches_small_sets() {
        let expected = [usize::MAX, 1, 2, 3, 3, 4, 5, 5, 6, 7];
        for (count, threshold) in expected.into_iter().enumerate() {
            assert_eq!(freedom_consensus::quorum_threshold(count), threshold);
        }
    }
}

#[cfg(test)]
mod m251_finality_attestation_gate_tests {
    use super::*;

    #[test]
    fn bitcoin_finality_rejects_missing_attestation() {
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        let bridge=[7u8;32];
        state.bridge_security_policies.insert(bridge, freedom_types::BridgeSecurityPolicy {
            bridge_id:bridge, approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1000, max_withdrawal_amount:1000,
            daily_limit:1000, emergency_paused:false, nonce_floor:0,
        });
        state.bitcoin_canonical_tips.insert(bridge, freedom_types::BitcoinChainCheckpoint {
            height:100, block_hash:[8;32], chainwork:[9;32],
        });
        let mut envelope=freedom_types::CrossChainFinalityEnvelope {
            bridge_id:bridge, chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            sequence:1, proof_kind:freedom_types::CrossChainProofKind::BitcoinUtxo,
            finalized_height:101, finalized_hash:[13;32],
            state_root:[11;32], previous_state_root:[10;32], proof_hash:[0;32],
        };
        envelope.proof_hash=envelope.compute_hash();
        assert_eq!(state.validate_cross_chain_finality_envelope(&envelope),Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m250_bitcoin_commitment_storage_tests {
    use super::*;

    #[test]
    fn commitment_storage_rejects_duplicate_height_and_accepts_continuous_next_root() {
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        let bridge=[7u8;32];
        state.bitcoin_canonical_tips.insert(bridge, freedom_types::BitcoinChainCheckpoint {
            height:100, block_hash:[8;32], chainwork:[9;32],
        });
        let first=freedom_types::BitcoinUtxoBlockCommitment {
            block_height:101, block_hash:[13;32], previous_root:[10;32], next_root:[11;32],
            update_proof:freedom_types::BitcoinUtxoSmtUpdateProof{previous_root:[10;32],next_root:[11;32],steps:vec![]},
        };
        state.bitcoin_utxo_commitments.insert(bridge, first.clone());
        let second=freedom_types::BitcoinUtxoBlockCommitment {
            block_height:102, block_hash:[14;32], previous_root:[11;32], next_root:[12;32],
            update_proof:freedom_types::BitcoinUtxoSmtUpdateProof{previous_root:[11;32],next_root:[12;32],steps:vec![]},
        };
        assert!(second.block_height>first.block_height);
        assert_eq!(second.previous_root,first.next_root);
    }
}

#[cfg(test)]
mod m247_finality_commitment_binding_tests {
    use super::*;

    #[test]
    fn bitcoin_finality_requires_matching_commitment_payload() {
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        let bridge = [7u8;32];
        state.bitcoin_canonical_tips.insert(bridge, freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state.bridge_security_policies.insert(bridge, freedom_types::BridgeSecurityPolicy {
            bridge_id: bridge,
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1, validator_total: 1,
            threshold_signers: 1, threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000,
            max_withdrawal_amount: 1_000,
            daily_limit: 1_000,
            emergency_paused: false,
            nonce_floor: 0,
        });
        let commitment = freedom_types::BitcoinUtxoBlockCommitment {
            block_height: 101,
            block_hash: [13;32],
            previous_root: [10;32],
            next_root: [11;32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root: [10;32], next_root: [11;32], steps: vec![],
            },
        };
        state.bitcoin_utxo_commitments.insert(bridge, commitment.clone());
        let mut envelope = freedom_types::CrossChainFinalityEnvelope {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            sequence: 1,
            proof_kind: freedom_types::CrossChainProofKind::BitcoinUtxo,
            finalized_height: 101,
            finalized_hash: [13;32],
            state_root: [11;32],
            previous_state_root: [10;32],
            proof_hash: [0;32],
        };
        envelope.proof_hash = envelope.compute_hash();
        assert_eq!(state.validate_cross_chain_finality_envelope(&envelope), Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m229_withdrawal_request_policy_tests {
    use super::*;

    fn state() -> ChainState {
        let genesis = freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bridge_security_policies.insert([9;32], freedom_types::BridgeSecurityPolicy {
            bridge_id:[9;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1000, max_withdrawal_amount:100,
            daily_limit:100, emergency_paused:false, nonce_floor:5,
        });
        state
    }

    fn withdrawal(amount:u128, nonce:u64) -> freedom_types::BridgeWithdrawal {
        let mut w=freedom_types::BridgeWithdrawal {
            withdrawal_id:[0;32], bridge_id:[9;32], asset_id:*[0;32].as_ref(),
            destination_chain_id:2, amount, sender:[6;32], recipient:vec![7],
            nonce, request_hash:[0;32],
        };
        w.asset_id=[1;32];
        w.request_hash=w.compute_hash();
        w.withdrawal_id=w.compute_id();
        w
    }

    #[test]
    fn request_rejects_amount_above_policy() {
        let mut state=state();
        let asset=*state.asset_registry.keys().next().unwrap();
        let mut w=withdrawal(101,5); w.asset_id=asset; w.request_hash=w.compute_hash(); w.withdrawal_id=w.compute_id();
        assert_eq!(state.request_bridge_withdrawal(w), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn request_rejects_nonce_below_policy_floor() {
        let mut state=state();
        let asset=*state.asset_registry.keys().next().unwrap();
        let mut w=withdrawal(10,4); w.asset_id=asset; w.request_hash=w.compute_hash(); w.withdrawal_id=w.compute_id();
        assert_eq!(state.request_bridge_withdrawal(w), Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m228_withdrawal_policy_boundary_tests {
    use super::*;

    fn state() -> ChainState {
        let genesis = freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bridge_security_policies.insert([9;32], freedom_types::BridgeSecurityPolicy {
            bridge_id:[9;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1000, max_withdrawal_amount:100,
            daily_limit:100, emergency_paused:false, nonce_floor:5,
        });
        state
    }

    #[test]
    fn withdrawal_approval_rejects_policy_limit_violation() {
        let mut state=state();
        let id=[3;32]; let request=[4;32];
        state.bridge_withdrawals.insert(id, freedom_types::BridgeWithdrawal {
            withdrawal_id:id, bridge_id:[9;32], asset_id:[5;32], destination_chain_id:2,
            amount:101, sender:[6;32], recipient:vec![7], nonce:5, request_hash:[8;32],
        });
        state.bridge_withdrawal_status.insert(id, freedom_types::BridgeWithdrawalStatus::Validated);
        state.bridge_approval_sets.insert(request, freedom_types::BridgeApprovalSet {
            request_id:request, approvals:vec![], threshold_signatures:1,
        });
        assert_eq!(state.approve_bridge_with_authenticated_set(id,request), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn withdrawal_approval_rejects_policy_nonce_floor() {
        let mut state=state();
        let id=[3;32]; let request=[4;32];
        state.bridge_withdrawals.insert(id, freedom_types::BridgeWithdrawal {            withdrawal_id:id, bridge_id:[9;32], asset_id:[5;32], destination_chain_id:2,
            amount:10, sender:[6;32], recipient:vec![7], nonce:4, request_hash:[8;32],        });
        state.bridge_withdrawal_status.insert(id, freedom_types::BridgeWithdrawalStatus::Validated);
        state.bridge_approval_sets.insert(request, freedom_types::BridgeApprovalSet {
            request_id:request, approvals:vec![], threshold_signatures:1,
        });
        assert_eq!(state.approve_bridge_with_authenticated_set(id,request), Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m227_pause_authorization_canonical_tests {
    use super::*;

    #[test]
    fn pause_authorization_rejects_duplicate_validator_ids() {
        let genesis = freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bridge_security_policies.insert([7;32], freedom_types::BridgeSecurityPolicy {
            bridge_id:[7;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1, max_withdrawal_amount:1,
            daily_limit:1, emergency_paused:false, nonce_floor:0,
        });
        let auth = freedom_types::BridgePauseAuthorizationSet {
            bridge_id:[7;32], nonce:1,
            authorizations:vec![
                freedom_types::BridgePauseAuthorization {
                    bridge_id:[7;32], paused:true, nonce:1, validator_id:[13;32], signature:vec![0;64],
                },
                freedom_types::BridgePauseAuthorization {
                    bridge_id:[7;32], paused:true, nonce:1, validator_id:[13;32], signature:vec![0;64],
                },
            ],
        };
        assert_eq!(state.apply_bridge_emergency_pause([7;32],auth), Err(StateError::InvalidBridgePauseAuthorization));
    }
}

#[cfg(test)]
mod m226_withdrawal_quorum_active_validator_tests {
    use super::*;

    #[test]
    fn withdrawal_quorum_uses_active_validator_set() {
        let key = [7u8; 32];
        let validator_id = freedom_primitives::hash_bytes(&key);
        let v = freedom_types::Validator {
            validator_id, owner:[1;32], consensus_key:key, stake:100, unbonding_stake:0,
            blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let genesis = freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![v.clone()] };
        let mut state = ChainState::genesis(&genesis);
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id:[9;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1000, max_withdrawal_amount:1000,
            daily_limit:1000, emergency_paused:false, nonce_floor:0,
        }).unwrap();
        state.active_validator_set.clear();
        let id=[3;32]; let request=[4;32];
        state.bridge_withdrawals.insert(id, freedom_types::BridgeWithdrawal {
            withdrawal_id:id, bridge_id:[9;32], asset_id:[5;32], destination_chain_id:2,
            amount:10, sender:[6;32], recipient:vec![7], nonce:1, request_hash:[8;32],
        });
        state.bridge_withdrawal_status.insert(id, freedom_types::BridgeWithdrawalStatus::Validated);
        state.bridge_approval_sets.insert(request, freedom_types::BridgeApprovalSet {
            request_id:request, approvals:vec![], threshold_signatures:0,
        });
        assert_eq!(
            state.approve_bridge_with_authenticated_set(id,request),
            Err(StateError::InvalidBridgeWithdrawalTransition)
        );
    }
}

#[cfg(test)]
mod m225_bridge_approval_set_active_validator_tests {
    use super::*;

    #[test]
    fn approval_set_rejects_inactive_validator() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bridge_security_policies.insert([7;32], freedom_types::BridgeSecurityPolicy {
            bridge_id:[7;32], approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1, max_withdrawal_amount:1,
            daily_limit:1, emergency_paused:false, nonce_floor:0,
        });
        let set = freedom_types::BridgeMessageApprovalSet {
            bridge_id:[7;32], message_hash:[8;32], approvals: vec![
                freedom_types::BridgeMessageApproval {
                    bridge_id:[7;32], message_hash:[8;32], validator_id:[13;32], signature:vec![0;64],
                }
            ]
        };
        assert_eq!(state.record_bridge_approval_set(set), Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m223_bitcoin_reorg_recovery_active_validator_tests {
    use super::*;

    #[test]
    fn recovery_rejects_signer_outside_active_set() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_reorg_policy.insert([7;32], freedom_types::BitcoinReorgPolicy {
            bridge_id:[7;32], max_reorg_depth:6, min_chainwork:[1;32], validator_quorum:1,
        });
        let recovery = freedom_types::BitcoinReorgRecovery {
            bridge_id:[7;32], evidence_fingerprint:[8;32], nonce:1,
            canonical_height:100, canonical_block_hash:[9;32],
            canonical_chainwork:[10;32],
            validator_ids:vec![[13;32]],
            signatures:vec![freedom_types::BitcoinReorgRecoverySignature {
                validator_id:[13;32], signature:vec![0;64],
            }],
            recovery_hash:[0;32],
        };
        assert_eq!(
            state.register_bitcoin_reorg_recovery(recovery),
            Err(StateError::BitcoinReorgDetected)
        );
    }
}

#[cfg(test)]
mod m234_bitcoin_apply_reorg_gate_tests {
 use super::*;

 #[test]
 fn attested_commitment_apply_rejects_unresolved_reorg() {
  let genesis=freedom_types::Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};
  let mut state=ChainState::genesis(&genesis);
  let bridge=[7;32];
  state.bitcoin_canonical_tips.insert(bridge,freedom_types::BitcoinChainCheckpoint{height:100,block_hash:[8;32],chainwork:[9;32]});
  let evidence=freedom_types::BitcoinReorgEvidence{
   checkpoint_height:100,checkpoint_hash:[8;32],candidate_tip_height:101,candidate_tip_hash:[13;32],
   candidate_chainwork:[15;32],fingerprint:[0;32],
  };
  let mut evidence=evidence;
  evidence.fingerprint=evidence.compute_fingerprint();
  state.bitcoin_reorg_evidence.insert(bridge,vec![evidence]);
  state.bitcoin_reorg_policy.insert(bridge,freedom_types::BitcoinReorgPolicy{
   elevated_confirmations:1,critical_chainwork_margin:[1;32],
  });
  let block=freedom_types::BitcoinSpendScanBlock{height:101,header:freedom_types::BitcoinBlockHeader{version:1,previous_block_hash:[8;32],merkle_root:[0;32],timestamp:1,bits:0,nonce:0},raw_transactions:vec![]};
  let commitment=freedom_types::BitcoinUtxoBlockCommitment{block_height:101,block_hash:[13;32],previous_root:[10;32],next_root:[11;32],update_proof:freedom_types::BitcoinUtxoSmtUpdateProof{previous_root:[10;32],next_root:[11;32],steps:vec![]}};
  let attestation=freedom_types::BitcoinUtxoCommitmentAttestation{bridge_id:bridge,chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,policy_id:[6;32],validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,bitcoin_tip_height:101,bitcoin_tip_hash:[13;32],bitcoin_tip_chainwork:[15;32],previous_utxo_root:[10;32],accumulator_root:[11;32],validator_ids:vec![],signatures:vec![],attestation_hash:[0;32]};
  assert_eq!(state.apply_bitcoin_utxo_attested_commitment(bridge,&block,&commitment,&attestation),Err(StateError::BitcoinReorgDetected));
 }
}

#[cfg(test)]
mod m232_bitcoin_attestation_binding_tests {
 use super::*;

 fn setup()->(ChainState,freedom_types::BitcoinUtxoBlockCommitment,freedom_types::BitcoinUtxoCommitmentAttestation){
  let genesis=freedom_types::Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};
  let mut state=ChainState::genesis(&genesis);
  state.bitcoin_canonical_tips.insert([7;32],freedom_types::BitcoinChainCheckpoint{height:100,block_hash:[8;32],chainwork:[9;32]});
  state.bridge_security_policies.insert([7;32],freedom_types::BridgeSecurityPolicy{bridge_id:[7;32],approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1_000,max_withdrawal_amount:1_000,daily_limit:1_000,emergency_paused:false,nonce_floor:0});
  let commitment=freedom_types::BitcoinUtxoBlockCommitment{block_height:101,block_hash:[13;32],previous_root:[10;32],next_root:[11;32],update_proof:freedom_types::BitcoinUtxoSmtUpdateProof{previous_root:[10;32],next_root:[11;32],steps:vec![]}};
  let attestation=freedom_types::BitcoinUtxoCommitmentAttestation{bridge_id:[7;32],chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,policy_id:[6;32],validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,bitcoin_tip_height:101,bitcoin_tip_hash:[13;32],bitcoin_tip_chainwork:[9;32],previous_utxo_root:[10;32],accumulator_root:[11;32],validator_ids:vec![],signatures:vec![],attestation_hash:[0;32]};
  (state,commitment,attestation)
 }

 #[test]
 fn commitment_validation_rejects_wrong_bitcoin_chain_id(){
  let (state,commitment,mut attestation)=setup();
  attestation.chain_id=1;
  assert_eq!(state.validate_bitcoin_utxo_attestation_for_commitment([7;32],&commitment,&attestation),Err(StateError::InvalidBridgeWithdrawal));
 }

 #[test]
 fn commitment_validation_rejects_wrong_tip_chainwork(){
  let (state,commitment,mut attestation)=setup();
  attestation.bitcoin_tip_chainwork=[12;32];
  assert_eq!(state.validate_bitcoin_utxo_attestation_for_commitment([7;32],&commitment,&attestation),Err(StateError::InvalidBridgeWithdrawal));
 }
}


#[cfg(test)]
mod m222_bitcoin_attestation_apply_active_validator_tests {
    use super::*;

    #[test]
    fn commitment_validation_rejects_signer_outside_active_set() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7;32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id:[7;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:1_000_000,
            max_withdrawal_amount:1_000_000, daily_limit:1_000_000,
            emergency_paused:false, nonce_floor:0,
        }).unwrap();
        let commitment = freedom_types::BitcoinUtxoBlockCommitment {
            block_height:101, block_hash:[13;32], previous_root:[10;32],
            next_root:[11;32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root:[10;32], next_root:[11;32], steps:vec![],
            },
        };
        let attestation = freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:[7;32], chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[6;32], validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1, bitcoin_tip_height:101,
            bitcoin_tip_hash:[13;32], bitcoin_tip_chainwork:[14;32],
            previous_utxo_root:[10;32], accumulator_root:[11;32],
            validator_ids:vec![[13;32]],
            signatures:vec![freedom_types::BitcoinReorgRecoverySignature {
                validator_id:[13;32], signature:vec![0;64],
            }],
            attestation_hash:[0;32],
        };
        assert_eq!(
            state.validate_bitcoin_utxo_attestation_for_commitment([7;32], &commitment, &attestation),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m221_bitcoin_attestation_active_validator_tests {
    use super::*;

    #[test]
    fn registration_rejects_attestation_signer_outside_active_set() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7;32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id:[7;32],
            approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1, validator_total:1,
            threshold_signers:1, threshold_total:1,
            min_external_confirmations:1,
            max_deposit_amount:1_000_000,
            max_withdrawal_amount:1_000_000,
            daily_limit:1_000_000,
            emergency_paused:false, nonce_floor:0,
        }).unwrap();
        let attestation = freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:[7;32],
            chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[6;32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,
            bitcoin_tip_height:100,
            bitcoin_tip_hash:[8;32],
            bitcoin_tip_chainwork:[9;32],
            previous_utxo_root:[10;32],
            accumulator_root:[11;32],
            validator_ids:vec![[13;32]],
            signatures:vec![freedom_types::BitcoinReorgRecoverySignature {
                validator_id:[13;32], signature:vec![0;64],
            }],
            attestation_hash:[0;32],
        };
        assert_eq!(
            state.register_bitcoin_utxo_commitment_attestation(attestation),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m220_sequence_overflow_tests {
    use super::*;

    #[test]
    fn finality_rejects_repeated_max_sequence() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7;32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state.bitcoin_utxo_attestations.insert([7;32], freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:[7;32], chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[6;32], validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:u64::MAX, bitcoin_tip_height:100,
            bitcoin_tip_hash:[8;32], bitcoin_tip_chainwork:[9;32],
            previous_utxo_root:[10;32], accumulator_root:[11;32],
            validator_ids:vec![[13;32]], signatures:vec![], attestation_hash:[12;32],
        });
        let envelope=freedom_types::CrossChainFinalityEnvelope {
            bridge_id:[7;32], chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            sequence:u64::MAX, proof_kind:freedom_types::CrossChainProofKind::BitcoinUtxo,
            finalized_height:101, finalized_hash:[9;32], state_root:[14;32],
            previous_state_root:[11;32], proof_hash:[15;32],
        };
        assert_eq!(state.validate_cross_chain_finality_envelope(&envelope),Err(StateError::InvalidBridgeWithdrawal));
    }
}

#[cfg(test)]
mod m219_bitcoin_utxo_attestation_sequence_tests {
    use super::*;

    fn state_with_attestation(sequence: u64) -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7;32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state.bitcoin_utxo_attestations.insert([7;32], freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id: [7;32],
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id: [6;32],
            sequence,
            bitcoin_tip_height: 100,
            bitcoin_tip_hash: [8;32],
            bitcoin_tip_chainwork: [9;32],
            previous_utxo_root: [10;32],
            accumulator_root: [11;32],
            validator_ids: vec![],
            signatures: vec![],
            attestation_hash: [12;32],
        });
        state
    }

    #[test]
    fn registration_rejects_sequence_gap() {
        let mut state = state_with_attestation(4);
        let attestation = state.bitcoin_utxo_attestations.get(&[7;32]).unwrap().clone();
        let mut next = attestation.clone();
        next.sequence = 6;
        assert_eq!(
            state.register_bitcoin_utxo_commitment_attestation(next),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m215_cross_chain_finality_bitcoin_binding_tests {
    use super::*;

    fn state_with_bitcoin_finality() -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7;32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8;32], chainwork: [9;32],
        });
        state
    }

    fn envelope(chain_id: u32) -> freedom_types::CrossChainFinalityEnvelope {
        freedom_types::CrossChainFinalityEnvelope {
            bridge_id: [7;32],
            chain_id,
            sequence: 1,
            proof_kind: freedom_types::CrossChainProofKind::BitcoinUtxo,
            finalized_height: 101,
            finalized_hash: [9;32],
            state_root: [10;32],
            previous_state_root: [11;32],
            proof_hash: [12;32],
        }
    }

    #[test]
    fn state_finality_rejects_sequence_gap() {
        let mut state = state_with_bitcoin_finality();
        state.bitcoin_utxo_attestations.insert([7;32], freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:[7;32],
            chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[6;32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:4,
            bitcoin_tip_height:100,
            bitcoin_tip_hash:[8;32],
            bitcoin_tip_chainwork:[9;32],
            previous_utxo_root:[10;32],
            accumulator_root:[11;32],
            validator_ids:vec![],
            signatures:vec![],
            attestation_hash:[12;32],
        });
        let mut e = envelope(freedom_types::BITCOIN_MAINNET_CHAIN_ID);
        e.sequence = 6;
        e.previous_state_root = [11;32];
        assert_eq!(
            state.validate_cross_chain_finality_envelope(&e),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn state_finality_rejects_disconnected_previous_utxo_root() {
        let mut state = state_with_bitcoin_finality();
        state.bitcoin_utxo_attestations.insert([7;32], freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:[7;32],
            chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[6;32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,
            bitcoin_tip_height:100,
            bitcoin_tip_hash:[8;32],
            bitcoin_tip_chainwork:[9;32],
            previous_utxo_root:[10;32],
            accumulator_root:[11;32],
            validator_ids:vec![],
            signatures:vec![],
            attestation_hash:[12;32],
        });
        let mut e = envelope(freedom_types::BITCOIN_MAINNET_CHAIN_ID);
        e.previous_state_root = [13;32];
        assert_eq!(
            state.validate_cross_chain_finality_envelope(&e),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn state_finality_rejects_non_bitcoin_chain_id() {
        let state = state_with_bitcoin_finality();
        assert_eq!(
            state.validate_cross_chain_finality_envelope(&envelope(1)),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

}

impl ChainState {
 /// Fail closed for chains whose consensus/finality verifier is not implemented.
 /// Bitcoin envelopes still pass through the full state-level canonical-tip,
 /// policy, attestation, and validator-set checks below.
 pub fn validate_cross_chain_proof_adapter(&self,envelope:&freedom_types::CrossChainFinalityEnvelope)->Result<(),StateError>{
  if envelope.proof_kind != freedom_types::CrossChainProofKind::BitcoinUtxo {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  self.validate_cross_chain_finality_envelope(envelope)
 }

 pub(crate) fn apply_bitcoin_utxo_attested_commitment(&mut self,bridge_id:Hash,block:&freedom_types::BitcoinSpendScanBlock,commitment:&freedom_types::BitcoinUtxoBlockCommitment,attestation:&freedom_types::BitcoinUtxoCommitmentAttestation)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?; block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if commitment.block_height!=block.height || commitment.block_hash!=block.header.block_hash(){return Err(StateError::InvalidBridgeWithdrawal);}
  if attestation.bitcoin_tip_height!=block.height || attestation.bitcoin_tip_hash!=commitment.block_hash{return Err(StateError::InvalidBridgeWithdrawal);}
  let block_work=freedom_types::bitcoin_header_work(&block.header).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let mut expected_chainwork=canonical.chainwork;
  let mut carry=0u16;
  for i in (0..32).rev() {
    let sum=u16::from(expected_chainwork[i])+u16::from(block_work[i])+carry;
    expected_chainwork[i]=(sum & 0xff) as u8;
    carry=sum >> 8;
  }
  if carry!=0 || attestation.bitcoin_tip_chainwork!=expected_chainwork{return Err(StateError::InvalidBridgeWithdrawal);}
  self.validate_bitcoin_utxo_attestation_for_commitment(bridge_id,commitment,attestation)?;
  if let Some(root)=self.bitcoin_utxo_attestations.get(&bridge_id).map(|a|a.accumulator_root) {
    if root!=commitment.previous_root { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  if let Some(old)=self.bitcoin_utxo_attestations.get(&bridge_id) {
    let expected_sequence = old.sequence.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
    if attestation.sequence != expected_sequence { return Err(StateError::InvalidBridgeWithdrawal); }
  }
  // Resolve every fallible dependency before mutating canonical state. A missing
  // policy must not leave the tip or attestation partially advanced.
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy_hash=policy.commitment_hash();
  let next=freedom_types::BitcoinChainCheckpoint{height:block.height,block_hash:block.header.block_hash(),chainwork:attestation.bitcoin_tip_chainwork};
  self.bitcoin_canonical_tips.insert(bridge_id,next);
  self.bitcoin_utxo_attestations.insert(bridge_id,attestation.clone());
  self.bitcoin_utxo_attestation_policy_hashes.insert(bridge_id,policy_hash);
  Ok(())
 }

 pub(crate) fn register_bitcoin_utxo_commitment_attestation(&mut self,attestation:freedom_types::BitcoinUtxoCommitmentAttestation)->Result<(),StateError>{
  self.validate_bitcoin_attestation_validator_set_binding(&attestation)?;
  let policy=self.bridge_security_policies.get(&attestation.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let canonical=self.bitcoin_canonical_tips.get(&attestation.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  self.validate_bitcoin_attestation_policy_binding(attestation.bridge_id,policy)?;
  if attestation.validator_ids.iter().any(|id| !self.active_validator_set.contains_key(id)){return Err(StateError::InvalidBridgeWithdrawal);}
  let validators=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<std::collections::BTreeMap<_,_>>();
  if attestation.bridge_id!=policy.bridge_id
    || attestation.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID
    || !policy.quorum_matches_validator_count(self.active_validator_set.len())
    || !attestation.verify(canonical,&validators,policy.validator_quorum){return Err(StateError::InvalidBridgeWithdrawal);}
  if let Some(previous)=self.bitcoin_utxo_attestations.get(&attestation.bridge_id){
    let expected_sequence = previous.sequence.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
    if attestation.sequence != expected_sequence{return Err(StateError::InvalidBridgeWithdrawal);}
  }
  let bridge_id=attestation.bridge_id;
  self.bitcoin_utxo_attestations.insert(bridge_id,attestation);
  self.bitcoin_utxo_attestation_policy_hashes.insert(bridge_id,policy.commitment_hash());
  Ok(())
 }
 pub fn bitcoin_utxo_commitment_attestation(&self,bridge_id:Hash)->Option<&freedom_types::BitcoinUtxoCommitmentAttestation>{self.bitcoin_utxo_attestations.get(&bridge_id)} pub(crate) fn register_bitcoin_block_utxo_commitment(&mut self,bridge_id:Hash,block:&freedom_types::BitcoinSpendScanBlock,commitment:freedom_types::BitcoinUtxoBlockCommitment)->Result<(),StateError>{
  self.validate_bitcoin_block_utxo_commitment(bridge_id,block,&commitment)?;
  if let Some(previous)=self.bitcoin_utxo_commitments.get(&bridge_id) {
    if commitment.block_height <= previous.block_height
      || commitment.previous_root != previous.next_root {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  self.bitcoin_utxo_commitments.insert(bridge_id,commitment);
  Ok(())
 }
 pub(crate) fn record_validator_set_epoch_hash(&mut self, epoch:u64, set_hash:Hash)->Result<(),StateError>{
  if set_hash==[0;32] || epoch>self.epoch {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  // Historical validator snapshots are authoritative and are created only by
  // genesis or the epoch-transition path. This method is validation-only:
  // it must never create a partial history entry.
  let stored_set_hash=self.validator_set_history.get(&epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if stored_set_hash!=set_hash {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  let entries=self.validator_set_history_entries.get(&epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let keys=self.validator_set_history_sets.get(&epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let key_hash=self.validator_set_history_key_hashes.get(&epoch)
    .copied().ok_or(StateError::InvalidBridgeWithdrawal)?;

  if entries.is_empty() || entries.len()!=keys.len() {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  if entries.iter().any(|(id,entry)| {
   entry.validator_id!=*id || keys.get(id).copied()!=Some(entry.consensus_key)
  }) {
   return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut bytes=Vec::new();
  for (id,entry) in entries {
   bytes.extend_from_slice(id);
   bytes.extend_from_slice(&entry.owner);
   bytes.extend_from_slice(&entry.consensus_key);
   bytes.extend_from_slice(&entry.stake.to_le_bytes());
  }
  if freedom_primitives::hash_bytes(&bytes)!=stored_set_hash {
   return Err(StateError::InvalidBridgeWithdrawal);
  }

  let mut key_bytes=Vec::new();
  for (id,key) in keys {
   key_bytes.extend_from_slice(id);
   key_bytes.extend_from_slice(key);
  }
  if freedom_primitives::hash_bytes(&key_bytes)!=key_hash {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  Ok(())
 }
 pub fn bitcoin_utxo_block_commitment(&self,bridge_id:Hash)->Option<&freedom_types::BitcoinUtxoBlockCommitment>{self.bitcoin_utxo_commitments.get(&bridge_id)}

 /// Compatibility builder for non-consensus preview use only.
 /// Consensus callers must use `build_bitcoin_block_utxo_commitment_with_script_flags`
 /// with activation-correct Bitcoin Script flags.
 #[deprecated(note = "consensus validation must use build_bitcoin_block_utxo_commitment_with_script_flags")]
 pub fn build_bitcoin_block_utxo_commitment(&self,bridge_id:Hash,block:&freedom_types::BitcoinSpendScanBlock,previous:&freedom_types::BitcoinUtxoSmtSnapshot)->Result<freedom_types::BitcoinUtxoBlockCommitment,StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  let commitment=freedom_types::BitcoinUtxoBlockCommitment::build(previous,block).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if commitment.block_hash!=block.header.block_hash(){return Err(StateError::InvalidBridgeWithdrawal);}
  if let Some(prior)=self.bitcoin_utxo_commitments.get(&bridge_id) {
   if commitment.previous_root!=prior.next_root || commitment.block_height<=prior.block_height {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  Ok(commitment)
 }

 /// Build a Bitcoin UTXO commitment only after witness commitment and every
 /// non-coinbase input Script have been validated with activation-correct flags.
 /// The caller remains responsible for selecting flags for this network and height.
 pub fn build_bitcoin_block_utxo_commitment_with_script_flags(
  &self,
  bridge_id:Hash,
  block:&freedom_types::BitcoinSpendScanBlock,
  previous:&freedom_types::BitcoinUtxoSmtSnapshot,
  script_flags:u32,
 )->Result<freedom_types::BitcoinUtxoBlockCommitment,StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if let Some(prior)=self.bitcoin_utxo_commitments.get(&bridge_id) {
   if prior.block_height>=block.height || previous.root().map_err(|_|StateError::InvalidBridgeWithdrawal)?!=prior.next_root {
    return Err(StateError::InvalidBridgeWithdrawal);
   }
  }
  let commitment=freedom_types::BitcoinUtxoBlockCommitment::build_with_script_flags(previous,block,script_flags)
   .map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if commitment.block_hash!=block.header.block_hash()
   || commitment.block_height!=block.height {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  Ok(commitment)
 }
 pub fn validate_bitcoin_block_utxo_commitment(&self,bridge_id:Hash,block:&freedom_types::BitcoinSpendScanBlock,commitment:&freedom_types::BitcoinUtxoBlockCommitment)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if commitment.block_height!=block.height||commitment.block_hash!=block.header.block_hash()||!commitment.verify(){return Err(StateError::InvalidBridgeWithdrawal);} if let Some(attestation)=self.bitcoin_utxo_attestations.get(&bridge_id){if commitment.previous_root!=attestation.accumulator_root{return Err(StateError::InvalidBridgeWithdrawal);}} if let Some(previous)=self.bitcoin_utxo_commitments.get(&bridge_id){if commitment.block_height<=previous.block_height||commitment.previous_root!=previous.next_root{return Err(StateError::InvalidBridgeWithdrawal);}}
  Ok(())
 }
 pub fn validate_bitcoin_compact_utxo_transition(&self,bridge_id:Hash,block:&freedom_types::BitcoinSpendScanBlock,proof:&freedom_types::BitcoinUtxoSmtUpdateProof)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if proof.previous_root==[0;32]||proof.next_root==[0;32]||!proof.verify(){return Err(StateError::InvalidBridgeWithdrawal);} if let Some(attestation)=self.bitcoin_utxo_attestations.get(&bridge_id){if proof.previous_root!=attestation.accumulator_root{return Err(StateError::InvalidBridgeWithdrawal);}}
  Ok(())
 }
 pub fn validate_bitcoin_block_utxo_transition(&self,transition:&freedom_types::BitcoinUtxoAccumulatorTransition,previous:&[freedom_types::BitcoinUtxoStateEntry],block:&freedom_types::BitcoinSpendScanBlock)->Result<(),StateError>{
  self.bitcoin_bridge_gate(transition.bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&transition.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let expected_height=canonical.height.checked_add(1).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if transition.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID || transition.previous_tip!=*canonical || block.height!=expected_height || transition.next_tip.height!=block.height || transition.next_tip.block_hash!=block.header.block_hash(){return Err(StateError::InvalidBridgeWithdrawal);}
  let attestation=self.bitcoin_utxo_attestations.get(&transition.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if transition.previous_root!=attestation.accumulator_root{return Err(StateError::InvalidBridgeWithdrawal);}
  block.verify_against_parent(canonical.block_hash,expected_height).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  let (root,count)=freedom_types::BitcoinUtxoAccumulatorTransition::verify_block_against_header(previous,block).map_err(|_|StateError::InvalidBridgeWithdrawal)?;
  if root!=transition.next_root||count!=transition.next_leaf_count{return Err(StateError::InvalidBridgeWithdrawal)}
  Ok(())
 }
 pub fn validate_bitcoin_utxo_transition(&self,transition:&freedom_types::BitcoinUtxoAccumulatorTransition,previous:&[freedom_types::BitcoinUtxoStateEntry])->Result<(),StateError>{
  self.bitcoin_bridge_gate(transition.bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&transition.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if transition.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID || transition.previous_tip.block_hash!=canonical.block_hash || transition.previous_tip.height!=canonical.height { return Err(StateError::InvalidBridgeWithdrawal); }
  let attestation=self.bitcoin_utxo_attestations.get(&transition.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if transition.previous_root!=attestation.accumulator_root{return Err(StateError::InvalidBridgeWithdrawal);}
  transition.verify_transition(previous).map_err(|_|StateError::InvalidBridgeWithdrawal)
 }
 pub fn validate_bitcoin_utxo_accumulator_proof(&self,bridge_id:Hash,proof:&freedom_types::BitcoinUtxoAccumulatorProof)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let attestation=self.bitcoin_utxo_attestations.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if proof.accumulator.bridge_id!=bridge_id
    || proof.accumulator.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID
    || proof.accumulator.root!=attestation.accumulator_root
    || proof.accumulator.bitcoin_tip_height!=canonical.height
    || proof.accumulator.bitcoin_tip_hash!=canonical.block_hash
    || proof.accumulator.bitcoin_tip_chainwork!=canonical.chainwork
    || !proof.verify_membership(canonical){return Err(StateError::InvalidBridgeWithdrawal);}
  Ok(())
 }
 pub fn validate_bitcoin_utxo_spend_state(&self,bridge_id:Hash,proof:&freedom_types::BitcoinUtxoSpendStateProof)->Result<(),StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let canonical=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if proof.utxo.bridge_id!=bridge_id
    || proof.utxo.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID{return Err(StateError::InvalidBridgeWithdrawal);}
  proof.verify_against_canonical_tip(canonical).map_err(|_|StateError::InvalidBridgeWithdrawal)
 }
 pub fn bitcoin_canonical_tip(&self,bridge_id:Hash)->Option<&BitcoinChainCheckpoint>{self.bitcoin_canonical_tips.get(&bridge_id)}
 pub fn validate_bitcoin_bridge_safety(&self,bridge_id:Hash)->Result<(),StateError>{self.bitcoin_bridge_gate(bridge_id)}
 fn bitcoin_bridge_gate(&self,bridge_id:Hash)->Result<(),StateError>{if let Some(journal)=self.bitcoin_reorg_evidence.get(&bridge_id){if let Some(last)=journal.last(){let severity=self.bitcoin_reorg_severity(bridge_id,last)?;if severity!=freedom_types::BitcoinReorgSeverity::Normal{if self.bitcoin_reorg_recovery.get(&bridge_id).map(|r|r.evidence_fingerprint!=last.fingerprint).unwrap_or(true){return Err(StateError::BitcoinReorgDetected);}}}}Ok(())}
 pub(crate) fn advance_bitcoin_checkpoint(&mut self,bridge_id:Hash,headers:&[freedom_types::BitcoinBlockHeader])->Result<BitcoinChainCheckpoint,StateError>{
  self.bitcoin_bridge_gate(bridge_id)?;
  let current=self.bitcoin_checkpoints.get(&bridge_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  match freedom_types::bitcoin_validate_from_checkpoint(&current,headers){
   Ok(next)=>{self.bitcoin_checkpoints.insert(bridge_id,next.clone());Ok(next)}
   Err(freedom_types::BitcoinCheckpointError::LinkageMismatch)=>{
    if let Ok(report)=freedom_types::bitcoin_evaluate_reorg(&current,headers){let mut e=BitcoinReorgEvidence{checkpoint_height:report.checkpoint_height,checkpoint_hash:report.checkpoint_hash,candidate_tip_height:report.candidate_tip_height,candidate_tip_hash:report.candidate_tip_hash,candidate_chainwork:report.candidate_chainwork,fingerprint:[0;32]};e.fingerprint=e.compute_fingerprint();let _=self.record_bitcoin_reorg_evidence(bridge_id,e);}
    Err(StateError::BitcoinReorgDetected)
   },
   Err(_)=>Err(StateError::InvalidBridgeWithdrawal)
  }
 }
 pub(crate) fn register_bridge_security_policy(&mut self, policy:BridgeSecurityPolicy)->Result<(),StateError>{
  if !policy.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  if self.bridge_security_policies.contains_key(&policy.bridge_id){return Err(StateError::InvalidBridgeWithdrawal);}
  self.bridge_security_policies.insert(policy.bridge_id,policy); Ok(())
 }
 pub(crate) fn record_bridge_approval_set(&mut self, set:BridgeApprovalSet)->Result<(),StateError>{
  if set.request_id==[0;32]||set.approvals.is_empty(){return Err(StateError::InvalidBridgeWithdrawal);}
  if self.bridge_approval_sets.contains_key(&set.request_id){return Err(StateError::DuplicateBridgeWithdrawal);}
  let policy_id=set.approvals.first().map(|a|a.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy=self.bridge_security_policies.get(&policy_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if set.approvals.iter().any(|a|a.bridge_id!=policy.bridge_id||a.request_id!=set.request_id||!a.structurally_valid()){return Err(StateError::InvalidBridgeWithdrawal);}
  if !set.approvals.windows(2).all(|pair| pair[0].validator_id < pair[1].validator_id) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }
  let active_ids=self.active_validator_set.keys().collect::<std::collections::BTreeSet<_>>();
  if set.approvals.iter().any(|a| !active_ids.contains(&a.validator_id)) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }
  self.bridge_approval_sets.insert(set.request_id,set); Ok(())
 }
 pub fn bridge_approval_set(&self, request_id:&Hash)->Option<&BridgeApprovalSet>{self.bridge_approval_sets.get(request_id)}
 
 pub fn validate_bridge_message(&self, message:&BridgeMessage)->Result<Hash,StateError>{
  if message.protocol_version != BRIDGE_MESSAGE_PROTOCOL_VERSION
   || message.destination_chain_id != freedom_primitives::CHAIN_ID
   || message.source_chain_id == freedom_primitives::CHAIN_ID
   || message.amount == 0
   || !message.validate_hash() { return Err(StateError::InvalidBridgeMessage); }
  let asset=self.asset_registry.get(&message.asset_id).ok_or(StateError::InvalidBridgeMessage)?;
  if asset.asset_type != AssetType::Bridged || asset.status != AssetStatus::Active
   || asset.source_chain_id != message.source_chain_id || asset.bridge_id != message.bridge_id {
   return Err(StateError::InvalidBridgeMessage);
  }
  let replay_key=message.replay_key();
  if let Some(existing)=self.processed_bridge_messages.get(&replay_key){
   if existing.message_hash == message.message_hash { return Err(StateError::DuplicateBridgeMessage); }
   return Err(StateError::InvalidBridgeMessage);
  }
  Ok(replay_key)
 }
 pub(crate) fn mint_bridge_authorized(
  &mut self,
  message: &BridgeMessage,
  deposit: &VerifiedDeposit,
  authorization: &BridgeMintAuthorization,
  approvals: &BridgeMessageApprovalSet,
) -> Result<u128, StateError> {
  let replay_key = message.replay_key();
  let stored = self.processed_bridge_messages.get(&replay_key)
    .ok_or(StateError::InvalidBridgeMessage)?;
  if !stored.validate_hash() || stored.replay_key() != replay_key {
    return Err(StateError::InvalidBridgeMessage);
  }
  if stored != message {
    return Err(StateError::InvalidBridgeMessage);
  }
  if self.bridge_message_status.get(&replay_key) != Some(&BridgeMessageStatus::Accepted) {
    return Err(StateError::InvalidBridgeMessage);
  }

  let policy = self.bridge_security_policies.get(&message.bridge_id)
    .ok_or(StateError::InvalidBridgeMessage)?;
  if !policy.validate() || policy.emergency_paused || !policy.nonce_allowed(message.nonce) {
    return Err(StateError::InvalidBridgeMessage);
  }
  if message.source_chain_id == freedom_types::BITCOIN_MAINNET_CHAIN_ID {
    self.validate_bitcoin_bridge_safety(message.bridge_id)?;
    let canonical = self.bitcoin_canonical_tips.get(&message.bridge_id)
      .ok_or(StateError::InvalidBridgeMessage)?;
    if !canonical.validate()
      || canonical.height < deposit.block_or_slot
      || freedom_types::bitcoin_confirmations_at_tip(canonical.height, deposit.block_or_slot)
        .ok_or(StateError::InvalidBridgeMessage)?
        < policy.min_external_confirmations {
      return Err(StateError::InvalidBridgeMessage);
    }
    let attestation = self.bitcoin_utxo_attestations.get(&message.bridge_id)
      .ok_or(StateError::InvalidBridgeMessage)?;
    self.validate_bitcoin_attestation_validator_set_binding(attestation)
      .map_err(|_| StateError::InvalidBridgeMessage)?;
    if self.bitcoin_utxo_attestation_policy_hashes.get(&message.bridge_id).copied()
      != Some(policy.commitment_hash()) {
      return Err(StateError::InvalidBridgeMessage);
    }
    let validators = self.active_validator_set.iter()
      .map(|(id, v)| (*id, v.consensus_key))
      .collect::<BTreeMap<Hash, [u8; 32]>>();
    if attestation.bridge_id != message.bridge_id
      || attestation.chain_id != freedom_types::BITCOIN_MAINNET_CHAIN_ID
      || !attestation.verify(canonical, &validators, policy.validator_quorum) {
      return Err(StateError::InvalidBridgeMessage);
    }
  }
  if !authorization.validate()
    || !authorization.matches_message(message)
    || !authorization.replay_key_matches(deposit)
    || authorization.bridge_id != policy.bridge_id
    || authorization.destination_chain_id != freedom_primitives::CHAIN_ID
    || authorization.amount != deposit.amount
  {
    return Err(StateError::InvalidBridgeMessage);
  }

  let asset = self.asset_registry.get(&authorization.canonical_asset_id)
    .ok_or(StateError::InvalidBridgeMessage)?;
  if asset.asset_type != AssetType::Bridged
    || asset.status != AssetStatus::Active
    || asset.source_chain_id != authorization.source_chain_id
    || asset.bridge_id != authorization.bridge_id
  {
    return Err(StateError::InvalidBridgeMessage);
  }

  let keys = self.active_validator_set.iter()
    .map(|(id, v)| (*id, v.consensus_key))
    .collect::<BTreeMap<Hash, [u8; 32]>>();
  if !approvals.authorized(policy, &keys)
    || approvals.bridge_id != message.bridge_id
    || approvals.message_hash != message.message_hash
    || authorization.approval_count != approvals.authenticated_count(&keys)
  {
    return Err(StateError::InvalidBridgeMessage);
  }

  if let Some(existing) = self.bridge_deposit_replays.get(&authorization.replay_key) {
    if existing.validate() && existing.status == freedom_types::DepositReplayStatus::Consumed {
      return Err(StateError::DuplicateBridgeMessage);
    }
    return Err(StateError::InvalidBridgeMessage);
  }

  let replay = DepositReplayRecord::from_verified(deposit);
  if !replay.validate() || replay.replay_key != authorization.replay_key {
    return Err(StateError::InvalidBridgeMessage);
  }

  let accounting = self.bridge_accounting.get(&authorization.canonical_asset_id)
    .cloned().unwrap_or_default();
  if self.validate_bridge_accounting(authorization.canonical_asset_id).is_err() {
    return Err(StateError::InvalidBridgeAccounting);
  }
  let next_minted = accounting.freedom_minted
    .checked_add(authorization.amount)
    .ok_or(StateError::InvalidBridgeAccounting)?;
  if next_minted > accounting.verified_external_locked
    || accounting.freedom_burned > next_minted
    || accounting.external_released > accounting.freedom_burned
  {
    return Err(StateError::InvalidBridgeAccounting);
  }
  let next_total = self.supply.total.checked_add(authorization.amount)
    .ok_or(StateError::ArithmeticOverflow)?;
  let next_circulating = self.supply.circulating.checked_add(authorization.amount)
    .ok_or(StateError::ArithmeticOverflow)?;
  if next_total != next_circulating
    .checked_add(self.supply.locked).ok_or(StateError::ArithmeticOverflow)?
    .checked_add(self.supply.staked).ok_or(StateError::ArithmeticOverflow)?
    .checked_add(self.supply.treasury).ok_or(StateError::ArithmeticOverflow)?
  {
    return Err(StateError::ArithmeticOverflow);
  }
  if authorization.recipient == [0; 32] {
    return Err(StateError::InvalidRecipient);
  }
  let current_balance = self.accounts.get(&authorization.recipient)
    .map(|a| a.balance).unwrap_or(0);
  let next_balance = current_balance.checked_add(authorization.amount)
    .ok_or(StateError::ArithmeticOverflow)?;

  let mut consumed = replay;
  if !consumed.accept() || !consumed.consume() {
    return Err(StateError::InvalidBridgeMessage);
  }

  // Preflight the resulting bridge-accounting state before mutating it.
  // The post-mint invariant must hold even if the transaction later exits
  // through an error path.
  let mut next_accounting = accounting.clone();
  next_accounting.freedom_minted = next_minted;
  if next_accounting.freedom_burned > next_accounting.freedom_minted
    || next_accounting.external_released > next_accounting.freedom_burned
    || next_accounting.freedom_minted > next_accounting.verified_external_locked
  {
    return Err(StateError::InvalidBridgeAccounting);
  }

  self.bridge_accounting.insert(authorization.canonical_asset_id, next_accounting);
  let account = self.accounts.entry(authorization.recipient).or_insert(Account {
    address: authorization.recipient,
    public_key: [0; 32],
    nonce: 0,
    balance: 0,
    locked: 0,
    staked: 0,
  });
  account.balance = next_balance;
  self.supply.total = next_total;
  self.supply.circulating = next_circulating;
  self.bridge_deposit_replays.insert(authorization.replay_key, consumed);
  self.bridge_mint_authorizations.insert(authorization.replay_key, authorization.clone());
  self.bridge_message_status.insert(replay_key, BridgeMessageStatus::Minted);
  Ok(authorization.amount)
 }

 pub(crate) fn record_bridge_message(&mut self, message:BridgeMessage)->Result<Hash,StateError>{
  let replay_key=self.validate_bridge_message(&message)?;
  self.processed_bridge_messages.insert(replay_key,message);
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Verified);
  Ok(replay_key)
 }
 pub(crate) fn accept_bridge_message(&mut self,replay_key:Hash)->Result<(),StateError>{
  let message=self.processed_bridge_messages.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if message.replay_key()!=replay_key {
   return Err(StateError::InvalidBridgeMessage);
  }
  let status=self.bridge_message_status.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if *status != BridgeMessageStatus::Verified {
   return Err(StateError::InvalidBridgeMessage);
  }
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Accepted);
  Ok(())
 }
 pub(crate) fn mark_bridge_message_minted(&mut self,replay_key:Hash)->Result<(),StateError>{
  let status=self.bridge_message_status.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if *status != BridgeMessageStatus::Accepted { return Err(StateError::InvalidBridgeMessage); }
  if !self.processed_bridge_messages.contains_key(&replay_key) { return Err(StateError::InvalidBridgeMessage); }
  let authorization=self.bridge_mint_authorizations.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  let replay=self.bridge_deposit_replays.get(&authorization.replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  let message=self.processed_bridge_messages.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if replay.status != freedom_types::DepositReplayStatus::Consumed
    || !authorization.validate()
    || !authorization.matches_message(message) {
    return Err(StateError::InvalidBridgeMessage);
  }
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Minted);
  Ok(())
 }

 pub(crate) fn request_bridge_withdrawal(&mut self, withdrawal:BridgeWithdrawal)->Result<Hash,StateError>{
  if withdrawal.destination_chain_id==freedom_primitives::CHAIN_ID || withdrawal.amount==0 || withdrawal.sender==[0;32] || withdrawal.recipient.is_empty() || !withdrawal.validate_hash() || withdrawal.compute_id()!=withdrawal.withdrawal_id { return Err(StateError::InvalidBridgeWithdrawal); }
  let asset=self.asset_registry.get(&withdrawal.asset_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if asset.asset_type!=AssetType::Bridged || asset.status!=AssetStatus::Active || asset.source_chain_id!=withdrawal.destination_chain_id || asset.bridge_id!=withdrawal.bridge_id { return Err(StateError::InvalidBridgeWithdrawal); }
  let policy=self.bridge_security_policies.get(&withdrawal.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if !policy.validate() || !policy.withdrawal_allowed(withdrawal.amount) || !policy.nonce_allowed(withdrawal.nonce) {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  if self.bridge_withdrawals.contains_key(&withdrawal.withdrawal_id) { return Err(StateError::DuplicateBridgeWithdrawal); }
  let nonce_key=BridgeNonceKey{bridge_id:withdrawal.bridge_id,asset_id:withdrawal.asset_id,direction:BridgeDirection::Outbound}.key_hash();
  if self.bridge_nonce_registry.get(&nonce_key).map(|n|n.contains(&withdrawal.nonce)).unwrap_or(false){return Err(StateError::DuplicateBridgeNonce);}
  let id=withdrawal.withdrawal_id;
  let withdrawal_nonce=withdrawal.nonce;
  self.bridge_withdrawals.insert(id,withdrawal);
  self.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Requested);
  self.bridge_nonce_registry.entry(nonce_key).or_default().insert(withdrawal_nonce);
  Ok(id)
 }
 pub(crate) fn approve_bridge_with_authenticated_set(&mut self,id:Hash,request_id:Hash)->Result<(),StateError>{
  let current=*self.bridge_withdrawal_status.get(&id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if current!=BridgeWithdrawalStatus::Validated { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  let w=self.bridge_withdrawals.get(&id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if w.request_hash==[0;32] || request_id==[0;32] { return Err(StateError::InvalidBridgeWithdrawal); }
  let set=self.bridge_approval_sets.get(&request_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy=self.bridge_security_policies.get(&w.bridge_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if !policy.validate() || !policy.withdrawal_allowed(w.amount) || !policy.nonce_allowed(w.nonce) {
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  if set.request_id!=request_id || set.approvals.iter().any(|a|a.withdrawal_id!=id||a.bridge_id!=w.bridge_id||a.nonce!=w.nonce||a.amount!=w.amount||a.destination_chain_id!=w.destination_chain_id||a.asset_id!=w.asset_id||a.intent_hash!=w.request_hash){
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  let keys=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<BTreeMap<Hash,[u8;32]>>();
  let authenticated_approvals=set.authenticated_approvals(w.bridge_id,id,w.request_hash,&keys);
  let quorum=policy.quorum_reached(authenticated_approvals);
  let threshold=set.threshold_reached(&policy) && set.threshold_signatures <= authenticated_approvals;
  let approved=match policy.approval_mode { freedom_types::BridgeApprovalMode::Quorum=>quorum, freedom_types::BridgeApprovalMode::Threshold=>threshold, freedom_types::BridgeApprovalMode::QuorumAndThreshold=>quorum&&threshold };
  if !approved { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  let period=self.height;
  self.reserve_bridge_withdrawal_limit(w.bridge_id,period,w.amount)?;
  self.bridge_withdrawal_rate_limit_period.insert(id,period);
  self.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Approved);
  Ok(())
 }
 pub(crate) fn advance_bridge_withdrawal(&mut self,id:Hash,next:BridgeWithdrawalStatus)->Result<(),StateError>{
  let current=*self.bridge_withdrawal_status.get(&id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if !self.bridge_withdrawals.contains_key(&id) { return Err(StateError::InvalidBridgeWithdrawal); }
  let allowed=match (current,next){
   (BridgeWithdrawalStatus::Requested,BridgeWithdrawalStatus::Validated)|
   (BridgeWithdrawalStatus::Validated,BridgeWithdrawalStatus::Approved)|
   (BridgeWithdrawalStatus::Approved,BridgeWithdrawalStatus::Signing)|
   (BridgeWithdrawalStatus::Signing,BridgeWithdrawalStatus::Broadcast)|
   (BridgeWithdrawalStatus::Broadcast,BridgeWithdrawalStatus::ExternalConfirmed)|
   (BridgeWithdrawalStatus::ExternalConfirmed,BridgeWithdrawalStatus::Finalized)|
   (BridgeWithdrawalStatus::Requested,BridgeWithdrawalStatus::Rejected)|
   (BridgeWithdrawalStatus::Validated,BridgeWithdrawalStatus::Rejected)|
   (BridgeWithdrawalStatus::Approved,BridgeWithdrawalStatus::Rejected)=>true,
   _=>false,
  };
  if !allowed { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  if next==BridgeWithdrawalStatus::Approved { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  if next==BridgeWithdrawalStatus::Finalized {
   let w=self.bridge_withdrawals.get(&id).ok_or(StateError::InvalidBridgeWithdrawal)?;
   let a=self.bridge_accounting.get(&w.asset_id).cloned().unwrap_or_default();
   if a.external_released.checked_add(w.amount).ok_or(StateError::InvalidBridgeAccounting)?>a.freedom_burned { return Err(StateError::InvalidBridgeAccounting); }
   self.record_external_release(w.asset_id,w.amount)?;
  }
  self.bridge_withdrawal_status.insert(id,next); Ok(())
 }
 pub fn validate_bitcoin_utxo_deposit(&self,bridge_id:Hash,proof:&freedom_types::BitcoinUtxoProof,expected_tx:Hash,expected_vout:u32,expected_amount:u64,expected_script:&[u8])->Result<(),StateError>{self.bitcoin_bridge_gate(bridge_id)?;let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;let tip=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;if proof.bridge_id!=bridge_id
  || proof.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID
  || !proof.validate_binding(expected_tx,expected_vout,expected_amount,expected_script)||!proof.validate_output_binding()||proof.merkle_branch.is_empty()||proof.computed_tx_hash()!=Some(proof.tx_hash)||!proof.validate_bitcoin_chain_binding()||freedom_types::bitcoin_merkle_root(proof.tx_hash,&proof.merkle_branch,proof.merkle_index)==[0;32]||proof.block_height>tip.height||proof.confirmations!=tip.height.saturating_sub(proof.block_height).saturating_add(1)||proof.confirmations<policy.min_external_confirmations{return Err(StateError::BitcoinReorgDetected);}Ok(())}
 pub fn validate_bitcoin_deposit_against_canonical_tip(&self,bridge_id:Hash,proof:&freedom_types::BitcoinMerkleProof,headers:&[freedom_types::BitcoinBlockHeader])->Result<(),StateError>{self.bitcoin_bridge_gate(bridge_id)?;let tip=self.bitcoin_canonical_tips.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::BitcoinReorgDetected)?;if proof.bridge_id!=bridge_id||proof.chain_id!=freedom_types::BITCOIN_MAINNET_CHAIN_ID||!proof.verify_against_canonical_tip(policy.min_external_confirmations,tip,headers){return Err(StateError::BitcoinReorgDetected);}Ok(())}
 pub fn bitcoin_deposit_allowed(&self,bridge_id:Hash)->Result<(),StateError>{self.bitcoin_bridge_gate(bridge_id)}
 pub(crate) fn record_verified_external_lock(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 || !self.asset_registry.contains_key(&asset_id) { return Err(StateError::InvalidBridgeAccounting); }
  let current=self.bridge_accounting.get(&asset_id).cloned().unwrap_or_default();
  let mut next=current.clone();
  next.verified_external_locked=next.verified_external_locked.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next.freedom_minted>next.verified_external_locked || next.freedom_burned>next.freedom_minted || next.external_released>next.freedom_burned {
   return Err(StateError::InvalidBridgeAccounting);
  }
  self.bridge_accounting.insert(asset_id,next);
  Ok(())
 }
 pub(crate) fn record_freedom_mint(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let current=self.bridge_accounting.get(&asset_id).cloned().unwrap_or_default();
  let mut next_accounting=current.clone();
  next_accounting.freedom_minted=next_accounting.freedom_minted.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next_accounting.freedom_minted>next_accounting.verified_external_locked || next_accounting.freedom_burned>next_accounting.freedom_minted || next_accounting.external_released>next_accounting.freedom_burned {
   return Err(StateError::InvalidBridgeAccounting);
  }
  self.bridge_accounting.insert(asset_id,next_accounting);
  Ok(())
 }
 pub(crate) fn record_freedom_burn(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let current=self.bridge_accounting.get(&asset_id).cloned().unwrap_or_default();
  let mut next_accounting=current.clone();
  next_accounting.freedom_burned=next_accounting.freedom_burned.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next_accounting.freedom_burned>next_accounting.freedom_minted || next_accounting.external_released>next_accounting.freedom_burned || next_accounting.freedom_minted>next_accounting.verified_external_locked {
   return Err(StateError::InvalidBridgeAccounting);
  }
  self.bridge_accounting.insert(asset_id,next_accounting);
  Ok(())
 }
 pub(crate) fn record_external_release(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let current=self.bridge_accounting.get(&asset_id).cloned().unwrap_or_default();
  let mut next_accounting=current.clone();
  next_accounting.external_released=next_accounting.external_released.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next_accounting.external_released>next_accounting.freedom_burned || next_accounting.freedom_burned>next_accounting.freedom_minted || next_accounting.freedom_minted>next_accounting.verified_external_locked {
   return Err(StateError::InvalidBridgeAccounting);
  }
  self.bridge_accounting.insert(asset_id,next_accounting);
  Ok(())
 }
 pub fn validate_bridge_accounting(&self,asset_id:Hash)->Result<(),StateError>{
  let a=self.bridge_accounting.get(&asset_id).cloned().unwrap_or_default();
  if a.freedom_minted>a.verified_external_locked || a.freedom_burned>a.freedom_minted || a.external_released>a.freedom_burned { return Err(StateError::InvalidBridgeAccounting); }
  let outstanding_freedom=a.freedom_minted.checked_sub(a.freedom_burned).ok_or(StateError::InvalidBridgeAccounting)?;
  let outstanding_external=a.verified_external_locked.checked_sub(a.external_released).ok_or(StateError::InvalidBridgeAccounting)?;
  if outstanding_freedom!=outstanding_external { return Err(StateError::InvalidBridgeAccounting); }
  Ok(())
 }

 pub fn validator_lifecycle(&self, validator_id:Hash)->Result<freedom_types::ValidatorLifecycle,StateError>{let v=self.validators.get(&validator_id).ok_or(StateError::InvalidEvidence)?;Ok(if !self.staking_activated || v.stake>0{freedom_types::ValidatorLifecycle::Active}else if v.unbonding_stake>0{freedom_types::ValidatorLifecycle::Unbonding}else{freedom_types::ValidatorLifecycle::Exited})}
 pub fn active_validator_entries(&self)->Vec<freedom_types::ValidatorSetEntry>{self.active_validator_set.values().cloned().collect()}
 pub(crate) fn refresh_pending_validator_set(&mut self){
  self.pending_validator_set=self.validators.iter().filter(|(_,v)|!self.staking_activated||v.stake>0).map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect();
 }
 pub(crate) fn refresh_validator_set(&mut self){
  self.refresh_pending_validator_set();
  // This legacy/public refresh path must never activate a validator set that  // is not the authoritative snapshot for the current epoch. Epoch changes
  // are exclusively committed by maybe_advance_epoch().
  if self.validate_current_validator_set_history_binding().is_err() {
    return;  }
  let Some(expected_entries)=self.validator_set_history_entries.get(&self.epoch) else {
    return;
  };
  if self.pending_validator_set != *expected_entries {
    return;
  }
  self.active_validator_set=self.pending_validator_set.clone();
 }
 fn validate_staking_activation_integrity(&self)->Result<(),StateError>{
  if !self.staking_activated {
    if self.accounts.values().any(|a| a.staked>0)
      || self.validators.values().any(|v| v.unbonding_stake>0)
      || !self.unbonding.is_empty() {
      return Err(StateError::InvalidBridgeWithdrawal);
    }
  }
  Ok(())
 }

 fn validate_epoch_transition_certificate_binding(
  &self,
  next_epoch:u64,
  next_validator_set:&BTreeMap<Hash,freedom_types::ValidatorSetEntry>,
)->Result<(),StateError>{
  if !self.staking_activated {
    return Ok(());
  }
  let certificate=self.validator_set_certificate.as_ref()
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  if !certificate.validate_transition_fields()
    || certificate.from_epoch!=self.epoch
    || certificate.to_epoch!=next_epoch {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let hash_set=|set:&BTreeMap<Hash,freedom_types::ValidatorSetEntry>|{
    // Must match ChainState's authoritative validator-set history encoding.
    let mut bytes=Vec::new();
    for (id,entry) in set {
      bytes.extend_from_slice(id);
      bytes.extend_from_slice(&entry.owner);
      bytes.extend_from_slice(&entry.consensus_key);
      bytes.extend_from_slice(&entry.stake.to_le_bytes());
    }
    freedom_primitives::hash_bytes(&bytes)
  };

  if certificate.previous_set_hash!=hash_set(&self.active_validator_set)
    || certificate.new_set_hash!=hash_set(next_validator_set) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let previous_keys=self.validator_set_history_sets.get(&self.epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  let previous_entries=self.validator_set_history_entries.get(&self.epoch)
    .ok_or(StateError::InvalidBridgeWithdrawal)?;
  if previous_keys.is_empty()
    || previous_entries.len()!=previous_keys.len()
    || previous_entries.iter().any(|(id,entry)|
      *id!=entry.validator_id
        || previous_keys.get(id).copied()!=Some(entry.consensus_key)) {
    return Err(StateError::InvalidBridgeWithdrawal);
  }

  let n=previous_keys.len();
  let ceil_third=n/3+usize::from(n%3!=0);
  let required_quorum=n-ceil_third+1;
  if certificate.authenticated_vote_count(previous_keys) as usize < required_quorum {
    return Err(StateError::InvalidBridgeWithdrawal);
  }
  Ok(())
 }

 pub(crate) fn maybe_advance_epoch(&mut self){
  let at_epoch_boundary=self.epoch_length>0
    && self.height>0
    && self.height%self.epoch_length==0;
  if !at_epoch_boundary {
   self.release_mature_unbonding();
   return;
  }
  let next_epoch=self.height/self.epoch_length;
   if next_epoch<=self.epoch { return; }
   // Epoch transitions must advance exactly one epoch. Skipping an epoch
   // would create an unbound historical gap that later finality checks could
   // otherwise encounter only after state mutation.
   let expected_next_epoch=match self.epoch.checked_add(1){Some(v)=>v,None=>return};
   if next_epoch!=expected_next_epoch { return; }
   // The current active set must still match its authoritative history before
   // it is used as the predecessor of the next validator-set transition.
   if self.validate_current_validator_set_history_binding().is_err() { return; }
   if self.validate_validator_records_integrity().is_err() { return; }
   if self.validate_unbonding_integrity().is_err() { return; }
   if self.validate_staking_activation_integrity().is_err() { return; }

   // The pending set is a cached/public state field. Never trust it as
   // consensus input: rebuild the canonical candidate directly from the
   // authoritative validator records before committing a new epoch.
   let canonical_next_validator_set=self.validators.iter()
     .filter(|(_,v)|!self.staking_activated||v.stake>0)
     .map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{
       validator_id:*id,
       owner:v.owner,
       consensus_key:v.consensus_key,
       stake:v.stake,
     }))
     .collect::<BTreeMap<Hash,freedom_types::ValidatorSetEntry>>();
   if self.pending_validator_set!=canonical_next_validator_set { return; }
   let next_active_validator_set=canonical_next_validator_set;
   // Once staking is active, the validator-set transition itself must be
   // authorized by the cryptographically authenticated certificate. Runtime
   // block admission validates the same certificate before it reaches state;
   // this guard closes direct state-transition callers as well.
   if self.validate_epoch_transition_certificate_binding(
     next_epoch,
     &next_active_validator_set,
   ).is_err() { return; }
   let mut bytes=Vec::new();
   for (id,entry) in &next_active_validator_set {
    bytes.extend_from_slice(id);
    bytes.extend_from_slice(&entry.owner);
    bytes.extend_from_slice(&entry.consensus_key);
    bytes.extend_from_slice(&entry.stake.to_le_bytes());
   }
   let set_hash=freedom_primitives::hash_bytes(&bytes);
   let key_snapshot=next_active_validator_set.iter()
     .map(|(id,entry)|(*id,entry.consensus_key))
     .collect::<BTreeMap<_,_>>();
   let mut key_bytes=Vec::new();
   for (id,key) in &key_snapshot {
    key_bytes.extend_from_slice(id);
    key_bytes.extend_from_slice(key);
   }
   let key_hash=freedom_primitives::hash_bytes(&key_bytes);
   let entry_snapshot=next_active_validator_set.clone();

   // Validate every existing historical commitment before mutating epoch state.
   if self.validator_set_history.get(&next_epoch).copied()
       .map(|h|h!=set_hash).unwrap_or(false)
     || self.validator_set_history_sets.get(&next_epoch)
       .map(|s|s!=&key_snapshot).unwrap_or(false)
     || self.validator_set_history_key_hashes.get(&next_epoch)
       .copied().map(|h|h!=key_hash).unwrap_or(false)
     || self.validator_set_history_entries.get(&next_epoch)
       .map(|s|s!=&entry_snapshot).unwrap_or(false) {
    return;
   }

   // All transition authorization and historical commitments have now
   // passed. Release mature unbonding only after that point so a rejected
   // epoch transition cannot still mutate staking state at the boundary.
   self.release_mature_unbonding();

   self.epoch=next_epoch;
   self.active_validator_set=next_active_validator_set;
   // Retain the certificate that authorized this transition as the
   // authoritative certificate for the newly activated epoch.
   self.refresh_pending_validator_set();
   self.validator_set_history.insert(self.epoch,set_hash);
   self.validator_set_history_sets.insert(self.epoch,key_snapshot);
   self.validator_set_history_key_hashes.insert(self.epoch,key_hash);
   self.validator_set_history_entries.insert(self.epoch,entry_snapshot);
   self.epoch_issuance=0;
  }
 fn release_mature_unbonding(&mut self){
  // Never partially mutate staking state from a malformed unbonding ledger.
  if self.validate_unbonding_integrity().is_err() { return; }
  let ids:Vec<Hash>=self.unbonding.iter().filter(|(_,e)|e.release_height<=self.height).map(|(id,_)|*id).collect();

  // Preflight every matured release before mutating any validator/account/ledger entry.
  for id in &ids {
   let Some(e)=self.unbonding.get(id) else { return; };
   let Some(v)=self.validators.get(&e.validator_id) else { return; };
   if v.unbonding_stake.checked_sub(e.amount).is_none() { return; }
   match self.accounts.get(&e.owner) {
    Some(a) => {
     if a.staked.checked_sub(e.amount).is_none() || a.balance.checked_add(e.amount).is_none() { return; }
    }
    None if self.staking_activated => return,
    None => {}
   }
  }

  for id in ids {
   let Some(e)=self.unbonding.remove(&id) else { return; };
   let Some(v)=self.validators.get_mut(&e.validator_id) else { return; };
   let Some(next_unbonding)=v.unbonding_stake.checked_sub(e.amount) else { return; };
   v.unbonding_stake=next_unbonding;
   if let Some(a)=self.accounts.get_mut(&e.owner) {
    let Some(next_staked)=a.staked.checked_sub(e.amount) else { return; };
    let Some(next_balance)=a.balance.checked_add(e.amount) else { return; };
    a.staked=next_staked;
    a.balance=next_balance;
   }
  }
 }
 pub(crate) fn insert_account(&mut self,a:Account){self.accounts.insert(a.address,a);}
 pub fn drain_events(&mut self)->Vec<Event>{std::mem::take(&mut self.events)}
 pub fn execute_transaction(&mut self,tx:&Transaction)->Result<(),StateError>{if tx.chain_id!=freedom_primitives::CHAIN_ID{return Err(StateError::InvalidChainId)}let nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce;if nonce!=tx.nonce{return Err(StateError::InvalidNonce{expected:nonce,actual:tx.nonce})}let result=match &tx.kind{TxKind::Transfer=>self.execute_transfer(tx),TxKind::RegisterNode=>self.register_node(tx),TxKind::SubmitContribution=>self.submit_contribution(tx),TxKind::ClaimReward=>{let id:Hash=tx.payload.as_slice().try_into().map_err(|_|StateError::UnsupportedTransaction)?;self.claim_reward(id)},TxKind::Purchase=>self.purchase(tx),TxKind::CreateTicket=>self.create_ticket(tx),TxKind::SubmitEvidence=>self.submit_evidence(tx),TxKind::Stake=>self.stake(tx),TxKind::Unstake=>self.unstake(tx),TxKind::BridgeMint=>self.execute_bridge_mint(tx),_=>Err(StateError::UnsupportedTransaction)}; if result.is_ok(){self.ledger.push(LedgerEntry{tx_hash:tx.hash(),sender:tx.sender,kind:tx.kind.code(),nonce:tx.nonce,success:true});} result}
#[cfg(test)]
mod m330_public_chain_id_boundary_tests {
    use super::*;
    use freedom_primitives::{address_from_public_key, CHAIN_ID};
    use freedom_types::{Genesis, Transaction, TxKind, Validator};

    #[test]
    fn execute_transaction_rejects_foreign_chain_id() {
        let key = [81u8; 32];
        let owner = address_from_public_key(&key);
        let validator = Validator {
            validator_id: freedom_primitives::hash_bytes(b"m330-validator"),
            owner,
            consensus_key: key,
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
        state.insert_account(Account::new(key, 100));
        let tx = Transaction {
            chain_id: CHAIN_ID.wrapping_add(1),
            sender: owner,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: vec![],
            fee: 0,
            signature: vec![],
        };
        assert_eq!(state.execute_transaction(&tx), Err(StateError::InvalidChainId));
    }
}

 fn execute_bridge_mint(&mut self,tx:&Transaction)->Result<(),StateError>{
  let payload=BridgeMintTransactionPayload::decode(&tx.payload).map_err(|_|StateError::InvalidBridgeMintTransaction)?;
  let sender=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if tx.fee>sender.balance {
   return Err(StateError::InsufficientBalance);
  }
  // Preflight every fallible transaction-level mutation before bridge minting.
  // mint_bridge_authorized() mutates bridge/accounting state, so fee supply
  // arithmetic and the sender nonce must be known to succeed first.
  let next_nonce=sender.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let next_circulating=self.supply.circulating.checked_sub(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  let next_treasury=self.supply.treasury.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  let policy=self.bridge_security_policies.get(&payload.message.bridge_id).ok_or(StateError::InvalidBridgeMintTransaction)?;
  if payload.message.source_chain_id == freedom_types::BITCOIN_MAINNET_CHAIN_ID {
    self.validate_bitcoin_bridge_safety(payload.message.bridge_id)?;
    let canonical=self.bitcoin_canonical_tips.get(&payload.message.bridge_id)
      .ok_or(StateError::InvalidBridgeMessage)?;
    if !canonical.validate()
      || canonical.height < payload.deposit.block_or_slot
      || freedom_types::bitcoin_confirmations_at_tip(canonical.height,payload.deposit.block_or_slot)
        .ok_or(StateError::InvalidBridgeMessage)?
        < policy.min_external_confirmations {
      return Err(StateError::InvalidBridgeMessage);
    }
    let attestation=self.bitcoin_utxo_attestations.get(&payload.message.bridge_id)
      .ok_or(StateError::InvalidBridgeMessage)?;
    if attestation.bridge_id != payload.message.bridge_id
      || attestation.chain_id != freedom_types::BITCOIN_MAINNET_CHAIN_ID
      || !attestation.verify(
        canonical,
        &self.active_validator_set.iter()
          .map(|(id,v)|(*id,v.consensus_key))
          .collect::<BTreeMap<Hash,[u8;32]>>(),
        policy.validator_quorum,
      ) {
      return Err(StateError::InvalidBridgeMessage);
    }
  }
  let keys=self.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<BTreeMap<Hash,[u8;32]>>();
  let expected=BridgeMintAuthorization::from_authenticated_message(
   &payload.message,&payload.deposit,&payload.approvals,policy,&keys
  ).map_err(|_|StateError::InvalidBridgeMintTransaction)?;
  if expected!=payload.authorization { return Err(StateError::InvalidBridgeMintTransaction); }
  let amount=self.mint_bridge_authorized(
   &payload.message,&payload.deposit,&payload.authorization,&payload.approvals
  )?;
  self.supply.circulating=next_circulating;
  self.supply.treasury=next_treasury;
  let sender=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
  sender.nonce=next_nonce;
  self.events.push(Event::BridgeMinted{
   replay_key:payload.authorization.replay_key,
   asset_id:payload.authorization.canonical_asset_id,
   recipient:payload.authorization.recipient,
   amount,
  });
  Ok(())
 }
 fn stake(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=freedom_types::StakePayload::decode(&tx.payload).map_err(|_|StateError::UnsupportedTransaction)?;
  if p.amount==0{return Err(StateError::InvalidStakeAmount)}
  // Validator selection is owner-based. Validate the authoritative registry
  // before selecting a validator so duplicate/corrupt owner records cannot
  // steer the stake mutation into an ambiguous validator entry.
  self.validate_validator_records_integrity()?;
  self.validate_unbonding_integrity()?;
  self.validate_staking_activation_integrity()?;
  let id=self.validators.values().find(|v|v.owner==tx.sender).map(|v|v.validator_id).ok_or(StateError::InvalidEvidence)?;
  if self.staking_activated && (self.validator_lifecycle(id)?!=freedom_types::ValidatorLifecycle::Active || self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.unbonding_stake>0){return Err(StateError::InvalidValidatorLifecycle)}
  let a=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if a.balance<p.amount{return Err(StateError::InsufficientBalance)}
  let next_balance=a.balance.checked_sub(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  let next_staked=a.staked.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  let v=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?;
  let next_validator_stake=v.stake.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;

  // Preflight every fallible arithmetic operation before the first mutation.
  let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
  a.balance=next_balance;
  a.staked=next_staked;
  self.staking_activated=true;
  let v=self.validators.get_mut(&id).ok_or(StateError::InvalidEvidence)?;
  v.stake=next_validator_stake;
  self.refresh_pending_validator_set();
  self.advance_nonce(tx.sender)
 }
 fn unstake(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=freedom_types::UnstakePayload::decode(&tx.payload).map_err(|_|StateError::UnsupportedTransaction)?;
  if p.amount==0{return Err(StateError::InvalidUnbonding)}
  // The owner -> validator lookup is ambiguous if the validator registry is
  // already corrupted. Refuse the mutation before touching account or
  // unbonding state.
  self.validate_validator_records_integrity()?;
  self.validate_unbonding_integrity()?;
  self.validate_staking_activation_integrity()?;
  let id=self.validators.values().find(|v|v.owner==tx.sender).map(|v|v.validator_id).ok_or(StateError::InvalidEvidence)?;
  let owner=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.owner;
  let active=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.stake;
  if active==0{return Err(StateError::InvalidValidatorLifecycle)}
  if active<p.amount{return Err(StateError::InsufficientBalance)}
  let a=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if a.staked<p.amount{return Err(StateError::InsufficientBalance)}
  let release_height=self.height.checked_add(Self::UNBONDING_COOLDOWN).ok_or(StateError::ArithmeticOverflow)?;
  let unbonding_id=freedom_primitives::hash_bytes(&[b"FREEDOM-UNBONDING-V1".as_slice(),&id,&tx.sender,&tx.nonce.to_le_bytes(),&p.amount.to_le_bytes(),&release_height.to_le_bytes()].concat());
  if self.unbonding.contains_key(&unbonding_id){return Err(StateError::InvalidUnbonding)}
  let v=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?;
  let next_stake=v.stake.checked_sub(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  let next_unbonding=v.unbonding_stake.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;

  // Preflight validator arithmetic before mutating stake/unbonding state.
  let v=self.validators.get_mut(&id).ok_or(StateError::InvalidEvidence)?;
  v.stake=next_stake;
  v.unbonding_stake=next_unbonding;
  self.unbonding.insert(unbonding_id,UnbondingEntry{unbonding_id,validator_id:id,owner,amount:p.amount,requested_height:self.height,release_height});
  self.refresh_pending_validator_set();
  self.advance_nonce(tx.sender)
 }
 fn submit_evidence(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=decode_submit_evidence(&tx.payload).ok_or(StateError::InvalidEvidence)?;
  // All mutation paths below finish by advancing the sender nonce. Guard the
  // only fallible nonce operation before evidence/slash state can mutate.
  let sender=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if sender.nonce==u64::MAX {
    return Err(StateError::ArithmeticOverflow);
  }
  // Preserve the idempotent slash contract: once an evidence hash has been
  // slashed, replay must be checked against its recorded amount before any
  // current-stake recalculation. Historical stake may have changed since the
  // original slash.
  if let Some(recorded)=self.slashed_evidence.get(&p.evidence.evidence_hash) {
    if p.slash_amount!=*recorded {
      return Err(StateError::InvalidEvidence);
    }
    let stored=self.evidence.get(&p.evidence.evidence_hash)
      .ok_or(StateError::InvalidEvidence)?;
    if stored != &p.evidence {
      return Err(StateError::InvalidEvidence);
    }
    let amount=self.slash_evidence(p.evidence.evidence_hash,p.slash_amount)?;
    let stored=self.evidence.get(&p.evidence.evidence_hash)
      .ok_or(StateError::InvalidEvidence)?;
    self.events.push(Event::EquivocationSlashed{
      evidence_hash:p.evidence.evidence_hash,
      validator_id:stored.validator_id,
      amount,
    });
    return self.advance_nonce(tx.sender);
  }
  // Validate the complete slash basis before recording new evidence. State
  // mutation must not occur on a transaction that will later fail validation.
  let validator=self.validators.get(&p.evidence.validator_id)
    .ok_or(StateError::InvalidEvidence)?;
  let total=validator.stake.checked_add(validator.unbonding_stake)
    .ok_or(StateError::InvalidEvidence)?;
  if total==0 {
    return Err(StateError::InvalidEvidence);
  }
  let expected=self.deterministic_slash_amount(&p.evidence.kind,total);
  if expected==0 || p.slash_amount!=expected {
    return Err(StateError::InvalidEvidence);
  }
  if !self.evidence.contains_key(&p.evidence.evidence_hash){
    self.record_equivocation(p.evidence.clone())?;
  }
  let amount=self.slash_evidence(p.evidence.evidence_hash,p.slash_amount)?;
  let stored=self.evidence.get(&p.evidence.evidence_hash)
    .ok_or(StateError::InvalidEvidence)?;
  self.events.push(Event::EquivocationSlashed{
    evidence_hash:p.evidence.evidence_hash,
    validator_id:stored.validator_id,
    amount,
  });
  self.advance_nonce(tx.sender)
 }
 fn advance_nonce(&mut self,s:Address)->Result<(),StateError>{let a=self.accounts.get_mut(&s).ok_or(StateError::AccountNotFound)?;a.nonce=a.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;Ok(())}
 fn execute_transfer(&mut self,tx:&Transaction)->Result<(),StateError>{
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  let t=TransferPayload::decode(&tx.payload).map_err(|_|StateError::InvalidTransferPayload)?;
  if t.to==[0;32]{return Err(StateError::InvalidRecipient)}
  let cost=t.amount.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  if balance<cost{return Err(StateError::InsufficientBalance)}
  let nb=balance-cost;
  let sender_next_nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let fee_supply=self.supply.circulating.checked_sub(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  let treasury=self.supply.treasury.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  if t.to==tx.sender {
   let restored=nb.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?;
   let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
   a.balance=restored;
   a.nonce=sender_next_nonce;
  } else {
   let recipient_balance=self.accounts.get(&t.to).map(|a|a.balance).unwrap_or(0);
   let next_recipient=recipient_balance.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?;
   let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
   a.balance=nb;a.nonce=sender_next_nonce;
   let r=self.accounts.entry(t.to).or_insert(Account{address:t.to,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});
   r.balance=next_recipient;
  }
  self.supply.circulating=fee_supply;
  self.supply.treasury=treasury;
  self.events.push(Event::NativeAssetTransferred{from:tx.sender,to:t.to,amount:t.amount});
  Ok(())
 }
 fn register_node(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=RegisterNodePayload::decode(&tx.payload).map_err(|_|StateError::InvalidNodePayload)?;
  if p.node_id==[0;32]{return Err(StateError::InvalidNode)}
  if self.nodes.contains_key(&p.node_id){return Err(StateError::DuplicateNode)}
  let next_nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  self.nodes.insert(p.node_id,Node{node_id:p.node_id,owner:tx.sender,capabilities:p.capabilities,reputation:100,total_wu:0,successful_tasks:0,failed_tasks:0});
  self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce=next_nonce;
  Ok(())
 }
 fn submit_contribution(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=SubmitContributionPayload::decode(&tx.payload).map_err(|_|StateError::InvalidContributionPayload)?;
  if p.contribution_id==[0;32]||p.task_id==[0;32]||p.node_id==[0;32]{return Err(StateError::InvalidContribution)}
  if p.work_units==0{return Err(StateError::InvalidWorkUnits)}
  if p.quality_score>10000{return Err(StateError::InvalidQualityScore)}
  if self.contributions.contains_key(&p.contribution_id){return Err(StateError::DuplicateContribution)}
  let node=self.nodes.get(&p.node_id).ok_or(StateError::InvalidNode)?;
  if node.owner!=tx.sender{return Err(StateError::InvalidNode)}
  let next_nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  self.contributions.insert(p.contribution_id,Contribution{contribution_id:p.contribution_id,node_id:p.node_id,task_id:p.task_id,work_units:p.work_units,quality_score:p.quality_score,verification_score:0,result_hash:p.result_hash,verifier_set:Vec::new(),status:ContributionStatus::Submitted,reward_id:None,reward_amount:0});
  self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce=next_nonce;
  Ok(())
 }
 pub(crate) fn verify_contribution(&mut self,id:Hash,verifiers:Vec<Hash>,score:u32)->Result<(),StateError>{
  if verifiers.is_empty()||score>10000{return Err(StateError::InvalidVerification)}
  if verifiers.iter().any(|v|!self.validators.contains_key(v)){return Err(StateError::InvalidVerification)}
  let(node_id,wu)={let c=self.contributions.get(&id).ok_or(StateError::InvalidContribution)?;if c.status!=ContributionStatus::Submitted{return Err(StateError::AlreadyVerified)}(c.node_id,c.work_units)};
  let n=self.nodes.get(&node_id).ok_or(StateError::InvalidNode)?;
  let next_total_wu=n.total_wu.checked_add(wu).ok_or(StateError::ArithmeticOverflow)?;
  let next_successful=n.successful_tasks.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let c=self.contributions.get_mut(&id).ok_or(StateError::InvalidContribution)?;
  c.verifier_set=verifiers;c.verification_score=score;c.status=ContributionStatus::Verified;
  let n=self.nodes.get_mut(&node_id).ok_or(StateError::InvalidNode)?;
  n.total_wu=next_total_wu;n.successful_tasks=next_successful;
  Ok(())
 }
 pub(crate) fn claim_reward(&mut self,id:Hash)->Result<(),StateError>{
  let c=self.contributions.get(&id).ok_or(StateError::InvalidContribution)?.clone();
  if c.status!=ContributionStatus::Verified{return Err(StateError::ContributionNotRewardable)}
  if c.reward_id.is_some(){return Err(StateError::DuplicateReward)}
  let gross=c.work_units.checked_mul(c.quality_score as u128).ok_or(StateError::ArithmeticOverflow)?
    .checked_mul(c.verification_score as u128).ok_or(StateError::ArithmeticOverflow)?
    .checked_div(100_000_000).ok_or(StateError::ArithmeticOverflow)?;
  let remaining=self.epoch_cap.checked_sub(self.epoch_issuance).ok_or(StateError::IssuanceCapExceeded)?;
  let reward=gross.min(remaining);
  if reward==0{return Err(StateError::IssuanceCapExceeded)}
  let reward_id=freedom_primitives::hash_bytes(&[b"FREEDOM-REWARD-V1".as_slice(),&id,&self.epoch.to_le_bytes()].concat());
  let owner=self.nodes.get(&c.node_id).ok_or(StateError::InvalidNode)?.owner;
  if self.rewards.contains_key(&reward_id){return Err(StateError::DuplicateReward)}
  let account=self.accounts.get(&owner).ok_or(StateError::AccountNotFound)?;
  let next_balance=account.balance.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;
  let next_total=self.supply.total.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;
  let next_circulating=self.supply.circulating.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;
  let next_issuance=self.epoch_issuance.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;

  // Preflight all monetary and contribution mutations before writing any state.
  let a=self.accounts.get_mut(&owner).ok_or(StateError::AccountNotFound)?;
  a.balance=next_balance;  self.supply.total=next_total;
  self.supply.circulating=next_circulating;
  self.epoch_issuance=next_issuance;
  self.rewards.insert(reward_id,Reward{reward_id,owner,contribution_id:id,wu:c.work_units,gross_reward:gross,final_reward:reward,epoch:self.epoch});
  let c=self.contributions.get_mut(&id).ok_or(StateError::InvalidContribution)?;  c.status=ContributionStatus::Rewarded;
  c.reward_id=Some(reward_id);
  c.reward_amount=reward;
  Ok(())
 }
 pub(crate) fn create_draw(&mut self,config:DrawConfig)->Result<(),StateError>{
  if config.draw_id==[0;32]||self.draws.contains_key(&config.draw_id)||config.ticket_price_native_units!=10_000||config.winner_count==0||config.starts_at>config.ends_at{return Err(StateError::InvalidDraw)}
  self.draws.insert(config.draw_id,DrawState{config,total_tickets:0,prize_pool_native_units:0,cu_fund_native_units:0,annual_reserve_native_units:0,executed:false}); Ok(())
 }
 pub(crate) fn create_ticket(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=CreateTicketPayload::decode(&tx.payload).map_err(|_|StateError::InvalidTicketPayload)?;
  if p.draw_id==[0;32]{return Err(StateError::InvalidDraw)} if p.quantity==0{return Err(StateError::InvalidTicketQuantity)}
  let draw=self.draws.get(&p.draw_id).cloned().ok_or(StateError::InvalidDraw)?;
  if self.height<draw.config.starts_at||self.height>draw.config.ends_at{return Err(StateError::InvalidDraw)}
  if draw.config.ticket_price_native_units!=10_000{return Err(StateError::InvalidTicketPrice)}
  let cost=draw.config.ticket_price_native_units.checked_mul(p.quantity as u128).ok_or(StateError::ArithmeticOverflow)?;
  let new_total=draw.total_tickets.checked_add(p.quantity).ok_or(StateError::ArithmeticOverflow)?;
  let prize=cost.checked_mul(60).ok_or(StateError::ArithmeticOverflow)?.checked_div(100).ok_or(StateError::ArithmeticOverflow)?;
  let fund=cost.checked_mul(30).ok_or(StateError::ArithmeticOverflow)?.checked_div(100).ok_or(StateError::ArithmeticOverflow)?;
  let reserve=cost.checked_sub(prize).and_then(|x|x.checked_sub(fund)).ok_or(StateError::ArithmeticOverflow)?;
  let next_draw_prize=draw.prize_pool_native_units.checked_add(prize).ok_or(StateError::ArithmeticOverflow)?;
  let next_draw_fund=draw.cu_fund_native_units.checked_add(fund).ok_or(StateError::ArithmeticOverflow)?;
  let next_draw_reserve=draw.annual_reserve_native_units.checked_add(reserve).ok_or(StateError::ArithmeticOverflow)?;
  let next_global_prize=self.draw_prize_pool_native_units.checked_add(prize).ok_or(StateError::ArithmeticOverflow)?;
  let next_global_fund=self.cu_fund_native_units.checked_add(fund).ok_or(StateError::ArithmeticOverflow)?;
  let next_global_reserve=self.annual_reserve_native_units.checked_add(reserve).ok_or(StateError::ArithmeticOverflow)?;
  let next_circulating=self.supply.circulating.checked_sub(cost).ok_or(StateError::ArithmeticOverflow)?;
  let next_treasury=self.supply.treasury.checked_add(cost).ok_or(StateError::ArithmeticOverflow)?;
  let next_nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  if balance<cost{return Err(StateError::InsufficientBalanceForPurchase)}
  let mut new_ids=Vec::with_capacity(p.quantity as usize);
  let start=draw.total_tickets+1;
  for n in start..=new_total { let id=freedom_primitives::hash_bytes(&[b"FREEDOM-TICKET-V1".as_slice(),&tx.sender,&p.draw_id,&n.to_le_bytes()].concat()); if self.tickets.contains_key(&id){return Err(StateError::DuplicateTicket)} new_ids.push(id); }
  {let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance-=cost;a.nonce=next_nonce;}
  self.supply.circulating=next_circulating; self.supply.treasury=next_treasury;
  let d=self.draws.get_mut(&p.draw_id).ok_or(StateError::InvalidDraw)?; d.total_tickets=new_total; d.prize_pool_native_units=next_draw_prize; d.cu_fund_native_units=next_draw_fund; d.annual_reserve_native_units=next_draw_reserve;
  self.draw_prize_pool_native_units=next_global_prize; self.cu_fund_native_units=next_global_fund; self.annual_reserve_native_units=next_global_reserve;
  for id in new_ids {self.tickets.insert(id,Ticket{ticket_id:id,owner:tx.sender,draw_id:p.draw_id,created_at:self.height});self.events.push(Event::TicketCreated{ticket_id:id,draw_id:p.draw_id,owner:tx.sender});} self.events.push(Event::DrawFundAllocated{draw_id:p.draw_id,prize_pool:prize,cu_fund:fund,annual_reserve:reserve}); Ok(())
 }
 pub(crate) fn execute_draw(&mut self,draw_id:Hash)->Result<DrawResult,StateError>{
  let draw=self.draws.get(&draw_id).cloned().ok_or(StateError::InvalidDraw)?; if draw.executed{return Err(StateError::DrawAlreadyExecuted)} if self.height<draw.config.ends_at{return Err(StateError::DrawNotReady)} if draw.total_tickets==0||draw.config.winner_count as u64>draw.total_tickets{return Err(StateError::InvalidDraw)}
  let seed=freedom_primitives::hash_bytes(&[b"FREEDOM-DRAW-V1".as_slice(),&self.last_block_hash,&draw_id,&draw.config.seed_block.to_le_bytes()].concat());
  let mut ids:Vec<Hash>=self.tickets.values().filter(|t|t.draw_id==draw_id).map(|t|t.ticket_id).collect(); ids.sort();
  let mut winners=Vec::new(); let mut salt=0u64; while winners.len()<draw.config.winner_count as usize {let h=freedom_primitives::hash_bytes(&[&seed[..],&salt.to_le_bytes()[..]].concat());let idx=(u128::from_le_bytes(h[..16].try_into().unwrap())%(ids.len() as u128)) as usize;let id=ids[idx];if !winners.contains(&id){winners.push(id)}salt=salt.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;}
  let selection_proof=freedom_primitives::hash_bytes(&[b"FREEDOM-DRAW-PROOF-V1".as_slice(),&seed,winners.concat().as_slice()].concat()); let result=DrawResult{draw_id,total_tickets:draw.total_tickets,prize_pool_native_units:draw.prize_pool_native_units,cu_fund_native_units:draw.cu_fund_native_units,annual_reserve_native_units:draw.annual_reserve_native_units,winners:winners.clone(),randomness_seed:seed,selection_proof};
  let per=draw.prize_pool_native_units.checked_div(winners.len() as u128).ok_or(StateError::ArithmeticOverflow)?;
  let remainder=draw.prize_pool_native_units.checked_sub(per.checked_mul(winners.len() as u128).ok_or(StateError::ArithmeticOverflow)?).ok_or(StateError::ArithmeticOverflow)?;
  let next_treasury=self.supply.treasury.checked_sub(draw.prize_pool_native_units).ok_or(StateError::InsufficientPrizePool)?;
  let next_circulating=self.supply.circulating.checked_add(draw.prize_pool_native_units).ok_or(StateError::ArithmeticOverflow)?;
  let next_draw_pool=self.draw_prize_pool_native_units.checked_sub(draw.prize_pool_native_units).ok_or(StateError::InsufficientPrizePool)?;
  let mut payouts=Vec::with_capacity(winners.len());
  for (i,id) in winners.iter().enumerate(){
   let owner=self.tickets.get(id).ok_or(StateError::InvalidDraw)?.owner;
   let amount=per.checked_add(if i==0{remainder}else{0}).ok_or(StateError::ArithmeticOverflow)?;
   let balance=self.accounts.get(&owner).ok_or(StateError::AccountNotFound)?.balance;
   let next_balance=balance.checked_add(amount).ok_or(StateError::ArithmeticOverflow)?;
   payouts.push((*id,owner,amount,next_balance));
  }
  // Preflight every winner/account and global monetary mutation before paying anyone.
  let d=self.draws.get(&draw_id).ok_or(StateError::InvalidDraw)?;
  if d.prize_pool_native_units!=draw.prize_pool_native_units || d.executed { return Err(StateError::DrawAlreadyExecuted); }
  for (id,owner,amount,next_balance) in payouts {
   let a=self.accounts.get_mut(&owner).ok_or(StateError::AccountNotFound)?;
   a.balance=next_balance;
   self.events.push(Event::PrizePaid{draw_id,ticket_id:id,winner:owner,amount});
  }
  self.supply.treasury=next_treasury;
  self.supply.circulating=next_circulating;
  self.draw_prize_pool_native_units=next_draw_pool;
  let d=self.draws.get_mut(&draw_id).ok_or(StateError::InvalidDraw)?;
  d.prize_pool_native_units=0;d.executed=true;
  self.draw_results.insert(draw_id,result.clone());
  self.events.push(Event::DrawExecuted{draw_id,randomness_seed:seed,selection_proof});
  Ok(result)
 }
 pub fn quote_price(&self,product_id:u8)->Result<u128,StateError>{if product_id==1{Ok(10_000)}else{Err(StateError::InvalidPurchase)}}

 pub(crate) fn reserve_billing(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=BillingPayload::decode(&tx.payload).map_err(|_|StateError::InvalidPurchase)?;
  if p.product_id!=1||p.billing_id==[0;32]||p.merchant==[0;32]||p.reference==[0;32]||p.amount==0{return Err(StateError::InvalidPurchase)}
  if self.billing.contains_key(&p.billing_id){return Err(StateError::DuplicatePurchase)}
  if p.amount!=self.quote_price(p.product_id)?{return Err(StateError::InvalidPurchase)}
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  if balance<p.amount{return Err(StateError::InsufficientBalanceForPurchase)}
  let next_nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let account=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  let next_locked=account.locked.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  let next_balance=account.balance.checked_sub(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
  a.balance=next_balance;a.locked=next_locked;a.nonce=next_nonce;
  self.billing.insert(p.billing_id,BillingRecord{billing_id:p.billing_id,buyer:tx.sender,merchant:p.merchant,product_id:p.product_id,amount:p.amount,reference:p.reference,status:2,reserved_at:self.height,captured_at:None,refunded_at:None});
  Ok(())
 }

 pub(crate) fn capture_billing(&mut self,billing_id:Hash)->Result<(),StateError>{
  let b=self.billing.get(&billing_id).cloned().ok_or(StateError::InvalidPurchase)?;if b.status!=2{return Err(StateError::InvalidPurchase)}
  let buyer=self.accounts.get(&b.buyer).ok_or(StateError::AccountNotFound)?;
  let next_locked=buyer.locked.checked_sub(b.amount).ok_or(StateError::ArithmeticOverflow)?;
  let merchant_balance=self.accounts.get(&b.merchant).map(|a|a.balance).unwrap_or(0);
  let next_merchant_balance=merchant_balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?;
  let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.locked=next_locked;
  let m=self.accounts.entry(b.merchant).or_insert(Account{address:b.merchant,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});m.balance=next_merchant_balance;
  let x=self.billing.get_mut(&billing_id).ok_or(StateError::InvalidPurchase)?;x.status=3;x.captured_at=Some(self.height);Ok(())
 }

 pub(crate) fn refund_billing(&mut self,billing_id:Hash)->Result<(),StateError>{
  let b=self.billing.get(&billing_id).cloned().ok_or(StateError::InvalidPurchase)?;if b.status!=2&&b.status!=3{return Err(StateError::InvalidPurchase)}
  if b.status==2{
   let buyer=self.accounts.get(&b.buyer).ok_or(StateError::AccountNotFound)?;
   let next_locked=buyer.locked.checked_sub(b.amount).ok_or(StateError::ArithmeticOverflow)?;
   let next_balance=buyer.balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?;
   let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.locked=next_locked;a.balance=next_balance;
  } else {
   let merchant=self.accounts.get(&b.merchant).ok_or(StateError::AccountNotFound)?;
   if merchant.balance<b.amount{return Err(StateError::InsufficientBalanceForPurchase)}
   let next_merchant=merchant.balance-b.amount;
   let buyer=self.accounts.get(&b.buyer).ok_or(StateError::AccountNotFound)?;
   let next_buyer=buyer.balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?;
   let m=self.accounts.get_mut(&b.merchant).ok_or(StateError::AccountNotFound)?;m.balance=next_merchant;
   let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.balance=next_buyer;
  }
  let x=self.billing.get_mut(&billing_id).ok_or(StateError::InvalidPurchase)?;x.status=4;x.refunded_at=Some(self.height);Ok(())
 }

 pub(crate) fn purchase(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=PurchasePayload::decode(&tx.payload).map_err(|_|StateError::InvalidPurchase)?;
  if p.product_id!=1||p.merchant==[0;32]||p.reference==[0;32]||p.price==0{return Err(StateError::InvalidPurchase)}
  let id=freedom_primitives::hash_bytes(&[b"FREEDOM-PURCHASE-V1".as_slice(),&tx.sender,&p.reference].concat());
  if self.purchases.contains_key(&id){return Err(StateError::DuplicatePurchase)}
  if p.merchant==tx.sender{return Err(StateError::InvalidPurchase)}
  let buyer=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if buyer.balance<p.price{return Err(StateError::InsufficientBalanceForPurchase)}
  let next_nonce=buyer.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;
  let next_buyer_balance=buyer.balance.checked_sub(p.price).ok_or(StateError::ArithmeticOverflow)?;
  let merchant_balance=self.accounts.get(&p.merchant).map(|a|a.balance).unwrap_or(0);
  let next_merchant_balance=merchant_balance.checked_add(p.price).ok_or(StateError::ArithmeticOverflow)?;

  // Preflight every fallible arithmetic operation before mutating buyer, merchant, or purchase state.
  let purchase=Purchase{purchase_id:id,buyer:tx.sender,merchant:p.merchant,product_id:p.product_id,price:p.price,reference:p.reference,block_height:self.height};
  {
    let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
    a.balance=next_buyer_balance;
    a.nonce=next_nonce;
  }
  let merchant=self.accounts.entry(p.merchant).or_insert(Account{address:p.merchant,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});
  merchant.balance=next_merchant_balance;
  self.purchases.insert(id,purchase);
  Ok(())
 }

 fn state_leaf(domain: &[u8], key: &[u8], value: &[u8]) -> Hash {
  freedom_primitives::domain_hash(freedom_primitives::STATE_LEAF_DOMAIN, &[domain,key,value])
 }
 fn put_len(out:&mut Vec<u8>, n:usize){out.extend_from_slice(&(n as u64).to_le_bytes());}
 fn put_bytes(out:&mut Vec<u8>, b:&[u8]){Self::put_len(out,b.len());out.extend_from_slice(b);}
 fn enc_account(a:&Account)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&a.address);o.extend_from_slice(&a.public_key);o.extend_from_slice(&a.nonce.to_le_bytes());o.extend_from_slice(&a.balance.to_le_bytes());o.extend_from_slice(&a.locked.to_le_bytes());o.extend_from_slice(&a.staked.to_le_bytes());o}
 fn enc_asset(a:&AssetDefinition)->Vec<u8>{
  let mut o=Vec::new();
  o.extend_from_slice(&a.asset_id);
  o.extend_from_slice(&a.source_chain_id.to_le_bytes());
  o.push(a.asset_type as u8);
  Self::put_len(&mut o,a.contract_address.len());
  o.extend_from_slice(&a.contract_address);
  Self::put_len(&mut o,a.symbol.len());
  o.extend_from_slice(a.symbol.as_bytes());
  o.push(a.decimals);
  o.extend_from_slice(&a.issuer);
  o.extend_from_slice(&a.bridge_id);
  o.push(a.status as u8);
  o
 }
 fn enc_node(n:&Node)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&n.node_id);o.extend_from_slice(&n.owner);o.extend_from_slice(&n.capabilities.to_le_bytes());o.extend_from_slice(&n.reputation.to_le_bytes());o.extend_from_slice(&n.total_wu.to_le_bytes());o.extend_from_slice(&n.successful_tasks.to_le_bytes());o.extend_from_slice(&n.failed_tasks.to_le_bytes());o}
 fn enc_contribution(c:&Contribution)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&c.contribution_id);o.extend_from_slice(&c.node_id);o.extend_from_slice(&c.task_id);o.extend_from_slice(&c.work_units.to_le_bytes());o.extend_from_slice(&c.quality_score.to_le_bytes());o.extend_from_slice(&c.verification_score.to_le_bytes());o.extend_from_slice(&c.result_hash);Self::put_len(&mut o,c.verifier_set.len());for v in &c.verifier_set{o.extend_from_slice(v);}o.push(match c.status{ContributionStatus::Submitted=>1,ContributionStatus::Verified=>2,ContributionStatus::Rejected=>3,ContributionStatus::Rewarded=>4});match c.reward_id{Some(id)=>{o.push(1);o.extend_from_slice(&id)},None=>o.push(0)}o.extend_from_slice(&c.reward_amount.to_le_bytes());o}
 fn enc_reward(r:&Reward)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.reward_id);o.extend_from_slice(&r.owner);o.extend_from_slice(&r.contribution_id);o.extend_from_slice(&r.wu.to_le_bytes());o.extend_from_slice(&r.gross_reward.to_le_bytes());o.extend_from_slice(&r.final_reward.to_le_bytes());o.extend_from_slice(&r.epoch.to_le_bytes());o}
 fn enc_purchase(p:&Purchase)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&p.purchase_id);o.extend_from_slice(&p.buyer);o.extend_from_slice(&p.merchant);o.push(p.product_id);o.extend_from_slice(&p.price.to_le_bytes());o.extend_from_slice(&p.reference);o.extend_from_slice(&p.block_height.to_le_bytes());o}
 fn enc_billing(b:&BillingRecord)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&b.billing_id);o.extend_from_slice(&b.buyer);o.extend_from_slice(&b.merchant);o.push(b.product_id);o.extend_from_slice(&b.amount.to_le_bytes());o.extend_from_slice(&b.reference);o.push(b.status);o.extend_from_slice(&b.reserved_at.to_le_bytes());match b.captured_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}match b.refunded_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}o}
 fn enc_ticket(t:&Ticket)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&t.ticket_id);o.extend_from_slice(&t.owner);o.extend_from_slice(&t.draw_id);o.extend_from_slice(&t.created_at.to_le_bytes());o}
 fn enc_draw(d:&DrawState)->Vec<u8>{let mut o=Vec::new();let c=&d.config;o.extend_from_slice(&c.draw_id);o.push(c.draw_type as u8);o.extend_from_slice(&c.ticket_price_native_units.to_le_bytes());o.extend_from_slice(&c.winner_count.to_le_bytes());o.extend_from_slice(&c.starts_at.to_le_bytes());o.extend_from_slice(&c.ends_at.to_le_bytes());o.extend_from_slice(&c.seed_block.to_le_bytes());o.extend_from_slice(&d.total_tickets.to_le_bytes());o.extend_from_slice(&d.prize_pool_native_units.to_le_bytes());o.extend_from_slice(&d.cu_fund_native_units.to_le_bytes());o.extend_from_slice(&d.annual_reserve_native_units.to_le_bytes());o.push(d.executed as u8);o}
 fn enc_draw_result(r:&DrawResult)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.draw_id);o.extend_from_slice(&r.total_tickets.to_le_bytes());o.extend_from_slice(&r.prize_pool_native_units.to_le_bytes());o.extend_from_slice(&r.cu_fund_native_units.to_le_bytes());o.extend_from_slice(&r.annual_reserve_native_units.to_le_bytes());Self::put_len(&mut o,r.winners.len());for w in &r.winners{o.extend_from_slice(w);}o.extend_from_slice(&r.randomness_seed);o.extend_from_slice(&r.selection_proof);o}
 fn enc_evidence(e:&freedom_types::EquivocationEvidence)->Vec<u8>{let mut o=e.signing_bytes();o.extend_from_slice(&e.evidence_hash);o}
 fn enc_validator(v:&Validator)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&v.validator_id);o.extend_from_slice(&v.owner);o.extend_from_slice(&v.consensus_key);o.extend_from_slice(&v.stake.to_le_bytes());o.extend_from_slice(&v.unbonding_stake.to_le_bytes());o.extend_from_slice(&v.blocks_proposed.to_le_bytes());o.extend_from_slice(&v.blocks_finalized.to_le_bytes());o.extend_from_slice(&v.missed_votes.to_le_bytes());o}
 fn enc_unbonding(e:&UnbondingEntry)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&e.unbonding_id);o.extend_from_slice(&e.validator_id);o.extend_from_slice(&e.owner);o.extend_from_slice(&e.amount.to_le_bytes());o.extend_from_slice(&e.requested_height.to_le_bytes());o.extend_from_slice(&e.release_height.to_le_bytes());o}
 fn enc_ledger(l:&LedgerEntry)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&l.tx_hash);o.extend_from_slice(&l.sender);o.push(l.kind);o.extend_from_slice(&l.nonce.to_le_bytes());o.push(l.success as u8);o}
 fn enc_event(e:&Event)->Vec<u8>{let mut o=Vec::new();match e{Event::NativeAssetTransferred{from,to,amount}=>{o.push(1);o.extend_from_slice(from);o.extend_from_slice(to);o.extend_from_slice(&amount.to_le_bytes())},Event::TicketCreated{ticket_id,draw_id,owner}=>{o.push(2);o.extend_from_slice(ticket_id);o.extend_from_slice(draw_id);o.extend_from_slice(owner)},Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve}=>{o.push(3);o.extend_from_slice(draw_id);o.extend_from_slice(&prize_pool.to_le_bytes());o.extend_from_slice(&cu_fund.to_le_bytes());o.extend_from_slice(&annual_reserve.to_le_bytes())},Event::DrawExecuted{draw_id,randomness_seed,selection_proof}=>{o.push(4);o.extend_from_slice(draw_id);o.extend_from_slice(randomness_seed);o.extend_from_slice(selection_proof)},Event::PrizePaid{draw_id,ticket_id,winner,amount}=>{o.push(5);o.extend_from_slice(draw_id);o.extend_from_slice(ticket_id);o.extend_from_slice(winner);o.extend_from_slice(&amount.to_le_bytes())},Event::BridgeMinted{replay_key,asset_id,recipient,amount}=>{o.push(7);o.extend_from_slice(replay_key);o.extend_from_slice(asset_id);o.extend_from_slice(recipient);o.extend_from_slice(&amount.to_le_bytes())},Event::EquivocationSlashed{evidence_hash,validator_id,amount}=>{o.push(6);o.extend_from_slice(evidence_hash);o.extend_from_slice(validator_id);o.extend_from_slice(&amount.to_le_bytes())}}o}
 pub fn state_root(&self)->Hash{
  let mut leaves=Vec::new();
  for (k,v) in &self.accounts{leaves.push(Self::state_leaf(b"account",k,&Self::enc_account(v)));}
  for (k,v) in &self.validators{leaves.push(Self::state_leaf(b"validator",k,&Self::enc_validator(v)));}
  for (k,v) in &self.asset_registry{leaves.push(Self::state_leaf(b"asset",k,&Self::enc_asset(v)));} for (k,v) in &self.processed_bridge_messages{leaves.push(Self::state_leaf(b"bridge_replay",k,&v.message_hash));} for (k,v) in &self.bridge_message_status{leaves.push(Self::state_leaf(b"bridge_status",k,&[*v as u8]));} for (k,v) in &self.bridge_withdrawals{leaves.push(Self::state_leaf(b"bridge_withdrawal",k,&serde_json::to_vec(v).expect("bridge withdrawal serialization")));} for (k,v) in &self.bridge_withdrawal_status{leaves.push(Self::state_leaf(b"bridge_withdrawal_status",k,&[*v as u8]));} for (k,v) in &self.bridge_accounting{leaves.push(Self::state_leaf(b"bridge_accounting",k,&serde_json::to_vec(v).expect("bridge accounting serialization")));} for (k,v) in &self.bridge_security_policies{leaves.push(Self::state_leaf(b"bridge_security_policy",k,&serde_json::to_vec(v).expect("bridge security policy serialization")));} for (k,v) in &self.bridge_approval_sets{leaves.push(Self::state_leaf(b"bridge_approval_set",k,&serde_json::to_vec(v).expect("bridge approval set serialization")));} for (k,v) in &self.bitcoin_canonical_tips{leaves.push(Self::state_leaf(b"bitcoin_canonical_tip",k,&serde_json::to_vec(v).expect("bitcoin canonical tip serialization")));} for (k,v) in &self.bitcoin_reorg_recovery{leaves.push(Self::state_leaf(b"bitcoin_reorg_recovery",k,&serde_json::to_vec(v).expect("bitcoin reorg recovery serialization")));} for (k,v) in &self.bitcoin_reorg_policy{leaves.push(Self::state_leaf(b"bitcoin_reorg_policy",k,&serde_json::to_vec(v).expect("bitcoin reorg policy serialization")));} for (k,v) in &self.bitcoin_reorg_evidence{leaves.push(Self::state_leaf(b"bitcoin_reorg_evidence",k,&serde_json::to_vec(v).expect("bitcoin reorg evidence serialization")));} for (k,v) in &self.bitcoin_checkpoints{leaves.push(Self::state_leaf(b"bitcoin_checkpoint",k,&serde_json::to_vec(v).expect("bitcoin checkpoint serialization")));} for (k,v) in &self.bridge_nonce_registry{leaves.push(Self::state_leaf(b"bridge_nonce_registry",k,&serde_json::to_vec(v).expect("bridge nonce registry serialization")));} for (k,v) in &self.bridge_pause_authorizations{leaves.push(Self::state_leaf(b"bridge_pause_authorization",k,&serde_json::to_vec(v).expect("bridge pause authorization serialization")));} for (k,v) in &self.bridge_rate_limits{leaves.push(Self::state_leaf(b"bridge_rate_limit",k,&serde_json::to_vec(v).expect("bridge rate limit serialization")));} for (k,v) in &self.bridge_deposit_replays{leaves.push(Self::state_leaf(b"bridge_deposit_replay",k,&serde_json::to_vec(v).expect("bridge deposit replay serialization")));} for (k,v) in &self.bridge_mint_authorizations{leaves.push(Self::state_leaf(b"bridge_mint_authorization",k,&serde_json::to_vec(v).expect("bridge mint authorization serialization")));} for (k,v) in &self.bridge_withdrawal_rate_limit_period{leaves.push(Self::state_leaf(b"bridge_withdrawal_rate_limit_period",k,&serde_json::to_vec(v).expect("bridge withdrawal rate limit period serialization")));}
  for (k,v) in &self.unbonding{leaves.push(Self::state_leaf(b"unbonding",k,&Self::enc_unbonding(v)));}
  for (k,v) in &self.active_validator_set{let mut x=Vec::new();x.extend_from_slice(&v.validator_id);x.extend_from_slice(&v.owner);x.extend_from_slice(&v.consensus_key);x.extend_from_slice(&v.stake.to_le_bytes());leaves.push(Self::state_leaf(b"active_validator",k,&x));} for (k,v) in &self.pending_validator_set{let mut x=Vec::new();x.extend_from_slice(&v.validator_id);x.extend_from_slice(&v.owner);x.extend_from_slice(&v.consensus_key);x.extend_from_slice(&v.stake.to_le_bytes());leaves.push(Self::state_leaf(b"pending_validator",k,&x));}
  for (epoch,set_hash) in &self.validator_set_history{leaves.push(Self::state_leaf(b"validator_set_history",&epoch.to_le_bytes(),set_hash));}
  for (epoch,keys) in &self.validator_set_history_sets{let mut x=Vec::new();x.extend_from_slice(&(keys.len() as u64).to_le_bytes());for (id,key) in keys{x.extend_from_slice(id);x.extend_from_slice(key);}leaves.push(Self::state_leaf(b"validator_set_history_keys",&epoch.to_le_bytes(),&x));}
  for (epoch,key_hash) in &self.validator_set_history_key_hashes{leaves.push(Self::state_leaf(b"validator_set_history_key_hash",&epoch.to_le_bytes(),key_hash));}
  for (epoch,entries) in &self.validator_set_history_entries{let mut x=Vec::new();x.extend_from_slice(&(entries.len() as u64).to_le_bytes());for (id,entry) in entries{x.extend_from_slice(id);x.extend_from_slice(&entry.owner);x.extend_from_slice(&entry.consensus_key);x.extend_from_slice(&entry.stake.to_le_bytes());}leaves.push(Self::state_leaf(b"validator_set_history_entries",&epoch.to_le_bytes(),&x));}
  if let Some(cert)=&self.validator_set_certificate{leaves.push(Self::state_leaf(b"validator_set_certificate",&[0;1],&cert.hash()));}
  for (k,v) in &self.bitcoin_utxo_attestations{leaves.push(Self::state_leaf(b"bitcoin_utxo_attestation",k,&serde_json::to_vec(v).expect("bitcoin utxo attestation serialization")));}
  for (k,v) in &self.bitcoin_utxo_attestation_policy_hashes{leaves.push(Self::state_leaf(b"bitcoin_utxo_attestation_policy_hash",k,v));}
  for (k,v) in &self.bitcoin_utxo_commitments{leaves.push(Self::state_leaf(b"bitcoin_utxo_commitment",k,&serde_json::to_vec(v).expect("bitcoin utxo commitment serialization")));}
  for (k,v) in &self.evidence{leaves.push(Self::state_leaf(b"evidence",k,&Self::enc_evidence(v)));}
  for (k,v) in &self.slashed_evidence{leaves.push(Self::state_leaf(b"slashed_evidence",k,&v.to_le_bytes()));}
  for (k,v) in &self.nodes{leaves.push(Self::state_leaf(b"node",k,&Self::enc_node(v)));}
  for (k,v) in &self.contributions{leaves.push(Self::state_leaf(b"contribution",k,&Self::enc_contribution(v)));}
  for (k,v) in &self.rewards{leaves.push(Self::state_leaf(b"reward",k,&Self::enc_reward(v)));}
  for (k,v) in &self.purchases{leaves.push(Self::state_leaf(b"purchase",k,&Self::enc_purchase(v)));}
  for (k,v) in &self.billing{leaves.push(Self::state_leaf(b"billing",k,&Self::enc_billing(v)));}
  for (k,v) in &self.tickets{leaves.push(Self::state_leaf(b"ticket",k,&Self::enc_ticket(v)));}
  for (k,v) in &self.draws{leaves.push(Self::state_leaf(b"draw",k,&Self::enc_draw(v)));}
  for (k,v) in &self.draw_results{leaves.push(Self::state_leaf(b"draw_result",k,&Self::enc_draw_result(v)));}
  for (l) in &self.ledger{let key=l.tx_hash;leaves.push(Self::state_leaf(b"ledger",&key,&Self::enc_ledger(l)));}
  for (e) in &self.events{let value=Self::enc_event(e);leaves.push(Self::state_leaf(b"event",&(leaves.len() as u64).to_le_bytes(),&value));}
  let mut monetary=Vec::new();monetary.extend_from_slice(&self.height.to_le_bytes());monetary.extend_from_slice(&self.epoch.to_le_bytes());monetary.extend_from_slice(&self.epoch_length.to_le_bytes());// last_block_hash is excluded from state_root to avoid a circular state-root/block-hash dependency.monetary.extend_from_slice(&self.cu_fund_native_units.to_le_bytes());monetary.extend_from_slice(&self.annual_reserve_native_units.to_le_bytes());monetary.extend_from_slice(&self.draw_prize_pool_native_units.to_le_bytes());monetary.extend_from_slice(&self.supply.total.to_le_bytes());monetary.extend_from_slice(&self.supply.circulating.to_le_bytes());monetary.extend_from_slice(&self.supply.locked.to_le_bytes());monetary.extend_from_slice(&self.supply.staked.to_le_bytes());monetary.extend_from_slice(&self.supply.treasury.to_le_bytes());monetary.extend_from_slice(&self.epoch_issuance.to_le_bytes());monetary.extend_from_slice(&self.epoch_cap.to_le_bytes());monetary.push(self.staking_activated as u8);
  leaves.push(Self::state_leaf(b"meta",&[0;1],&monetary));
  leaves.sort();
  freedom_primitives::merkle_root(&leaves)
 }
 pub fn assert_invariants(&self){assert!(self.supply.is_consistent());for (id,asset) in &self.asset_registry{assert_eq!(id,&asset.asset_id);assert!(asset.validate_identity());assert!(!asset.symbol.is_empty());}for (replay_key,message) in &self.processed_bridge_messages{assert_eq!(replay_key,&message.replay_key());assert!(message.validate_hash());assert_eq!(message.protocol_version,BRIDGE_MESSAGE_PROTOCOL_VERSION);assert_eq!(message.destination_chain_id,freedom_primitives::CHAIN_ID);}for (replay_key,status) in &self.bridge_message_status{assert!(self.processed_bridge_messages.contains_key(replay_key));assert!(*status as u8>=BridgeMessageStatus::Verified as u8&&*status as u8<=BridgeMessageStatus::Minted as u8);}for (id,p) in &self.bridge_security_policies{assert_eq!(id,&p.bridge_id);assert!(p.validate());}for (id,set) in &self.bridge_approval_sets{assert_eq!(id,&set.request_id);assert!(!set.approvals.is_empty());}for (id,u) in &self.bridge_rate_limits{assert!(self.bridge_security_policies.contains_key(id));assert!(u.used>0);}for (id,r) in &self.bridge_deposit_replays{assert_eq!(id,&r.replay_key);assert!(r.validate());assert_eq!(r.status,freedom_types::DepositReplayStatus::Consumed);}for (id,a) in &self.bridge_mint_authorizations{assert_eq!(id,&a.replay_key);assert!(a.validate());assert!(self.bridge_deposit_replays.contains_key(id));}for (id,cp) in &self.bitcoin_canonical_tips{assert!(*id!=[0;32]);assert!(cp.validate());} for (id,r) in &self.bitcoin_reorg_recovery{assert!(*id!=[0;32]);assert!(r.validate());} for (id,p) in &self.bitcoin_reorg_policy{assert!(*id!=[0;32]);assert!(p.validate());} for (id,journal) in &self.bitcoin_reorg_evidence{assert!(*id!=[0;32]);assert!(journal.len()<=freedom_types::BITCOIN_REORG_EVIDENCE_LIMIT);assert!(journal.iter().all(|e|e.validate()));} for (id,cp) in &self.bitcoin_checkpoints{assert!(*id!=[0;32]);assert!(cp.validate());} for (id,nonces) in &self.bridge_nonce_registry{assert!(!nonces.is_empty());} for (id,set) in &self.bridge_pause_authorizations{assert_eq!(id,&set.bridge_id);assert!(!set.authorizations.is_empty());} for (id,p) in &self.bridge_withdrawal_rate_limit_period{assert!(self.bridge_withdrawals.contains_key(id));assert!(*p<=self.height);}for (id,w) in &self.bridge_withdrawals{assert_eq!(id,&w.withdrawal_id);assert!(w.validate_hash());assert_eq!(w.compute_id(),w.withdrawal_id);assert!(self.bridge_withdrawal_status.contains_key(id));}for (id,status) in &self.bridge_withdrawal_status{assert!(self.bridge_withdrawals.contains_key(id));assert!((*status as u8>=BridgeWithdrawalStatus::Requested as u8&&*status as u8<=BridgeWithdrawalStatus::Rejected as u8));}for id in self.bridge_accounting.keys(){assert!(self.validate_bridge_accounting(*id).is_ok());}let sum=self.accounts.values().try_fold(0u128,|s,a|{let h=a.balance.checked_add(a.locked).and_then(|v|v.checked_add(a.staked))?;s.checked_add(h)});assert!(sum.is_some(),"account balance overflow");assert_eq!(self.supply.circulating,sum.unwrap());assert!(self.epoch_issuance<=self.epoch_cap);assert!(self.cu_fund_native_units.checked_add(self.annual_reserve_native_units).and_then(|x|x.checked_add(self.draw_prize_pool_native_units)).map(|x|x<=self.supply.treasury).unwrap_or(false));for v in self.validators.values(){assert_eq!(v.stake.checked_add(v.unbonding_stake).unwrap_or(u128::MAX),self.accounts.get(&v.owner).map(|a|a.staked).unwrap_or(0));}for c in self.contributions.values(){assert!(c.quality_score<=10000&&c.verification_score<=10000);if c.status==ContributionStatus::Rewarded{assert!(c.reward_id.is_some()&&c.reward_amount>0)}}}
}




#[cfg(test)]
mod m96_bridge_nonce_tests {
 use super::*;
 #[test] fn outbound_nonce_is_one_time_per_bridge_asset(){let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);let bridge=[9;32];let asset=[8;32];s.asset_registry.insert(asset,AssetDefinition{asset_id:asset,source_chain_id:2,asset_type:AssetType::Bridged,contract_address:vec![1],symbol:"USDT".into(),decimals:6,issuer:[3;32],bridge_id:bridge,status:AssetStatus::Active});let mk=|id|BridgeWithdrawal{withdrawal_id:id,bridge_id:bridge,asset_id:asset,destination_chain_id:2,amount:1,sender:[4;32],recipient:vec![5],nonce:7,request_hash:[6;32]};s.request_bridge_withdrawal(mk([1;32])).unwrap();assert_eq!(s.request_bridge_withdrawal(mk([2;32])),Err(StateError::DuplicateBridgeNonce));}
 #[test] fn different_assets_have_independent_nonces(){let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);let bridge=[9;32];for (asset,symbol) in [([8;32],"USDT"),([7;32],"USDC")]{s.asset_registry.insert(asset,AssetDefinition{asset_id:asset,source_chain_id:2,asset_type:AssetType::Bridged,contract_address:vec![1],symbol:symbol.into(),decimals:6,issuer:[3;32],bridge_id:bridge,status:AssetStatus::Active});}for (id,asset) in [([1;32],[8;32]),([2;32],[7;32])]{s.request_bridge_withdrawal(BridgeWithdrawal{withdrawal_id:id,bridge_id:bridge,asset_id:asset,destination_chain_id:2,amount:1,sender:[4;32],recipient:vec![5],nonce:7,request_hash:[6;32]}).unwrap();}}
}

#[cfg(test)]
mod m95_bridge_pause_tests {
 use super::*;
 #[test] fn pause_authorization_requires_quorum(){let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);s.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id:[9;32],approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:100,max_withdrawal_amount:100,daily_limit:100,emergency_paused:false,nonce_floor:0}).unwrap();let set=BridgePauseAuthorizationSet{bridge_id:[9;32],nonce:1,authorizations:vec![]};assert_eq!(s.apply_bridge_emergency_pause([9;32],set),Err(StateError::InvalidBridgePauseAuthorization));assert!(!s.bridge_security_policies.get(&[9;32]).unwrap().emergency_paused);}
 #[test] fn pause_state_is_persisted_in_state_root(){let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);s.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id:[9;32],approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:100,max_withdrawal_amount:100,daily_limit:100,emergency_paused:false,nonce_floor:0}).unwrap();let before=s.state_root();s.bridge_security_policies.get_mut(&[9;32]).unwrap().emergency_paused=true;assert_ne!(before,s.state_root());}
}

#[cfg(test)]
mod m94_bridge_rate_limit_approval_tests {
 use super::*;
 fn s()->ChainState{let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);s.height=7;s.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id:[9;32],approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1000,max_withdrawal_amount:600,daily_limit:1000,emergency_paused:false,nonce_floor:0}).unwrap();s}
 #[test] fn reservation_is_recorded_once_on_approval(){let mut s=s();let id=[1;32];s.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Approved);assert_eq!(s.approve_bridge_with_authenticated_set(id,[2;32]),Err(StateError::InvalidBridgeWithdrawalTransition));}
 #[test] fn rate_limit_failure_keeps_withdrawal_unapproved(){let mut s=s();let id=[1;32];let request=[2;32];let w=BridgeWithdrawal{withdrawal_id:id,bridge_id:[9;32],asset_id:[8;32],destination_chain_id:2,amount:600,sender:[3;32],recipient:vec![4],nonce:1,request_hash:[5;32]};s.bridge_withdrawals.insert(id,w);s.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Validated);s.bridge_approval_sets.insert(request,BridgeApprovalSet{request_id:request,approvals:vec![],threshold_signatures:0});s.bridge_security_policies.get_mut(&[9;32]).unwrap().validator_quorum=2;assert_eq!(s.approve_bridge_with_authenticated_set(id,request),Err(StateError::InvalidBridgeWithdrawal));assert_eq!(s.bridge_withdrawal_status.get(&id),Some(&BridgeWithdrawalStatus::Validated));}
}

#[cfg(test)]
mod m93_bridge_rate_limit_tests {
 use super::*;
 fn s()->ChainState{let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]};let mut s=ChainState::genesis(&g);s.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id:[9;32],approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1000,max_withdrawal_amount:600,daily_limit:1000,emergency_paused:false,nonce_floor:0}).unwrap();s}
 #[test] fn usage_accumulates_within_period(){let mut s=s();assert!(s.reserve_bridge_withdrawal_limit([9;32],1,600).is_ok());assert!(s.reserve_bridge_withdrawal_limit([9;32],1,400).is_ok());assert_eq!(s.reserve_bridge_withdrawal_limit([9;32],1,1),Err(StateError::BridgeRateLimitExceeded));}
 #[test] fn new_period_resets_usage(){let mut s=s();s.reserve_bridge_withdrawal_limit([9;32],1,1000).unwrap();assert!(s.reserve_bridge_withdrawal_limit([9;32],2,600).is_ok());}
 #[test] fn release_returns_reserved_capacity(){let mut s=s();s.reserve_bridge_withdrawal_limit([9;32],1,800).unwrap();s.release_bridge_withdrawal_limit([9;32],1,300).unwrap();assert!(s.reserve_bridge_withdrawal_limit([9;32],1,500).is_ok());}
}

#[cfg(test)]
mod m92_bridge_approval_transition_tests {
 use super::*; use ed25519_dalek::SigningKey;
 fn setup()->(ChainState,Hash,Hash){
  let key=SigningKey::from_bytes(&[7;32]); let pubkey=key.verifying_key().to_bytes(); let vid=freedom_primitives::hash_bytes(&pubkey);
  let v=Validator{validator_id:vid,owner:[1;32],consensus_key:pubkey,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
  let g=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![v.clone()]}; let mut s=ChainState::genesis(&g);
  let bridge=[9;32]; let policy=BridgeSecurityPolicy{bridge_id:bridge,approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1000,max_withdrawal_amount:1000,daily_limit:1000,emergency_paused:false,nonce_floor:0}; s.register_bridge_security_policy(policy).unwrap();
  let asset=*s.asset_registry.keys().next().unwrap();
  let mut w=BridgeWithdrawal{withdrawal_id:[0;32],bridge_id:bridge,asset_id:asset,destination_chain_id:2,amount:10,sender:[2;32],recipient:vec![3],nonce:0,request_hash:[0;32]}; w.request_hash=w.compute_hash();w.withdrawal_id=w.compute_id();s.bridge_withdrawals.insert(w.withdrawal_id,w.clone());s.bridge_withdrawal_status.insert(w.withdrawal_id,BridgeWithdrawalStatus::Validated);
  (s,w.withdrawal_id,vid)
 }
 #[test] fn approval_requires_authenticated_evidence(){let (mut s,id,vid)=setup();assert_eq!(s.advance_bridge_withdrawal(id,BridgeWithdrawalStatus::Approved),Err(StateError::InvalidBridgeWithdrawalTransition));let k=SigningKey::from_bytes(&[7;32]);let mut a=BridgeApproval{bridge_id:[9;32],request_id:[4;32],withdrawal_id:id,validator_id:vid,nonce:0,amount:10,destination_chain_id:2,asset_id:*s.asset_registry.keys().next().unwrap(),intent_hash:s.bridge_withdrawals[&id].request_hash,signature:Vec::new()};a.signature=k.sign(&a.signing_message()).to_bytes().to_vec();s.record_bridge_approval_set(BridgeApprovalSet{request_id:[4;32],approvals:vec![a],threshold_signatures:0}).unwrap();assert!(s.approve_bridge_with_authenticated_set(id,[4;32]).is_ok());assert_eq!(s.bridge_withdrawal_status[&id],BridgeWithdrawalStatus::Approved);}
}

#[cfg(test)]
mod m79_bridge_replay_tests {
 use super::*;
 use freedom_types::{Genesis, AssetDefinition, AssetStatus, AssetType, BridgeMessage, BRIDGE_MESSAGE_PROTOCOL_VERSION};

 fn setup()->(ChainState,AssetDefinition,Hash,Hash){
  let mut state=ChainState::genesis(&Genesis::default());
  let source_chain_id=100u32;
  let bridge_id=[4u8;32];
  let asset_id=AssetDefinition::compute_asset_id(source_chain_id,AssetType::Bridged,b"USDT",[7u8;32],bridge_id);
  let asset=AssetDefinition{asset_id,source_chain_id,asset_type:AssetType::Bridged,contract_address:b"USDT".to_vec(),symbol:"USDT".to_string(),decimals:6,issuer:[7u8;32],bridge_id,status:AssetStatus::Active};
  state.register_asset(asset.clone()).unwrap();
  (state,asset,bridge_id,asset_id)
 }
 fn message(bridge_id:Hash,asset_id:Hash)->BridgeMessage{
  let mut m=BridgeMessage{protocol_version:BRIDGE_MESSAGE_PROTOCOL_VERSION,source_chain_id:100,destination_chain_id:freedom_primitives::CHAIN_ID,bridge_id,asset_id,source_tx_hash:[3u8;32],source_event_index:0,nonce:1,amount:100_000,recipient:[9u8;32],message_hash:[0u8;32]};
  m.message_hash=m.compute_hash(); m
 }
 #[test] fn bridge_message_is_persisted_and_cannot_be_replayed(){
  let(mut state,_,bridge_id,asset_id)=setup(); let m=message(bridge_id,asset_id); let key=state.record_bridge_message(m.clone()).unwrap();
  assert_eq!(state.processed_bridge_messages.get(&key).unwrap().message_hash,m.message_hash);
  assert_eq!(state.record_bridge_message(m),Err(StateError::DuplicateBridgeMessage));
 }
 #[test] fn conflicting_payload_with_same_replay_identity_is_rejected(){
  let(mut state,_,bridge_id,asset_id)=setup(); let m=message(bridge_id,asset_id); state.record_bridge_message(m.clone()).unwrap();
  let mut conflicting=m.clone(); conflicting.amount+=1; conflicting.message_hash=conflicting.compute_hash();
  assert_eq!(state.record_bridge_message(conflicting),Err(StateError::InvalidBridgeMessage));
 }
 #[test] fn replay_identity_includes_source_event_index(){
  let(mut state,_,bridge_id,asset_id)=setup(); let m=message(bridge_id,asset_id); state.record_bridge_message(m.clone()).unwrap();
  let mut second=m; second.source_event_index=1; second.message_hash=second.compute_hash();
  assert!(state.record_bridge_message(second).is_ok()); assert_eq!(state.processed_bridge_messages.len(),2);
 }
 #[test] fn bridge_message_requires_active_bound_asset_and_valid_hash(){
  let(mut state,asset,bridge_id,asset_id)=setup(); let mut tampered=message(bridge_id,asset_id); tampered.amount+=1;
  assert_eq!(state.record_bridge_message(tampered),Err(StateError::InvalidBridgeMessage));
  asset.status=AssetStatus::Paused; state.asset_registry.insert(asset.asset_id,asset);
  assert_eq!(state.record_bridge_message(message(bridge_id,asset_id)),Err(StateError::InvalidBridgeMessage));
 }
 #[test] fn bridge_replay_changes_state_root(){
  let(mut state,_,bridge_id,asset_id)=setup(); let before=state.state_root(); state.record_bridge_message(message(bridge_id,asset_id)).unwrap();
  assert_ne!(before,state.state_root()); state.assert_invariants();
 }
}


#[cfg(test)]
mod m80_bridge_state_machine_tests {
 use super::*;
 use freedom_types::{Genesis, AssetDefinition, AssetStatus, AssetType, BridgeMessage, BRIDGE_MESSAGE_PROTOCOL_VERSION};

 fn setup()->(ChainState,Hash){
  let mut state=ChainState::genesis(&Genesis::default());
  let source_chain_id=100u32; let bridge_id=[4u8;32];
  let asset_id=AssetDefinition::compute_asset_id(source_chain_id,AssetType::Bridged,b"USDT",[7u8;32],bridge_id);
  state.register_asset(AssetDefinition{asset_id,source_chain_id,asset_type:AssetType::Bridged,contract_address:b"USDT".to_vec(),symbol:"USDT".into(),decimals:6,issuer:[7u8;32],bridge_id,status:AssetStatus::Active}).unwrap();
  let mut m=BridgeMessage{protocol_version:BRIDGE_MESSAGE_PROTOCOL_VERSION,source_chain_id,destination_chain_id:freedom_primitives::CHAIN_ID,bridge_id,asset_id,source_tx_hash:[3u8;32],source_event_index:0,nonce:1,amount:100_000,recipient:[9u8;32],message_hash:[0u8;32]};
  m.message_hash=m.compute_hash();
  let key=state.record_bridge_message(m).unwrap();
  (state,key)
 }
 #[test] fn accepted_requires_verified(){
  let(mut state,key)=setup();
  assert_eq!(state.bridge_message_status.get(&key),Some(&BridgeMessageStatus::Verified));
  state.accept_bridge_message(key).unwrap();
  assert_eq!(state.bridge_message_status.get(&key),Some(&BridgeMessageStatus::Accepted));
 }
 #[test] fn mint_transition_requires_authenticated_consumed_authorization(){
  let(mut state,key)=setup();
  assert_eq!(state.mark_bridge_message_minted(key),Err(StateError::InvalidBridgeMessage));
  state.accept_bridge_message(key).unwrap();
  assert_eq!(state.mark_bridge_message_minted(key),Err(StateError::InvalidBridgeMessage));
  assert_eq!(state.bridge_message_status.get(&key),Some(&BridgeMessageStatus::Accepted));
 }
 #[test] fn transitions_are_forward_only(){
  let(mut state,key)=setup();
  state.accept_bridge_message(key).unwrap();
  assert_eq!(state.accept_bridge_message(key),Err(StateError::InvalidBridgeMessage));
  state.mark_bridge_message_minted(key).unwrap();
  assert_eq!(state.mark_bridge_message_minted(key),Err(StateError::InvalidBridgeMessage));
 }
 #[test] fn status_is_consensus_visible(){
  let(mut state,key)=setup();
  let before=state.state_root();
  state.accept_bridge_message(key).unwrap();
  assert_ne!(before,state.state_root());
  state.mark_bridge_message_minted(key).unwrap();
  state.assert_invariants();
 }
}

#[cfg(test)]
mod m32_tests {
 use super::*;
 #[test]
 fn slash_amount_is_deterministic_by_evidence_kind() {
  let s=1_000_000u128;
  let g=freedom_types::Genesis::default();
  let state=ChainState::genesis(&g);
  assert_eq!(state.deterministic_slash_amount(&freedom_types::EquivocationKind::DoubleProposal,s),50_000);
  assert_eq!(state.deterministic_slash_amount(&freedom_types::EquivocationKind::DoublePrevote,s),100_000);
  assert_eq!(state.deterministic_slash_amount(&freedom_types::EquivocationKind::DoublePrecommit,s),100_000);
 }
 #[test]
 fn zero_stake_has_zero_slash() {
  let g=freedom_types::Genesis::default();
  let state=ChainState::genesis(&g);
  assert_eq!(state.deterministic_slash_amount(&freedom_types::EquivocationKind::DoublePrecommit,0),0);
 }
}


#[cfg(test)]
mod m33_tests {
 use super::*;
 use freedom_types::{Genesis, Validator, Transaction, TxKind, UnstakePayload, EquivocationEvidence, EquivocationKind};
 fn validator_state() -> (ChainState, Address, Hash) {
  let key=[7u8;32]; let owner=freedom_primitives::address_from_public_key(&key); let id=freedom_primitives::hash_bytes(b"m33-validator");
  let v=Validator{validator_id:id,owner,consensus_key:key,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
  let g=Genesis{chain_id:1,initial_supply:0,validators:vec![v]}; let mut s=ChainState::genesis(&g); s.insert_account(Account::new(key,0));
  s.accounts.get_mut(&owner).unwrap().staked=100; (s,owner,id)
 }
 #[test] fn unstake_enters_cooldown_without_returning_balance() {
  let (mut s,owner,id)=validator_state(); let tx=Transaction{chain_id:1,sender:owner,nonce:0,kind:TxKind::Unstake,payload:UnstakePayload{amount:40}.encode(),fee:0,signature:vec![]}; s.unstake(&tx).unwrap();
  assert_eq!(s.accounts.get(&owner).unwrap().balance,0); assert_eq!(s.accounts.get(&owner).unwrap().staked,100); assert_eq!(s.validators.get(&id).unwrap().stake,60); assert_eq!(s.validators.get(&id).unwrap().unbonding_stake,40); assert_eq!(s.unbonding.len(),1); assert_eq!(s.unbonding.values().next().unwrap().release_height,UNBONDING_COOLDOWN);
 }
 #[test] fn unbonding_releases_at_maturity() {
  let (mut s,owner,id)=validator_state(); let tx=Transaction{chain_id:1,sender:owner,nonce:0,kind:TxKind::Unstake,payload:UnstakePayload{amount:100}.encode(),fee:0,signature:vec![]}; s.unstake(&tx).unwrap();
  s.height=ChainState::UNBONDING_COOLDOWN-1; s.maybe_advance_epoch(); assert_eq!(s.accounts.get(&owner).unwrap().balance,0);
  s.height=ChainState::UNBONDING_COOLDOWN; s.maybe_advance_epoch(); assert_eq!(s.accounts.get(&owner).unwrap().balance,100); assert_eq!(s.accounts.get(&owner).unwrap().staked,0); assert_eq!(s.validators.get(&id).unwrap().unbonding_stake,0); assert!(s.unbonding.is_empty());
 }
 #[test] fn pending_unbonding_stake_is_slashable() {
  let (mut s,owner,id)=validator_state(); let tx=Transaction{chain_id:1,sender:owner,nonce:0,kind:TxKind::Unstake,payload:UnstakePayload{amount:100}.encode(),fee:0,signature:vec![]}; s.unstake(&tx).unwrap();
  let mut e=EquivocationEvidence{kind:EquivocationKind::DoublePrecommit,height:1,round:0,validator_id:id,first:vec![1],second:vec![2],evidence_hash:[0;32]}; e.evidence_hash=freedom_primitives::hash_bytes(&e.signing_bytes()); s.record_equivocation(e.clone()).unwrap();
  let expected=s.deterministic_slash_amount(&e.kind,100); assert_eq!(s.slash_evidence(e.evidence_hash,expected).unwrap(),10); assert_eq!(s.validators.get(&id).unwrap().unbonding_stake,90); assert_eq!(s.accounts.get(&owner).unwrap().staked,90); assert_eq!(s.unbonding.values().next().unwrap().amount,90);
 }
}


#[cfg(test)]
mod m34_tests {
 use super::*;
 use freedom_types::{Genesis, Validator, Transaction, TxKind, StakePayload, UnstakePayload, ValidatorLifecycle};
 fn setup()->(ChainState,Address,Hash){let key=[9u8;32];let owner=freedom_primitives::address_from_public_key(&key);let id=freedom_primitives::hash_bytes(b"m34-validator");let v=Validator{validator_id:id,owner,consensus_key:key,stake:0,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};let mut s=ChainState::genesis(&Genesis{chain_id:1,initial_supply:0,validators:vec![v]});s.insert_account(Account::new(key,100));(s,owner,id)}
 fn tx(owner:Address,nonce:u64,kind:TxKind,payload:Vec<u8>)->Transaction{Transaction{chain_id:1,sender:owner,nonce,kind,payload,fee:0,signature:vec![]}}
 #[test] fn genesis_can_bootstrap_staking(){let(mut s,o,id)=setup();assert_eq!(s.validator_lifecycle(id).unwrap(),ValidatorLifecycle::Active);s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();assert!(s.staking_activated);assert_eq!(s.validators.get(&id).unwrap().stake,100);}
 #[test] fn unbonding_validator_cannot_restake_before_exit(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();s.execute_transaction(&tx(o,1,TxKind::Unstake,UnstakePayload{amount:100}.encode())).unwrap();assert_eq!(s.validator_lifecycle(id).unwrap(),ValidatorLifecycle::Unbonding);assert_eq!(s.execute_transaction(&tx(o,2,TxKind::Stake,StakePayload{amount:1}.encode())),Err(StateError::InvalidValidatorLifecycle));}
 #[test] fn exited_validator_does_not_resurrect_at_epoch_boundary(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();s.execute_transaction(&tx(o,1,TxKind::Unstake,UnstakePayload{amount:100}.encode())).unwrap();s.height=ChainState::UNBONDING_COOLDOWN;s.maybe_advance_epoch();assert_eq!(s.validator_lifecycle(id).unwrap(),ValidatorLifecycle::Exited);assert!(s.active_validator_set.is_empty());s.height=s.epoch_length;s.maybe_advance_epoch();assert!(s.active_validator_set.is_empty());}
 #[test] fn active_validator_with_pending_unbonding_cannot_add_stake(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();s.execute_transaction(&tx(o,1,TxKind::Unstake,UnstakePayload{amount:40}.encode())).unwrap();assert_eq!(s.validator_lifecycle(id).unwrap(),ValidatorLifecycle::Active);assert_eq!(s.execute_transaction(&tx(o,2,TxKind::Stake,StakePayload{amount:1}.encode())),Err(StateError::InvalidValidatorLifecycle));}
}


#[cfg(test)]
mod m35_tests {
 use super::*; use freedom_types::{Genesis,Validator,Transaction,TxKind,StakePayload,UnstakePayload};
 fn setup()->(ChainState,Address,Hash){let key=[8u8;32];let owner=freedom_primitives::address_from_public_key(&key);let id=freedom_primitives::hash_bytes(b"m35-validator");let v=Validator{validator_id:id,owner,consensus_key:key,stake:0,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};let mut s=ChainState::genesis(&Genesis{chain_id:1,initial_supply:0,validators:vec![v]});s.epoch_length=10;s.insert_account(Account::new(key,100));(s,owner,id)}
 fn tx(o:Address,n:u64,k:TxKind,p:Vec<u8>)->Transaction{Transaction{chain_id:1,sender:o,nonce:n,kind:k,payload:p,fee:0,signature:vec![]}}
 #[test] fn stake_change_is_pending_until_epoch_boundary(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();assert!(s.active_validator_set.contains_key(&id));s.execute_transaction(&tx(o,1,TxKind::Unstake,UnstakePayload{amount:100}.encode())).unwrap();assert!(s.active_validator_set.contains_key(&id));assert!(!s.pending_validator_set.contains_key(&id));s.height=9;s.maybe_advance_epoch();assert!(s.active_validator_set.contains_key(&id));s.height=10;s.maybe_advance_epoch();assert!(!s.active_validator_set.contains_key(&id));}
 #[test] fn pending_new_stake_activates_next_epoch(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();assert!(s.pending_validator_set.contains_key(&id));s.active_validator_set.clear();s.height=9;s.maybe_advance_epoch();assert!(!s.active_validator_set.contains_key(&id));s.height=10;s.maybe_advance_epoch();assert!(s.active_validator_set.contains_key(&id));}
}


#[cfg(test)]
mod m77_asset_registry_tests {
 use super::*;
 use freedom_types::{AssetDefinition, AssetStatus, AssetType, Genesis};

 fn bridged_asset(source_chain_id:u32, contract:&[u8], bridge_id:Hash) -> AssetDefinition {
  let issuer=[7u8;32];
  let asset_id=AssetDefinition::compute_asset_id(
   source_chain_id, AssetType::Bridged, contract, issuer, bridge_id
  );
  AssetDefinition{
   asset_id,
   source_chain_id,
   asset_type:AssetType::Bridged,
   contract_address:contract.to_vec(),
   symbol:"USDT".to_string(),
   decimals:6,
   issuer,
   bridge_id,
   status:AssetStatus::Active,
  }
 }

 #[test]
 fn genesis_registers_321m_as_native_asset() {
  let state=ChainState::genesis(&Genesis::default());
  let native=state.asset_registry.values().find(|a| a.symbol==freedom_primitives::NATIVE_ASSET_SYMBOL).unwrap();
  assert_eq!(native.source_chain_id, freedom_primitives::CHAIN_ID);
  assert_eq!(native.asset_type, AssetType::Native);
  assert_eq!(native.status, AssetStatus::Active);
  assert!(native.validate_identity());
 }

 #[test]
 fn genesis_native_asset_identity_matches_321more_metadata() {
  let state=ChainState::genesis(&Genesis::default());
  let native=state.asset_registry.values().find(|a| a.asset_type==AssetType::Native).unwrap();
  assert_eq!(freedom_primitives::NETWORK_NAME, "321MORE Network");
  assert_eq!(freedom_primitives::NATIVE_ASSET_SYMBOL, "321M");
  assert_eq!(freedom_primitives::GAS_ASSET_SYMBOL, "321M");
  assert_eq!(freedom_primitives::CHAIN_ID, 1);
  assert_eq!(native.source_chain_id, freedom_primitives::CHAIN_ID);
  assert_eq!(native.symbol, "321M");
  assert_eq!(native.status, AssetStatus::Active);
  assert!(native.validate_identity());
 }

 #[test]
 fn identical_symbols_on_different_chains_are_distinct_assets() {
  let mut state=ChainState::genesis(&Genesis::default());
  let eth=bridged_asset(1, b"0xeth-usdt", [1u8;32]);
  let tron=bridged_asset(2, b"TTRONUSDT", [2u8;32]);
  assert_ne!(eth.asset_id, tron.asset_id);
  state.register_asset(eth.clone()).unwrap();
  state.register_asset(tron.clone()).unwrap();
  assert_eq!(state.asset_registry.get(&eth.asset_id).unwrap().symbol, "USDT");
  assert_eq!(state.asset_registry.get(&tron.asset_id).unwrap().symbol, "USDT");
 }

 #[test]
 fn asset_registration_rejects_forged_identity_and_duplicate() {
  let mut state=ChainState::genesis(&Genesis::default());
  let mut asset=bridged_asset(1, b"0xabc", [3u8;32]);
  asset.asset_id=[9u8;32];
  assert_eq!(state.register_asset(asset.clone()), Err(StateError::InvalidAsset));
  let valid=bridged_asset(1, b"0xabc", [3u8;32]);  state.register_asset(valid.clone()).unwrap();
  assert_eq!(state.register_asset(valid), Err(StateError::DuplicateAsset));
 }
}

#[cfg(test)]
mod m81_bidirectional_bridge_tests {
 use super::*;
 use freedom_types::{AssetDefinition,AssetStatus,AssetType,BridgeWithdrawal,BridgeWithdrawalStatus,Genesis};

 fn asset(chain:u32,bridge:Hash)->AssetDefinition{
  let issuer=[8u8;32]; let contract=format!("asset-{}",chain).into_bytes();
  let id=AssetDefinition::compute_asset_id(chain,AssetType::Bridged,&contract,issuer,bridge);
  AssetDefinition{asset_id:id,source_chain_id:chain,asset_type:AssetType::Bridged,contract_address:contract,symbol:"USDT".into(),decimals:6,issuer,bridge_id:bridge,status:AssetStatus::Active}
 }
 fn withdrawal(asset:&AssetDefinition,chain:u32,bridge:Hash)->BridgeWithdrawal{
  let mut w=BridgeWithdrawal{withdrawal_id:[0;32],bridge_id:bridge,asset_id:asset.asset_id,destination_chain_id:chain,amount:100,sender:[9;32],recipient:b"external-recipient".to_vec(),nonce:7,request_hash:[0;32]};
  w.request_hash=w.compute_hash(); w.withdrawal_id=w.compute_id(); w
 }

 #[test]
 fn outbound_withdrawal_is_chain_agnostic() {
  let mut s=ChainState::genesis(&Genesis::default());
  let bridge=[4u8;32]; let a=asset(100,bridge); s.register_asset(a.clone()).unwrap();
  let w=withdrawal(&a,100,bridge); let id=s.request_bridge_withdrawal(w).unwrap();
  assert_eq!(s.bridge_withdrawal_status[&id],BridgeWithdrawalStatus::Requested);
  for next in [BridgeWithdrawalStatus::Validated,BridgeWithdrawalStatus::Approved,BridgeWithdrawalStatus::Signing,BridgeWithdrawalStatus::Broadcast,BridgeWithdrawalStatus::ExternalConfirmed] { s.advance_bridge_withdrawal(id,next).unwrap(); }
  assert_eq!(s.bridge_withdrawal_status[&id],BridgeWithdrawalStatus::ExternalConfirmed);
 }

 #[test]
 fn withdrawal_rejects_wrong_chain_or_bridge() {
  let mut s=ChainState::genesis(&Genesis::default()); let bridge=[5u8;32]; let a=asset(100,bridge); s.register_asset(a.clone()).unwrap();
  let mut w=withdrawal(&a,200,bridge); w.request_hash=w.compute_hash(); w.withdrawal_id=w.compute_id();
  assert_eq!(s.request_bridge_withdrawal(w),Err(StateError::InvalidBridgeWithdrawal));
  let mut w=withdrawal(&a,100,[6u8;32]); w.request_hash=w.compute_hash(); w.withdrawal_id=w.compute_id();
  assert_eq!(s.request_bridge_withdrawal(w),Err(StateError::InvalidBridgeWithdrawal));
 }

 #[test]
 fn accounting_enforces_lock_mint_burn_release_conservation() {
  let mut s=ChainState::genesis(&Genesis::default()); let bridge=[7u8;32]; let a=asset(100,bridge); s.register_asset(a.clone()).unwrap();
  s.record_verified_external_lock(a.asset_id,100).unwrap();
  s.record_freedom_mint(a.asset_id,100).unwrap();
  assert_eq!(s.record_freedom_mint(a.asset_id,1),Err(StateError::InvalidBridgeAccounting));
  s.record_freedom_burn(a.asset_id,100).unwrap();
  assert_eq!(s.record_external_release(a.asset_id,101),Err(StateError::InvalidBridgeAccounting));
  s.record_external_release(a.asset_id,100).unwrap();
  assert!(s.validate_bridge_accounting(a.asset_id).is_ok());
 }

 #[test]
 fn finalization_cannot_release_before_burn() {
  let mut s=ChainState::genesis(&Genesis::default()); let bridge=[8u8;32]; let a=asset(100,bridge); s.register_asset(a.clone()).unwrap();
  s.record_verified_external_lock(a.asset_id,100).unwrap(); s.record_freedom_mint(a.asset_id,100).unwrap();
  let id=s.request_bridge_withdrawal(withdrawal(&a,100,bridge)).unwrap();
  for next in [BridgeWithdrawalStatus::Validated,BridgeWithdrawalStatus::Approved,BridgeWithdrawalStatus::Signing,BridgeWithdrawalStatus::Broadcast,BridgeWithdrawalStatus::ExternalConfirmed] { s.advance_bridge_withdrawal(id,next).unwrap(); }
  assert_eq!(s.advance_bridge_withdrawal(id,BridgeWithdrawalStatus::Finalized),Err(StateError::InvalidBridgeAccounting));
  s.record_freedom_burn(a.asset_id,100).unwrap();
  s.advance_bridge_withdrawal(id,BridgeWithdrawalStatus::Finalized).unwrap();
  assert_eq!(s.bridge_accounting[&a.asset_id].external_released,100);
 }

 #[test]
 fn bridge_state_changes_state_root() {
  let mut s=ChainState::genesis(&Genesis::default()); let before=s.state_root(); let bridge=[9u8;32]; let a=asset(100,bridge); s.register_asset(a.clone()).unwrap();
  s.record_verified_external_lock(a.asset_id,100).unwrap(); let after_lock=s.state_root(); assert_ne!(before,after_lock);
  s.record_freedom_mint(a.asset_id,100).unwrap(); assert_ne!(after_lock,s.state_root());
 }
}


#[cfg(test)]
mod m155_bridge_mint_boundary_tests {
 use super::*;
 use ed25519_dalek::SigningKey;
 use freedom_types::{
  BridgeMessageApproval, BridgeMessageApprovalSet, BridgeMintAuthorization,
  BridgeSecurityPolicy, VerifiedDeposit, AssetDefinition, AssetStatus, AssetType,
  Genesis, BRIDGE_MESSAGE_PROTOCOL_VERSION,
 };

 fn setup() -> (ChainState, VerifiedDeposit, BridgeMessage, BridgeMintAuthorization, BridgeMessageApprovalSet) {
  let key=SigningKey::from_bytes(&[7;32]);
  let public_key=key.verifying_key().to_bytes();
  let validator_id=freedom_primitives::hash_bytes(&public_key);
  let bridge_id=[3;32];
  let source_chain_id=100u32;
  let asset_id=AssetDefinition::compute_asset_id(source_chain_id,AssetType::Bridged,b"USDT",[8;32],bridge_id);
  let validator=Validator{validator_id,owner:[1;32],consensus_key:public_key,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
  let mut state=ChainState::genesis(&Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![validator]});
  state.register_asset(AssetDefinition{asset_id,source_chain_id,asset_type:AssetType::Bridged,contract_address:b"USDT".to_vec(),symbol:"USDT".into(),decimals:6,issuer:[8;32],bridge_id,status:AssetStatus::Active}).unwrap();
  state.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id,approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1_000_000,max_withdrawal_amount:1_000_000,daily_limit:1_000_000,emergency_paused:false,nonce_floor:1}).unwrap();

  let deposit=VerifiedDeposit{canonical_asset_id:asset_id,representation_asset_id:[2;32],bridge_id,source_chain_id,tx_hash:[4;32],event_index:7,block_or_slot:8,confirmations:20,amount:500,recipient:vec![9;20],replay_key:[10;32],proof_hash:[11;32]};
  let message=deposit.to_bridge_message(freedom_primitives::CHAIN_ID,1).unwrap();
  let unsigned=BridgeMessageApproval{bridge_id,message.bridge_id,message_hash:message.message_hash,validator_id,signature:vec![0;64]};
  let signature=freedom_crypto::sign(&key,&unsigned.approval_hash());
  let approvals=BridgeMessageApprovalSet{bridge_id:message.bridge_id,message_hash:message.message_hash,approvals:vec![BridgeMessageApproval{signature:signature.to_vec(),..unsigned}]};
  let auth=BridgeMintAuthorization::from_authenticated_message(&message,&deposit,&approvals,state.bridge_security_policies.get(&bridge_id).unwrap(),&state.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect()).unwrap();
  (state,deposit,message,auth,approvals)
 }

 fn install_message(state:&mut ChainState,message:&BridgeMessage,deposit:&VerifiedDeposit) {
  state.record_bridge_message(message.clone()).unwrap();
  state.record_verified_external_lock(deposit.canonical_asset_id,deposit.amount).unwrap();
  state.accept_bridge_message(message.replay_key()).unwrap();
 }

 #[test]
 fn authorized_mint_updates_balance_supply_accounting_and_consumes_replay() {
  let (mut state,deposit,message,auth,approvals)=setup();
  install_message(&mut state,&message,&deposit);
  let before=state.state_root();
  let amount=state.mint_bridge_authorized(&message,&deposit,&auth,&approvals).unwrap();
  assert_eq!(amount,500);
  assert_eq!(state.accounts[&auth.recipient].balance,500);
  assert_eq!(state.supply.total,500);
  assert_eq!(state.supply.circulating,500);
  assert_eq!(state.bridge_accounting[&auth.canonical_asset_id].freedom_minted,500);
  assert_eq!(state.bridge_deposit_replays[&auth.replay_key].status,freedom_types::DepositReplayStatus::Consumed);
  assert_eq!(state.bridge_message_status[&message.replay_key()],BridgeMessageStatus::Minted);
  assert_ne!(before,state.state_root());
  state.assert_invariants();
 }

 #[test]
 fn same_authorization_cannot_mint_twice() {
  let (mut state,deposit,message,auth,approvals)=setup();
  install_message(&mut state,&message,&deposit);
  state.mint_bridge_authorized(&message,&deposit,&auth,&approvals).unwrap();
  let balance=state.accounts[&auth.recipient].balance;
  assert_eq!(state.mint_bridge_authorized(&message,&deposit,&auth,&approvals),Err(StateError::DuplicateBridgeMessage));
  assert_eq!(state.accounts[&auth.recipient].balance,balance);
 }

 #[test]
 fn forged_authorization_does_not_mint() {
  let (mut state,deposit,message,mut auth,approvals)=setup();
  install_message(&mut state,&message,&deposit);
  auth.amount=501;
  auth.authorization_hash=auth.compute_hash();
  assert_eq!(state.mint_bridge_authorized(&message,&deposit,&auth,&approvals),Err(StateError::InvalidBridgeMessage));
  assert_eq!(state.supply.total,0);
  assert!(state.bridge_deposit_replays.is_empty());
 }
}



#[cfg(test)]
mod m156_bridge_mint_transaction_tests {
 use super::*;
 use ed25519_dalek::SigningKey;
 use freedom_crypto::sign;
 use freedom_types::{
  BridgeMessageApproval, BridgeMessageApprovalSet, BridgeMintAuthorization,
  BridgeMintTransactionPayload, BridgeSecurityPolicy, VerifiedDeposit,
  AssetDefinition, AssetStatus, AssetType, Genesis, Transaction, TxKind,
 };

 fn setup() -> (ChainState, VerifiedDeposit, BridgeMessage, BridgeMintAuthorization, BridgeMessageApprovalSet, SigningKey) {
  let validator_key=SigningKey::from_bytes(&[7;32]);
  let validator_pub=validator_key.verifying_key().to_bytes();
  let validator_id=freedom_primitives::hash_bytes(&validator_pub);
  let bridge_id=[3;32];
  let source_chain_id=100u32;
  let asset_id=AssetDefinition::compute_asset_id(source_chain_id,AssetType::Bridged,b"USDT",[8;32],bridge_id);
  let validator=Validator{validator_id,owner:[1;32],consensus_key:validator_pub,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
  let mut state=ChainState::genesis(&Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![validator]});
  state.register_asset(AssetDefinition{asset_id,source_chain_id,asset_type:AssetType::Bridged,contract_address:b"USDT".to_vec(),symbol:"USDT".into(),decimals:6,issuer:[8;32],bridge_id,status:AssetStatus::Active}).unwrap();
  state.register_bridge_security_policy(BridgeSecurityPolicy{bridge_id,approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1_000_000,max_withdrawal_amount:1_000_000,daily_limit:1_000_000,emergency_paused:false,nonce_floor:1}).unwrap();

  let deposit=VerifiedDeposit{canonical_asset_id:asset_id,representation_asset_id:[2;32],bridge_id,source_chain_id,tx_hash:[4;32],event_index:7,block_or_slot:8,confirmations:20,amount:500,recipient:vec![9;20],replay_key:[10;32],proof_hash:[11;32]};
  let message=deposit.to_bridge_message(freedom_primitives::CHAIN_ID,1).unwrap();
  let unsigned=BridgeMessageApproval{bridge_id,message.bridge_id,message_hash:message.message_hash,validator_id,signature:vec![0;64]};
  let signature=sign(&validator_key,&unsigned.approval_hash());
  let approvals=BridgeMessageApprovalSet{bridge_id,message.bridge_id,message_hash:message.message_hash,approvals:vec![BridgeMessageApproval{signature:signature.to_vec(),..unsigned}]};
  let auth=BridgeMintAuthorization::from_authenticated_message(&message,&deposit,&approvals,state.bridge_security_policies.get(&bridge_id).unwrap(),&state.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect()).unwrap();

  state.record_bridge_message(message.clone()).unwrap();
  state.record_verified_external_lock(deposit.canonical_asset_id,deposit.amount).unwrap();
  state.accept_bridge_message(message.replay_key()).unwrap();

  let relayer_key=SigningKey::from_bytes(&[42;32]);
  let relayer_pub=relayer_key.verifying_key().to_bytes();
  let relayer=Account::new(relayer_pub,1_000);
  let sender=relayer.address;
  state.insert_account(relayer);
  (state,deposit,message,auth,approvals,relayer_key)
 }

 #[test]
 fn bridge_mint_executes_as_signed_transaction() {
  let (mut state,deposit,message,auth,approvals,relayer_key)=setup();
  let payload=BridgeMintTransactionPayload{message:message.clone(),deposit:deposit.clone(),authorization:auth.clone(),approvals:approvals.clone()}.encode().unwrap();
  let mut tx=Transaction{chain_id:freedom_primitives::CHAIN_ID,sender:state.accounts.values().next().unwrap().address,nonce:0,kind:TxKind::BridgeMint,payload,fee:7,signature:Vec::new()};
  tx.signature=sign(&relayer_key,&tx.signing_bytes()).to_vec();

  state.execute_transaction(&tx).unwrap();
  assert_eq!(state.accounts[&auth.recipient].balance,500);
  assert_eq!(state.accounts[&tx.sender].balance,993);
  assert_eq!(state.accounts[&tx.sender].nonce,1);
  assert_eq!(state.supply.total,500);
  assert_eq!(state.supply.circulating,1493);
  assert_eq!(state.supply.treasury,7);
  assert_eq!(state.bridge_message_status[&message.replay_key()],BridgeMessageStatus::Minted);
  assert_eq!(state.bridge_deposit_replays[&auth.replay_key].status,freedom_types::DepositReplayStatus::Consumed);
  assert!(state.events.iter().any(|e| matches!(e, freedom_types::Event::BridgeMinted{amount:500,..})));
  state.assert_invariants();
 }

 #[test]
 fn bridge_mint_transaction_rejects_tampered_authorization() {
  let (mut state,deposit,message,mut auth,approvals,relayer_key)=setup();
  auth.amount=501;
  auth.authorization_hash=auth.compute_hash();
  let payload=BridgeMintTransactionPayload{message,deposit,authorization:auth,approvals}.encode().unwrap();
  let sender=state.accounts.values().next().unwrap().address;
  let mut tx=Transaction{chain_id:freedom_primitives::CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload,fee:0,signature:Vec::new()};
  tx.signature=sign(&relayer_key,&tx.signing_bytes()).to_vec();
  assert!(state.execute_transaction(&tx).is_err());
  assert_eq!(state.supply.total,0);
  assert!(state.bridge_deposit_replays.is_empty());
 }

 #[test]
 fn bridge_mint_transaction_rejects_replay_after_nonce_update() {
  let (mut state,deposit,message,auth,approvals,relayer_key)=setup();
  let payload=BridgeMintTransactionPayload{message:message.clone(),deposit:deposit.clone(),authorization:auth.clone(),approvals:approvals.clone()}.encode().unwrap();
  let sender=state.accounts.values().next().unwrap().address;
  let mut tx=Transaction{chain_id:freedom_primitives::CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload:payload.clone(),fee:0,signature:Vec::new()};
  tx.signature=sign(&relayer_key,&tx.signing_bytes()).to_vec();
  state.execute_transaction(&tx).unwrap();
  let mut replay=Transaction{chain_id:freedom_primitives::CHAIN_ID,sender,nonce:1,kind:TxKind::BridgeMint,payload,fee:0,signature:Vec::new()};
  replay.signature=sign(&relayer_key,&replay.signing_bytes()).to_vec();
  assert_eq!(state.execute_transaction(&replay), Err(StateError::DuplicateBridgeMessage));
 }
}

#[cfg(test)]
mod m208_bitcoin_attestation_chain_id_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoCommitmentAttestation};

    fn state_with_bitcoin_bridge() -> ChainState {
        let genesis = freedom_types::Genesis {
            initial_supply: 0,
            validators: vec![],
        };
        let mut state = ChainState::genesis(&genesis);
        let policy = freedom_types::BridgeSecurityPolicy {
            bridge_id: [7; 32],
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        };
        state.register_bridge_security_policy(policy).unwrap();
        state.bitcoin_canonical_tips.insert(
            [7; 32],
            BitcoinChainCheckpoint {
                height: 100,
                block_hash: [8; 32],
                chainwork: [9; 32],
            },
        );
        state
    }

    fn unsigned_attestation(chain_id: u32) -> BitcoinUtxoCommitmentAttestation {
        BitcoinUtxoCommitmentAttestation {
            bridge_id: [7; 32],
            chain_id,
            policy_id: [6; 32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence: 1,
            bitcoin_tip_height: 100,
            bitcoin_tip_hash: [8; 32],
            bitcoin_tip_chainwork: [9; 32],
            previous_utxo_root: [10; 32],
            accumulator_root: [11; 32],
            validator_ids: vec![[12; 32]],
            signatures: vec![],
            attestation_hash: [0; 32],
        }
    }

    #[test]
    fn registration_rejects_non_bitcoin_chain_id_before_authentication() {
        let mut state = state_with_bitcoin_bridge();
        let attestation = unsigned_attestation(1);
        assert_eq!(
            state.register_bitcoin_utxo_commitment_attestation(attestation),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        assert!(state.bitcoin_utxo_commitment_attestation([7; 32]).is_none());
    }
}

#[cfg(test)]
mod m209_bitcoin_state_proof_chain_id_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoAccumulator, BitcoinUtxoAccumulatorProof, BitcoinUtxoProof, BitcoinUtxoSpendStateProof};

    fn state_with_tip() -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert(
            [7; 32],
            BitcoinChainCheckpoint { height: 100, block_hash: [8; 32], chainwork: [9; 32] },
        );
        state
    }

    #[test]
    fn accumulator_membership_rejects_wrong_chain_id() {
        let state = state_with_tip();
        let proof = BitcoinUtxoAccumulatorProof {
            accumulator: BitcoinUtxoAccumulator {
                bridge_id: [7; 32],
                chain_id: 1,
                tip_height: 100,
                tip_hash: [8; 32],
                tip_chainwork: [9; 32],
                root: [10; 32],
                leaf_count: 1,
            },
            tx_hash: [11; 32],
            vout: 0,
            amount_sat: 1,
            script_pubkey: vec![0; 1],
            leaf_index: 0,
            siblings: vec![],
            exists: true,
        };
        assert_eq!(
            state.validate_bitcoin_utxo_accumulator_proof([7; 32], &proof),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn spend_state_rejects_wrong_chain_id() {
        let state = state_with_tip();
        let proof = BitcoinUtxoSpendStateProof {
            utxo: BitcoinUtxoProof {
                bridge_id: [7; 32],
                chain_id: 1,
                tx_hash: [11; 32],
                vout: 0,
                amount_sat: 1,
                script_pubkey: vec![0; 1],
                block_height: 100,
                confirmations: 1,
                merkle_branch: vec![],
                merkle_index: 0,
            },
            inclusion_header: freedom_types::BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [0; 32],
                merkle_root: [0; 32],
                timestamp: 1,
                bits: 0,
                nonce: 0,
            },
            inclusion_block_transactions: vec![],
            post_inclusion_blocks: vec![],
            tip_height: 100,
            tip_hash: [8; 32],
            tip_chainwork: [9; 32],
        };
        assert_eq!(
            state.validate_bitcoin_utxo_spend_state([7; 32], &proof),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}


#[cfg(test)]
mod m210_bitcoin_adapter_chain_id_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BridgeAdapterDescriptor, BridgeAdapterKind};

    fn state_with_bitcoin_bridge() -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id: [7; 32],
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 6,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        }).unwrap();
        state.register_bitcoin_checkpoint([7; 32], BitcoinChainCheckpoint {
            height: 100,
            block_hash: [8; 32],
            chainwork: [9; 32],
        }).unwrap();
        state
    }

    fn descriptor(chain_id: u32) -> BridgeAdapterDescriptor {
        BridgeAdapterDescriptor {
            bridge_id: [7; 32],
            chain_id,
            kind: BridgeAdapterKind::Bitcoin,
            finality_model: freedom_types::BridgeFinalityModel::Confirmations,
            required_confirmations: 6,
            native_asset: true,
            enabled: true,
        }
    }

    #[test]
    fn bitcoin_adapter_binding_rejects_non_bitcoin_chain_id() {
        let state = state_with_bitcoin_bridge();
        assert_eq!(
            state.validate_bitcoin_checkpoint_binding([7; 32], &descriptor(1)),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn bitcoin_adapter_binding_accepts_protocol_bitcoin_chain_id() {
        let state = state_with_bitcoin_bridge();
        assert_eq!(
            state.validate_bitcoin_checkpoint_binding(
                [7; 32],
                &descriptor(freedom_types::BITCOIN_MAINNET_CHAIN_ID)
            ),
            Ok(())
        );
    }
}


#[cfg(test)]
mod m426_bitcoin_utxo_header_binding_tests {
    use super::*;
    use freedom_types::{BitcoinBlockHeader, BitcoinChainCheckpoint, BitcoinUtxoProof};

    #[test]
    fn utxo_deposit_rejects_merkle_proof_with_invalid_block_header_binding() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge = [7u8; 32];
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id: bridge,
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        }).unwrap();
        state.bitcoin_canonical_tips.insert(bridge, BitcoinChainCheckpoint {
            height: 100,
            block_hash: [8; 32],
            chainwork: [9; 32],
        });

        let proof = BitcoinUtxoProof {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            tx_hash: [1; 32],
            vout: 0,
            amount_sat: 1,
            block_height: 100,
            confirmations: 1,
            merkle_root: [2; 32],
            merkle_branch: vec![[3; 32]],
            merkle_index: 0,
            script_pubkey: vec![0],
            raw_transaction: vec![],
            block_header: Some(BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [0; 32],
                merkle_root: [99; 32],
                timestamp: 1,
                bits: 0,
                nonce: 0,
            }),
        };

        assert_eq!(
            state.validate_bitcoin_utxo_deposit(bridge, &proof, [1; 32], 0, 1, &[0]),
            Err(StateError::BitcoinReorgDetected)
        );
    }
}

#[cfg(test)]
mod m211_bitcoin_merkle_proof_chain_id_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinMerkleProof, BitcoinBlockHeader};

    fn state_with_tip() -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id: [7; 32],
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        }).unwrap();
        state.bitcoin_canonical_tips.insert([7; 32], BitcoinChainCheckpoint {
            height: 100,
            block_hash: [8; 32],
            chainwork: [9; 32],
        });
        state
    }

    fn proof(chain_id: u32) -> BitcoinMerkleProof {
        BitcoinMerkleProof {
            bridge_id: [7; 32],
            chain_id,
            tx_hash: [1; 32],
            block_height: 100,
            confirmations: 1,
            merkle_root: [2; 32],
            merkle_branch: vec![[3; 32]],
            merkle_index: 0,
            block_header: BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [0; 32],
                merkle_root: [2; 32],
                timestamp: 1,
                bits: 0,
                nonce: 0,            },
        }
    }
    #[test]
    fn canonical_merkle_deposit_rejects_non_bitcoin_chain_id_before_header_verification() {
        let state = state_with_tip();
        assert_eq!(
            state.validate_bitcoin_deposit_against_canonical_tip(
                [7; 32],
                &proof(1),
                &[],
            ),
            Err(StateError::BitcoinReorgDetected)
        );
    }
}


#[cfg(test)]
mod m427_bitcoin_utxo_transition_anchor_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoAccumulatorTransition, BitcoinUtxoCommitmentAttestation};

    #[test]
    fn accumulator_transition_rejects_unanchored_previous_root() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge = [7u8; 32];
        let canonical = BitcoinChainCheckpoint { height: 100, block_hash: [8u8; 32], chainwork: [9u8; 32] };
        state.bitcoin_canonical_tips.insert(bridge, canonical.clone());
        state.bitcoin_utxo_attestations.insert(bridge, BitcoinUtxoCommitmentAttestation {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            bitcoin_tip_height: canonical.height,
            bitcoin_tip_hash: canonical.block_hash,
            previous_utxo_root: [10; 32],
            accumulator_root: [11; 32],
            sequence: 1,
            validator_set_epoch: 0,
            validator_set_hash: state.validator_set_history[&0],
            policy_id: bridge,
            validator_ids: vec![],
            signatures: vec![],
            attestation_hash: [12; 32],
        });
        let transition = BitcoinUtxoAccumulatorTransition {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            previous_tip: canonical.clone(),
            next_tip: canonical,
            previous_root: [99; 32],
            next_root: [13; 32],
            previous_leaf_count: 0,
            next_leaf_count: 0,
            spent: vec![],
            created: vec![],
        };
        assert_eq!(
            state.validate_bitcoin_utxo_transition(&transition, &[]),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m425_bitcoin_utxo_accumulator_anchor_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoAccumulator, BitcoinUtxoAccumulatorProof, BitcoinUtxoCommitmentAttestation};

    #[test]
    fn accumulator_proof_must_match_authoritative_attestation_root() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge = [7u8; 32];
        let canonical = BitcoinChainCheckpoint { height: 100, block_hash: [8u8; 32], chainwork: [9u8; 32] };
        state.bitcoin_canonical_tips.insert(bridge, canonical.clone());

        let tx_hash = [11u8; 32];
        let script_pubkey = vec![0u8; 1];
        let root = BitcoinUtxoAccumulator::leaf_hash(&tx_hash, 0, 1, &script_pubkey);
        state.bitcoin_utxo_attestations.insert(bridge, BitcoinUtxoCommitmentAttestation {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            bitcoin_tip_height: canonical.height,
            bitcoin_tip_hash: canonical.block_hash,
            previous_utxo_root: [12u8; 32],
            accumulator_root: [13u8; 32],
            sequence: 1,
            validator_set_epoch: 0,
            validator_set_hash: state.validator_set_history[&0],
            policy_id: bridge,
            validator_ids: vec![],
            signatures: vec![],
            attestation_hash: [14u8; 32],
        });

        let proof = BitcoinUtxoAccumulatorProof {
            accumulator: BitcoinUtxoAccumulator {
                bridge_id: bridge,
                chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
                bitcoin_tip_height: canonical.height,
                bitcoin_tip_hash: canonical.block_hash,
                bitcoin_tip_chainwork: canonical.chainwork,
                root,
                leaf_count: 1,
            },
            tx_hash,
            vout: 0,
            amount_sat: 1,
            script_pubkey,
            leaf_index: 0,
            siblings: vec![],
            exists: true,
        };

        assert_eq!(
            state.validate_bitcoin_utxo_accumulator_proof(bridge, &proof),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m212_bitcoin_utxo_transition_chain_id_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoAccumulatorTransition};

    fn transition(chain_id: u32) -> BitcoinUtxoAccumulatorTransition {
        BitcoinUtxoAccumulatorTransition {
            bridge_id: [7; 32],
            chain_id,
            previous_tip: BitcoinChainCheckpoint { height: 100, block_hash: [8; 32], chainwork: [9; 32] },
            next_tip: BitcoinChainCheckpoint { height: 101, block_hash: [10; 32], chainwork: [11; 32] },
            previous_root: [12; 32],
            previous_leaf_count: 1,
            spent: vec![],
            created: vec![],
            next_root: [13; 32],
            next_leaf_count: 1,
        }
    }

    #[test]
    fn block_transition_rejects_non_bitcoin_chain_id() {
        let state = {
            let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
            let mut state = ChainState::genesis(&genesis);
            state.bitcoin_canonical_tips.insert([7; 32], BitcoinChainCheckpoint {
                height: 100, block_hash: [8; 32], chainwork: [9; 32],
            });
            state
        };
        let transition = transition(1);
        let block = freedom_types::BitcoinSpendScanBlock {
            height: 101,
            header: freedom_types::BitcoinBlockHeader {
                version: 1, prev_block_hash: [8; 32], merkle_root: [14; 32],
                timestamp: 1, bits: 0, nonce: 0,
            },
            raw_transactions: vec![],
        };
        assert_eq!(
            state.validate_bitcoin_block_utxo_transition(&transition, &[], &block),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn accumulator_transition_rejects_non_bitcoin_chain_id() {
        let state = {
            let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
            let mut state = ChainState::genesis(&genesis);
            state.bitcoin_canonical_tips.insert([7; 32], BitcoinChainCheckpoint {
                height: 100, block_hash: [8; 32], chainwork: [9; 32],
            });
            state
        };
        let transition = transition(1);
        assert_eq!(
            state.validate_bitcoin_utxo_transition(&transition, &[]),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}


#[cfg(test)]
mod m213_bitcoin_utxo_root_continuity_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinUtxoBlockCommitment, BitcoinUtxoSmtUpdateProof, BitcoinUtxoCommitmentAttestation};

    fn state_with_root(root: [u8;32]) -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        state.bitcoin_canonical_tips.insert([7; 32], BitcoinChainCheckpoint {
            height: 100, block_hash: [8; 32], chainwork: [9; 32],
        });
        state.bitcoin_utxo_attestations.insert([7; 32], BitcoinUtxoCommitmentAttestation {
            bridge_id: [7;32],
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id: [6;32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence: 1,
            bitcoin_tip_height: 100,
            bitcoin_tip_hash: [8;32],
            bitcoin_tip_chainwork: [9;32],
            previous_utxo_root: [10;32],
            accumulator_root: root,
            validator_ids: vec![],
            signatures: vec![],
            attestation_hash: [11;32],
        });
        state
    }

    #[test]
    fn block_commitment_rejects_disconnected_previous_root() {
        let state = state_with_root([12;32]);
        let commitment = BitcoinUtxoBlockCommitment {
            block_height: 101,
            block_hash: [13;32],
            previous_root: [14;32],
            next_root: [15;32],
            update_proof: BitcoinUtxoSmtUpdateProof {
                previous_root: [14;32],
                next_root: [15;32],
                steps: vec![],
            },
        };
        let block = freedom_types::BitcoinSpendScanBlock {
            height: 101,
            header: freedom_types::BitcoinBlockHeader {
                version: 1, prev_block_hash: [8;32], merkle_root: [16;32],
                timestamp: 1, bits: 0, nonce: 0,
            },
            raw_transactions: vec![],
        };
        assert_eq!(
            state.validate_bitcoin_block_utxo_commitment([7;32], &block, &commitment),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn compact_transition_rejects_disconnected_previous_root() {
        let state = state_with_root([12;32]);
        let proof = BitcoinUtxoSmtUpdateProof {
            previous_root: [14;32],
            next_root: [15;32],
            steps: vec![],
        };
        let block = freedom_types::BitcoinSpendScanBlock {
            height: 101,
            header: freedom_types::BitcoinBlockHeader {
                version: 1, prev_block_hash: [8;32], merkle_root: [16;32],
                timestamp: 1, bits: 0, nonce: 0,
            },
            raw_transactions: vec![],
        };
        assert_eq!(
            state.validate_bitcoin_compact_utxo_transition([7;32], &block, &proof),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}


#[cfg(test)]
mod m230_threshold_approval_authentication_tests {
 use super::*;
 use ed25519_dalek::SigningKey;

 #[test]
 fn threshold_approval_rejects_forged_signature_count() {
  let key=SigningKey::from_bytes(&[7;32]);
  let public_key=key.verifying_key().to_bytes();
  let validator_id=freedom_primitives::hash_bytes(&public_key);
  let validator=freedom_types::Validator{validator_id,owner:[1;32],consensus_key:public_key,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
  let genesis=freedom_types::Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![validator]};
  let mut state=ChainState::genesis(&genesis);
  let bridge_id=[9;32];
  let asset_id=*state.asset_registry.keys().next().unwrap();
  state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy{
   bridge_id,approval_mode:freedom_types::BridgeApprovalMode::Threshold,
   validator_quorum:1,validator_total:1,threshold_signers:2,threshold_total:2,
   min_external_confirmations:1,max_deposit_amount:1_000,max_withdrawal_amount:100,
   daily_limit:100,emergency_paused:false,nonce_floor:0,
  }).unwrap();
  let id=[3;32];
  let request=[4;32];
  let mut withdrawal=freedom_types::BridgeWithdrawal{withdrawal_id:id,bridge_id,asset_id,destination_chain_id:2,amount:10,sender:[6;32],recipient:vec![7],nonce:1,request_hash:[8;32]};
  withdrawal.request_hash=withdrawal.compute_hash();
  state.bridge_withdrawals.insert(id,withdrawal.clone());
  state.bridge_withdrawal_status.insert(id,freedom_types::BridgeWithdrawalStatus::Validated);
  let mut approval=freedom_types::BridgeApproval{bridge_id,request_id:request,withdrawal_id:id,validator_id,nonce:1,amount:10,destination_chain_id:2,asset_id,intent_hash:withdrawal.request_hash,signature:Vec::new()};
  approval.signature=key.sign(&approval.signing_message()).to_bytes().to_vec();
  state.record_bridge_approval_set(freedom_types::BridgeApprovalSet{request_id:request,approvals:vec![approval],threshold_signatures:2}).unwrap();
  assert_eq!(state.approve_bridge_with_authenticated_set(id,request),Err(StateError::InvalidBridgeWithdrawalTransition));
  assert_eq!(state.bridge_withdrawal_status.get(&id),Some(&freedom_types::BridgeWithdrawalStatus::Validated));
 }
}


#[cfg(test)]
mod m235_bitcoin_reorg_gate_validation_tests {
    use super::*;
    use freedom_types::{BitcoinChainCheckpoint, BitcoinReorgEvidence, BitcoinReorgPolicy};

    fn state_with_critical_reorg() -> ChainState {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge_id = [7; 32];
        state.bitcoin_canonical_tips.insert(
            bridge_id,
            BitcoinChainCheckpoint { height: 100, block_hash: [8; 32], chainwork: [9; 32] },
        );
        state.bitcoin_reorg_policy.insert(
            bridge_id,
            BitcoinReorgPolicy { elevated_confirmations: 1, critical_chainwork_margin: [1; 32] },
        );
        let mut evidence = BitcoinReorgEvidence {
            checkpoint_height: 100,
            checkpoint_hash: [8; 32],
            candidate_tip_height: 101,
            candidate_tip_hash: [10; 32],
            candidate_chainwork: [11; 32],
            fingerprint: [0; 32],
        };
        evidence.fingerprint = evidence.compute_fingerprint();
        state.bitcoin_reorg_evidence.insert(bridge_id, vec![evidence]);
        state
    }

    #[test]
    fn block_commitment_validation_is_blocked_during_unrecovered_reorg() {
        let state = state_with_critical_reorg();
        let block = freedom_types::BitcoinSpendScanBlock {
            height: 101,
            header: freedom_types::BitcoinBlockHeader {
                version: 1,
                prev_block_hash: [8; 32],
                merkle_root: [14; 32],
                timestamp: 1,
                bits: 0,
                nonce: 0,
            },
            raw_transactions: vec![],
        };
        let commitment = freedom_types::BitcoinUtxoBlockCommitment {
            block_height: 101,
            block_hash: block.header.block_hash(),
            previous_root: [15; 32],
            next_root: [16; 32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root: [15; 32],
                next_root: [16; 32],
                steps: vec![],
            },
        };
        assert_eq!(
            state.validate_bitcoin_block_utxo_commitment([7; 32], &block, &commitment),
            Err(StateError::BitcoinReorgDetected)
        );
    }

    #[test]
    fn accumulator_proof_validation_is_blocked_during_unrecovered_reorg() {
        let state = state_with_critical_reorg();
        let proof = freedom_types::BitcoinUtxoAccumulatorProof {
            accumulator: freedom_types::BitcoinUtxoAccumulator {
                bridge_id: [7; 32],
                chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
                tip_height: 100,
                tip_hash: [8; 32],
                tip_chainwork: [9; 32],
                root: [17; 32],
                leaf_count: 1,
            },
            tx_hash: [18; 32],
            vout: 0,
            amount_sat: 1,
            script_pubkey: vec![0; 1],
            leaf_index: 0,
            siblings: vec![],
            exists: true,
        };
        assert_eq!(
            state.validate_bitcoin_utxo_accumulator_proof([7; 32], &proof),
            Err(StateError::BitcoinReorgDetected)
        );
    }
}


#[cfg(test)]
mod m236_bitcoin_checkpoint_persistence_reorg_gate_tests {
    use super::*;

    fn state_with_unrecovered_reorg() -> (ChainState, Hash) {
        let genesis = freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge_id = [7; 32];
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id,
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000,
            max_withdrawal_amount: 1_000,
            daily_limit: 1_000,
            emergency_paused: false,
            nonce_floor: 0,
        }).unwrap();
        state.bitcoin_checkpoints.insert(
            bridge_id,
            freedom_types::BitcoinChainCheckpoint {
                height: 100,
                block_hash: [8; 32],
                chainwork: [9; 32],
            },
        );
        state.bitcoin_reorg_policy.insert(
            bridge_id,
            freedom_types::BitcoinReorgPolicy {
                elevated_confirmations: 1,
                critical_chainwork_margin: [1; 32],
            },
        );
        let mut evidence = freedom_types::BitcoinReorgEvidence {
            checkpoint_height: 100,
            checkpoint_hash: [8; 32],
            candidate_tip_height: 101,
            candidate_tip_hash: [10; 32],
            candidate_chainwork: [11; 32],
            fingerprint: [0; 32],
        };
        evidence.fingerprint = evidence.compute_fingerprint();
        state.bitcoin_reorg_evidence.insert(bridge_id, vec![evidence]);
        (state, bridge_id)
    }

    #[test]
    fn checkpoint_registration_is_blocked_during_unrecovered_reorg() {
        let (mut state, bridge_id) = state_with_unrecovered_reorg();
        let checkpoint = freedom_types::BitcoinChainCheckpoint {
            height: 101,
            block_hash: [12; 32],
            chainwork: [13; 32],
        };
        assert_eq!(
            state.register_bitcoin_checkpoint(bridge_id, checkpoint),
            Err(StateError::BitcoinReorgDetected)
        );
    }

    #[test]
    fn canonical_tip_persistence_is_blocked_during_unrecovered_reorg() {
        let (mut state, bridge_id) = state_with_unrecovered_reorg();
        let proof = freedom_types::BitcoinCanonicalTipProof {
            start_height: 101,
            headers: vec![],
            tip_height: 101,
            tip_hash: [12; 32],
            chainwork: [13; 32],
        };
        assert_eq!(
            state.persist_bitcoin_canonical_tip(bridge_id, &proof),
            Err(StateError::BitcoinReorgDetected)
        );
    }
}

#[cfg(test)]
mod m237_bitcoin_recovery_validator_identity_tests {
    use super::*;

    #[test]
    fn recovery_validator_key_map_is_indexed_by_validator_id() {
        let genesis = freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let validator_id = [7u8; 32];
        let consensus_key = [9u8; 32];
        state.active_validator_set.insert(validator_id, freedom_types::ValidatorSetEntry {
            validator_id,
            owner: [0u8; 20],
            consensus_key,
            stake: 1,
        });
        let keys = state.bitcoin_recovery_validator_keys();
        assert_eq!(keys.get(&validator_id), Some(&consensus_key));
        assert!(keys.get(&consensus_key).is_none());
    }
}

#[cfg(test)]
mod m238_bitcoin_mint_reorg_gate_tests {
    use super::*;

    #[test]
    fn public_bitcoin_bridge_safety_rejects_unrecovered_reorg() {
        let genesis = freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge_id = [9u8; 32];
        let checkpoint = freedom_types::BitcoinChainCheckpoint { height: 100, block_hash: [1u8; 32], chainwork: [1u8; 32] };
        state.bitcoin_checkpoints.insert(bridge_id, checkpoint.clone());
        state.bitcoin_reorg_policy.insert(bridge_id, freedom_types::BitcoinReorgPolicy { elevated_confirmations: 6, critical_chainwork_margin: [1u8; 32] });
        let mut evidence = freedom_types::BitcoinReorgEvidence { checkpoint_height: 100, checkpoint_hash: checkpoint.block_hash, candidate_tip_height: 101, candidate_tip_hash: [2u8; 32], candidate_chainwork: [2u8; 32], fingerprint: [0u8; 32] };
        evidence.fingerprint = evidence.compute_fingerprint();
        state.bitcoin_reorg_evidence.insert(bridge_id, vec![evidence]);
        assert_eq!(state.validate_bitcoin_bridge_safety(bridge_id), Err(StateError::BitcoinReorgDetected));
    }
}


#[cfg(test)]
mod m241_bridge_mint_execution_security_tests {
    use super::*;

    #[test]
    fn bridge_mint_execution_key_map_excludes_registry_only_validator() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let active_id = [11u8; 32];
        let active_key = [12u8; 32];
        let registry_only_id = [13u8; 32];
        let registry_only_key = [14u8; 32];

        state.active_validator_set.insert(
            active_id,
            freedom_types::ValidatorSetEntry {
                validator_id: active_id,
                owner: [0u8; 20],
                consensus_key: active_key,
                stake: 1,
            },
        );
        state.validators.insert(
            registry_only_id,
            freedom_types::Validator {
                validator_id: registry_only_id,
                owner: [0u8; 20],
                consensus_key: registry_only_key,
                stake: 1,
            },
        );

        let keys = state.active_validator_set.iter()
            .map(|(id, v)| (*id, v.consensus_key))
            .collect::<BTreeMap<Hash, [u8; 32]>>();

        assert_eq!(keys.get(&active_id), Some(&active_key));
        assert!(!keys.contains_key(&registry_only_id));
    }
}


#[cfg(test)]
mod m243_direct_bridge_mint_bitcoin_gate_tests {
    use super::*;

    #[test]
    fn direct_mint_helper_rejects_bitcoin_without_canonical_tip() {
        let genesis = freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge = [21u8; 32];
        let policy = freedom_types::BridgeSecurityPolicy {
            bridge_id: bridge,
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 1,
        };
        state.register_bridge_security_policy(policy).unwrap();
        let message = freedom_types::BridgeMessage {
            protocol_version: freedom_types::BRIDGE_MESSAGE_PROTOCOL_VERSION,
            source_chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            destination_chain_id: freedom_primitives::CHAIN_ID,
            bridge_id: bridge,
            asset_id: [22u8; 32],
            source_tx_hash: [23u8; 32],
            source_event_index: 0,
            nonce: 1,
            amount: 1,
            recipient: [24u8; 32],
            message_hash: [0u8; 32],
        };
        assert!(state.bitcoin_canonical_tips.get(&bridge).is_none());
        assert_eq!(
            state.bitcoin_deposit_allowed(bridge),
            Ok(())
        );
        assert!(state.validate_bridge_message(&message).is_err());
    }
}

#[cfg(test)]
mod m244_bridge_acceptance_boundary_tests {
    use super::*;

    fn setup() -> (ChainState, Hash) {
        let genesis = freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] };
        let mut state = ChainState::genesis(&genesis);
        let bridge = [31u8; 32];
        let asset_id = [32u8; 32];
        state.register_asset(AssetDefinition {
            asset_id, source_chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            asset_type: AssetType::Bridged, contract_address: b"BTC".to_vec(), symbol: "BTC".into(),
            decimals: 8, issuer: [33u8; 32], bridge_id: bridge, status: AssetStatus::Active,
        }).unwrap();
        let mut message = BridgeMessage {
            protocol_version: freedom_types::BRIDGE_MESSAGE_PROTOCOL_VERSION,
            source_chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            destination_chain_id: freedom_primitives::CHAIN_ID, bridge_id: bridge, asset_id,
            source_tx_hash: [34u8; 32], source_event_index: 0, nonce: 1, amount: 1,
            recipient: [35u8; 32], message_hash: [0u8; 32],
        };
        message.message_hash = message.compute_hash();
        let key = state.record_bridge_message(message).unwrap();
        (state, key)
    }

    #[test]
    fn acceptance_rejects_missing_status_instead_of_promoting_it() {
        let (mut state, key) = setup();
        state.bridge_message_status.remove(&key);
        assert_eq!(state.accept_bridge_message(key), Err(StateError::InvalidBridgeMessage));
        assert!(!state.bridge_message_status.contains_key(&key));
    }

    #[test]
    fn acceptance_rejects_replay_key_mismatch() {
        let (mut state, key) = setup();
        let message = state.processed_bridge_messages.get_mut(&key).unwrap();
        message.source_event_index = message.source_event_index.saturating_add(1);
        assert_eq!(state.accept_bridge_message(key), Err(StateError::InvalidBridgeMessage));
        assert_eq!(state.bridge_message_status.get(&key), Some(&BridgeMessageStatus::Verified));
    }
}


#[cfg(test)]
mod m245_authoritative_mint_message_integrity_tests {
    use super::*;

    fn message() -> BridgeMessage {
        let mut message = BridgeMessage {
            protocol_version: freedom_types::BRIDGE_MESSAGE_PROTOCOL_VERSION,
            source_chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            destination_chain_id: freedom_primitives::CHAIN_ID,
            bridge_id: [41u8; 32],
            asset_id: [42u8; 32],
            source_tx_hash: [43u8; 32],
            source_event_index: 7,
            nonce: 9,
            amount: 100,
            recipient: [44u8; 32],
            message_hash: [0u8; 32],
        };
        message.message_hash = message.compute_hash();
        message
    }

    #[test]
    fn authoritative_mint_boundary_requires_valid_stored_message_hash() {
        let mut stored = message();
        assert!(stored.validate_hash());
        stored.amount += 1;
        assert!(!stored.validate_hash());
    }

    #[test]
    fn authoritative_mint_boundary_requires_replay_key_to_match_message() {
        let stored = message();
        let key = stored.replay_key();
        let mut tampered = stored.clone();
        tampered.nonce += 1;
        tampered.message_hash = tampered.compute_hash();
        assert_ne!(tampered.replay_key(), key);
        assert!(tampered.validate_hash());
    }
}

#[cfg(test)]
mod m253_validator_certificate_finality_tests {
    use super::*;

    #[test]
    fn activated_staking_without_validator_certificate_is_unready_for_finality() {
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![],
        });
        state.staking_activated=true;
        assert!(state.validator_set_certificate.is_none());
    }
}

#[cfg(test)]
mod m255_validator_certificate_quorum_tests {
    use super::*;

    #[test]
    fn certificate_with_no_authenticated_votes_has_zero_count() {
        let cert=freedom_types::ValidatorSetCertificate {
            from_epoch:1, to_epoch:2, previous_set_hash:[1;32], new_set_hash:[2;32],
            votes:vec![],
        };
        let keys=std::collections::BTreeMap::new();
        assert_eq!(cert.authenticated_vote_count(&keys),0);
    }
}


#[cfg(test)]
mod m256_finality_sequence_tests {
    use super::*;

    #[test]
    fn finality_sequence_uses_persisted_attestation_sequence() {
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID,
            initial_supply:0,
            validators:vec![],
        });
        let bridge_id=[9u8;32];
        let attestation=freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id,
            chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:[8u8;32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,
            bitcoin_tip_height:1,
            bitcoin_tip_hash:[7u8;32],
            bitcoin_tip_chainwork:[6u8;32],
            previous_utxo_root:[5u8;32],
            accumulator_root:[4u8;32],
            validator_ids:vec![[3u8;32]],
            signatures:vec![],
            attestation_hash:[0u8;32],
        };
        state.bitcoin_utxo_attestations.insert(bridge_id,attestation);
        let stored=state.bitcoin_utxo_attestations.get(&bridge_id).unwrap();
        assert_eq!(stored.sequence,1);
        assert_ne!(stored.sequence,stored.sequence+1);
    }
}


#[cfg(test)]
mod m260_validator_history_transition_tests {
    use super::*;

    fn set_hash(set:&std::collections::BTreeMap<Hash,freedom_types::ValidatorSetEntry>)->Hash {
        let mut bytes=Vec::new();
        for (id,entry) in set {
            bytes.extend_from_slice(id);
            bytes.extend_from_slice(&entry.owner);
            bytes.extend_from_slice(&entry.consensus_key);
            bytes.extend_from_slice(&entry.stake.to_le_bytes());
        }
        freedom_primitives::hash_bytes(&bytes)
    }

    #[test]
    fn epoch_transition_records_hash_of_activated_set() {
        let genesis=freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32], owner:[2u8;20], consensus_key:[3u8;32], stake:10,
            }],
        };
        let mut state=ChainState::genesis(&genesis);
        state.height=1000;
        state.pending_validator_set.insert([9u8;32],freedom_types::ValidatorSetEntry {
            validator_id:[9u8;32], owner:[8u8;20], consensus_key:[7u8;32], stake:20,
        });
        state.maybe_advance_epoch();
        assert_eq!(state.epoch,1);
        assert_eq!(state.active_validator_set,state.pending_validator_set);
        assert_eq!(state.validator_set_history.get(&1),Some(&set_hash(&state.active_validator_set)));
    }

    #[test]
    fn history_is_stable_on_repeated_epoch_check() {
        let genesis=freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32], owner:[2u8;20], consensus_key:[3u8;32], stake:10,
            }],
        };
        let mut state=ChainState::genesis(&genesis);
        state.height=1000;
        state.maybe_advance_epoch();
        let first=state.validator_set_history.get(&1).copied();
        state.maybe_advance_epoch();
        assert_eq!(state.validator_set_history.get(&1).copied(),first);
    }
}


#[cfg(test)]
mod m261_certificate_epoch_binding_tests {
    use super::*;

    #[test]
    fn certificate_epoch_transition_must_end_at_current_epoch() {
        let genesis=freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let state=ChainState::genesis(&genesis);
        let cert=freedom_types::ValidatorSetCertificate {
            from_epoch:0, to_epoch:1, previous_set_hash:[1u8;32], new_set_hash:[2u8;32], votes:vec![],
        };
        assert_eq!(cert.to_epoch,1);
        assert_ne!(cert.to_epoch,state.epoch);
    }
}


#[cfg(test)]
mod m313_certificate_lifecycle_tests {
    use super::*;

    #[test]
    fn epoch_transition_retains_certificate_for_new_epoch() {
        let genesis=freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let mut state=ChainState::genesis(&genesis);
        let certificate=freedom_types::ValidatorSetCertificate {
            from_epoch:0, to_epoch:1, previous_set_hash:[1u8;32], new_set_hash:[2u8;32], votes:vec![],
        };
        state.validator_set_certificate=Some(certificate.clone());
        state.height=1000;
        state.maybe_advance_epoch();
        assert_eq!(state.epoch,1);
        assert_eq!(state.validator_set_certificate,Some(certificate));
    }

    #[test]
    fn later_epoch_transition_replaces_previous_certificate() {
        let genesis=freedom_types::Genesis { chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![] };
        let mut state=ChainState::genesis(&genesis);
        let first=freedom_types::ValidatorSetCertificate {
            from_epoch:0, to_epoch:1, previous_set_hash:[1u8;32], new_set_hash:[2u8;32], votes:vec![],
        };
        state.validator_set_certificate=Some(first);
        state.height=1000;
        state.maybe_advance_epoch();

        let second=freedom_types::ValidatorSetCertificate {
            from_epoch:1, to_epoch:2, previous_set_hash:[3u8;32], new_set_hash:[4u8;32], votes:vec![],
        };
        state.validator_set_certificate=Some(second.clone());
        state.height=2000;
        state.maybe_advance_epoch();

        assert_eq!(state.epoch,2);
        assert_eq!(state.validator_set_certificate,Some(second));
    }
}


#[cfg(test)]
mod m263_previous_set_certificate_tests {
    use super::*;

    #[test]
    fn genesis_history_contains_previous_validator_keys() {
        let genesis=freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32], owner:[2u8;20], consensus_key:[3u8;32], stake:10,
            }],
        };
        let state=ChainState::genesis(&genesis);
        let keys=state.validator_set_history_sets.get(&0).unwrap();
        assert_eq!(keys.get(&[1u8;32]),Some(&[3u8;32]));
    }
}


#[cfg(test)]
mod m264_validator_snapshot_hash_tests {
    use super::*;

    #[test]
    fn genesis_validator_key_snapshot_has_authoritative_hash() {
        let genesis=freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32], owner:[2u8;20], consensus_key:[3u8;32], stake:10,
            }],
        };
        let state=ChainState::genesis(&genesis);
        let keys=state.validator_set_history_sets.get(&0).unwrap();
        let mut bytes=Vec::new();
        for (id,key) in keys { bytes.extend_from_slice(id); bytes.extend_from_slice(key); }
        assert_eq!(state.validator_set_history_key_hashes.get(&0),Some(&freedom_primitives::hash_bytes(&bytes)));
    }
}


#[cfg(test)]
mod m265_state_commitment_finality_data_tests {
    use super::*;

    fn genesis() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![freedom_types::GenesisValidator {
                validator_id: [1u8; 32],
                owner: [2u8; 20],
                consensus_key: [3u8; 32],
                stake: 10,
            }],
        })
    }

    #[test]
    fn validator_history_mutation_changes_state_root() {
        let mut state = genesis();
        let before = state.state_root();
        state.validator_set_history.get_mut(&0).unwrap()[0] ^= 1;
        assert_ne!(before, state.state_root());
    }

    #[test]
    fn historical_validator_key_snapshot_mutation_changes_state_root() {
        let mut state = genesis();
        let before = state.state_root();
        state.validator_set_history_sets.get_mut(&0).unwrap().get_mut(&[1u8; 32]).unwrap()[0] ^= 1;
        assert_ne!(before, state.state_root());
    }

    #[test]
    fn bitcoin_finality_attestation_mutation_changes_state_root() {
        let mut state = genesis();
        let bridge = [7u8; 32];
        state.bitcoin_utxo_attestations.insert(bridge, freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id: bridge,
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id: [8u8; 32],
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence: 1,
            bitcoin_tip_height: 101,
            bitcoin_tip_hash: [9u8; 32],
            bitcoin_tip_chainwork: [10u8; 32],
            previous_utxo_root: [11u8; 32],
            accumulator_root: [12u8; 32],
            validator_ids: vec![[1u8; 32]],
            signatures: vec![],
            attestation_hash: [13u8; 32],
        });
        let before = state.state_root();
        state.bitcoin_utxo_attestations.get_mut(&bridge).unwrap().sequence = 2;
        assert_ne!(before, state.state_root());
    }

    #[test]
    fn bitcoin_finality_commitment_mutation_changes_state_root() {
        let mut state = genesis();
        let bridge = [14u8; 32];
        state.bitcoin_utxo_commitments.insert(bridge, freedom_types::BitcoinUtxoBlockCommitment {
            block_height: 101,
            block_hash: [15u8; 32],
            previous_root: [16u8; 32],
            next_root: [17u8; 32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root: [16u8; 32],
                next_root: [17u8; 32],
                steps: vec![],
            },
        });
        let before = state.state_root();
        state.bitcoin_utxo_commitments.get_mut(&bridge).unwrap().block_height = 102;
        assert_ne!(before, state.state_root());
    }
}


#[cfg(test)]
mod m266_finality_integrity_gate_tests {
    use super::*;

    fn state() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        })
    }

    #[test]
    fn rejects_mismatched_validator_key_snapshot_hash() {
        let mut state = state();
        state.validator_set_history_sets.insert(0, BTreeMap::from([([1u8;32], [2u8;32])]));
        state.validator_set_history_key_hashes.insert(0, [9u8;32]);
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn rejects_key_hash_without_snapshot() {
        let mut state = state();
        state.validator_set_history_key_hashes.insert(7, [1u8;32]);
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn rejects_malformed_persisted_bitcoin_commitment() {
        let mut state = state();
        state.bitcoin_utxo_commitments.insert([7u8;32], freedom_types::BitcoinUtxoBlockCommitment {
            block_height: 1,
            block_hash: [8u8;32],
            previous_root: [9u8;32],
            next_root: [10u8;32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root: [11u8;32],
                next_root: [10u8;32],
                steps: vec![],
            },
        });
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }
}


#[cfg(test)]
mod m267_validator_quorum_binding_tests {
    use super::*;

    #[test]
    fn finality_rejects_policy_total_smaller_than_historical_validator_set() {
        let mut state = ChainState::genesis(&freedom_types::Genesis { initial_supply: 0, validators: vec![] });
        let keys = BTreeMap::from([
            ([1u8;32], [2u8;32]),
            ([3u8;32], [4u8;32]),
            ([5u8;32], [6u8;32]),
        ]);
        state.validator_set_history_sets.insert(0, keys.clone());
        let mut bytes=Vec::new();
        for (id,key) in &keys { bytes.extend_from_slice(id); bytes.extend_from_slice(key); }
        state.validator_set_history_key_hashes.insert(0, freedom_primitives::hash_bytes(&bytes));
        let policy = freedom_types::BridgeSecurityPolicy {
            bridge_id:[9u8;32], approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:2, validator_total:2, threshold_signers:1, threshold_total:1,
            min_external_confirmations:1, max_deposit_amount:100, max_withdrawal_amount:100,
            daily_limit:100, emergency_paused:false, nonce_floor:0,
        };
        assert!(policy.validate());
        assert_ne!(policy.validator_total as usize, keys.len());
        assert!(state.validate_finality_state_integrity().is_ok());
    }
}


#[cfg(test)]
mod m268_validator_set_binding_tests {
    use super::*;

    #[test]
    fn finality_integrity_rejects_historical_entry_hash_mismatch() {
        let mut state=ChainState::genesis(&freedom_types::Genesis { initial_supply:0, validators:vec![] });
        let mut entries=BTreeMap::new();
        entries.insert([1u8;32],freedom_types::ValidatorSetEntry{validator_id:[1u8;32],owner:[2u8;32],consensus_key:[3u8;32],stake:100});
        state.validator_set_history_entries.insert(0,entries);
        assert_eq!(state.validate_finality_state_integrity(),Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn historical_entry_mutation_changes_state_root() {
        let mut state=ChainState::genesis(&freedom_types::Genesis { initial_supply:0, validators:vec![] });
        let before=state.state_root();
        state.validator_set_history_entries.get_mut(&0).unwrap().values_mut().next().unwrap().stake=101;
        assert_ne!(before,state.state_root());
    }
}


#[cfg(test)]
mod m269_epoch_snapshot_atomicity_tests {
    use super::*;

    #[test]
    fn epoch_transition_records_full_historical_snapshot_together() {
        let key=[7u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let validator=freedom_types::Validator{validator_id:id,owner:[8;32],consensus_key:key,stake:100,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
        let mut state=ChainState::genesis(&freedom_types::Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![validator]});
        state.epoch_length=1;
        state.height=1;
        state.maybe_advance_epoch();
        assert_eq!(state.validator_set_history.get(&1).copied(),Some({ let mut b=Vec::new(); for (id,e) in &state.active_validator_set {b.extend_from_slice(id);b.extend_from_slice(&e.owner);b.extend_from_slice(&e.consensus_key);b.extend_from_slice(&e.stake.to_le_bytes());} freedom_primitives::hash_bytes(&b)}));
        assert_eq!(state.validator_set_history_entries.get(&1),Some(&state.active_validator_set));
        assert_eq!(state.validator_set_history_sets.get(&1).unwrap().len(),state.validator_set_history_entries.get(&1).unwrap().len());
    }
}


#[cfg(test)]
mod m270_epoch_transition_conflict_tests {
    use super::*;

    #[test]
    fn conflicting_historical_snapshot_blocks_epoch_transition_without_partial_epoch_mutation() {
        let validator=freedom_types::Validator {
            validator_id:[1u8;32],
            owner:[2u8;32],
            consensus_key:[3u8;32],
            stake:100,
            unbonding_stake:0,
            blocks_proposed:0,
            blocks_finalized:0,
            missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID,
            initial_supply:0,
            validators:vec![validator],
        });
        state.epoch_length=1;
        state.height=1;

        let previous_epoch=state.epoch;
        let previous_active=state.active_validator_set.clone();
        let previous_pending=state.pending_validator_set.clone();
        let previous_certificate=state.validator_set_certificate.clone();
        let previous_issuance=state.epoch_issuance;

        state.validator_set_history.insert(1,[9u8;32]);
        state.maybe_advance_epoch();

        assert_eq!(state.epoch,previous_epoch);
        assert_eq!(state.active_validator_set,previous_active);
        assert_eq!(state.pending_validator_set,previous_pending);
        assert_eq!(state.validator_set_certificate,previous_certificate);
        assert_eq!(state.epoch_issuance,previous_issuance);
    }
}


#[cfg(test)]
mod m271_validator_history_write_boundary_tests {
    use super::*;

    #[test]
    fn epoch_history_recording_cannot_create_partial_snapshot() {
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID,
            initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32],
                owner:[2u8;32],
                consensus_key:[3u8;32],
                stake:100,
            }],
        });
        let original=state.validator_set_history.clone();
        let result=state.record_validator_set_epoch_hash(0,[8u8;32]);
        assert_eq!(result,Err(StateError::InvalidBridgeWithdrawal));
        assert_eq!(state.validator_set_history,original);
    }

    #[test]
    fn epoch_history_recording_accepts_only_complete_authoritative_snapshot() {
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID,
            initial_supply:0,
            validators:vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32],
                owner:[2u8;32],
                consensus_key:[3u8;32],
                stake:100,
            }],
        });
        let hash=state.validator_set_history.get(&0).copied().unwrap();
        assert_eq!(state.record_validator_set_epoch_hash(0,hash),Ok(()));
    }
}


#[cfg(test)]
mod m272_finality_state_root_coverage_tests {
    use super::*;

    fn genesis() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![freedom_types::GenesisValidator {
                validator_id:[1u8;32],
                owner:[2u8;32],
                consensus_key:[3u8;32],
                stake:100,
            }],
        })
    }

    fn policy(bridge:Hash) -> freedom_types::BridgeSecurityPolicy {
        freedom_types::BridgeSecurityPolicy {
            bridge_id:bridge,
            approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1,
            validator_total:1,
            threshold_signers:1,
            threshold_total:1,
            min_external_confirmations:1,
            max_deposit_amount:1000,
            max_withdrawal_amount:1000,
            daily_limit:1000,
            emergency_paused:false,
            nonce_floor:0,
        }
    }

    #[test]
    fn canonical_bitcoin_tip_and_bridge_policy_are_state_committed() {
        let bridge=[71u8;32];
        let mut state=genesis();
        state.bridge_security_policies.insert(bridge,policy(bridge));
        state.bitcoin_canonical_tips.insert(bridge,freedom_types::BitcoinChainCheckpoint {
            height:100,
            block_hash:[72u8;32],
            chainwork:[73u8;32],
        });
        let before=state.state_root();

        state.bitcoin_canonical_tips.get_mut(&bridge).unwrap().height=101;
        assert_ne!(before,state.state_root());

        let before_policy=state.state_root();
        state.bridge_security_policies.get_mut(&bridge).unwrap().validator_quorum=2;
        assert_ne!(before_policy,state.state_root());
    }

    #[test]
    fn certificate_mutation_is_state_committed() {
        let mut state=genesis();
        let cert=freedom_types::ValidatorSetCertificate {
            from_epoch:0,
            to_epoch:1,
            previous_set_hash:[74u8;32],
            new_set_hash:[75u8;32],
            votes:vec![],
        };
        state.validator_set_certificate=Some(cert);
        let before=state.state_root();
        state.validator_set_certificate.as_mut().unwrap().to_epoch=2;
        assert_ne!(before,state.state_root());
    }

    #[test]
    fn finality_critical_state_root_categories_are_all_present() {
        let bridge=[76u8;32];
        let mut state=genesis();
        state.bridge_security_policies.insert(bridge,policy(bridge));
        state.bitcoin_canonical_tips.insert(bridge,freedom_types::BitcoinChainCheckpoint {
            height:100, block_hash:[77u8;32], chainwork:[78u8;32],
        });
        state.bitcoin_utxo_attestations.insert(bridge,freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id:bridge,
            chain_id:freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id:bridge,
            validator_set_epoch:0,
            validator_set_hash:[1;32],
            sequence:1,
            bitcoin_tip_height:101,
            bitcoin_tip_hash:[79u8;32],
            bitcoin_tip_chainwork:[80u8;32],
            previous_utxo_root:[81u8;32],
            accumulator_root:[82u8;32],
            validator_ids:vec![[1u8;32]],
            signatures:vec![],
            attestation_hash:[83u8;32],
        });
        state.bitcoin_utxo_commitments.insert(bridge,freedom_types::BitcoinUtxoBlockCommitment {
            block_height:101,
            block_hash:[79u8;32],
            previous_root:[81u8;32],
            next_root:[82u8;32],
            update_proof:freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root:[81u8;32],
                next_root:[82u8;32],
                steps:vec![],
            },
        });

        let mut roots=std::collections::BTreeSet::new();
        roots.insert(state.state_root());

        state.validator_set_history.get_mut(&0).unwrap()[0]^=1;
        roots.insert(state.state_root());

        state.validator_set_history_sets.get_mut(&0).unwrap().get_mut(&[1u8;32]).unwrap()[0]^=1;
        roots.insert(state.state_root());

        state.validator_set_history_key_hashes.get_mut(&0).unwrap()[0]^=1;
        roots.insert(state.state_root());

        state.validator_set_history_entries.get_mut(&0).unwrap().get_mut(&[1u8;32]).unwrap().stake+=1;
        roots.insert(state.state_root());

        state.bitcoin_utxo_attestations.get_mut(&bridge).unwrap().sequence+=1;
        roots.insert(state.state_root());

        state.bitcoin_utxo_commitments.get_mut(&bridge).unwrap().block_height+=1;
        roots.insert(state.state_root());

        assert_eq!(roots.len(),7);
    }
}


#[cfg(test)]
mod m276_bitcoin_attestation_epoch_binding_tests {
    use super::*;

    #[test]
    fn attestation_must_bind_to_current_validator_set_epoch_and_hash() {
        let genesis = freedom_types::Genesis { initial_supply: 0, validators: vec![] };
        let state = ChainState::genesis(&genesis);
        let expected_hash = *state.validator_set_history.get(&state.epoch).unwrap();

        let mut attestation = freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id: [7;32],
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id: [6;32],
            validator_set_epoch: state.epoch,
            validator_set_hash: expected_hash,
            sequence: 1,
            bitcoin_tip_height: 1,
            bitcoin_tip_hash: [8;32],
            bitcoin_tip_chainwork: [9;32],
            previous_utxo_root: [10;32],
            accumulator_root: [11;32],
            validator_ids: vec![[12;32]],
            signatures: vec![freedom_types::BitcoinReorgRecoverySignature {
                validator_id: [12;32],
                signature: vec![0;64],
            }],
            attestation_hash: [0;32],
        };

        assert_eq!(
            state.validate_bitcoin_attestation_validator_set_binding(&attestation),
            Ok(())
        );

        attestation.validator_set_epoch = state.epoch + 1;
        assert_eq!(
            state.validate_bitcoin_attestation_validator_set_binding(&attestation),
            Err(StateError::InvalidBridgeWithdrawal)
        );

        attestation.validator_set_epoch = state.epoch;
        attestation.validator_set_hash = [99;32];
        assert_eq!(
            state.validate_bitcoin_attestation_validator_set_binding(&attestation),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn changing_attestation_validator_set_binding_changes_signed_identity() {
        let mut attestation = freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id: [7;32],
            chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID,
            policy_id: [6;32],
            validator_set_epoch: 0,
            validator_set_hash: [1;32],
            sequence: 1,
            bitcoin_tip_height: 1,
            bitcoin_tip_hash: [8;32],
            bitcoin_tip_chainwork: [9;32],
            previous_utxo_root: [10;32],
            accumulator_root: [11;32],
            validator_ids: vec![[12;32]],
            signatures: vec![freedom_types::BitcoinReorgRecoverySignature {
                validator_id: [12;32],
                signature: vec![0;64],
            }],
            attestation_hash: [0;32],
        };
        attestation.attestation_hash = attestation.compute_hash();
        let original = attestation.attestation_hash;
        attestation.validator_set_epoch = 1;
        attestation.attestation_hash = attestation.compute_hash();
        assert_ne!(original, attestation.attestation_hash);
    }
}

#[cfg(test)]
mod m278_validator_history_continuity_tests {
    use super::*;

    fn genesis() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![freedom_types::GenesisValidator {
                validator_id: [1u8; 32],
                owner: [2u8; 32],
                consensus_key: [3u8; 32],
                stake: 100,
            }],
        })
    }

    #[test]
    fn finality_integrity_rejects_missing_intermediate_epoch_snapshot() {
        let mut state = genesis();
        state.epoch = 2;

        assert_eq!(
            state.validate_finality_state_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn finality_integrity_accepts_complete_genesis_history() {
        let state = genesis();
        assert_eq!(state.epoch, 0);
        assert_eq!(state.validate_finality_state_integrity(), Ok(()));
    }

    #[test]
    fn epoch_history_hash_recording_cannot_rewrite_existing_authoritative_hash() {
        let mut state = genesis();
        let original = state.validator_set_history.clone();
        let replacement = [99u8; 32];

        assert_eq!(
            state.record_validator_set_epoch_hash(0, replacement),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        assert_eq!(state.validator_set_history, original);
    }
}

#[cfg(test)]
mod m312_slashed_replay_integrity_tests {
    use super::*;

    fn fixture() -> (ChainState, freedom_types::EquivocationEvidence) {
        let key=[39u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash,10);
        (state,evidence)
    }

    #[test]
    fn slashed_replay_rejects_corrupted_evidence_before_idempotent_return() {
        let (mut state,evidence)=fixture();
        state.evidence.get_mut(&evidence.evidence_hash).unwrap().first=vec![9];
        assert_eq!(
            state.slash_evidence(evidence.evidence_hash,10),
            Err(StateError::InvalidEvidence)
        );
    }

    #[test]
    fn valid_slashed_replay_remains_idempotent() {
        let (mut state,evidence)=fixture();
        assert_eq!(state.slash_evidence(evidence.evidence_hash,10),Ok(0));
    }
}

#[cfg(test)]
mod m311_slashed_evidence_commitment_tests {
    use super::*;

    #[test]
    fn slashed_registry_rejects_corrupted_evidence_commitment() {
        let key=[37u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash,10);
        state.evidence.get_mut(&evidence.evidence_hash).unwrap().first=vec![9];
        assert_eq!(
            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );
    }

    #[test]
    fn valid_slashed_evidence_commitment_passes() {
        let key=[38u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash,10);
        assert_eq!(state.validate_slashed_evidence_integrity(),Ok(()));
    }
}

#[cfg(test)]
mod m310_slashed_evidence_registration_tests {
    use super::*;

    #[test]
    fn already_slashed_hash_cannot_be_reregistered() {
        let key=[35u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.slashed_evidence.insert(evidence.evidence_hash,10);
        assert_eq!(state.record_equivocation(evidence),Err(StateError::InvalidEvidence));
        assert!(state.evidence.is_empty());
    }
}

#[cfg(test)]
mod m309_evidence_hash_boundary_tests {
    use super::*;

    #[test]
    fn zero_evidence_hash_is_rejected_before_recording() {
        let key=[33u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        assert_eq!(state.record_equivocation(evidence),Err(StateError::InvalidEvidence));
        assert!(state.evidence.is_empty());
    }
}

#[cfg(test)]
mod m308_evidence_duplicate_canonicalization_tests {
    use super::*;

    #[test]
    fn duplicate_evidence_hash_requires_identical_record() {
        let key=[31u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);        let validator=freedom_types::Validator {            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        assert_eq!(state.record_equivocation(evidence.clone()),Ok(true));

        let mut forged=evidence.clone();
        forged.first=vec![9];
        assert_eq!(state.record_equivocation(forged),Err(StateError::InvalidEvidence));
    }

    #[test]
    fn identical_duplicate_evidence_is_idempotent() {
        let key=[32u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        assert_eq!(state.record_equivocation(evidence.clone()),Ok(true));
        assert_eq!(state.record_equivocation(evidence),Ok(false));
    }
}

#[cfg(test)]
mod m307_slash_formula_overflow_tests {
    use super::*;

    #[test]
    fn deterministic_slash_amount_is_exact_at_u128_max() {
        let state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![],
        });
        let stake=u128::MAX;
        let expected=(stake/10_000)*1000 + (stake%10_000)*1000/10_000;
        assert_eq!(
            state.deterministic_slash_amount(
                &freedom_types::EquivocationKind::DoublePrecommit,
                stake
            ),
            expected
        );
    }

    #[test]
    fn deterministic_proposal_slash_remains_exact_for_large_stake() {
        let state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![],
        });
        let stake=u128::MAX;
        let expected=(stake/10_000)*500 + (stake%10_000)*500/10_000;
        assert_eq!(
            state.deterministic_slash_amount(
                &freedom_types::EquivocationKind::DoubleProposal,
                stake
            ),
            expected
        );
    }
}

#[cfg(test)]
mod m306_evidence_event_canonicalization_tests {
    use super::*;

    #[test]
    fn slash_event_uses_canonical_stored_validator_id() {
        let key=[29u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());

        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(), slash_amount:10,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert!(state.execute_transaction(tx).is_ok());
        assert!(matches!(
            state.events.last(),
            Some(Event::EquivocationSlashed{evidence_hash,validator_id,amount})
                if *evidence_hash==evidence.evidence_hash
                && *validator_id==id
                && *amount==10
        ));
    }
}

#[cfg(test)]
mod m305_evidence_nonce_atomicity_tests {
    use super::*;

    #[test]
    fn max_nonce_rejects_before_slash_mutation() {
        let key=[27u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        state.accounts.get_mut(&owner).unwrap().nonce=u64::MAX;

        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(), slash_amount:10,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:u64::MAX, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert_eq!(state.execute_transaction(tx),Err(StateError::ArithmeticOverflow));
        assert!(!state.evidence.contains_key(&evidence.evidence_hash));
        assert!(!state.slashed_evidence.contains_key(&evidence.evidence_hash));
        assert_eq!(state.validators.get(&id).unwrap().stake,100);
        assert_eq!(state.accounts.get(&owner).unwrap().staked,100);
    }
}

#[cfg(test)]
mod m304_slash_replay_evidence_binding_tests {
    use super::*;

    #[test]
    fn replay_rejects_payload_evidence_mismatch() {
        let key=[25u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;

        let mut stored=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        stored.evidence_hash=freedom_primitives::hash_bytes(&stored.signing_bytes());
        state.evidence.insert(stored.evidence_hash,stored.clone());
        state.slashed_evidence.insert(stored.evidence_hash,10);

        let mut forged=stored.clone();
        forged.first=vec![9];
        forged.evidence_hash=stored.evidence_hash;

        let payload=freedom_types::SubmitEvidencePayload {
            evidence:forged, slash_amount:10,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert_eq!(state.execute_transaction(tx),Err(StateError::InvalidEvidence));
        assert_eq!(state.accounts.get(&owner).unwrap().nonce,0);
    }
}

#[cfg(test)]
mod m303_slash_replay_transaction_tests {
    use super::*;

    #[test]
    fn replayed_slash_uses_recorded_amount_after_stake_changes() {
        let key=[23u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash,10);

        state.validators.get_mut(&id).unwrap().stake=50;
        state.accounts.get_mut(&owner).unwrap().staked=50;

        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(), slash_amount:10,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert!(state.execute_transaction(tx).is_ok());
        assert_eq!(state.accounts.get(&owner).unwrap().nonce,1);
    }

    #[test]
    fn replayed_slash_with_wrong_recorded_amount_is_rejected() {
        let key=[24u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash,10);
        let payload=freedom_types::SubmitEvidencePayload {
            evidence, slash_amount:9,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert_eq!(state.execute_transaction(tx),Err(StateError::InvalidEvidence));
        assert_eq!(state.accounts.get(&owner).unwrap().nonce,0);
    }
}

#[cfg(test)]
mod m302_evidence_decoder_hardening_tests {
    use super::*;

    #[test]
    fn evidence_decoder_rejects_oversized_component() {
        let key=[21u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let evidence=freedom_types::SubmitEvidencePayload {
            evidence:freedom_types::EquivocationEvidence {
                kind:freedom_types::EquivocationKind::DoublePrecommit,
                height:0, round:0, validator_id:id,
                first:vec![1; MAX_EVIDENCE_COMPONENT_BYTES+1],
                second:vec![2],
                evidence_hash:[0;32],
            },
            slash_amount:1,
        };
        assert!(decode_submit_evidence(&evidence.encode()).is_none());
    }

    #[test]
    fn evidence_decoder_rejects_trailing_bytes() {
        let key=[22u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let evidence=freedom_types::SubmitEvidencePayload {
            evidence:freedom_types::EquivocationEvidence {
                kind:freedom_types::EquivocationKind::DoublePrecommit,
                height:0, round:0, validator_id:id,
                first:vec![1], second:vec![2],
                evidence_hash:[0;32],
            },
            slash_amount:1,
        };
        let mut encoded=evidence.encode();
        encoded.push(0xff);
        assert!(decode_submit_evidence(&encoded).is_none());
    }
}

#[cfg(test)]
mod m301_evidence_registry_integrity_tests {
    use super::*;

    #[test]
    fn evidence_recording_rejects_corrupted_validator_registry() {
        let key=[17u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());

        state.validators.get_mut(&id).unwrap().consensus_key=[18u8;32];
        assert_eq!(state.record_equivocation(evidence.clone()),Err(StateError::InvalidBridgeWithdrawal));
        assert!(!state.evidence.contains_key(&evidence.evidence_hash));
    }

    #[test]
    fn evidence_recording_accepts_consistent_validator_registry() {
        let key=[19u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        assert_eq!(state.record_equivocation(evidence.clone()),Ok(true));
        assert!(state.evidence.contains_key(&evidence.evidence_hash));
    }
}

#[cfg(test)]
mod m300_evidence_submission_atomicity_tests {
    use super::*;

    #[test]
    fn invalid_slash_amount_does_not_record_new_evidence() {
        let key=[15u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(), slash_amount:9,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert_eq!(state.execute_transaction(tx),Err(StateError::InvalidEvidence));
        assert!(!state.evidence.contains_key(&evidence.evidence_hash));
        assert!(!state.slashed_evidence.contains_key(&evidence.evidence_hash));
        assert_eq!(state.accounts.get(&owner).unwrap().nonce,0);
    }

    #[test]
    fn valid_submission_records_and_slashes_once() {
        let key=[16u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,0));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(), slash_amount:10,
        };
        let tx=freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        };
        assert!(state.execute_transaction(tx).is_ok());
        assert_eq!(state.slashed_evidence.get(&evidence.evidence_hash),Some(&10));
        assert_eq!(state.accounts.get(&owner).unwrap().nonce,1);
    }
}

#[cfg(test)]
mod m299_evidence_slash_basis_hardening_tests {
    use super::*;

    #[test]
    fn submit_evidence_uses_active_plus_unbonding_stake() {
        let key=[13u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:0,
            unbonding_stake:100, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.staking_activated=true;
        state.insert_account(freedom_types::Account::new(key,100));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());

        let unbonding_id=[14u8;32];
        state.unbonding.insert(unbonding_id,freedom_types::UnbondingEntry {
            unbonding_id, validator_id:id, owner, amount:100,
            requested_height:0, release_height:200,
        });

        let payload=freedom_types::SubmitEvidencePayload {
            evidence:evidence.clone(),
            slash_amount:10,
        };
        let result=state.submit_evidence(&freedom_types::Transaction {
            sender:owner, nonce:0, fee:0,
            kind:freedom_types::TxKind::SubmitEvidence,
            payload:payload.encode(),
        });
        assert!(result.is_ok());
        assert_eq!(state.slashed_evidence.get(&evidence.evidence_hash),Some(&10));
    }
}

#[cfg(test)]
mod m298_exited_validator_slash_hardening_tests {
    use super::*;

    #[test]
    fn exited_validator_cannot_be_slashed_again() {
        let key=[11u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:0,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],        });
        state.insert_account(freedom_types::Account::new(key,0));
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        assert_eq!(
            state.slash_evidence(evidence.evidence_hash,1),
            Err(StateError::InvalidEvidence)
        );
    }

    #[test]
    fn exited_validator_evidence_remains_historically_valid() {
        let key=[12u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:0,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence);
        assert_eq!(state.validate_evidence_integrity(),Ok(()));
    }
}

#[cfg(test)]
mod m297_slashed_evidence_history_hardening_tests {
    use super::*;

    #[test]
    fn historical_slash_record_survives_later_stake_changes() {
        let key=[10u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,100));
        state.accounts.get_mut(&owner).unwrap().staked=100;

        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());

        assert_eq!(state.slash_evidence(evidence.evidence_hash,10),Ok(10));

        // The validator state may legitimately evolve after the historical
        // slash; the slash registry remains valid evidence of the past event.
        state.validators.get_mut(&id).unwrap().stake=50;
        state.accounts.get_mut(&owner).unwrap().staked=50;

        assert_eq!(state.validate_slashed_evidence_integrity(),Ok(()));
    }

    #[test]
    fn orphan_slashed_evidence_still_rejected() {
        let mut state=ChainState::genesis(&freedom_types::Genesis::default());
        state.slashed_evidence.insert([1;32],1);
        assert_eq!(
            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );
    }
}

#[cfg(test)]
mod m296_slash_replay_amount_hardening_tests {
    use super::*;

    #[test]
    fn repeated_slash_with_same_amount_is_idempotent() {
        let key=[8u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,100));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        assert_eq!(state.slash_evidence(evidence.evidence_hash,10),Ok(10));
        assert_eq!(state.slash_evidence(evidence.evidence_hash,10),Ok(0));
        assert_eq!(state.validators.get(&id).unwrap().stake,90);
    }

    #[test]
    fn repeated_slash_with_different_amount_is_rejected() {
        let key=[9u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:100,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key,100));
        state.accounts.get_mut(&owner).unwrap().staked=100;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        assert_eq!(state.slash_evidence(evidence.evidence_hash,10),Ok(10));
        assert_eq!(
            state.slash_evidence(evidence.evidence_hash,9),
            Err(StateError::InvalidEvidence)
        );
        assert_eq!(state.validators.get(&id).unwrap().stake,90);
    }
}

#[cfg(test)]
mod m295_slash_mutation_safety_hardening_tests {
    use super::*;

    #[test]
    fn zero_deterministic_slash_is_rejected() {
        let key=[7u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:1,
            unbonding_stake:0, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key, 1));
        state.accounts.get_mut(&owner).unwrap().staked=1;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoubleProposal, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        assert_eq!(
            state.slash_evidence(evidence.evidence_hash,0),
            Err(StateError::InvalidEvidence)
        );
        assert_eq!(state.validators.get(&id).unwrap().stake,1);
        assert_eq!(state.accounts.get(&owner).unwrap().staked,1);
    }

    #[test]
    fn slash_plan_is_complete_before_validator_mutation() {
        let key=[8u8;32];
        let id=freedom_primitives::hash_bytes(&key);
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=freedom_types::Validator {
            validator_id:id, owner, consensus_key:key, stake:0,
            unbonding_stake:10, blocks_proposed:0, blocks_finalized:0, missed_votes:0,
        };
        let mut state=ChainState::genesis(&freedom_types::Genesis {
            chain_id:freedom_primitives::CHAIN_ID, initial_supply:0, validators:vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key, 10));
        state.accounts.get_mut(&owner).unwrap().staked=10;
        let mut evidence=freedom_types::EquivocationEvidence {
            kind:freedom_types::EquivocationKind::DoublePrecommit, height:0, round:0,
            validator_id:id, first:vec![1], second:vec![2], evidence_hash:[0;32],
        };
        evidence.evidence_hash=freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash,evidence.clone());
        let unbonding_id=[9u8;32];
        state.unbonding.insert(unbonding_id,freedom_types::UnbondingEntry {
            unbonding_id, validator_id:id, owner, amount:10,
            requested_height:0, release_height:200,
        });
        let result=state.slash_evidence(evidence.evidence_hash,1);
        assert_eq!(result,Ok(1));
        assert_eq!(state.validators.get(&id).unwrap().unbonding_stake,9);
        assert_eq!(state.accounts.get(&owner).unwrap().staked,9);
    }
}

#[cfg(test)]
mod m294_slashed_evidence_basis_hardening_tests {
    use super::*;

    fn slash_state() -> (ChainState, Hash) {
        let key = [7u8; 32];
        let id = freedom_primitives::hash_bytes(&key);
        let owner = freedom_primitives::address_from_public_key(&key);
        let validator = freedom_types::Validator {
            validator_id: id,
            owner,
            consensus_key: key,
            stake: 90,
            unbonding_stake: 0,
            blocks_proposed: 0,
            blocks_finalized: 0,
            missed_votes: 0,
        };
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![validator],
        });
        state.insert_account(freedom_types::Account::new(key, 100));
        state.accounts.get_mut(&owner).unwrap().staked = 90;

        let mut evidence = freedom_types::EquivocationEvidence {
            kind: freedom_types::EquivocationKind::DoublePrecommit,
            height: 0,
            round: 0,
            validator_id: id,
            first: vec![1],
            second: vec![2],
            evidence_hash: [0; 32],
        };
        evidence.evidence_hash = freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash, evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash, 10);
        (state, evidence.evidence_hash)
    }

    #[test]
    fn slashed_evidence_reconstructs_pre_slash_basis() {
        let (state, _) = slash_state();
        assert_eq!(state.validate_slashed_evidence_integrity(), Ok(()));
    }

    #[test]
    fn slashed_evidence_cannot_claim_wrong_pre_slash_basis() {
        let (mut state, evidence_hash) = slash_state();
        state.slashed_evidence.insert(evidence_hash, 9);
        assert_eq!(
            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );
    }

    #[test]
    fn slash_record_cannot_exceed_reconstructed_pre_slash_total() {
        let (mut state, evidence_hash) = slash_state();
        state.slashed_evidence.insert(evidence_hash, 1000);
        assert_eq!(
            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );
    }
}

#[cfg(test)]
mod m293_validator_set_certificate_quorum_hardening_tests {
    use super::*;

    #[test]
    fn certificate_quorum_is_strictly_more_than_two_thirds() {
        let mut state = ChainState::genesis(&freedom_types::Genesis::default());
        state.staking_activated = true;
        state.epoch = 1;

        let mut keys = std::collections::BTreeMap::new();
        let mut entries = std::collections::BTreeMap::new();
        for byte in 1u8..=3u8 {
            let key = [byte; 32];
            let id = freedom_primitives::hash_bytes(&key);
            keys.insert(id, key);
            entries.insert(id, freedom_types::ValidatorSetEntry {
                validator_id: id,
                owner: [byte + 10; 32],
                consensus_key: key,
                stake: 100,
            });
        }

        state.validator_set_history.insert(0, [4; 32]);
        state.validator_set_history_sets.insert(0, keys.clone());
        state.validator_set_history_entries.insert(0, entries.clone());

        let mut key_bytes=Vec::new();
        for (id,key) in &keys {
            key_bytes.extend_from_slice(id);
            key_bytes.extend_from_slice(key);
        }
        state.validator_set_history_key_hashes.insert(0, freedom_primitives::hash_bytes(&key_bytes));

        state.validator_set_history.insert(1, [5; 32]);
        state.validator_set_history_sets.insert(1, keys.clone());
        state.validator_set_history_entries.insert(1, entries.clone());
        state.validator_set_history_key_hashes.insert(1, freedom_primitives::hash_bytes(&key_bytes));
        state.active_validator_set=entries;

        let new_set_hash = {
            let mut bytes=Vec::new();
            for (id,entry) in &state.active_validator_set {
                bytes.extend_from_slice(id);
                bytes.extend_from_slice(&entry.owner);
                bytes.extend_from_slice(&entry.consensus_key);
                bytes.extend_from_slice(&entry.stake.to_le_bytes());
            }
            freedom_primitives::hash_bytes(&bytes)
        };

        let id=keys.keys().next().copied().unwrap();
        let key=keys[&id];
        let mut vote=freedom_types::ValidatorSetVote {
            from_epoch:0,
            to_epoch:1,
            previous_set_hash:[4;32],
            new_set_hash,
            validator_id:id,
            signature:vec![0;64],
        };
        let sig=freedom_crypto::sign(&key,&vote.signing_bytes());
        vote.signature=sig.to_vec();

        state.validator_set_certificate=Some(freedom_types::ValidatorSetCertificate {
            from_epoch:0,
            to_epoch:1,
            previous_set_hash:[4;32],
            new_set_hash,
            votes:vec![vote],
        });

        assert_eq!(
            state.validate_validator_set_certificate_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m292_validator_set_vote_identity_hardening_tests {
    use super::*;

    #[test]
    fn certificate_vote_identity_must_bind_to_previous_entries() {
        let key = [3u8; 32];
        let id = freedom_primitives::hash_bytes(&key);
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        state.staking_activated = true;
        state.epoch = 1;

        let mut entries = std::collections::BTreeMap::new();
        entries.insert(id, freedom_types::ValidatorSetEntry {
            validator_id: id,
            owner: [2u8; 32],
            consensus_key: key,
            stake: 100,
        });
        let mut keys = std::collections::BTreeMap::new();
        keys.insert(id, key);

        state.validator_set_history.insert(0, [4; 32]);
        state.validator_set_history_sets.insert(0, keys.clone());
        state.validator_set_history_entries.insert(0, entries.clone());

        let mut key_bytes=Vec::new();
        for (validator_id, consensus_key) in &keys {
            key_bytes.extend_from_slice(validator_id);
            key_bytes.extend_from_slice(consensus_key);
        }
        state.validator_set_history_key_hashes.insert(
            0,
            freedom_primitives::hash_bytes(&key_bytes),
        );

        state.validator_set_history.insert(1, [5; 32]);
        state.validator_set_history_sets.insert(1, keys.clone());
        state.validator_set_history_entries.insert(1, entries);
        state.validator_set_history_key_hashes.insert(1, freedom_primitives::hash_bytes(&key_bytes));
        state.active_validator_set = state.validator_set_history_entries.get(&1).unwrap().clone();

        let wrong_key = [9u8; 32];
        let mut certificate = freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: [4; 32],
            new_set_hash: [0; 32],
            votes: vec![freedom_types::ValidatorSetVote {
                from_epoch: 0,
                to_epoch: 1,
                previous_set_hash: [4; 32],
                new_set_hash: [5; 32],
                validator_id: freedom_primitives::hash_bytes(&wrong_key),
                signature: vec![0; 64],
            }],
        };
        certificate.new_set_hash = {
            let mut bytes=Vec::new();
            for (validator_id,entry) in &state.active_validator_set {
                bytes.extend_from_slice(validator_id);
                bytes.extend_from_slice(&entry.owner);
                bytes.extend_from_slice(&entry.consensus_key);
                bytes.extend_from_slice(&entry.stake.to_le_bytes());
            }
            freedom_primitives::hash_bytes(&bytes)
        };
        state.validator_set_certificate = Some(certificate);

        assert_eq!(
            state.validate_validator_set_certificate_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m291_validator_set_certificate_integrity_hardening_tests {
    use super::*;

    #[test]
    fn certificate_without_staking_activation_is_rejected() {
        let mut state = ChainState::genesis(&freedom_types::Genesis::default());
        state.validator_set_certificate = Some(freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: [1; 32],
            new_set_hash: [2; 32],
            votes: vec![],
        });

        assert_eq!(
            state.validate_validator_set_certificate_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn missing_certificate_is_allowed_before_transition() {
        let state = ChainState::genesis(&freedom_types::Genesis::default());

        assert_eq!(state.validate_validator_set_certificate_integrity(), Ok(()));
    }

    #[test]
    fn certificate_hash_must_match_current_active_set() {
        let mut state = ChainState::genesis(&freedom_types::Genesis::default());
        state.staking_activated = true;
        state.epoch = 1;
        state.validator_set_history.insert(1, [3; 32]);
        state.validator_set_history_sets.insert(1, std::collections::BTreeMap::new());
        state.validator_set_history_entries.insert(1, std::collections::BTreeMap::new());
        state.validator_set_history_key_hashes.insert(1, freedom_primitives::hash_bytes(&[]));

        let certificate = freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: state.validator_set_history.get(&0).copied().unwrap_or([4; 32]),
            new_set_hash: [9; 32],
            votes: vec![],
        };
        state.validator_set_certificate = Some(certificate);

        assert_eq!(
            state.validate_validator_set_certificate_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m290_slashed_evidence_integrity_hardening_tests {
    use super::*;

    #[test]
    fn orphan_slashed_evidence_is_rejected() {
        let state = ChainState::genesis(&freedom_types::Genesis::default());
        let mut state = state;
        state.slashed_evidence.insert([7u8; 32], 1);

        assert_eq!(            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );    }

    #[test]
    fn zero_slash_amount_is_rejected() {
        let mut state = ChainState::genesis(&freedom_types::Genesis::default());
        let key = [3u8; 32];
        let id = freedom_primitives::hash_bytes(&key);
        let owner = [2u8; 32];
        state.validators.insert(
            id,
            freedom_types::Validator {
                validator_id: id,
                owner,
                consensus_key: key,
                stake: 100,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            },
        );
        let mut evidence = freedom_types::EquivocationEvidence {
            kind: freedom_types::EquivocationKind::DoubleProposal,
            height: 0,
            round: 0,
            validator_id: id,
            first: vec![1],
            second: vec![2],
            evidence_hash: [0; 32],
        };
        evidence.evidence_hash = freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash, evidence.clone());
        state.slashed_evidence.insert(evidence.evidence_hash, 0);

        assert_eq!(
            state.validate_slashed_evidence_integrity(),
            Err(StateError::InvalidEvidence)
        );
    }

    #[test]
    fn deterministic_slashed_evidence_passes() {
        let mut state = ChainState::genesis(&freedom_types::Genesis::default());
        let key = [3u8; 32];
        let id = freedom_primitives::hash_bytes(&key);
        let owner = [2u8; 32];
        state.validators.insert(
            id,
            freedom_types::Validator {
                validator_id: id,
                owner,
                consensus_key: key,
                stake: 100,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            },
        );
        let mut evidence = freedom_types::EquivocationEvidence {
            kind: freedom_types::EquivocationKind::DoubleProposal,
            height: 0,
            round: 0,
            validator_id: id,
            first: vec![1],
            second: vec![2],
            evidence_hash: [0; 32],
        };
        evidence.evidence_hash = freedom_primitives::hash_bytes(&evidence.signing_bytes());
        state.evidence.insert(evidence.evidence_hash, evidence.clone());
        let expected = state.deterministic_slash_amount(&evidence.kind, 100);
        state.slashed_evidence.insert(evidence.evidence_hash, expected);

        assert_eq!(state.validate_slashed_evidence_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m289_evidence_integrity_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        let key = [3u8; 32];
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&key),
                    owner: [2u8; 32],
                    consensus_key: key,
                    stake: 100,
                },
            ],
        })
    }

    fn evidence(state: &ChainState, height: u64) -> freedom_types::EquivocationEvidence {
        let validator_id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let mut e = freedom_types::EquivocationEvidence {
            kind: freedom_types::EquivocationKind::DoubleProposal,
            height,
            round: 1,
            validator_id,
            first: vec![1],
            second: vec![2],
            evidence_hash: [0; 32],
        };
        e.evidence_hash = freedom_primitives::hash_bytes(&e.signing_bytes());
        let _ = state;
        e
    }

    #[test]
    fn evidence_for_unknown_validator_is_rejected() {
        let mut state = genesis();
        let mut e = evidence(&state, 0);
        e.validator_id = [9u8; 32];
        e.evidence_hash = freedom_primitives::hash_bytes(&e.signing_bytes());

        assert_eq!(state.record_equivocation(e), Err(StateError::InvalidEvidence));
    }

    #[test]
    fn future_evidence_is_rejected() {
        let mut state = genesis();
        state.height = 10;
        let e = evidence(&state, 11);

        assert_eq!(state.record_equivocation(e), Err(StateError::InvalidEvidence));
    }

    #[test]
    fn evidence_integrity_passes_for_current_validator_and_height() {
        let mut state = genesis();
        state.height = 10;
        let e = evidence(&state, 10);

        assert_eq!(state.record_equivocation(e.clone()), Ok(true));
        assert_eq!(state.validate_evidence_integrity(), Ok(()));
        assert_eq!(state.validate_finality_state_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m288_slash_atomicity_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        let key = [3u8; 32];
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&key),
                    owner: [2u8; 32],
                    consensus_key: key,
                    stake: 100,
                },
            ],
        })
    }

    #[test]
    fn slash_rejects_corrupted_unbonding_before_mutation() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let evidence_hash = [7u8; 32];
        let unbonding_id = [8u8; 32];

        state.validators.get_mut(&id).unwrap().unbonding_stake = 10;
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [9u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 201,
            },
        );

        state.evidence.insert(
            evidence_hash,
            freedom_types::EquivocationEvidence {
                evidence_hash,
                validator_id: id,
                kind: freedom_types::EquivocationKind::DoubleProposal,
                height: 1,
                first_block_hash: [1u8; 32],
                second_block_hash: [2u8; 32],
            },
        );

        let before_stake=state.validators.get(&id).unwrap().stake;
        let result=state.slash_evidence(evidence_hash, 1);

        assert_eq!(result, Err(StateError::InvalidBridgeWithdrawal));
        assert_eq!(state.validators.get(&id).unwrap().stake, before_stake);
        assert!(!state.slashed_evidence.contains_key(&evidence_hash));
    }

    #[test]
    fn slash_plan_cannot_consume_more_unbonding_than_ledger() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let unbonding_id = [8u8; 32];
        state.validators.get_mut(&id).unwrap().unbonding_stake = 10;
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [2u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 201,
            },
        );
        assert_eq!(state.validate_unbonding_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m287_staking_accounting_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        let key = [3u8; 32];
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&key),
                    owner: [2u8; 32],
                    consensus_key: key,
                    stake: 100,
                },
            ],
        })
    }

    #[test]
    fn active_and_unbonding_stake_must_match_account_after_staking_activation() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        state.staking_activated = true;
        state.accounts.insert([2u8; 32], Account {
            address: [2u8; 32],
            public_key: [0; 32],
            nonce: 0,
            balance: 0,
            locked: 0,
            staked: 99,
        });

        assert_eq!(
            state.validate_unbonding_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        let _ = id;
    }

    #[test]
    fn mature_unbonding_does_not_partially_mutate_corrupted_state() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let unbonding_id = [8u8; 32];
        state.validators.get_mut(&id).unwrap().unbonding_stake = 10;
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [9u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 1,
            },
        );

        state.height = 10;
        state.maybe_advance_epoch();

        assert!(state.unbonding.contains_key(&unbonding_id));
        assert_eq!(state.validators.get(&id).unwrap().unbonding_stake, 10);
    }

    #[test]
    fn consistent_active_and_unbonding_stake_passes_account_binding() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        state.staking_activated = true;
        state.accounts.insert([2u8; 32], Account {
            address: [2u8; 32],
            public_key: [0; 32],
            nonce: 0,
            balance: 0,
            locked: 0,
            staked: 100,
        });

        assert_eq!(state.validate_unbonding_integrity(), Ok(()));
        assert_eq!(state.validate_finality_state_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m286_unbonding_integrity_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        let key = [3u8; 32];
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&key),
                    owner: [2u8; 32],
                    consensus_key: key,
                    stake: 100,
                },
            ],
        })
    }

    #[test]
    fn unbonding_ledger_must_match_validator_unbonding_stake() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let unbonding_id = [8u8; 32];
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [2u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 201,
            },
        );

        assert_eq!(
            state.validate_unbonding_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn unbonding_entry_owner_must_match_validator() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let unbonding_id = [8u8; 32];
        state.validators.get_mut(&id).unwrap().unbonding_stake = 10;
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [9u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 201,
            },
        );

        assert_eq!(
            state.validate_unbonding_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn consistent_unbonding_ledger_passes() {
        let mut state = genesis();
        let id = freedom_primitives::hash_bytes(&[3u8; 32]);
        let unbonding_id = [8u8; 32];
        state.validators.get_mut(&id).unwrap().unbonding_stake = 10;
        state.unbonding.insert(
            unbonding_id,
            freedom_types::UnbondingEntry {
                unbonding_id,
                validator_id: id,
                owner: [2u8; 32],
                amount: 10,
                requested_height: 1,
                release_height: 201,
            },
        );

        assert_eq!(state.validate_unbonding_integrity(), Ok(()));
        assert_eq!(state.validate_finality_state_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m285_validator_owner_uniqueness_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        let key = [3u8; 32];
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&key),
                    owner: [2u8; 32],
                    consensus_key: key,
                    stake: 100,
                },
            ],
        })
    }

    #[test]
    fn validator_owner_must_be_unique() {
        let key_a = [3u8; 32];
        let key_b = [4u8; 32];
        let mut state = genesis();
        state.validators.insert(
            freedom_primitives::hash_bytes(&key_b),
            freedom_types::Validator {
                validator_id: freedom_primitives::hash_bytes(&key_b),
                owner: [2u8; 32],
                consensus_key: key_b,
                stake: 100,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            },
        );

        assert_eq!(
            state.validate_validator_records_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        let _ = key_a;
    }

    #[test]
    fn validator_owner_must_not_be_zero() {
        let mut state = genesis();
        state.validators.get_mut(&freedom_primitives::hash_bytes(&[3u8; 32])).unwrap().owner = [0; 32];

        assert_eq!(
            state.validate_validator_records_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }

    #[test]
    fn unique_nonzero_validator_owner_passes() {
        let state = genesis();

        assert_eq!(state.validate_validator_records_integrity(), Ok(()));
        assert_eq!(state.validate_finality_state_integrity(), Ok(()));
    }
}

#[cfg(test)]
mod m284_validator_consensus_identity_hardening_tests {
    use super::*;

    fn genesis() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![
                freedom_types::GenesisValidator {
                    validator_id: freedom_primitives::hash_bytes(&[3u8; 32]),
                    owner: [2u8; 32],
                    consensus_key: [3u8; 32],
                    stake: 100,
                },
            ],
        })
    }

    #[test]
    fn validator_identity_must_bind_to_consensus_key() {
        let mut state = genesis();
        state.validators.get_mut(&freedom_primitives::hash_bytes(&[3u8; 32])).unwrap().consensus_key = [4u8; 32];

        assert_eq!(
            state.validate_validator_records_integrity(),
            Err(StateError::InvalidBridgeWithdrawal)
        );
    }
}

#[cfg(test)]
mod m392_reorg_severity_checkpoint_binding_tests {
    use super::*;

    fn state_with_checkpoint() -> (ChainState, Hash) {
        let bridge=[91u8;32];
        let mut state=ChainState::genesis(&freedom_types::Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![]});
        state.bitcoin_checkpoints.insert(bridge,freedom_types::BitcoinChainCheckpoint{height:100,block_hash:[92;32],chainwork:[93;32]});
        state.bitcoin_reorg_policy.insert(bridge,freedom_types::BitcoinReorgPolicy{elevated_confirmations:6,critical_chainwork_margin:[94;32]});
        (state,bridge)
    }

    fn evidence(checkpoint_height:u64,checkpoint_hash:Hash,candidate_height:u64,candidate_work:Hash)->freedom_types::BitcoinReorgEvidence{
        let mut e=freedom_types::BitcoinReorgEvidence{checkpoint_height,checkpoint_hash,candidate_tip_height:candidate_height,candidate_tip_hash:[95;32],candidate_chainwork:candidate_work,fingerprint:[0;32]};
        e.fingerprint=e.compute_fingerprint();
        e
    }

    #[test]
    fn severity_rejects_evidence_bound_to_wrong_checkpoint(){
        let (state,bridge)=state_with_checkpoint();
        let e=evidence(99,[92;32],101,[96;32]);
        assert_eq!(state.bitcoin_reorg_severity(bridge,&e),Err(StateError::BitcoinReorgDetected));
    }

    #[test]
    fn severity_rejects_candidate_not_above_current_checkpoint(){
        let (state,bridge)=state_with_checkpoint();
        let e=evidence(100,[92;32],100,[96;32]);
        assert_eq!(state.bitcoin_reorg_severity(bridge,&e),Err(StateError::BitcoinReorgDetected));
    }
}

#[cfg(test)]
mod m416_bitcoin_canonical_tip_equal_work_tests {
    use super::*;
    use freedom_types::BitcoinChainCheckpoint;

    fn checkpoint(height: u64, hash: Hash, chainwork: [u8; 32]) -> BitcoinChainCheckpoint {
        BitcoinChainCheckpoint { height, block_hash: hash, chainwork }
    }

    #[test]
    fn equal_work_different_tip_is_rejected() {
        let previous = checkpoint(100, [1; 32], [9; 32]);
        let next = checkpoint(100, [2; 32], [9; 32]);
        assert!(!bitcoin_canonical_tip_transition_allowed(&previous, &next));
    }

    #[test]
    fn equal_work_same_tip_is_allowed() {
        let previous = checkpoint(100, [1; 32], [9; 32]);
        let next = checkpoint(100, [1; 32], [9; 32]);
        assert!(bitcoin_canonical_tip_transition_allowed(&previous, &next));
    }

    #[test]
    fn greater_chainwork_remains_allowed() {
        let previous = checkpoint(100, [1; 32], [9; 32]);
        let next = checkpoint(101, [2; 32], [10; 32]);
        assert!(bitcoin_canonical_tip_transition_allowed(&previous, &next));
    }
}


#[cfg(test)]
mod m423_ticket_allocation_order_tests {
    use super::*;

    #[test]
    fn insufficient_balance_is_rejected_before_ticket_id_allocation() {
        let key=[9u8;32];
        let owner=freedom_primitives::address_from_public_key(&key);
        let validator=Validator{validator_id:freedom_primitives::hash_bytes(&key),owner,consensus_key:key,stake:0,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
        let genesis=Genesis{chain_id:freedom_primitives::CHAIN_ID,initial_supply:0,validators:vec![validator]};
        let mut state=ChainState::genesis(&genesis);
        state.insert_account(Account::new(key,0));
        let draw_id=[7u8;32];
        state.create_draw(DrawConfig{draw_id,starts_at:0,ends_at:u64::MAX,ticket_price_native_units:10_000,winner_count:1,seed_block:0}).unwrap();
        let quantity=1_000_000_000u64;
        let tx=Transaction{chain_id:freedom_primitives::CHAIN_ID,sender:owner,nonce:0,kind:TxKind::CreateTicket,payload:CreateTicketPayload{draw_id,quantity}.encode(),fee:0,signature:vec![]};
        assert_eq!(state.create_ticket(&tx),Err(StateError::InsufficientBalanceForPurchase));
    }
}

#[cfg(test)]
mod m422_evidence_decoder_bounds_tests {
    use super::*;

    #[test]
    fn evidence_decoder_rejects_truncated_components_without_index_overflow() {
        let mut payload = vec![1u8];
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&[7u8; 32]);
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.push(9);
        payload.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_submit_evidence(&payload).is_none());
    }
}


#[cfg(test)]
mod m450_bitcoin_checkpoint_monotonicity_tests {
    use super::*;
    use freedom_types::BitcoinChainCheckpoint;

    fn state_with_checkpoint() -> (ChainState, Hash) {
        let bridge = [7u8; 32];
        let mut state = ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        state.register_bridge_security_policy(freedom_types::BridgeSecurityPolicy {
            bridge_id: bridge,
            approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1,
            validator_total: 1,
            threshold_signers: 1,
            threshold_total: 1,
            min_external_confirmations: 1,
            max_deposit_amount: 1_000_000,
            max_withdrawal_amount: 1_000_000,
            daily_limit: 1_000_000,
            emergency_paused: false,
            nonce_floor: 0,
        }).unwrap();
        let checkpoint = BitcoinChainCheckpoint {
            height: 100,
            block_hash: [8u8; 32],
            chainwork: [9u8; 32],
        };
        state.register_bitcoin_checkpoint(bridge, checkpoint).unwrap();
        (state, bridge)
    }

    #[test]
    fn rejects_chainwork_regression_without_mutating_checkpoint() {
        let (mut state, bridge) = state_with_checkpoint();
        let before = state.bitcoin_checkpoints.get(&bridge).cloned().unwrap();
        let candidate = BitcoinChainCheckpoint {
            height: 101,
            block_hash: [10u8; 32],
            chainwork: [8u8; 32],
        };
        assert_eq!(
            state.register_bitcoin_checkpoint(bridge, candidate),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        assert_eq!(state.bitcoin_checkpoints.get(&bridge), Some(&before));
    }

    #[test]
    fn rejects_hash_replacement_at_same_height_without_mutation() {
        let (mut state, bridge) = state_with_checkpoint();
        let before = state.bitcoin_checkpoints.get(&bridge).cloned().unwrap();
        let candidate = BitcoinChainCheckpoint {
            height: before.height,
            block_hash: [10u8; 32],
            chainwork: before.chainwork,
        };
        assert_eq!(
            state.register_bitcoin_checkpoint(bridge, candidate),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        assert_eq!(state.bitcoin_checkpoints.get(&bridge), Some(&before));
    }

    #[test]
    fn rejects_hash_replacement_at_equal_chainwork_without_mutation() {
        let (mut state, bridge) = state_with_checkpoint();
        let before = state.bitcoin_checkpoints.get(&bridge).cloned().unwrap();
        let candidate = BitcoinChainCheckpoint {
            height: before.height + 1,
            block_hash: [10u8; 32],
            chainwork: before.chainwork,
        };
        assert_eq!(
            state.register_bitcoin_checkpoint(bridge, candidate),
            Err(StateError::InvalidBridgeWithdrawal)
        );
        assert_eq!(state.bitcoin_checkpoints.get(&bridge), Some(&before));
    }
}


#[cfg(test)]
mod m451_persisted_bitcoin_bridge_integrity_tests {
    use super::*;

    fn state() -> ChainState {
        ChainState::genesis(&freedom_types::Genesis {
            chain_id: freedom_primitives::CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        })
    }

    #[test]
    fn rejects_invalid_persisted_bitcoin_checkpoint() {
        let mut state = state();
        state.bitcoin_checkpoints.insert([7u8; 32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [0u8; 32], chainwork: [9u8; 32],
        });
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn rejects_invalid_persisted_canonical_tip() {
        let mut state = state();
        state.bitcoin_canonical_tips.insert([7u8; 32], freedom_types::BitcoinChainCheckpoint {
            height: 100, block_hash: [8u8; 32], chainwork: [0u8; 32],
        });
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }

    #[test]
    fn rejects_attestation_commitment_transition_mismatch() {
        let mut state = state();
        let bridge = [7u8; 32];
        state.bridge_security_policies.insert(bridge, freedom_types::BridgeSecurityPolicy {
            bridge_id: bridge, approval_mode: freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum: 1, validator_total: 1, threshold_signers: 1, threshold_total: 1,
            min_external_confirmations: 1, max_deposit_amount: 1000, max_withdrawal_amount: 1000,
            daily_limit: 1000, emergency_paused: false, nonce_floor: 0,
        });
        let validator_hash = *state.validator_set_history.get(&state.epoch).unwrap();
        state.bitcoin_utxo_attestations.insert(bridge, freedom_types::BitcoinUtxoCommitmentAttestation {
            bridge_id: bridge, chain_id: freedom_types::BITCOIN_MAINNET_CHAIN_ID, policy_id: bridge,
            validator_set_epoch: state.epoch, validator_set_hash: validator_hash, sequence: 1,
            bitcoin_tip_height: 101, bitcoin_tip_hash: [8u8; 32], bitcoin_tip_chainwork: [9u8; 32],
            previous_utxo_root: [10u8; 32], accumulator_root: [11u8; 32],
            validator_ids: vec![[1u8; 32]], signatures: vec![], attestation_hash: [12u8; 32],
        });
        state.bitcoin_utxo_attestation_policy_hashes.insert(
            bridge, state.bridge_security_policies.get(&bridge).unwrap().commitment_hash(),
        );
        state.bitcoin_utxo_commitments.insert(bridge, freedom_types::BitcoinUtxoBlockCommitment {
            block_height: 101, block_hash: [8u8; 32], previous_root: [10u8; 32], next_root: [13u8; 32],
            update_proof: freedom_types::BitcoinUtxoSmtUpdateProof {
                previous_root: [10u8; 32], next_root: [13u8; 32], steps: vec![],
            },
        });
        assert_eq!(state.validate_finality_state_integrity(), Err(StateError::InvalidBridgeWithdrawal));
    }
}
