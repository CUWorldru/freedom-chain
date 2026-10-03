use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use freedom_primitives::{address_from_public_key, Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, RegisterNodePayload, SubmitContributionPayload, BillingPayload, PurchasePayload, CreateTicketPayload, DrawConfig, DrawResult, Event, Transaction, TxKind, TransferPayload, Validator};

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

#[derive(Clone, Debug, Serialize, Deserialize)] pub struct ChainState { pub height:u64,pub last_block_hash:Hash,pub accounts:BTreeMap<Address,Account>,pub validators:BTreeMap<Hash,Validator>,pub nodes:BTreeMap<Hash,Node>,pub contributions:BTreeMap<Hash,Contribution>,pub rewards:BTreeMap<Hash,Reward>,pub purchases:BTreeMap<Hash,Purchase>,pub billing:BTreeMap<Hash,BillingRecord>,pub tickets:BTreeMap<Hash,Ticket>,pub draws:BTreeMap<Hash,DrawState>,pub draw_results:BTreeMap<Hash,DrawResult>,pub cu_fund_mfdm:u128,pub annual_reserve_mfdm:u128,pub draw_prize_pool_mfdm:u128,pub events:Vec<Event>,pub ledger:Vec<LedgerEntry>,pub supply:Supply,pub epoch_issuance:u128,pub epoch_cap:u128,pub evidence:BTreeMap<Hash,freedom_types::EquivocationEvidence>,pub slashed_evidence:BTreeMap<Hash,u128> }
#[derive(Debug,PartialEq,Eq)] pub enum StateError { AccountNotFound,InvalidNonce{expected:u64,actual:u64},InsufficientBalance,InvalidTransferPayload,InvalidRecipient,ArithmeticOverflow,UnsupportedTransaction,InvalidNodePayload,InvalidNode,DuplicateNode,InvalidContributionPayload,InvalidContribution,DuplicateContribution,InvalidWorkUnits,InvalidQualityScore,InvalidVerification,AlreadyVerified,ContributionNotRewardable,DuplicateReward,InvalidReward,IssuanceCapExceeded,InvalidPurchase,DuplicatePurchase,InsufficientBalanceForPurchase,InvalidEvidence,InvalidTicketPayload,InvalidDraw,InvalidTicketQuantity,InvalidTicketPrice,DuplicateTicket,DrawNotReady,DrawAlreadyExecuted,InsufficientPrizePool }
impl ChainState {
 pub fn record_equivocation(&mut self,evidence:freedom_types::EquivocationEvidence)->Result<bool,StateError>{
  freedom_consensus::validate_evidence(&evidence).map_err(|_|StateError::InvalidEvidence)?;
  if self.evidence.contains_key(&evidence.evidence_hash){return Ok(false)}
  self.evidence.insert(evidence.evidence_hash,evidence); Ok(true)
 }
 pub fn slash_evidence(&mut self,evidence_hash:Hash,amount:u128)->Result<u128,StateError>{
  if self.slashed_evidence.contains_key(&evidence_hash){return Ok(0)}
  let evidence=self.evidence.get(&evidence_hash).ok_or(StateError::InvalidEvidence)?;
  let validator=self.validators.get_mut(&evidence.validator_id).ok_or(StateError::InvalidEvidence)?;
  let slash=amount.min(validator.stake);
  validator.stake-=slash;
  self.slashed_evidence.insert(evidence_hash,slash);
  Ok(slash)
 }

