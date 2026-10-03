use std::collections::BTreeMap;
use freedom_primitives::{address_from_public_key, Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, RegisterNodePayload, SubmitContributionPayload, Transaction, TxKind, TransferPayload, Validator};

#[derive(Clone, Debug)] pub struct Account { pub address:Address,pub public_key:[u8;32],pub nonce:u64,pub balance:u128,pub locked:u128,pub staked:u128 }
impl Account { pub fn new(public_key:[u8;32],balance:u128)->Self{Self{address:address_from_public_key(&public_key),public_key,nonce:0,balance,locked:0,staked:0}} }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node { pub node_id:Hash,pub owner:Address,pub capabilities:u64,pub reputation:u64,pub total_wu:u128,pub successful_tasks:u64,pub failed_tasks:u64 }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum ContributionStatus { Submitted, Verified, Rejected }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contribution { pub contribution_id:Hash,pub node_id:Hash,pub task_id:Hash,pub work_units:u128,pub quality_score:u32,pub verification_score:u32,pub result_hash:Hash,pub verifier_set:Vec<Hash>,pub status:ContributionStatus }

#[derive(Clone, Debug)]
pub struct ChainState { pub height:u64,pub last_block_hash:Hash,pub accounts:BTreeMap<Address,Account>,pub validators:BTreeMap<Hash,Validator>,pub nodes:BTreeMap<Hash,Node>,pub contributions:BTreeMap<Hash,Contribution>,pub supply:Supply }

