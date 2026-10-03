fn decode_submit_evidence(b:&[u8])->Option<freedom_types::SubmitEvidencePayload>{
 let mut i=0usize;
 let kind=match *b.get(i)?{1=>freedom_types::EquivocationKind::DoublePrevote,2=>freedom_types::EquivocationKind::DoublePrecommit,3=>freedom_types::EquivocationKind::DoubleProposal,_=>return None}; i+=1;
 let height=u64::from_le_bytes(b.get(i..i+8)?.try_into().ok()?); i+=8;
 let round=u64::from_le_bytes(b.get(i..i+8)?.try_into().ok()?); i+=8;
 let mut validator_id=[0u8;32]; validator_id.copy_from_slice(b.get(i..i+32)?); i+=32;
 let n1=u32::from_le_bytes(b.get(i..i+4)?.try_into().ok()?) as usize;i+=4;
 let first=b.get(i..i+n1)?.to_vec();i+=n1;
 let n2=u32::from_le_bytes(b.get(i..i+4)?.try_into().ok()?) as usize;i+=4;
 let second=b.get(i..i+n2)?.to_vec();i+=n2;
 let mut evidence_hash=[0u8;32];evidence_hash.copy_from_slice(b.get(i..i+32)?);i+=32;
 let mut slash=[0u8;16];slash.copy_from_slice(b.get(i..i+16)?);
 Some(freedom_types::SubmitEvidencePayload{evidence:freedom_types::EquivocationEvidence{kind,height,round,validator_id,first,second,evidence_hash},slash_amount:u128::from_le_bytes(slash)})
}
use std::collections::{BTreeMap,BTreeSet};
use serde::{Deserialize, Serialize};
use freedom_primitives::{address_from_public_key, Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, RegisterNodePayload, SubmitContributionPayload, BillingPayload, PurchasePayload, CreateTicketPayload, DrawConfig, DrawResult, Event, Transaction, TxKind, TransferPayload, Validator, UnbondingEntry, AssetDefinition, AssetType, AssetStatus, BridgeMessage, BRIDGE_MESSAGE_PROTOCOL_VERSION, BridgeSecurityPolicy, BridgeApprovalSet, BridgeRateLimitUsage, BridgeNonceKey, BridgeDirection, BitcoinChainCheckpoint};

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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)] pub struct DrawState { pub config:DrawConfig,pub total_tickets:u64,pub prize_pool_mfdm:u128,pub cu_fund_mfdm:u128,pub annual_reserve_mfdm:u128,pub executed:bool }

#[derive(Clone, Debug, Serialize, Deserialize)] pub struct ChainState { pub height:u64,pub last_block_hash:Hash,pub accounts:BTreeMap<Address,Account>,pub validators:BTreeMap<Hash,Validator>,#[serde(default)] pub unbonding:BTreeMap<Hash,UnbondingEntry>,pub nodes:BTreeMap<Hash,Node>,pub contributions:BTreeMap<Hash,Contribution>,pub rewards:BTreeMap<Hash,Reward>,pub purchases:BTreeMap<Hash,Purchase>,pub billing:BTreeMap<Hash,BillingRecord>,pub tickets:BTreeMap<Hash,Ticket>,pub draws:BTreeMap<Hash,DrawState>,pub draw_results:BTreeMap<Hash,DrawResult>,pub cu_fund_mfdm:u128,pub annual_reserve_mfdm:u128,pub draw_prize_pool_mfdm:u128,pub events:Vec<Event>,pub ledger:Vec<LedgerEntry>,pub supply:Supply,pub epoch:u64,pub epoch_length:u64,pub active_validator_set:BTreeMap<Hash,freedom_types::ValidatorSetEntry>,#[serde(default)] pub asset_registry:BTreeMap<Hash,AssetDefinition>,#[serde(default)] pub processed_bridge_messages:BTreeMap<Hash,BridgeMessage>,#[serde(default)] pub bridge_message_status:BTreeMap<Hash,BridgeMessageStatus>,#[serde(default)] pub bridge_withdrawals:BTreeMap<Hash,BridgeWithdrawal>,#[serde(default)] pub bridge_withdrawal_status:BTreeMap<Hash,BridgeWithdrawalStatus>,#[serde(default)] pub bridge_accounting:BTreeMap<Hash,BridgeAccounting>,#[serde(default)] pub bridge_security_policies:BTreeMap<Hash,BridgeSecurityPolicy>,#[serde(default)] pub bridge_approval_sets:BTreeMap<Hash,BridgeApprovalSet>,#[serde(default)] pub bridge_pause_authorizations:BTreeMap<Hash,BridgePauseAuthorizationSet>,#[serde(default)] pub bridge_nonce_registry:BTreeMap<Hash,BTreeSet<u64>>,#[serde(default)] pub bridge_rate_limits:BTreeMap<Hash,BridgeRateLimitUsage>,#[serde(default)] pub bridge_withdrawal_rate_limit_period:BTreeMap<Hash,u64>,#[serde(default)] pub bitcoin_checkpoints:BTreeMap<Hash,BitcoinChainCheckpoint>,#[serde(default)] pub pending_validator_set:BTreeMap<Hash,freedom_types::ValidatorSetEntry>,#[serde(default)] pub validator_set_certificate:Option<freedom_types::ValidatorSetCertificate>,#[serde(default)] pub staking_activated:bool,pub epoch_issuance:u128,pub epoch_cap:u128,pub evidence:BTreeMap<Hash,freedom_types::EquivocationEvidence>,pub slashed_evidence:BTreeMap<Hash,u128> }
#[derive(Debug,PartialEq,Eq)] pub enum StateError { AccountNotFound,InvalidNonce{expected:u64,actual:u64},InsufficientBalance,InvalidTransferPayload,InvalidRecipient,ArithmeticOverflow,UnsupportedTransaction,InvalidNodePayload,InvalidNode,DuplicateNode,InvalidContributionPayload,InvalidContribution,DuplicateContribution,InvalidWorkUnits,InvalidQualityScore,InvalidVerification,AlreadyVerified,ContributionNotRewardable,DuplicateReward,InvalidReward,IssuanceCapExceeded,InvalidPurchase,DuplicatePurchase,InsufficientBalanceForPurchase,InvalidEvidence,InvalidTicketPayload,InvalidDraw,InvalidTicketQuantity,InvalidTicketPrice,DuplicateTicket,DrawNotReady,DrawAlreadyExecuted,InsufficientPrizePool,InvalidUnbonding,InvalidValidatorLifecycle,InvalidStakeAmount,InvalidAsset,DuplicateAsset,InvalidBridgeMessage,DuplicateBridgeMessage,InvalidBridgeWithdrawal,DuplicateBridgeWithdrawal,InvalidBridgeWithdrawalTransition,InvalidBridgeAccounting,BridgeRateLimitExceeded,InvalidBridgePauseAuthorization,DuplicateBridgeNonce }
impl ChainState {
 pub fn record_equivocation(&mut self,evidence:freedom_types::EquivocationEvidence)->Result<bool,StateError>{
  if evidence.first.is_empty() || evidence.second.is_empty() || evidence.first == evidence.second { return Err(StateError::InvalidEvidence); }
  let expected = freedom_primitives::hash_bytes(&evidence.signing_bytes());
  if expected != evidence.evidence_hash { return Err(StateError::InvalidEvidence); }
  if self.evidence.contains_key(&evidence.evidence_hash){return Ok(false)}
  self.evidence.insert(evidence.evidence_hash,evidence); Ok(true)
 }
 pub fn slash_evidence(&mut self,evidence_hash:Hash,amount:u128)->Result<u128,StateError>{
  if self.slashed_evidence.contains_key(&evidence_hash){return Ok(0)}
  let evidence=self.evidence.get(&evidence_hash).ok_or(StateError::InvalidEvidence)?;
  let (owner,total)={let v=self.validators.get(&evidence.validator_id).ok_or(StateError::InvalidEvidence)?;(v.owner,v.stake.saturating_add(v.unbonding_stake))};
  let expected=self.deterministic_slash_amount(&evidence.kind,total);
  if amount!=expected{return Err(StateError::InvalidEvidence)}
  let slash=expected.min(total);
  let (active_cut,pending_cut)={let v=self.validators.get_mut(&evidence.validator_id).ok_or(StateError::InvalidEvidence)?;let x=slash.min(v.stake);v.stake-=x;let y=(slash-x).min(v.unbonding_stake);v.unbonding_stake-=y;(x,y)};
  if active_cut+pending_cut!=slash{return Err(StateError::InvalidEvidence)}
  if let Some(a)=self.accounts.get_mut(&owner){a.staked=a.staked.saturating_sub(slash)}else{return Err(StateError::AccountNotFound)}
  let mut pending=pending_cut;
  let ids:Vec<Hash>=self.unbonding.iter().filter(|(_,e)|e.validator_id==evidence.validator_id).map(|(id,_)|*id).collect();
  for id in ids { if pending==0{break} if let Some(e)=self.unbonding.get_mut(&id){let x=pending.min(e.amount);e.amount-=x;pending-=x;} if self.unbonding.get(&id).map(|e|e.amount==0).unwrap_or(false){self.unbonding.remove(&id);} }
  if pending!=0{return Err(StateError::InvalidEvidence)}
  self.slashed_evidence.insert(evidence_hash,slash);self.refresh_pending_validator_set();Ok(slash)
 }
 pub fn deterministic_slash_amount(&self,kind:&freedom_types::EquivocationKind,stake:u128)->u128{
  let bps=match kind { freedom_types::EquivocationKind::DoubleProposal=>500, freedom_types::EquivocationKind::DoublePrevote=>1000, freedom_types::EquivocationKind::DoublePrecommit=>1000 };
  stake.saturating_mul(bps as u128)/10_000
 }