 pub fn genesis(g:&Genesis)->Self{assert_eq!(g.initial_supply,GENESIS_SUPPLY,"Genesis FDM must be zero");let validators=g.validators.iter().cloned().map(|v|(v.validator_id,v)).collect();Self{height:0,last_block_hash:[0;32],accounts:BTreeMap::new(),validators,nodes:BTreeMap::new(),contributions:BTreeMap::new(),rewards:BTreeMap::new(),purchases:BTreeMap::new(),billing:BTreeMap::new(),tickets:BTreeMap::new(),draws:BTreeMap::new(),draw_results:BTreeMap::new(),cu_fund_mfdm:0,annual_reserve_mfdm:0,draw_prize_pool_mfdm:0,events:Vec::new(),ledger:Vec::new(),supply:Supply::genesis(),epoch_issuance:0,epoch_cap:1_000_000,evidence:BTreeMap::new(),slashed_evidence:BTreeMap::new()}}
 pub fn insert_account(&mut self,a:Account){self.accounts.insert(a.address,a);}
 pub fn drain_events(&mut self)->Vec<Event>{std::mem::take(&mut self.events)}
 pub fn execute_transaction(&mut self,tx:&Transaction)->Result<(),StateError>{let nonce=self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?.nonce;if nonce!=tx.nonce{return Err(StateError::InvalidNonce{expected:nonce,actual:tx.nonce})}let result=match &tx.kind{TxKind::Transfer=>self.execute_transfer(tx),TxKind::RegisterNode=>self.register_node(tx),TxKind::SubmitContribution=>self.submit_contribution(tx),TxKind::ClaimReward=>self.claim_reward(tx),TxKind::Purchase=>self.purchase(tx),TxKind::CreateTicket=>self.create_ticket(tx),_=>Err(StateError::UnsupportedTransaction)}; if result.is_ok(){self.ledger.push(LedgerEntry{tx_hash:tx.hash(),sender:tx.sender,kind:tx.kind.code(),nonce:tx.nonce,success:true});} result}
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
 fn enc_node(n:&Node)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&n.node_id);o.extend_from_slice(&n.owner);o.extend_from_slice(&n.capabilities.to_le_bytes());o.extend_from_slice(&n.reputation.to_le_bytes());o.extend_from_slice(&n.total_wu.to_le_bytes());o.extend_from_slice(&n.successful_tasks.to_le_bytes());o.extend_from_slice(&n.failed_tasks.to_le_bytes());o}
 fn enc_contribution(c:&Contribution)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&c.contribution_id);o.extend_from_slice(&c.node_id);o.extend_from_slice(&c.task_id);o.extend_from_slice(&c.work_units.to_le_bytes());o.extend_from_slice(&c.quality_score.to_le_bytes());o.extend_from_slice(&c.verification_score.to_le_bytes());o.extend_from_slice(&c.result_hash);put_len(&mut o,c.verifier_set.len());for v in &c.verifier_set{o.extend_from_slice(v);}o.push(match c.status{ContributionStatus::Submitted=>1,ContributionStatus::Verified=>2,ContributionStatus::Rejected=>3,ContributionStatus::Rewarded=>4});match c.reward_id{Some(id)=>{o.push(1);o.extend_from_slice(&id)},None=>o.push(0)}o.extend_from_slice(&c.reward_amount.to_le_bytes());o}
 fn enc_reward(r:&Reward)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.reward_id);o.extend_from_slice(&r.owner);o.extend_from_slice(&r.contribution_id);o.extend_from_slice(&r.wu.to_le_bytes());o.extend_from_slice(&r.gross_reward.to_le_bytes());o.extend_from_slice(&r.final_reward.to_le_bytes());o.extend_from_slice(&r.epoch.to_le_bytes());o}
 fn enc_purchase(p:&Purchase)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&p.purchase_id);o.extend_from_slice(&p.buyer);o.extend_from_slice(&p.merchant);o.push(p.product_id);o.extend_from_slice(&p.price.to_le_bytes());o.extend_from_slice(&p.reference);o.extend_from_slice(&p.block_height.to_le_bytes());o}
 fn enc_billing(b:&BillingRecord)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&b.billing_id);o.extend_from_slice(&b.buyer);o.extend_from_slice(&b.merchant);o.push(b.product_id);o.extend_from_slice(&b.amount.to_le_bytes());o.extend_from_slice(&b.reference);o.push(b.status);o.extend_from_slice(&b.reserved_at.to_le_bytes());match b.captured_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}match b.refunded_at{Some(v)=>{o.push(1);o.extend_from_slice(&v.to_le_bytes())},None=>o.push(0)}o}
 fn enc_ticket(t:&Ticket)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&t.ticket_id);o.extend_from_slice(&t.owner);o.extend_from_slice(&t.draw_id);o.extend_from_slice(&t.created_at.to_le_bytes());o}
 fn enc_draw(d:&DrawState)->Vec<u8>{let mut o=Vec::new();let c=&d.config;o.extend_from_slice(&c.draw_id);o.push(c.draw_type as u8);o.extend_from_slice(&c.ticket_price_mfdm.to_le_bytes());o.extend_from_slice(&c.winner_count.to_le_bytes());o.extend_from_slice(&c.starts_at.to_le_bytes());o.extend_from_slice(&c.ends_at.to_le_bytes());o.extend_from_slice(&c.seed_block.to_le_bytes());o.extend_from_slice(&d.total_tickets.to_le_bytes());o.extend_from_slice(&d.prize_pool_mfdm.to_le_bytes());o.extend_from_slice(&d.cu_fund_mfdm.to_le_bytes());o.extend_from_slice(&d.annual_reserve_mfdm.to_le_bytes());o.push(d.executed as u8);o}
 fn enc_draw_result(r:&DrawResult)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&r.draw_id);o.extend_from_slice(&r.total_tickets.to_le_bytes());o.extend_from_slice(&r.prize_pool_mfdm.to_le_bytes());o.extend_from_slice(&r.cu_fund_mfdm.to_le_bytes());o.extend_from_slice(&r.annual_reserve_mfdm.to_le_bytes());put_len(&mut o,r.winners.len());for w in &r.winners{o.extend_from_slice(w);}o.extend_from_slice(&r.randomness_seed);o.extend_from_slice(&r.selection_proof);o}
 fn enc_validator(v:&Validator)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&v.validator_id);o.extend_from_slice(&v.owner);o.extend_from_slice(&v.consensus_key);o.extend_from_slice(&v.stake.to_le_bytes());o.extend_from_slice(&v.blocks_proposed.to_le_bytes());o.extend_from_slice(&v.blocks_finalized.to_le_bytes());o.extend_from_slice(&v.missed_votes.to_le_bytes());o}
 fn enc_ledger(l:&LedgerEntry)->Vec<u8>{let mut o=Vec::new();o.extend_from_slice(&l.tx_hash);o.extend_from_slice(&l.sender);o.push(l.kind);o.extend_from_slice(&l.nonce.to_le_bytes());o.push(l.success as u8);o}
 fn enc_event(e:&Event)->Vec<u8>{let mut o=Vec::new();match e{Event::FdmTransferred{from,to,amount}=>{o.push(1);o.extend_from_slice(from);o.extend_from_slice(to);o.extend_from_slice(&amount.to_le_bytes())},Event::TicketCreated{ticket_id,draw_id,owner}=>{o.push(2);o.extend_from_slice(ticket_id);o.extend_from_slice(draw_id);o.extend_from_slice(owner)},Event::DrawFundAllocated{draw_id,prize_pool,cu_fund,annual_reserve}=>{o.push(3);o.extend_from_slice(draw_id);o.extend_from_slice(&prize_pool.to_le_bytes());o.extend_from_slice(&cu_fund.to_le_bytes());o.extend_from_slice(&annual_reserve.to_le_bytes())},Event::DrawExecuted{draw_id,randomness_seed,selection_proof}=>{o.push(4);o.extend_from_slice(draw_id);o.extend_from_slice(randomness_seed);o.extend_from_slice(selection_proof)},Event::PrizePaid{draw_id,ticket_id,winner,amount}=>{o.push(5);o.extend_from_slice(draw_id);o.extend_from_slice(ticket_id);o.extend_from_slice(winner);o.extend_from_slice(&amount.to_le_bytes())}}o}
 pub fn state_root(&self)->Hash{
  let mut leaves=Vec::new();
  for (k,v) in &self.accounts{leaves.push(state_leaf(b"account",k,&enc_account(v)));}
  for (k,v) in &self.validators{leaves.push(state_leaf(b"validator",k,&enc_validator(v)));}
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
  let mut monetary=Vec::new();monetary.extend_from_slice(&self.height.to_le_bytes());// last_block_hash is excluded from state_root to avoid a circular state-root/block-hash dependency.monetary.extend_from_slice(&self.cu_fund_mfdm.to_le_bytes());monetary.extend_from_slice(&self.annual_reserve_mfdm.to_le_bytes());monetary.extend_from_slice(&self.draw_prize_pool_mfdm.to_le_bytes());monetary.extend_from_slice(&self.supply.total.to_le_bytes());monetary.extend_from_slice(&self.supply.circulating.to_le_bytes());monetary.extend_from_slice(&self.supply.locked.to_le_bytes());monetary.extend_from_slice(&self.supply.staked.to_le_bytes());monetary.extend_from_slice(&self.supply.treasury.to_le_bytes());monetary.extend_from_slice(&self.epoch_issuance.to_le_bytes());monetary.extend_from_slice(&self.epoch_cap.to_le_bytes());
  leaves.push(state_leaf(b"meta",&[0;1],&monetary));
  leaves.sort();
  freedom_primitives::merkle_root(&leaves)
 }
 pub fn assert_invariants(&self){assert!(self.supply.is_consistent());let sum=self.accounts.values().try_fold(0u128,|s,a|{let h=a.balance.checked_add(a.locked).and_then(|v|v.checked_add(a.staked))?;s.checked_add(h)});assert!(sum.is_some(),"account balance overflow");assert_eq!(self.supply.circulating,sum.unwrap());assert!(self.epoch_issuance<=self.epoch_cap);assert!(self.cu_fund_mfdm.checked_add(self.annual_reserve_mfdm).and_then(|x|x.checked_add(self.draw_prize_pool_mfdm)).map(|x|x<=self.supply.treasury).unwrap_or(false));for c in self.contributions.values(){assert!(c.quality_score<=10000&&c.verification_score<=10000);if c.status==ContributionStatus::Rewarded{assert!(c.reward_id.is_some()&&c.reward_amount>0)}}}
}