#[derive(Debug,PartialEq,Eq)]
pub enum StateError {
 AccountNotFound,InvalidNonce{expected:u64,actual:u64},InsufficientBalance,InvalidTransferPayload,InvalidRecipient,
 ArithmeticOverflow,UnsupportedTransaction,InvalidNodePayload,InvalidNode,DuplicateNode,InvalidContributionPayload,
 InvalidContribution,DuplicateContribution,InvalidWorkUnits,InvalidQualityScore,InvalidVerification,AlreadyVerified
}
impl ChainState {
 pub fn genesis(genesis:&Genesis)->Self{assert_eq!(genesis.initial_supply,GENESIS_SUPPLY,"Genesis FDM must be zero");let validators=genesis.validators.iter().cloned().map(|v|(v.validator_id,v)).collect();Self{height:0,last_block_hash:[0u8;32],accounts:BTreeMap::new(),validators,nodes:BTreeMap::new(),contributions:BTreeMap::new(),supply:Supply::genesis()}}
 pub fn insert_account(&mut self,a:Account){self.accounts.insert(a.address,a);}
 pub fn execute_transaction(&mut self,tx:&Transaction)->Result<(),StateError>{
  let nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce;
  if nonce!=tx.nonce{return Err(StateError::InvalidNonce{expected:nonce,actual:tx.nonce})}
  match &tx.kind{TxKind::Transfer=>self.execute_transfer(tx),TxKind::RegisterNode=>self.register_node(tx),TxKind::SubmitContribution=>self.submit_contribution(tx),_=>Err(StateError::UnsupportedTransaction)}
 }
 fn advance_nonce(&mut self,sender:Address)->Result<(),StateError>{let a=self.accounts.get_mut(&sender).ok_or(StateError::AccountNotFound)?;a.nonce=a.nonce.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;Ok(())}
 fn execute_transfer(&mut self,tx:&Transaction)->Result<(),StateError>{
  let balance=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.balance;
  let t=TransferPayload::decode(&tx.payload).map_err(|_|StateError::InvalidTransferPayload)?;if t.to==[0u8;32]{return Err(StateError::InvalidRecipient)}
  let cost=t.amount.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;if balance<cost{return Err(StateError::InsufficientBalance)}
  let nb=balance-cost;
  if t.to==tx.sender{let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance=nb.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?}
  else{let a=self.accounts.get_mut(&tx.sender).ok_or(StateError::AccountNotFound)?;a.balance=nb;let r=self.accounts.entry(t.to).or_insert(Account{address:t.to,public_key:[0u8;32],nonce:0,balance:0,locked:0,staked:0});r.balance=r.balance.checked_add(t.amount).ok_or(StateError::ArithmeticOverflow)?}
  self.supply.circulating=self.supply.circulating.checked_sub(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  self.supply.treasury=self.supply.treasury.checked_add(tx.fee).ok_or(StateError::ArithmeticOverflow)?;
  self.advance_nonce(tx.sender)
 }
 fn register_node(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=RegisterNodePayload::decode(&tx.payload).map_err(|_|StateError::InvalidNodePayload)?;if p.node_id==[0u8;32]{return Err(StateError::InvalidNode)}if self.nodes.contains_key(&p.node_id){return Err(StateError::DuplicateNode)}
  self.nodes.insert(p.node_id,Node{node_id:p.node_id,owner:tx.sender,capabilities:p.capabilities,reputation:100,total_wu:0,successful_tasks:0,failed_tasks:0});self.advance_nonce(tx.sender)
 }
 fn submit_contribution(&mut self,tx:&Transaction)->Result<(),StateError>{
  let p=SubmitContributionPayload::decode(&tx.payload).map_err(|_|StateError::InvalidContributionPayload)?;
  if p.contribution_id==[0u8;32]||p.task_id==[0u8;32]||p.node_id==[0u8;32]{return Err(StateError::InvalidContribution)}
  if p.work_units==0{return Err(StateError::InvalidWorkUnits)}if p.quality_score>10000{return Err(StateError::InvalidQualityScore)}if self.contributions.contains_key(&p.contribution_id){return Err(StateError::DuplicateContribution)}
  let node=self.nodes.get(&p.node_id).ok_or(StateError::InvalidNode)?;if node.owner!=tx.sender{return Err(StateError::InvalidNode)}
  self.contributions.insert(p.contribution_id,Contribution{contribution_id:p.contribution_id,node_id:p.node_id,task_id:p.task_id,work_units:p.work_units,quality_score:p.quality_score,verification_score:0,result_hash:p.result_hash,verifier_set:Vec::new(),status:ContributionStatus::Submitted});self.advance_nonce(tx.sender)
 }
 pub fn verify_contribution(&mut self,id:Hash,verifiers:Vec<Hash>,score:u32)->Result<(),StateError>{
  if verifiers.is_empty()||score>10000{return Err(StateError::InvalidVerification)}
  if verifiers.iter().any(|v|!self.validators.contains_key(v)){return Err(StateError::InvalidVerification)}
  let (node_id,wu)={let c=self.contributions.get(&id).ok_or(StateError::InvalidContribution)?;if c.status!=ContributionStatus::Submitted{return Err(StateError::AlreadyVerified)}(c.node_id,c.work_units)};
  let c=self.contributions.get_mut(&id).ok_or(StateError::InvalidContribution)?;c.verifier_set=verifiers;c.verification_score=score;c.status=ContributionStatus::Verified;
  let n=self.nodes.get_mut(&node_id).ok_or(StateError::InvalidNode)?;n.total_wu=n.total_wu.checked_add(wu).ok_or(StateError::ArithmeticOverflow)?;n.successful_tasks=n.successful_tasks.checked_add(1).ok_or(StateError::ArithmeticOverflow)?;Ok(())
 }
 pub fn assert_invariants(&self){assert!(self.supply.is_consistent());let sum=self.accounts.values().try_fold(0u128,|s,a|{let h=a.balance.checked_add(a.locked).and_then(|v|v.checked_add(a.staked))?;s.checked_add(h)});assert!(sum.is_some(),"account balance overflow");assert_eq!(self.supply.circulating,sum.unwrap());for c in self.contributions.values(){assert!(c.quality_score<=10000);assert!(c.verification_score<=10000);}}
}