 pub const UNBONDING_COOLDOWN: u64 = 200;
 pub fn genesis(g:&Genesis)->Self{assert_eq!(g.initial_supply,GENESIS_SUPPLY,"Genesis FDM must be zero");let validators=g.validators.iter().cloned().map(|v|(v.validator_id,v)).collect();Self{height:0,last_block_hash:[0;32],accounts:BTreeMap::new(),validators,nodes:BTreeMap::new(),contributions:BTreeMap::new(),rewards:BTreeMap::new(),purchases:BTreeMap::new(),billing:BTreeMap::new(),tickets:BTreeMap::new(),draws:BTreeMap::new(),draw_results:BTreeMap::new(),cu_fund_mfdm:0,annual_reserve_mfdm:0,draw_prize_pool_mfdm:0,events:Vec::new(),ledger:Vec::new(),supply:Supply::genesis(),epoch:0,epoch_length:1000,active_validator_set:validators.iter().map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect(),asset_registry: Self::genesis_asset_registry(),processed_bridge_messages:BTreeMap::new(),bridge_message_status:BTreeMap::new(),bridge_withdrawals:BTreeMap::new(),bridge_withdrawal_status:BTreeMap::new(),bridge_accounting:BTreeMap::new(),bridge_security_policies:BTreeMap::new(),bridge_approval_sets:BTreeMap::new(),bridge_pause_authorizations:BTreeMap::new(),bridge_nonce_registry:BTreeMap::new(),bridge_rate_limits:BTreeMap::new(),bridge_withdrawal_rate_limit_period:BTreeMap::new(),bitcoin_checkpoints:BTreeMap::new(),pending_validator_set:validators.iter().map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect(),validator_set_certificate:None,staking_activated:false,epoch_issuance:0,epoch_cap:1_000_000,unbonding:BTreeMap::new(),evidence:BTreeMap::new(),slashed_evidence:BTreeMap::new()}}
 pub fn genesis_asset_registry() -> BTreeMap<Hash, AssetDefinition> {
  let contract_address = b"FRC".to_vec();
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
   symbol: "FRC".to_string(),
   decimals: 18,
   issuer: [0; 32],
   bridge_id: [0; 32],
   status: AssetStatus::Active,
  };
  BTreeMap::from([(asset_id, asset)])
 }
 pub fn register_asset(&mut self, asset: AssetDefinition) -> Result<(), StateError> {
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
 pub fn reserve_bridge_withdrawal_limit(&mut self,bridge_id:Hash,period:u64,amount:u128)->Result<(),StateError>{
  if amount==0{return Err(StateError::BridgeRateLimitExceeded);}
  let policy=self.bridge_security_policies.get(&bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if policy.emergency_paused||amount>policy.max_withdrawal_amount{return Err(StateError::BridgeRateLimitExceeded);}
  let usage=self.bridge_rate_limits.entry(bridge_id).or_insert(BridgeRateLimitUsage{period,used:0});
  if usage.period!=period {usage.period=period;usage.used=0;}
  let next=usage.used.checked_add(amount).ok_or(StateError::BridgeRateLimitExceeded)?;
  if next>policy.daily_limit{return Err(StateError::BridgeRateLimitExceeded);}
  usage.used=next; Ok(())
 }
 pub fn release_bridge_withdrawal_limit(&mut self,bridge_id:Hash,period:u64,amount:u128)->Result<(),StateError>{
  let usage=self.bridge_rate_limits.get_mut(&bridge_id).ok_or(StateError::BridgeRateLimitExceeded)?;
  if usage.period!=period||amount>usage.used{return Err(StateError::BridgeRateLimitExceeded);}
  usage.used-=amount; Ok(())
 }
 pub fn apply_bridge_emergency_pause(&mut self,bridge_id:Hash,auth_set:BridgePauseAuthorizationSet)->Result<(),StateError>{
  let policy=self.bridge_security_policies.get(&bridge_id).cloned().ok_or(StateError::InvalidBridgePauseAuthorization)?;
  if auth_set.bridge_id!=bridge_id||auth_set.nonce==0||auth_set.authorizations.is_empty(){return Err(StateError::InvalidBridgePauseAuthorization);}
  if let Some(previous)=self.bridge_pause_authorizations.get(&bridge_id){if auth_set.nonce<=previous.nonce{return Err(StateError::InvalidBridgePauseAuthorization);}}
  let keys=self.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<BTreeMap<Hash,[u8;32]>>();
  if auth_set.authenticated_count(&keys)<policy.validator_quorum{return Err(StateError::InvalidBridgePauseAuthorization);}
  if auth_set.authorizations.iter().any(|a|a.bridge_id!=bridge_id||a.nonce!=auth_set.nonce||!keys.contains_key(&a.validator_id)){return Err(StateError::InvalidBridgePauseAuthorization);}
  let mut next=policy.clone();next.emergency_paused=auth_set.authorizations.first().map(|a|a.paused).ok_or(StateError::InvalidBridgePauseAuthorization)?;
  if auth_set.authorizations.iter().any(|a|a.paused!=next.emergency_paused){return Err(StateError::InvalidBridgePauseAuthorization);}
  self.bridge_security_policies.insert(bridge_id,next);
  self.bridge_pause_authorizations.insert(bridge_id,auth_set);
  Ok(())
 }
 pub fn register_bitcoin_checkpoint(&mut self,bridge_id:Hash,checkpoint:BitcoinChainCheckpoint)->Result<(),StateError>{
  if bridge_id==[0;32]||!checkpoint.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  if let Some(old)=self.bitcoin_checkpoints.get(&bridge_id){if checkpoint.height<old.height||checkpoint.height==old.height&&checkpoint.block_hash!=old.block_hash{return Err(StateError::InvalidBridgeWithdrawal);}}
  self.bitcoin_checkpoints.insert(bridge_id,checkpoint);Ok(())
 }
 pub fn register_bridge_security_policy(&mut self, policy:BridgeSecurityPolicy)->Result<(),StateError>{
  if !policy.validate(){return Err(StateError::InvalidBridgeWithdrawal);}
  if self.bridge_security_policies.contains_key(&policy.bridge_id){return Err(StateError::InvalidBridgeWithdrawal);}
  self.bridge_security_policies.insert(policy.bridge_id,policy); Ok(())
 }
 pub fn record_bridge_approval_set(&mut self, set:BridgeApprovalSet)->Result<(),StateError>{
  if set.request_id==[0;32]||set.approvals.is_empty(){return Err(StateError::InvalidBridgeWithdrawal);}
  if self.bridge_approval_sets.contains_key(&set.request_id){return Err(StateError::DuplicateBridgeWithdrawal);}
  let policy_id=set.approvals.first().map(|a|a.bridge_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy=self.bridge_security_policies.get(&policy_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if set.approvals.iter().any(|a|a.bridge_id!=policy.bridge_id||a.request_id!=set.request_id||!a.structurally_valid()){return Err(StateError::InvalidBridgeWithdrawal);}
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
 pub fn record_bridge_message(&mut self, message:BridgeMessage)->Result<Hash,StateError>{
  let replay_key=self.validate_bridge_message(&message)?;
  self.processed_bridge_messages.insert(replay_key,message);
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Verified);
  Ok(replay_key)
 }
 pub fn accept_bridge_message(&mut self,replay_key:Hash)->Result<(),StateError>{
  if !self.processed_bridge_messages.contains_key(&replay_key) { return Err(StateError::InvalidBridgeMessage); }
  if !self.bridge_message_status.contains_key(&replay_key) {
   self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Verified);
  }
  let status=self.bridge_message_status.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if *status != BridgeMessageStatus::Verified { return Err(StateError::InvalidBridgeMessage); }
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Accepted);
  Ok(())
 }
 pub fn mark_bridge_message_minted(&mut self,replay_key:Hash)->Result<(),StateError>{
  let status=self.bridge_message_status.get(&replay_key).ok_or(StateError::InvalidBridgeMessage)?;
  if *status != BridgeMessageStatus::Accepted { return Err(StateError::InvalidBridgeMessage); }
  if !self.processed_bridge_messages.contains_key(&replay_key) { return Err(StateError::InvalidBridgeMessage); }
  self.bridge_message_status.insert(replay_key,BridgeMessageStatus::Minted);
  Ok(())
 }

 pub fn request_bridge_withdrawal(&mut self, withdrawal:BridgeWithdrawal)->Result<Hash,StateError>{
  if withdrawal.destination_chain_id==freedom_primitives::CHAIN_ID || withdrawal.amount==0 || withdrawal.sender==[0;32] || withdrawal.recipient.is_empty() || !withdrawal.validate_hash() || withdrawal.compute_id()!=withdrawal.withdrawal_id { return Err(StateError::InvalidBridgeWithdrawal); }
  let asset=self.asset_registry.get(&withdrawal.asset_id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if asset.asset_type!=AssetType::Bridged || asset.status!=AssetStatus::Active || asset.source_chain_id!=withdrawal.destination_chain_id || asset.bridge_id!=withdrawal.bridge_id { return Err(StateError::InvalidBridgeWithdrawal); }
  if self.bridge_withdrawals.contains_key(&withdrawal.withdrawal_id) { return Err(StateError::DuplicateBridgeWithdrawal); }
  let nonce_key=BridgeNonceKey{bridge_id:withdrawal.bridge_id,asset_id:withdrawal.asset_id,direction:BridgeDirection::Outbound}.key_hash();
  if self.bridge_nonce_registry.get(&nonce_key).map(|n|n.contains(&withdrawal.nonce)).unwrap_or(false){return Err(StateError::DuplicateBridgeNonce);}
  let id=withdrawal.withdrawal_id;
  self.bridge_withdrawals.insert(id,withdrawal);
  self.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Requested);
  self.bridge_nonce_registry.entry(nonce_key).or_default().insert(withdrawal.nonce);
  Ok(id)
 }
 pub fn approve_bridge_with_authenticated_set(&mut self,id:Hash,request_id:Hash)->Result<(),StateError>{
  let current=*self.bridge_withdrawal_status.get(&id).ok_or(StateError::InvalidBridgeWithdrawal)?;
  if current!=BridgeWithdrawalStatus::Validated { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  let w=self.bridge_withdrawals.get(&id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if w.request_hash==[0;32] || request_id==[0;32] { return Err(StateError::InvalidBridgeWithdrawal); }
  let set=self.bridge_approval_sets.get(&request_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  let policy=self.bridge_security_policies.get(&w.bridge_id).cloned().ok_or(StateError::InvalidBridgeWithdrawal)?;
  if set.request_id!=request_id || set.approvals.iter().any(|a|a.withdrawal_id!=id||a.bridge_id!=w.bridge_id||a.nonce!=w.nonce||a.amount!=w.amount||a.destination_chain_id!=w.destination_chain_id||a.asset_id!=w.asset_id||a.intent_hash!=w.request_hash){
   return Err(StateError::InvalidBridgeWithdrawal);
  }
  let keys=self.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<BTreeMap<Hash,[u8;32]>>();
  let quorum=set.quorum_reached_authenticated(policy,w.bridge_id,id,w.request_hash,&keys);
  let threshold=set.threshold_reached(&policy);
  let approved=match policy.approval_mode { freedom_types::BridgeApprovalMode::Quorum=>quorum, freedom_types::BridgeApprovalMode::Threshold=>threshold, freedom_types::BridgeApprovalMode::QuorumAndThreshold=>quorum&&threshold };
  if !approved { return Err(StateError::InvalidBridgeWithdrawalTransition); }
  let period=self.height;
  self.reserve_bridge_withdrawal_limit(w.bridge_id,period,w.amount)?;
  self.bridge_withdrawal_rate_limit_period.insert(id,period);
  self.bridge_withdrawal_status.insert(id,BridgeWithdrawalStatus::Approved);
  Ok(())
 }
 pub fn advance_bridge_withdrawal(&mut self,id:Hash,next:BridgeWithdrawalStatus)->Result<(),StateError>{
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
 pub fn record_verified_external_lock(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 || !self.asset_registry.contains_key(&asset_id) { return Err(StateError::InvalidBridgeAccounting); }
  let a=self.bridge_accounting.entry(asset_id).or_default();
  a.verified_external_locked=a.verified_external_locked.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?; self.validate_bridge_accounting(asset_id)
 }
 pub fn record_freedom_mint(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let a=self.bridge_accounting.entry(asset_id).or_default();
  let next=a.freedom_minted.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next>a.verified_external_locked { return Err(StateError::InvalidBridgeAccounting); }
  a.freedom_minted=next; self.validate_bridge_accounting(asset_id)
 }
 pub fn record_freedom_burn(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let a=self.bridge_accounting.entry(asset_id).or_default();
  let next=a.freedom_burned.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next>a.freedom_minted { return Err(StateError::InvalidBridgeAccounting); }
  a.freedom_burned=next; self.validate_bridge_accounting(asset_id)
 }
 pub fn record_external_release(&mut self,asset_id:Hash,amount:u128)->Result<(),StateError>{
  if amount==0 { return Err(StateError::InvalidBridgeAccounting); }
  let a=self.bridge_accounting.entry(asset_id).or_default();
  let next=a.external_released.checked_add(amount).ok_or(StateError::InvalidBridgeAccounting)?;
  if next>a.freedom_burned { return Err(StateError::InvalidBridgeAccounting); }
  a.external_released=next; self.validate_bridge_accounting(asset_id)
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
 pub fn refresh_pending_validator_set(&mut self){
  self.pending_validator_set=self.validators.iter().filter(|(_,v)|!self.staking_activated||v.stake>0).map(|(id,v)|(*id,freedom_types::ValidatorSetEntry{validator_id:*id,owner:v.owner,consensus_key:v.consensus_key,stake:v.stake})).collect();
 }
 pub fn refresh_validator_set(&mut self){self.refresh_pending_validator_set();self.active_validator_set=self.pending_validator_set.clone();}
 pub fn maybe_advance_epoch(&mut self){
  self.release_mature_unbonding();
  if self.epoch_length>0 && self.height>0 && self.height%self.epoch_length==0 {self.epoch=self.height/self.epoch_length;self.active_validator_set=self.pending_validator_set.clone();self.refresh_pending_validator_set();self.epoch_issuance=0;}
 }
 fn release_mature_unbonding(&mut self){
  let ids:Vec<Hash>=self.unbonding.iter().filter(|(_,e)|e.release_height<=self.height).map(|(id,_)|*id).collect();
  for id in ids {
   if let Some(e)=self.unbonding.remove(&id) {
    if let Some(v)=self.validators.get_mut(&e.validator_id){v.unbonding_stake=v.unbonding_stake.saturating_sub(e.amount);}
    if let Some(a)=self.accounts.get_mut(&e.owner){a.staked=a.staked.saturating_sub(e.amount);a.balance=a.balance.checked_add(e.amount).expect("unbonding release overflow");}
   }
  }
 }
 pub fn insert_account(&mut self,a:Account){self.accounts.insert(a.address,a);}
 pub fn drain_events(&mut self)->Vec<Event>{std::mem::take(&mut self.events)}
 pub fn execute_transaction(&mut self,tx:&Transaction)->Result<(),StateError>{let nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce;if nonce!=tx.nonce{return Err(StateError::InvalidNonce{expected:nonce,actual:tx.nonce})}let result=match &tx.kind{TxKind::Transfer=>self.execute_transfer(tx),TxKind::RegisterNode=>self.register_node(tx),TxKind::SubmitContribution=>self.submit_contribution(tx),TxKind::ClaimReward=>self.claim_reward(tx),TxKind::Purchase=>self.purchase(tx),TxKind::CreateTicket=>self.create_ticket(tx),TxKind::SubmitEvidence=>self.submit_evidence(tx),TxKind::Stake=>self.stake(tx),TxKind::Unstake=>self.unstake(tx),_=>Err(StateError::UnsupportedTransaction)}; if result.is_ok(){self.ledger.push(LedgerEntry{tx_hash:tx.hash(),sender:tx.sender,kind:tx.kind.code(),nonce:tx.nonce,success:true});} result}
 fn stake(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=freedom_types::StakePayload::decode(&tx.payload).map_err(|_|StateError::UnsupportedTransaction)?;
  if p.amount==0{return Err(StateError::InvalidStakeAmount)}
  let id=self.validators.values().find(|v|v.owner==tx.sender).map(|v|v.validator_id).ok_or(StateError::InvalidEvidence)?;
  if self.staking_activated && (self.validator_lifecycle(id)?!=freedom_types::ValidatorLifecycle::Active || self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.unbonding_stake>0){return Err(StateError::InvalidValidatorLifecycle)}
  let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if a.balance<p.amount{return Err(StateError::InsufficientBalance)}
  a.balance-=p.amount;a.staked=a.staked.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;self.staking_activated=true;
  let v=self.validators.get_mut(&id).ok_or(StateError::InvalidEvidence)?;
  v.stake=v.stake.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  self.refresh_pending_validator_set();
  self.advance_nonce(tx.sender)
 }
 fn unstake(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=freedom_types::UnstakePayload::decode(&tx.payload).map_err(|_|StateError::UnsupportedTransaction)?;
  if p.amount==0{return Err(StateError::InvalidUnbonding)}
  let id=self.validators.values().find(|v|v.owner==tx.sender).map(|v|v.validator_id).ok_or(StateError::InvalidEvidence)?;
  let owner=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.owner;
  let active=self.validators.get(&id).ok_or(StateError::InvalidEvidence)?.stake;
  if active==0{return Err(StateError::InvalidValidatorLifecycle)}
  if active<p.amount{return Err(StateError::InsufficientBalance)}
  let a=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;
  if a.staked<p.amount{return Err(StateError::InsufficientBalance)}
  let release_height=self.height.checked_add(UNBONDING_COOLDOWN).ok_or(StateError::ArithmeticOverflow)?;
  let unbonding_id=freedom_primitives::hash_bytes(&[b"FREEDOM-UNBONDING-V1".as_slice(),&id,&tx.sender,&tx.nonce.to_le_bytes(),&p.amount.to_le_bytes(),&release_height.to_le_bytes()].concat());
  if self.unbonding.contains_key(&unbonding_id){return Err(StateError::InvalidUnbonding)}
  let v=self.validators.get_mut(&id).ok_or(StateError::InvalidEvidence)?;
  v.stake-=p.amount;v.unbonding_stake=v.unbonding_stake.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  self.unbonding.insert(unbonding_id,UnbondingEntry{unbonding_id,validator_id:id,owner,amount:p.amount,requested_height:self.height,release_height});
  self.refresh_pending_validator_set();
  self.advance_nonce(tx.sender)
 }
 fn submit_evidence(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=decode_submit_evidence(&tx.payload).ok_or(StateError::InvalidEvidence)?;
  if !self.evidence.contains_key(&p.evidence.evidence_hash){ self.record_equivocation(p.evidence.clone())?; }
  let expected=self.deterministic_slash_amount(&p.evidence.kind,self.validators.get(&p.evidence.validator_id).ok_or(StateError::InvalidEvidence)?.stake);
  if p.slash_amount!=expected{return Err(StateError::InvalidEvidence)}
  let amount=self.slash_evidence(p.evidence.evidence_hash,p.slash_amount)?;
  self.events.push(Event::EquivocationSlashed{evidence_hash:p.evidence.evidence_hash,validator_id:p.evidence.validator_id,amount});
  self.advance_nonce(tx.sender)
 }
 fn advance_nonce(&mut self,s:Address)->Result<(),StateError>{let a=self.accounts.get_mut(&s).ok_or(StateError::AccountNotFound)?;a.nonce=a.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;Ok(())}
 fn execute_transfer(&mut self,tx:&Transaction)->Result<(),StateError>{let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;let t=TransferPayload::decode(&tx.payload).map_err(|_|StateError::InvalidTransferPayload)?;if t.to==[0;32]{return Err(StateError::InvalidRecipient)}let cost=t.amount.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;if balance<cost{return Err(StateError::InsufficientBalance)}let nb=balance-cost;if t.to==tx.sender{let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance=nb.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?}else{let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance=nb;let r=self.accounts.entry(t.to).or_insert(Account{address:t.to,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});r.balance=r.balance.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?}self.supply.circulating=self.supply.circulating.checked_sub(tx.fee).ok_or(StateError::ArithmeticOverflow)?;self.supply.treasury=self.supply.treasury.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;self.events.push(Event::FdmTransferred{from:tx.sender,to:t.to,amount:t.amount});self.advance_nonce(tx.sender)}
 fn register_node(&mut self,tx:&Transaction)->Result<(),StateError>{let p=RegisterNodePayload::decode(&tx.payload).map_err(|_|StateError::InvalidNodePayload)?;if p.node_id==[0;32]{return Err(StateError::InvalidNode)}if self.nodes.contains_key(&p.node_id){return Err(StateError::DuplicateNode)}self.nodes.insert(p.node_id,Node{node_id:p.node_id,owner:tx.sender,capabilities:p.capabilities,reputation:100,total_wu:0,successful_tasks:0,failed_tasks:0});self.advance_nonce(tx.sender)}
 fn submit_contribution(&mut self,tx:&Transaction)->Result<(),StateError>{let p=SubmitContributionPayload::decode(&tx.payload).map_err(|_|StateError::InvalidContributionPayload)?;if p.contribution_id==[0;32]||p.task_id==[0;32]||p.node_id==[0;32]{return Err(StateError::InvalidContribution)}if p.work_units==0{return Err(StateError::InvalidWorkUnits)}if p.quality_score>10000{return Err(StateError::InvalidQualityScore)}if self.contributions.contains_key(&p.contribution_id){return Err(StateError::DuplicateContribution)}let node=self.nodes.get(&p.node_id).ok_or(StateError::InvalidNode)?;if node.owner!=tx.sender{return Err(StateError::InvalidNode)}self.contributions.insert(p.contribution_id,Contribution{contribution_id:p.contribution_id,node_id:p.node_id,task_id:p.task_id,work_units:p.work_units,quality_score:p.quality_score,verification_score:0,result_hash:p.result_hash,verifier_set:Vec::new(),status:ContributionStatus::Submitted,reward_id:None,reward_amount:0});self.advance_nonce(tx.sender)}
 pub fn verify_contribution(&mut self,id:Hash,verifiers:Vec<Hash>,score:u32)->Result<(),StateError>{if verifiers.is_empty()||score>10000{return Err(StateError::InvalidVerification)}if verifiers.iter().any(|v|!self.validators.contains_key(v)){return Err(StateError::InvalidVerification)}let(node_id,wu)={let c=self.contributions.get(&id).ok_or(StateError::InvalidContribution)?;if c.status!=ContributionStatus::Submitted{return Err(StateError::AlreadyVerified)}(c.node_id,c.work_units)};let c=self.contributions.get_mut(&id).ok_or(StateError::InvalidContribution)?;c.verifier_set=verifiers;c.verification_score=score;c.status=ContributionStatus::Verified;let n=self.nodes.get_mut(&node_id).ok_or(StateError::InvalidNode)?;n.total_wu=n.total_wu.checked_add(wu).ok_or(StateError::ArithmeticOverflow)?;n.successful_tasks=n.successful_tasks.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;Ok(())}
 pub fn claim_reward(&mut self,id:Hash)->Result<(),StateError>{let c=self.contributions.get(&id).ok_or(StateError::InvalidContribution)?.clone();if c.status!=ContributionStatus::Verified{return Err(StateError::ContributionNotRewardable)}if c.reward_id.is_some(){return Err(StateError::DuplicateReward)}let gross=c.work_units.checked_mul(c.quality_score as u128).ok_or(StateError::ArithmeticOverflow)?.checked_mul(c.verification_score as u128).ok_or(StateError::ArithmeticOverflow)?.checked_div(100_000_000).ok_or(StateError::ArithmeticOverflow)?;let remaining=self.epoch_cap.checked_sub(self.epoch_issuance).ok_or(StateError::IssuanceCapExceeded)?;let reward=gross.min(remaining);if reward==0{return Err(StateError::IssuanceCapExceeded)}let reward_id=freedom_primitives::hash_bytes(&[b"FREEDOM-REWARD-V1".as_slice(),&id,&self.epoch.to_le_bytes()].concat());let owner=self.nodes.get(&c.node_id).ok_or(StateError::InvalidNode)?.owner;if self.rewards.contains_key(&reward_id){return Err(StateError::DuplicateReward)}let a=self.accounts.get_mut(&owner).ok_or(StateError::AccountNotFound)?;a.balance=a.balance.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;self.supply.total=self.supply.total.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;self.supply.circulating=self.supply.circulating.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;self.epoch_issuance=self.epoch_issuance.checked_add(reward).ok_or(StateError::ArithmeticOverflow)?;self.rewards.insert(reward_id,Reward{reward_id,owner,contribution_id:id,wu:c.work_units,gross_reward:gross,final_reward:reward,epoch:self.epoch});let c=self.contributions.get_mut(&id).ok_or(StateError::InvalidContribution)?;c.status=ContributionStatus::Rewarded;c.reward_id=Some(reward_id);c.reward_amount=reward;Ok(())}
 pub fn create_draw(&mut self,config:DrawConfig)->Result<(),StateError>{
  if config.draw_id==[0;32]||self.draws.contains_key(&config.draw_id)||config.ticket_price_mfdm!=10_000||config.winner_count==0||config.starts_at>config.ends_at{return Err(StateError::InvalidDraw)}
  self.draws.insert(config.draw_id,DrawState{config,total_tickets:0,prize_pool_mfdm:0,cu_fund_mfdm:0,annual_reserve_mfdm:0,executed:false}); Ok(())
 }
 pub fn create_ticket(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=CreateTicketPayload::decode(&tx.payload).map_err(|_|StateError::InvalidTicketPayload)?;
  if p.draw_id==[0;32]{return Err(StateError::InvalidDraw)} if p.quantity==0{return Err(StateError::InvalidTicketQuantity)}
  let draw=self.draws.get(&p.draw_id).cloned().ok_or(StateError::InvalidDraw)?;
  if self.height<draw.config.starts_at||self.height>draw.config.ends_at{return Err(StateError::InvalidDraw)}
  if draw.config.ticket_price_mfdm!=10_000{return Err(StateError::InvalidTicketPrice)}
  let cost=draw.config.ticket_price_mfdm.checked_mul(p.quantity as u128).ok_or(StateError::ArithmeticOverflow)?;
  let new_total=draw.total_tickets.checked_add(p.quantity).ok_or(StateError::ArithmeticOverflow)?;
  let mut new_ids=Vec::with_capacity(p.quantity as usize);
  let start=draw.total_tickets+1;
  for n in start..=new_total { let id=freedom_primitives::hash_bytes(&[b"FREEDOM-TICKET-V1".as_slice(),&tx.sender,&p.draw_id,&n.to_le_bytes()].concat()); if self.tickets.contains_key(&id){return Err(StateError::DuplicateTicket)} new_ids.push(id); }
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance; if balance<cost{return Err(StateError::InsufficientBalanceForPurchase)}
  {let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance-=cost;}
  self.supply.circulating=self.supply.circulating.checked_sub(cost).ok_or(StateError::ArithmeticOverflow)?; self.supply.treasury=self.supply.treasury.checked_add(cost).ok_or(StateError::ArithmeticOverflow)?;
  let prize=cost.checked_mul(60).ok_or(StateError::ArithmeticOverflow)?.checked_div(100).ok_or(StateError::ArithmeticOverflow)?;
  let fund=cost.checked_mul(30).ok_or(StateError::ArithmeticOverflow)?.checked_div(100).ok_or(StateError::ArithmeticOverflow)?;
  let reserve=cost.checked_sub(prize).and_then(|x|x.checked_sub(fund)).ok_or(StateError::ArithmeticOverflow)?;
  let d=self.draws.get_mut(&p.draw_id).ok_or(StateError::InvalidDraw)?; d.total_tickets=new_total; d.prize_pool_mfdm=d.prize_pool_mfdm.checked_add(prize).ok_or(StateError::ArithmeticOverflow)?; d.cu_fund_mfdm=d.cu_fund_mfdm.checked_add(fund).ok_or(StateError::ArithmeticOverflow)?; d.annual_reserve_mfdm=d.annual_reserve_mfdm.checked_add(reserve).ok_or(StateError::ArithmeticOverflow)?;
  self.draw_prize_pool_mfdm=self.draw_prize_pool_mfdm.checked_add(prize).ok_or(StateError::ArithmeticOverflow)?; self.cu_fund_mfdm=self.cu_fund_mfdm.checked_add(fund).ok_or(StateError::ArithmeticOverflow)?; self.annual_reserve_mfdm=self.annual_reserve_mfdm.checked_add(reserve).ok_or(StateError::ArithmeticOverflow)?;
  for id in new_ids {self.tickets.insert(id,Ticket{ticket_id:id,owner:tx.sender,draw_id:p.draw_id,created_at:self.height});self.events.push(Event::TicketCreated{ticket_id:id,draw_id:p.draw_id,owner:tx.sender});} self.events.push(Event::DrawFundAllocated{draw_id:p.draw_id,prize_pool:prize,cu_fund:fund,annual_reserve:reserve}); self.advance_nonce(tx.sender)
 }
 pub fn execute_draw(&mut self,draw_id:Hash)->Result<DrawResult,StateError>{
  let draw=self.draws.get(&draw_id).cloned().ok_or(StateError::InvalidDraw)?; if draw.executed{return Err(StateError::DrawAlreadyExecuted)} if self.height<draw.config.ends_at{return Err(StateError::DrawNotReady)} if draw.total_tickets==0||draw.config.winner_count as u64>draw.total_tickets{return Err(StateError::InvalidDraw)}
  let seed=freedom_primitives::hash_bytes(&[b"FREEDOM-DRAW-V1".as_slice(),&self.last_block_hash,&draw_id,&draw.config.seed_block.to_le_bytes()].concat());
  let mut ids:Vec<Hash>=self.tickets.values().filter(|t|t.draw_id==draw_id).map(|t|t.ticket_id).collect(); ids.sort();
  let mut winners=Vec::new(); let mut salt=0u64; while winners.len()<draw.config.winner_count as usize {let h=freedom_primitives::hash_bytes(&[&seed,&salt.to_le_bytes()].concat());let idx=(u128::from_le_bytes(h[..16].try_into().unwrap())%(ids.len() as u128)) as usize;let id=ids[idx];if !winners.contains(&id){winners.push(id)}salt=salt.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;}
  let selection_proof=freedom_primitives::hash_bytes(&[b"FREEDOM-DRAW-PROOF-V1".as_slice(),&seed,winners.concat().as_slice()].concat()); let result=DrawResult{draw_id,total_tickets:draw.total_tickets,prize_pool_mfdm:draw.prize_pool_mfdm,cu_fund_mfdm:draw.cu_fund_mfdm,annual_reserve_mfdm:draw.annual_reserve_mfdm,winners:winners.clone(),randomness_seed:seed,selection_proof};
  let per=draw.prize_pool_mfdm.checked_div(winners.len() as u128).ok_or(StateError::ArithmeticOverflow)?;let remainder=draw.prize_pool_mfdm.checked_sub(per.checked_mul(winners.len() as u128).ok_or(StateError::ArithmeticOverflow)?).ok_or(StateError::ArithmeticOverflow)?;
  for (i,id) in winners.iter().enumerate(){let owner=self.tickets.get(id).ok_or(StateError::InvalidDraw)?.owner;let amount=per.checked_add(if i==0{remainder}else{0}).ok_or(StateError::ArithmeticOverflow)?;let a=self.accounts.get_mut(&owner).ok_or(StateError::AccountNotFound)?;a.balance=a.balance.checked_add(amount).ok_or(StateError::ArithmeticOverflow)?;self.supply.treasury=self.supply.treasury.checked_sub(amount).ok_or(StateError::InsufficientPrizePool)?;self.supply.circulating=self.supply.circulating.checked_add(amount).ok_or(StateError::ArithmeticOverflow)?;self.events.push(Event::PrizePaid{draw_id,ticket_id:*id,winner:owner,amount});}
  self.draw_prize_pool_mfdm=self.draw_prize_pool_mfdm.checked_sub(draw.prize_pool_mfdm).ok_or(StateError::InsufficientPrizePool)?;let d=self.draws.get_mut(&draw_id).ok_or(StateError::InvalidDraw)?;d.prize_pool_mfdm=0;d.executed=true;self.draw_results.insert(draw_id,result.clone());self.events.push(Event::DrawExecuted{draw_id,randomness_seed:seed,selection_proof});Ok(result)
 }
 pub fn quote_price(&self,product_id:u8)->Result<u128,StateError>{if product_id==1{Ok(10_000)}else{Err(StateError::InvalidPurchase)}}

 pub fn reserve_billing(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=BillingPayload::decode(&tx.payload).map_err(|_|StateError::InvalidPurchase)?;
  if p.product_id!=1||p.billing_id==[0;32]||p.merchant==[0;32]||p.reference==[0;32]||p.amount==0{return Err(StateError::InvalidPurchase)}
  if self.billing.contains_key(&p.billing_id){return Err(StateError::DuplicatePurchase)}
  if p.amount!=self.quote_price(p.product_id)?{return Err(StateError::InvalidPurchase)}
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  if balance<p.amount{return Err(StateError::InsufficientBalanceForPurchase)}
  let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance-=p.amount;a.locked=a.locked.checked_add(p.amount).ok_or(StateError::ArithmeticOverflow)?;
  self.billing.insert(p.billing_id,BillingRecord{billing_id:p.billing_id,buyer:tx.sender,merchant:p.merchant,product_id:p.product_id,amount:p.amount,reference:p.reference,status:2,reserved_at:self.height,captured_at:None,refunded_at:None});
  self.advance_nonce(tx.sender)
 }

 pub fn capture_billing(&mut self,billing_id:Hash)->Result<(),StateError>{
  let b=self.billing.get(&billing_id).cloned().ok_or(StateError::InvalidPurchase)?;if b.status!=2{return Err(StateError::InvalidPurchase)}
  let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.locked=a.locked.checked_sub(b.amount).ok_or(StateError::ArithmeticOverflow)?;
  let m=self.accounts.entry(b.merchant).or_insert(Account{address:b.merchant,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});m.balance=m.balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?;
  let x=self.billing.get_mut(&billing_id).ok_or(StateError::InvalidPurchase)?;x.status=3;x.captured_at=Some(self.height);Ok(())
 }

 pub fn refund_billing(&mut self,billing_id:Hash)->Result<(),StateError>{
  let b=self.billing.get(&billing_id).cloned().ok_or(StateError::InvalidPurchase)?;if b.status!=2&&b.status!=3{return Err(StateError::InvalidPurchase)}
  if b.status==2{let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.locked=a.locked.checked_sub(b.amount).ok_or(StateError::ArithmeticOverflow)?;a.balance=a.balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?}
  else{let m=self.accounts.get_mut(&b.merchant).ok_or(StateError::AccountNotFound)?;if m.balance<b.amount{return Err(StateError::InsufficientBalanceForPurchase)}m.balance-=b.amount;let a=self.accounts.get_mut(&b.buyer).ok_or(StateError::AccountNotFound)?;a.balance=a.balance.checked_add(b.amount).ok_or(StateError::ArithmeticOverflow)?}
  let x=self.billing.get_mut(&billing_id).ok_or(StateError::InvalidPurchase)?;x.status=4;x.refunded_at=Some(self.height);Ok(())
 }

 pub fn purchase(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=PurchasePayload::decode(&tx.payload).map_err(|_|StateError::InvalidPurchase)?;
  if p.product_id!=1||p.merchant==[0;32]||p.reference==[0;32]||p.price==0{return Err(StateError::InvalidPurchase)}
  let id=freedom_primitives::hash_bytes(&[b"FREEDOM-PURCHASE-V1".as_slice(),&tx.sender,&p.reference].concat());
  if self.purchases.contains_key(&id){return Err(StateError::DuplicatePurchase)}
  let buyer_balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  if buyer_balance<p.price{return Err(StateError::InsufficientBalanceForPurchase)}
  if p.merchant==tx.sender{return Err(StateError::InvalidPurchase)}
  {let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance-=p.price;}
  let merchant=self.accounts.entry(p.merchant).or_insert(Account{address:p.merchant,public_key:[0;32],nonce:0,balance:0,locked:0,staked:0});
  merchant.balance=merchant.balance.checked_add(p.price).ok_or(StateError::ArithmeticOverflow)?;
  self.purchases.insert(id,Purchase{purchase_id:id,buyer:tx.sender,merchant:p.merchant,product_id:p.product_id,price:p.price,reference:p.reference,block_height:self.height});
  self.advance_nonce(tx.sender)
 }

 fn state_leaf(domain: &[u8], key: &[u8], value: &[u8]) -> Hash {
  freedom_primitives::domain_hash(freedom_primitives::STATE_LEAF_DOMAIN, &[domain,key,value])
 }
 fn put_len(out:&mut Vec<u8>, n:usize){out.extend_from_slice(&(n as u64).to_le_bytes());}
 fn put_bytes(out:&mut Vec<u8>, b:&[u8]){put_len(out,b.len());out.extend_from_slice(b);}
 fn enc_account(a:&Account)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&a.address);o.extend_from_slice(&a.public_key);o.extend_from_slice(&a.nonce.to_le_bytes());o.extend_from_slice(&a.balance.to_le_bytes());o.extend_from_slice(&a.locked.to_le_bytes());o.extend_from_slice(&a.staked.to_le_bytes());o}
 fn enc_asset(a:&AssetDefinition)->Vec<u8>{
  let mut o=Vec::new();
  o.extend_from_slice(&a.asset_id);
  o.extend_from_slice(&a.source_chain_id.to_le_bytes());
  o.push(a.asset_type as u8);
  put_len(&mut o,a.contract_address.len());
  o.extend_from_slice(&a.contract_address);
  put_len(&mut o,a.symbol.len());
  o.extend_from_slice(a.symbol.as_bytes());
  o.push(a.decimals);
  o.extend_from_slice(&a.issuer);
  o.extend_from_slice(&a.bridge_id);
  o.push(a.status as u8);
  o
 }
 fn enc_node(n:&Node)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&n.node_id);o.extend_from_slice(&n.owner);o.extend_from_slice(&n.capabilities.to_le_bytes());o.extend_from_slice(&n.reputation.to_le_bytes());o.extend_from_slice(&n.total_wu.to_le_bytes());o.extend_from_slice(&n.successful_tasks.to_le_bytes());o.extend_from_slice(&n.failed_tasks.to_le_bytes());o}
 fn enc_contribution(c:&Contribution)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&c.contribution_id);o.extend_from_slice(&c.node_id);o.extend_from_slice(&c.task_id);o.extend_from_slice(&c.work_units.to_le_bytes());o.extend_from_slice(&c.quality_score.to_le_bytes());o.extend_from_slice(&c.verification_score.to_le_bytes());o.extend_from_slice(&c.result_hash);put_len(&mut o,c.verifier_set.len());for v in &c.verifier_set{o.extend_from_slice(v);}o.push(match c.status{ContributionStatus::Submitted=>1,ContributionStatus::Verified=>2,ContributionStatus::Rejected=>3,ContributionStatus::Rewarded=>4});match c.reward_id{Some(id)=>{o.push(1);o.extend_from_slice(&id)},None=>o.push(0)}o.extend_from_slice(&c.reward_amount.to_le_bytes());o}
 fn enc_reward(r:&Reward)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.reward_id);o.extend_from_slice(&r.owner);o.extend_from_slice(&r.contribution_id);o.extend_from_slice(&r.wu.to_le_bytes());o.extend_from_slice(&r.gross_reward.to_le_bytes());o.extend_from_slice(&r.final_reward.to_le_bytes());o.extend_from_slice(&r.epoch.to_le_bytes());o}
 fn enc_purchase(p:&Purchase)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&p.purchase_id);o.extend_from_slice(&p.buyer);o.extend_from_slice(&p.merchant);o.push(p.product_id);o.extend_from_slice(&p.price.to_le_bytes());o.extend_from_slice(&p.reference);o.extend_from_slice(&p.block_height.to_le_bytes());o}
 fn enc_billing(b:&BillingRecord)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&b.billing_id);o.extend_from_slice(&b.buyer);o.extend_from_slice(&b.merchant);o.push(b.product_id);o.extend_from_slice(&b.amount.to_le_bytes());o.extend_from_slice(&b.reference);o.push(b.status);o.extend_from_slice(&b.reserved_at.to_le_bytes());match b.captured_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}match b.refunded_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}o}
 fn enc_ticket(t:&Ticket)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&t.ticket_id);o.extend_from_slice(&t.owner);o.extend_from_slice(&t.draw_id);o.extend_from_slice(&t.created_at.to_le_bytes());o}
 fn enc_draw(d:&DrawState)->Vec<u8>{let mut o=Vec::new();let c=&d.config;o.extend_from_slice(&c.draw_id);o.push(c.draw_type as u8);o.extend_from_slice(&c.ticket_price_mfdm.to_le_bytes());o.extend_from_slice(&c.winner_count.to_le_bytes());o.extend_from_slice(&c.starts_at.to_le_bytes());o.extend_from_slice(&c.ends_at.to_le_bytes());o.extend_from_slice(&c.seed_block.to_le_bytes());o.extend_from_slice(&d.total_tickets.to_le_bytes());o.extend_from_slice(&d.prize_pool_mfdm.to_le_bytes());o.extend_from_slice(&d.cu_fund_mfdm.to_le_bytes());o.extend_from_slice(&d.annual_reserve_mfdm.to_le_bytes());o.push(d.executed as u8);o}
 fn enc_draw_result(r:&DrawResult)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.draw_id);o.extend_from_slice(&r.total_tickets.to_le_bytes());o.extend_from_slice(&r.prize_pool_mfdm.to_le_bytes());o.extend_from_slice(&r.cu_fund_mfdm.to_le_bytes());o.extend_from_slice(&r.annual_reserve_mfdm.to_le_bytes());put_len(&mut o,r.winners.len());for w in &r.winners{o.extend_from_slice(w);}o.extend_from_slice(&r.randomness_seed);o.extend_from_slice(&r.selection_proof);o}
 fn enc_evidence(e:&freedom_types::EquivocationEvidence)->Vec<u8>{let mut o=e.signing_bytes();o.extend_from_slice(&e.evidence_hash);o}
 fn enc_validator(v:&Validator)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&v.validator_id);o.extend_from_slice(&v.owner);o.extend_from_slice(&v.consensus_key);o.extend_from_slice(&v.stake.to_le_bytes());o.extend_from_slice(&v.unbonding_stake.to_le_bytes());o.extend_from_slice(&v.blocks_proposed.to_le_bytes());o.extend_from_slice(&v.blocks_finalized.to_le_bytes());o.extend_from_slice(&v.missed_votes.to_le_bytes());o}
 fn enc_unbonding(e:&UnbondingEntry)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&e.unbonding_id);o.extend_from_slice(&e.validator_id);o.extend_from_slice(&e.owner);o.extend_from_slice(&e.amount.to_le_bytes());o.extend_from_slice(&e.requested_height.to_le_bytes());o.extend_from_slice(&e.release_height.to_le_bytes());o}
 fn enc_ledger(l:&LedgerEntry)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&l.tx_hash);o.extend_from_slice(&l.sender);o.push(l.kind);o.extend_from_slice(&l.nonce.to_le_bytes());o.push(l.success as u8);o}
 fn enc_event(e:&Event)->Vec<u8>{let mut o=Vec::new();match e{Event::FdmTransferred{from,to,amount}=>{o.push(1);o.extend_from_slice(from);o.extend_from_slice(to);o.extend_from_slice(&amount.to_le_bytes())},Event::TicketCreated{ticket_id,draw_id,owner}=>{o.push(2);o.extend_from_slice(ticket_id);o.extend_from_slice(draw_id);o.extend_from_slice(owner)},Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve}=>{o.push(3);o.extend_from_slice(draw_id);o.extend_from_slice(&prize_pool.to_le_bytes());o.extend_from_slice(&cu_fund.to_le_bytes());o.extend_from_slice(&annual_reserve.to_le_bytes())},Event::DrawExecuted{draw_id,randomness_seed,selection_proof}=>{o.push(4);o.extend_from_slice(draw_id);o.extend_from_slice(randomness_seed);o.extend_from_slice(selection_proof)},Event::PrizePaid{draw_id,ticket_id,winner,amount}=>{o.push(5);o.extend_from_slice(draw_id);o.extend_from_slice(ticket_id);o.extend_from_slice(winner);o.extend_from_slice(&amount.to_le_bytes())},Event::EquivocationSlashed{evidence_hash,validator_id,amount}=>{o.push(6);o.extend_from_slice(evidence_hash);o.extend_from_slice(validator_id);o.extend_from_slice(&amount.to_le_bytes())}}o}
 pub fn state_root(&self)->Hash{
  let mut leaves=Vec::new();
  for (k,v) in &self.accounts{leaves.push(state_leaf(b"account",k,&enc_account(v)));}
  for (k,v) in &self.validators{leaves.push(state_leaf(b"validator",k,&enc_validator(v)));}
  for (k,v) in &self.asset_registry{leaves.push(state_leaf(b"asset",k,&enc_asset(v)));} for (k,v) in &self.processed_bridge_messages{leaves.push(state_leaf(b"bridge_replay",k,&v.message_hash));} for (k,v) in &self.bridge_message_status{leaves.push(state_leaf(b"bridge_status",k,&[*v as u8]));} for (k,v) in &self.bridge_withdrawals{leaves.push(state_leaf(b"bridge_withdrawal",k,&serde_json::to_vec(v).expect("bridge withdrawal serialization")));} for (k,v) in &self.bridge_withdrawal_status{leaves.push(state_leaf(b"bridge_withdrawal_status",k,&[*v as u8]));} for (k,v) in &self.bridge_accounting{leaves.push(state_leaf(b"bridge_accounting",k,&serde_json::to_vec(v).expect("bridge accounting serialization")));} for (k,v) in &self.bridge_security_policies{leaves.push(state_leaf(b"bridge_security_policy",k,&serde_json::to_vec(v).expect("bridge security policy serialization")));} for (k,v) in &self.bridge_approval_sets{leaves.push(state_leaf(b"bridge_approval_set",k,&serde_json::to_vec(v).expect("bridge approval set serialization")));} for (k,v) in &self.bitcoin_checkpoints{leaves.push(state_leaf(b"bitcoin_checkpoint",k,&serde_json::to_vec(v).expect("bitcoin checkpoint serialization")));} for (k,v) in &self.bridge_nonce_registry{leaves.push(state_leaf(b"bridge_nonce_registry",k,&serde_json::to_vec(v).expect("bridge nonce registry serialization")));} for (k,v) in &self.bridge_pause_authorizations{leaves.push(state_leaf(b"bridge_pause_authorization",k,&serde_json::to_vec(v).expect("bridge pause authorization serialization")));} for (k,v) in &self.bridge_rate_limits{leaves.push(state_leaf(b"bridge_rate_limit",k,&serde_json::to_vec(v).expect("bridge rate limit serialization")));} for (k,v) in &self.bridge_withdrawal_rate_limit_period{leaves.push(state_leaf(b"bridge_withdrawal_rate_limit_period",k,&serde_json::to_vec(v).expect("bridge withdrawal rate limit period serialization")));}
  for (k,v) in &self.unbonding{leaves.push(state_leaf(b"unbonding",k,&enc_unbonding(v)));}
  for (k,v) in &self.active_validator_set{let mut x=Vec::new();x.extend_from_slice(&v.validator_id);x.extend_from_slice(&v.owner);x.extend_from_slice(&v.consensus_key);x.extend_from_slice(&v.stake.to_le_bytes());leaves.push(state_leaf(b"active_validator",k,&x));} for (k,v) in &self.pending_validator_set{let mut x=Vec::new();x.extend_from_slice(&v.validator_id);x.extend_from_slice(&v.owner);x.extend_from_slice(&v.consensus_key);x.extend_from_slice(&v.stake.to_le_bytes());leaves.push(state_leaf(b"pending_validator",k,&x));} if let Some(cert)=&self.validator_set_certificate{leaves.push(state_leaf(b"validator_set_certificate",&[0;1],&cert.hash()));}
  for (k,v) in &self.evidence{leaves.push(state_leaf(b"evidence",k,&enc_evidence(v)));}
  for (k,v) in &self.slashed_evidence{leaves.push(state_leaf(b"slashed_evidence",k,&v.to_le_bytes()));}
  for (k,v) in &self.nodes{leaves.push(state_leaf(b"node",k,&enc_node(v)));}
  for (k,v) in &self.contributions{leaves.push(state_leaf(b"contribution",k,&enc_contribution(v)));}
  for (k,v) in &self.rewards{leaves.push(state_leaf(b"reward",k,&enc_reward(v)));}
  for (k,v) in &self.purchases{leaves.push(state_leaf(b"purchase",k,&enc_purchase(v)));}
  for (k,v) in &self.billing{leaves.push(state_leaf(b"billing",k,&enc_billing(v)));}
  for (k,v) in &self.tickets{leaves.push(state_leaf(b"ticket",k,&enc_ticket(v)));}
  for (k,v) in &self.draws{leaves.push(state_leaf(b"draw",k,&enc_draw(v)));}
  for (k,v) in &self.draw_results{leaves.push(state_leaf(b"draw_result",k,&enc_draw_result(v)));}
  for (k,v) in &self.ledger{let key=l.tx_hash;leaves.push(state_leaf(b"ledger",&key,&enc_ledger(l)));}
  for (e) in &self.events{let value=enc_event(e);leaves.push(state_leaf(b"event",&(leaves.len() as u64).to_le_bytes(),&value));}
  let mut monetary=Vec::new();monetary.extend_from_slice(&self.height.to_le_bytes());monetary.extend_from_slice(&self.epoch.to_le_bytes());monetary.extend_from_slice(&self.epoch_length.to_le_bytes());// last_block_hash is excluded from state_root to avoid a circular state-root/block-hash dependency.monetary.extend_from_slice(&self.cu_fund_mfdm.to_le_bytes());monetary.extend_from_slice(&self.annual_reserve_mfdm.to_le_bytes());monetary.extend_from_slice(&self.draw_prize_pool_mfdm.to_le_bytes());monetary.extend_from_slice(&self.supply.total.to_le_bytes());monetary.extend_from_slice(&self.supply.circulating.to_le_bytes());monetary.extend_from_slice(&self.supply.locked.to_le_bytes());monetary.extend_from_slice(&self.supply.staked.to_le_bytes());monetary.extend_from_slice(&self.supply.treasury.to_le_bytes());monetary.extend_from_slice(&self.epoch_issuance.to_le_bytes());monetary.extend_from_slice(&self.epoch_cap.to_le_bytes());monetary.push(self.staking_activated as u8);
  leaves.push(state_leaf(b"meta",&[0;1],&monetary));
  leaves.sort();
  freedom_primitives::merkle_root(&leaves)
 }
 pub fn assert_invariants(&self){assert!(self.supply.is_consistent());for (id,asset) in &self.asset_registry{assert_eq!(id,&asset.asset_id);assert!(asset.validate_identity());assert!(!asset.symbol.is_empty());}for (replay_key,message) in &self.processed_bridge_messages{assert_eq!(replay_key,&message.replay_key());assert!(message.validate_hash());assert_eq!(message.protocol_version,BRIDGE_MESSAGE_PROTOCOL_VERSION);assert_eq!(message.destination_chain_id,freedom_primitives::CHAIN_ID);}for (replay_key,status) in &self.bridge_message_status{assert!(self.processed_bridge_messages.contains_key(replay_key));assert!(*status as u8>=BridgeMessageStatus::Verified as u8&&*status as u8<=BridgeMessageStatus::Minted as u8);}for (id,p) in &self.bridge_security_policies{assert_eq!(id,&p.bridge_id);assert!(p.validate());}for (id,set) in &self.bridge_approval_sets{assert_eq!(id,&set.request_id);assert!(!set.approvals.is_empty());}for (id,u) in &self.bridge_rate_limits{assert!(self.bridge_security_policies.contains_key(id));assert!(u.used>0);}for (id,cp) in &self.bitcoin_checkpoints{assert!(*id!=[0;32]);assert!(cp.validate());} for (id,nonces) in &self.bridge_nonce_registry{assert!(!nonces.is_empty());} for (id,set) in &self.bridge_pause_authorizations{assert_eq!(id,&set.bridge_id);assert!(!set.authorizations.is_empty());} for (id,p) in &self.bridge_withdrawal_rate_limit_period{assert!(self.bridge_withdrawals.contains_key(id));assert!(*p<=self.height);}for (id,w) in &self.bridge_withdrawals{assert_eq!(id,&w.withdrawal_id);assert!(w.validate_hash());assert_eq!(w.compute_id(),w.withdrawal_id);assert!(self.bridge_withdrawal_status.contains_key(id));}for (id,status) in &self.bridge_withdrawal_status{assert!(self.bridge_withdrawals.contains_key(id));assert!((*status as u8>=BridgeWithdrawalStatus::Requested as u8&&*status as u8<=BridgeWithdrawalStatus::Rejected as u8));}for id in self.bridge_accounting.keys(){assert!(self.validate_bridge_accounting(*id).is_ok());}let sum=self.accounts.values().try_fold(0u128,|s,a|{let h=a.balance.checked_add(a.locked).and_then(|v|v.checked_add(a.staked))?;s.checked_add(h)});assert!(sum.is_some(),"account balance overflow");assert_eq!(self.supply.circulating,sum.unwrap());assert!(self.epoch_issuance<=self.epoch_cap);assert!(self.cu_fund_mfdm.checked_add(self.annual_reserve_mfdm).and_then(|x|x.checked_add(self.draw_prize_pool_mfdm)).map(|x|x<=self.supply.treasury).unwrap_or(false));for v in self.validators.values(){assert_eq!(v.stake.checked_add(v.unbonding_stake).unwrap_or(u128::MAX),self.accounts.get(&v.owner).map(|a|a.staked).unwrap_or(0));}for c in self.contributions.values(){assert!(c.quality_score<=10000&&c.verification_score<=10000);if c.status==ContributionStatus::Rewarded{assert!(c.reward_id.is_some()&&c.reward_amount>0)}}}
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
 #[test] fn mint_transition_requires_acceptance(){
  let(mut state,key)=setup();
  assert_eq!(state.mark_bridge_message_minted(key),Err(StateError::InvalidBridgeMessage));
  state.accept_bridge_message(key).unwrap();
  state.mark_bridge_message_minted(key).unwrap();
  assert_eq!(state.bridge_message_status.get(&key),Some(&BridgeMessageStatus::Minted));
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
  s.height=UNBONDING_COOLDOWN-1; s.maybe_advance_epoch(); assert_eq!(s.accounts.get(&owner).unwrap().balance,0);
  s.height=UNBONDING_COOLDOWN; s.maybe_advance_epoch(); assert_eq!(s.accounts.get(&owner).unwrap().balance,100); assert_eq!(s.accounts.get(&owner).unwrap().staked,0); assert_eq!(s.validators.get(&id).unwrap().unbonding_stake,0); assert!(s.unbonding.is_empty());
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
 #[test] fn exited_validator_does_not_resurrect_at_epoch_boundary(){let(mut s,o,id)=setup();s.execute_transaction(&tx(o,0,TxKind::Stake,StakePayload{amount:100}.encode())).unwrap();s.execute_transaction(&tx(o,1,TxKind::Unstake,UnstakePayload{amount:100}.encode())).unwrap();s.height=UNBONDING_COOLDOWN;s.maybe_advance_epoch();assert_eq!(s.validator_lifecycle(id).unwrap(),ValidatorLifecycle::Exited);assert!(s.active_validator_set.is_empty());s.height=s.epoch_length;s.maybe_advance_epoch();assert!(s.active_validator_set.is_empty());}
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
 fn genesis_registers_frc_as_native_asset() {
  let state=ChainState::genesis(&Genesis::default());
  let frc=state.asset_registry.values().find(|a| a.symbol=="FRC").unwrap();
  assert_eq!(frc.source_chain_id, freedom_primitives::CHAIN_ID);
  assert_eq!(frc.asset_type, AssetType::Native);
  assert_eq!(frc.status, AssetStatus::Active);
  assert!(frc.validate_identity());
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
  let valid=bridged_asset(1, b"0xabc", [3u8;32]);
  state.register_asset(valid.clone()).unwrap();
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
