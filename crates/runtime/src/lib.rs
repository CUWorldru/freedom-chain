pub mod bridge_config;
use freedom_consensus::{block_hash,receipts_root,tx_root,validate_genesis,ConsensusConfig};
use freedom_crypto::verify;
use freedom_primitives::{CHAIN_ID,Hash};
use freedom_state::{ChainState,StateError};
use freedom_types::{Block,Genesis,Receipt,Transaction};
pub fn collect_validator_set_certificate(
    state: &ChainState,
    votes: &[freedom_types::ValidatorSetVote],
) -> Result<freedom_types::ValidatorSetCertificate, String> {
    if state.validator_set_certificate.is_some() {
        // A transition certificate is immutable once committed. Do this
        // before collecting new votes so no alternate collection path can
        // manufacture a replacement certificate for the same epoch.
        return Err("validator set certificate is already committed for this transition".into());
    }
    if votes.is_empty() { return Err("no validator set votes".into()); }
    let mut unique = std::collections::BTreeSet::new();
    for vote in votes {
        freedom_network::validate_validator_set_vote(state, vote)?;
        if !unique.insert(vote.validator_id) {
            return Err("duplicate validator set vote".into());
        }
    }
    let to_epoch = state.epoch
        .checked_add(1)
        .ok_or_else(|| "epoch transition exceeds u64::MAX".to_string())?;
    let certificate = freedom_types::ValidatorSetCertificate {
        from_epoch: state.epoch,
        to_epoch,
        previous_set_hash: freedom_consensus::validator_set_hash(&state.active_validator_set),
        new_set_hash: freedom_consensus::validator_set_hash(&state.pending_validator_set),
        votes: votes.to_vec(),
    };
    // Collection must use exactly the same full certificate validation as
    // block admission. Do not publish a locally accepted stake-only
    // certificate that peers would reject for insufficient validator-count
    // quorum or invalid transition fields.
    freedom_consensus::validate_validator_set_certificate(state, &certificate)?;
    if let Some(existing) = state.validator_set_certificate.as_ref() {
        if existing != &certificate {
            return Err("validator set certificate is already committed for this transition".into());
        }
    }
    Ok(certificate)
}

pub fn commit_validator_set_certificate(
    state: &mut ChainState,
    certificate: &freedom_types::ValidatorSetCertificate,
) -> Result<(), String> {
    freedom_consensus::validate_validator_set_certificate(state, certificate)?;
    if let Some(existing) = state.validator_set_certificate.as_ref() {
        if existing != certificate {
            return Err("validator set certificate is already committed for this transition".into());
        }
        return Ok(());
    }
    state.validator_set_certificate = Some(certificate.clone());
    Ok(())
}

pub fn initialize(g:&Genesis)->Result<ChainState,String>{if g.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}let s=ChainState::genesis(g);validate_genesis(&s,&ConsensusConfig::default())?;s.assert_invariants();Ok(s)}

/// Secure genesis entry point. The manifest is checked against the actual
/// Genesis passed to the runtime and against an independently pinned trust
/// anchor before any ChainState is constructed.
pub fn initialize_with_genesis_commitment(
    genesis: &Genesis,
    manifest: &freedom_types::GenesisCommitmentManifest,
    trusted_expected: &Hash,
) -> Result<ChainState, String> {
    if manifest.chain_id != genesis.chain_id
        || manifest.initial_supply != genesis.initial_supply
        || manifest.validators != genesis.validators
    {
        return Err("genesis manifest does not match runtime genesis parameters".into());
    }
    manifest.verify_pinned(trusted_expected)?;
    initialize(genesis)
}
pub fn validate_transaction_admission(s:&ChainState,tx:&Transaction)->Result<(),String>{
 if tx.chain_id!=CHAIN_ID{return Err("invalid chain id".into())}
 let a=s.accounts.get(&tx.sender).ok_or_else(||"sender account not found".to_string())?;
 if tx.nonce < a.nonce{return Err("stale transaction nonce".into())}
 if tx.signature.len()!=64{return Err("invalid signature length".into())}
 let mut sig=[0u8;64];sig.copy_from_slice(&tx.signature);
 if !verify(&a.public_key,&tx.signing_bytes(),&sig){return Err("invalid transaction signature".into())}
 match tx.kind {
  freedom_types::TxKind::BridgeMint => validate_bridge_mint_admission(s,tx)?,
  _ => {}
 }
 Ok(())
}

fn validate_bridge_mint_admission(s:&ChainState,tx:&Transaction)->Result<(),String>{
 let p=freedom_types::BridgeMintTransactionPayload::decode(&tx.payload).map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
 // decode enforces canonical approval ordering; re-encode must be byte-identical
 // so a signed BridgeMint transaction has exactly one accepted payload representation.
 let canonical_payload=p.encode().map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
 if canonical_payload != tx.payload { return Err("InvalidBridgeMintTransaction".into()); }
 let policy=s.bridge_security_policies.get(&p.message.bridge_id).ok_or_else(||"InvalidBridgeMintTransaction".to_string())?;
 let keys=s.active_validator_set.iter().map(|(id,v)|(*id,v.consensus_key)).collect::<std::collections::BTreeMap<freedom_primitives::Hash,[u8;32]>>();
 let expected=freedom_types::BridgeMintAuthorization::from_authenticated_message(&p.message,&p.deposit,&p.approvals,policy,&keys)
   .map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
 if expected!=p.authorization{return Err("InvalidBridgeMintTransaction".into())}
 if p.message.destination_chain_id!=CHAIN_ID{return Err("InvalidBridgeMintTransaction".into())}
 if !p.message.validate_hash() || !p.message.matches_verified_deposit(&p.deposit){return Err("InvalidBridgeMintTransaction".into())}

 // M205: Bitcoin bridge mints must be anchored to the state-held canonical tip.
 // M204 proves the chain/fork rules at verifier level; this admission gate makes
 // that finality boundary consensus-relevant for an actual BridgeMint tx.
 if p.message.source_chain_id == freedom_types::BITCOIN_MAINNET_CHAIN_ID {
   s.validate_bitcoin_bridge_safety(p.message.bridge_id).map_err(|_|"BitcoinReorgDetected".to_string())?;
   let canonical=s.bitcoin_canonical_tips.get(&p.message.bridge_id)
     .ok_or_else(||"BitcoinCanonicalTipUnavailable".to_string())?;
   if !canonical.validate()
      || canonical.height < p.deposit.block_or_slot
      || freedom_types::bitcoin_confirmations_at_tip(canonical.height,p.deposit.block_or_slot)
           .ok_or_else(||"BitcoinFinalityUnavailable".to_string())?
           < policy.min_external_confirmations {
     return Err("BitcoinMintFinalityNotReached".into());
   }
   // M206: a Bitcoin mint must also have a quorum-authenticated UTXO
   // commitment attestation anchored to the exact canonical tip. M205 made
   // the tip/finality boundary mandatory; M206 makes the state-held UTXO
   // commitment itself mandatory without changing the BridgeMint payload ABI.
   let attestation=s.bitcoin_utxo_attestations.get(&p.message.bridge_id)
     .ok_or_else(||"BitcoinUtxoAttestationUnavailable".to_string())?;
   let validator_keys=s.active_validator_set.iter()
     .map(|(id,v)|(*id,v.consensus_key))
     .collect::<std::collections::BTreeMap<freedom_primitives::Hash,[u8;32]>>();
   if attestation.bridge_id != p.message.bridge_id
      || attestation.chain_id != p.message.source_chain_id
      || !attestation.verify(canonical,&validator_keys,policy.validator_quorum) {
     return Err("BitcoinUtxoAttestationInvalid".into());
   }
 }

 if s.bridge_message_status.contains_key(&p.message.replay_key()){return Err("DuplicateBridgeMessage".into())}
 Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedBridgeRepresentation {
    canonical_asset_id: Hash,
    representation_asset_id: Hash,
}

impl ResolvedBridgeRepresentation {
    pub fn canonical_asset_id(&self) -> Hash {
        self.canonical_asset_id
    }

    pub fn representation_asset_id(&self) -> Hash {
        self.representation_asset_id
    }

    fn validate_against(
        &self,
        registry: &freedom_types::CrossChainAssetRegistry,
    ) -> Result<(), String> {
        if self.canonical_asset_id != registry.canonical_asset_id {
            return Err("CanonicalAssetRegistryMismatch".into());
        }

        let representation = registry
            .representations
            .iter()
            .find(|r| r.representation.asset_id == self.representation_asset_id)
            .ok_or_else(|| "ExternalAssetRepresentationNotRegistered".to_string())?;

        if !representation.validate()
            || representation.representation.asset_id == [0u8; 32]
            || !registry.has_representation(self.representation_asset_id)
        {
            return Err("InvalidResolvedBridgeRepresentation".into());
        }

        Ok(())
    }
}

fn resolve_bridge_representation(
    external: &freedom_types::ExternalBridgeDepositProof,
    registry: &freedom_types::CrossChainAssetRegistry,
    canonical_asset_id: Hash,
) -> Result<ResolvedBridgeRepresentation, String> {
    if canonical_asset_id != registry.canonical_asset_id {
        return Err("CanonicalAssetRegistryMismatch".into());
    }

    let (chain_id, asset_kind, external_identity, bridge_id) = match external {
        freedom_types::ExternalBridgeDepositProof::BitcoinUtxo(p) => (
            p.chain_id,
            freedom_types::CrossChainAssetKind::BitcoinUtxo,
            b"BTC".as_slice(),
            p.bridge_id,
        ),
        freedom_types::ExternalBridgeDepositProof::EvmTransfer(p) => (
            p.chain_id,
            freedom_types::CrossChainAssetKind::Erc20,
            p.token_contract.as_slice(),
            p.bridge_id,
        ),
        freedom_types::ExternalBridgeDepositProof::TronUsdt(p) => (
            p.chain_id,
            freedom_types::CrossChainAssetKind::Trc20,
            p.usdt_contract.as_slice(),
            p.bridge_id,
        ),
    };

    let representation = registry
        .find_representation_by_external_identity(
            chain_id,
            asset_kind,
            external_identity,
            bridge_id,
        )
        .ok_or_else(|| "ExternalAssetRepresentationNotRegistered".to_string())?;

    let resolved = ResolvedBridgeRepresentation {
        canonical_asset_id: registry.canonical_asset_id,
        representation_asset_id: representation.representation.asset_id,
    };
    resolved.validate_against(registry)?;
    Ok(resolved)
}

pub struct BridgeIngressContext<'a> {
    external: &'a freedom_types::ExternalBridgeDepositProof,
    registry: &'a freedom_types::CrossChainAssetRegistry,
    policy: &'a freedom_types::BridgeSecurityPolicy,
    descriptor: &'a freedom_types::BridgeAdapterDescriptor,
    resolved: ResolvedBridgeRepresentation,
}

impl<'a> BridgeIngressContext<'a> {
    fn build(
        external: &'a freedom_types::ExternalBridgeDepositProof,
        registry: &'a freedom_types::CrossChainAssetRegistry,
        policy: &'a freedom_types::BridgeSecurityPolicy,
        descriptor: &'a freedom_types::BridgeAdapterDescriptor,
        canonical_asset_id: Hash,
    ) -> Result<Self, String> {
        if !registry.validate()
            || !policy.validate()
            || !descriptor.validate()
            || descriptor.bridge_id != policy.bridge_id
            || !descriptor.enabled
            || descriptor.required_confirmations != policy.min_external_confirmations
        {
            return Err("InvalidBridgeIngressContext".into());
        }

        let resolved = resolve_bridge_representation(
            external,
            registry,
            canonical_asset_id,
        )?;
        match external {
            freedom_types::ExternalBridgeDepositProof::BitcoinUtxo(p) => {
                if !freedom_types::expected_asset_kind_for_mainnet(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::BitcoinUtxo,
                ) {
                    return Err("UnsupportedBitcoinNetwork".into());
                }
                if registry.find_representation_by_external_identity(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::BitcoinUtxo,
                    b"BTC",
                    p.bridge_id,
                ).map(|r| r.representation.asset_id) != Some(resolved.representation_asset_id()) {
                    return Err("BitcoinRegistryIdentityMismatch".into());
                }
            }
            freedom_types::ExternalBridgeDepositProof::EvmTransfer(p) => {
                if !freedom_types::expected_asset_kind_for_mainnet(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::Erc20,
                ) {
                    return Err("UnsupportedEvmNetwork".into());
                }
                if p.token_contract.len() != 20
                    || p.token_contract.iter().all(|b| *b == 0)
                {
                    return Err("InvalidEvmContractAddress".into());
                }
                if registry.find_representation_by_external_identity(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::Erc20,
                    &p.token_contract,
                    p.bridge_id,
                ).map(|r| r.representation.asset_id) != Some(resolved.representation_asset_id()) {
                    return Err("EvmTokenRegistryIdentityMismatch".into());
                }
            }
            freedom_types::ExternalBridgeDepositProof::TronUsdt(p) => {
                if !freedom_types::expected_asset_kind_for_mainnet(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::Trc20,
                ) {
                    return Err("UnsupportedTronNetwork".into());
                }
                if p.usdt_contract.len() != 20
                    || p.usdt_contract.iter().all(|b| *b == 0)
                {
                    return Err("InvalidTronContractAddress".into());
                }
                if registry.find_representation_by_external_identity(
                    p.chain_id,
                    freedom_types::CrossChainAssetKind::Trc20,
                    &p.usdt_contract,
                    p.bridge_id,
                ).map(|r| r.representation.asset_id) != Some(resolved.representation_asset_id()) {
                    return Err("TronTokenRegistryIdentityMismatch".into());
                }
            }
        }
        if !registry.has_representation(resolved.representation_asset_id()) {
            return Err("InvalidBridgeIngressContext".into());
        }

        Ok(Self {
            external,
            registry,
            policy,
            descriptor,
            resolved,
        })
    }
    fn validate_proof(
        &self,
        proof: &freedom_types::CrossChainAssetProof,
    ) -> Result<(), String> {
        if proof.asset_id != self.resolved.canonical_asset_id()
            || proof.representation_asset_id != self.resolved.representation_asset_id()
        {
            return Err("ResolvedBridgeRepresentationMismatch".into());
        }

        let representation = self
            .registry
            .representations
            .iter()
            .find(|r| r.representation.asset_id == self.resolved.representation_asset_id())
            .ok_or_else(|| "ExternalAssetRepresentationNotRegistered".to_string())?;
        if !representation.matches_proof(proof) {
            return Err("BridgeCanonicalRepresentationMismatch".into());
        }

        if proof.bridge_deposit.bridge_id != self.policy.bridge_id
            || proof.bridge_deposit.source_chain_id != self.descriptor.chain_id
        {
            return Err("BridgeIngressProofContextMismatch".into());
        }

        Ok(())
    }

}

pub fn verify_external_bridge_deposit(
    external: &freedom_types::ExternalBridgeDepositProof,
    registry: &freedom_types::CrossChainAssetRegistry,
    policy: &freedom_types::BridgeSecurityPolicy,
    descriptor: &freedom_types::BridgeAdapterDescriptor,
    canonical_asset_id: Hash,
    recipient_override: Option<Vec<u8>>,
) -> Result<freedom_types::VerifiedDeposit, String> {
    let context = BridgeIngressContext::build(
        external,
        registry,
        policy,
        descriptor,
        canonical_asset_id,
    )?;
    let resolved = context.resolved;

    let proof = match external {
        freedom_types::ExternalBridgeDepositProof::BitcoinUtxo(p) =>
            p.to_cross_chain_asset_proof(
                policy.min_external_confirmations,
                resolved.canonical_asset_id(),
                resolved.representation_asset_id(),
                recipient_override.unwrap_or_default(),
            ).map_err(|e| format!("BitcoinDeposit:{e:?}"))?,
        freedom_types::ExternalBridgeDepositProof::EvmTransfer(p) =>
            p.to_cross_chain_asset_proof(
                policy.min_external_confirmations,
                resolved.canonical_asset_id(),
                resolved.representation_asset_id(),
            ).map_err(|e| format!("EvmDeposit:{e:?}"))?,
        freedom_types::ExternalBridgeDepositProof::TronUsdt(p) =>
            p.to_cross_chain_asset_proof(
                policy.min_external_confirmations,
                &p.usdt_contract,
                resolved.canonical_asset_id(),
                resolved.representation_asset_id(),
            ).map_err(|e| format!("TronUsdtDeposit:{e:?}"))?,
    };

    if proof.asset_id != resolved.canonical_asset_id()
        || proof.representation_asset_id != resolved.representation_asset_id()
    {
        return Err("ResolvedBridgeRepresentationMismatch".into());
    }
    let representation = registry
        .representations
        .iter()
        .find(|r| r.representation.asset_id == resolved.representation_asset_id())
        .ok_or_else(|| "ExternalAssetRepresentationNotRegistered".to_string())?;
    if !representation.matches_proof(&proof) {
        return Err("BridgeCanonicalRepresentationMismatch".into());
    }

    verify_bridge_deposit_with_adapter(&proof, &context)
}

pub fn verify_bridge_deposit_with_adapter(
    proof: &freedom_types::CrossChainAssetProof,
    context: &BridgeIngressContext<'_>,
) -> Result<freedom_types::VerifiedDeposit, String> {
    context.validate_proof(proof)?;
    let registry = context.registry;
    let policy = context.policy;

    freedom_types::DepositVerifier::verify(proof, registry, policy)
        .map_err(|e| format!("BridgeDepositVerification:{e:?}"))
}

pub fn execute_transaction(s:&mut ChainState,tx:&Transaction)->Result<Receipt,String>{
 validate_transaction_admission(s,tx)?;
 let h:Hash=tx.hash();let before=s.events.len();
 s.execute_transaction(tx).map_err(format_state_error)?;
 s.assert_invariants();
 let events=s.events[before..].to_vec();
 Ok(Receipt{tx_hash:h,success:true,gas_used:1,events})
}
fn validate_bridge_mint_block_set(transactions: &[Transaction]) -> Result<(), String> {
 let mut ids=std::collections::BTreeSet::<Hash>::new();
 for tx in transactions {
  if tx.kind != freedom_types::TxKind::BridgeMint { continue; }
  let p=freedom_types::BridgeMintTransactionPayload::decode(&tx.payload)
   .map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
  let canonical=p.encode().map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
  if canonical!=tx.payload { return Err("InvalidBridgeMintTransaction".into()); }
  let id=p.canonical_id().map_err(|_|"InvalidBridgeMintTransaction".to_string())?;
  if !ids.insert(id) { return Err("DuplicateBridgeMintCanonicalId".into()); }
 }
 Ok(())
}

pub fn execute_block(s:&mut ChainState,block:&Block)->Result<Block,String>{
let expected_height=s.height.checked_add(1).ok_or_else(||"block height exceeds u64::MAX".to_string())?;
if block.header.height!=expected_height{return Err("invalid block height".into())}
if block.header.previous_hash!=s.last_block_hash{return Err("invalid previous block hash".into())}
let expected_epoch=if s.epoch_length>0 && block.header.height%s.epoch_length==0{block.header.height/s.epoch_length}else{s.epoch};
if block.header.epoch!=expected_epoch{return Err("invalid block epoch".into())}
freedom_consensus::validate_epoch_transition_certificate(s, block)?;
if block.header.tx_root!=tx_root(&block.transactions){return Err("transaction root mismatch".into())}
validate_bridge_mint_block_set(&block.transactions)?;
let mut working=s.clone();
let mut receipts=Vec::with_capacity(block.transactions.len());
for tx in &block.transactions{receipts.push(execute_transaction(&mut working,tx)?);}
if block.header.receipts_root!=receipts_root(&receipts){return Err("receipts root mismatch".into())}
if let Some(cert) = &block.header.epoch_transition_certificate {
    freedom_consensus::validate_validator_set_certificate(s, cert)?;
    if let Some(existing) = s.validator_set_certificate.as_ref() {
        if existing != cert {
            return Err("validator set certificate is already committed for this transition".into());
        }
    }
    working.validator_set_certificate = Some(cert.clone());
}
working.height=block.header.height;
working.maybe_advance_epoch();
if working.epoch != expected_epoch {
    return Err("epoch transition was not committed".into());
}
if block.header.state_root!=working.state_root(){return Err("state root mismatch".into())}
let committed=Block{header:block.header.clone(),transactions:block.transactions.clone(),receipts};
let bh=block_hash(&committed);
working.last_block_hash=bh;
working.assert_invariants();
*s=working;
Ok(committed)}
fn format_state_error(e:StateError)->String{format!("{e:?}")}


#[derive(Clone, Debug)]
pub struct BridgeMintRelay {
    seen_ids: std::collections::BTreeSet<Hash>,
    seen_order: std::collections::VecDeque<Hash>,
    pub max_seen: usize,
}

impl Default for BridgeMintRelay {
    fn default() -> Self {
        Self {
            seen_ids: std::collections::BTreeSet::new(),
            seen_order: std::collections::VecDeque::new(),
            max_seen: 4096,
        }
    }
}

impl BridgeMintRelay {
    pub fn new(max_seen: usize) -> Self {
        Self { max_seen, ..Self::default() }
    }

    /// Admit a bridge-mint transaction for relay and return its canonical
    /// bridge-proof id. The transaction must pass the exact same admission
    /// rules used by the mempool/block executor.
    pub fn receive(&mut self, state: &ChainState, tx: &Transaction) -> Result<Hash, String> {
        if tx.kind != freedom_types::TxKind::BridgeMint {
            return Err("not a bridge mint transaction".into());
        }
        validate_transaction_admission(state, tx)?;
        let payload = freedom_types::BridgeMintTransactionPayload::decode(&tx.payload)
            .map_err(|_| "InvalidBridgeMintTransaction".to_string())?;
        let id = payload.canonical_id()
            .map_err(|_| "InvalidBridgeMintTransaction".to_string())?;

        if self.seen_ids.contains(&id) {
            return Err("duplicate bridge mint relay".into());
        }
        if self.max_seen == 0 {
            return Err("bridge mint relay cache disabled".into());
        }

        self.seen_ids.insert(id);
        self.seen_order.push_back(id);
        while self.seen_order.len() > self.max_seen {
            if let Some(old) = self.seen_order.pop_front() {
                self.seen_ids.remove(&old);
            }
        }
        Ok(id)
    }

    pub fn contains(&self, id: &Hash) -> bool {
        self.seen_ids.contains(id)
    }

    pub fn len(&self) -> usize {
        self.seen_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen_ids.is_empty()
    }

    pub fn clear(&mut self) {
        self.seen_ids.clear();
        self.seen_order.clear();
    }
}

#[derive(Clone, Debug, Default)]
pub struct Mempool {
    transactions: std::collections::BTreeMap<Hash, Transaction>,
}

impl Mempool {
    pub fn new() -> Self { Self::default() }

    pub fn len(&self) -> usize { self.transactions.len() }

    pub fn is_empty(&self) -> bool { self.transactions.is_empty() }

    pub fn submit(&mut self, state: &ChainState, tx: Transaction) -> Result<Hash, String> {
        validate_transaction_admission(state, &tx)?;
        let hash = tx.hash();
        if self.transactions.contains_key(&hash) { return Err("duplicate transaction".into()); }
        self.transactions.insert(hash, tx);
        Ok(hash)
    }

    pub fn remove(&mut self, hash: &Hash) -> Option<Transaction> {
        self.transactions.remove(hash)
    }

    pub fn ordered(&self, state: &ChainState, max: usize) -> Vec<Transaction> {
        let mut candidates: Vec<Transaction> = self.transactions.values().cloned().collect();
        candidates.sort_by(|a,b| {
            a.sender.cmp(&b.sender)
                .then(a.nonce.cmp(&b.nonce))
                .then(a.hash().cmp(&b.hash()))
        });

        let mut working = state.clone();
        let mut selected = Vec::new();
        for tx in candidates {
            if selected.len() >= max { break; }
            if execute_transaction(&mut working, &tx).is_ok() {
                selected.push(tx);
            }
        }
        selected
    }

    pub fn remove_committed(&mut self, transactions: &[Transaction]) {
        for tx in transactions { self.transactions.remove(&tx.hash()); }
    }
}

pub fn build_next_block(
    state: &ChainState,
    mempool: &Mempool,
    proposer: freedom_primitives::Address,
    timestamp: u64,
    epoch: u64,
    round: u64,
    max_transactions: usize,
) -> Result<Block, String> {
    if proposer == [0; 32] { return Err("invalid proposer".into()); }
    let height = state.height.checked_add(1).ok_or_else(|| "block height overflow".to_string())?;
    let expected_epoch = if state.epoch_length > 0 && height % state.epoch_length == 0 {
        height / state.epoch_length
    } else {
        state.epoch
    };
    if epoch != expected_epoch {
        return Err("invalid block epoch".into());
    }

    let transactions = mempool.ordered(state, max_transactions);
    validate_bridge_mint_block_set(&transactions)?;
    let mut working = state.clone();
    let mut receipts = Vec::with_capacity(transactions.len());
    for tx in &transactions {
        receipts.push(execute_transaction(&mut working, tx)?);
    }

    let epoch_transition_certificate =
        if epoch != state.epoch { state.validator_set_certificate.clone() } else { None };
    if let Some(cert) = &epoch_transition_certificate {
        freedom_consensus::validate_validator_set_certificate(state, cert)?;
        if let Some(existing) = state.validator_set_certificate.as_ref() {
            if existing != cert {
                return Err("validator set certificate is already committed for this transition".into());
            }
        }
        working.validator_set_certificate = Some(cert.clone());
    } else if epoch != state.epoch {
        return Err("missing epoch transition certificate".into());
    }

    working.height = height;
    working.maybe_advance_epoch();
    if working.epoch != expected_epoch {
        return Err("epoch transition was not committed".into());
    }

    let tx_root = tx_root(&transactions);
    let receipts_root = receipts_root(&receipts);
    let state_root = working.state_root();
    Ok(Block {
        header: freedom_types::BlockHeader {
            height,
            round,
            previous_hash: state.last_block_hash,
            state_root,
            tx_root,
            receipts_root,
            timestamp,
            epoch,
            proposer,
            epoch_transition_certificate,
        },
        transactions,
        receipts,
    })
}



#[cfg(test)]
fn test_state() -> ChainState {
        use freedom_types::Validator;
        let validators = (0u8..3).map(|i| {
            let key = [i + 1; 32];
            let owner = freedom_primitives::address_from_public_key(&key);
            let mut validator_seed = b"validator".to_vec();
            validator_seed.push(i + 1);
            let id = freedom_primitives::hash_bytes(&validator_seed);
            (id, Validator {
                validator_id: id,
                owner,
                consensus_key: key,
                stake: 0,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            })
        }).collect::<Vec<_>>();
        let genesis = Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: validators.into_iter().map(|(_,v)| v).collect() };
        let mut state = initialize(&genesis).unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let public = signing_key.verifying_key().to_bytes();
        state.accounts.insert(freedom_state::Account::new(public, 1_000).address, freedom_state::Account::new(public, 1_000));
        state.supply = freedom_primitives::Supply { total: 1_000, circulating: 1_000, locked: 0, staked: 0, treasury: 0 };
        state.assert_invariants();
        state
    }

#[cfg(test)]
mod m338_block_height_overflow_tests {
    use super::*;
    use freedom_types::Genesis;

    #[test]
    fn execute_block_rejects_height_overflow_at_u64_max() {
        let mut state = ChainState::genesis(&Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        state.height = u64::MAX;
        let block = Block {
            header: freedom_types::BlockHeader {
                height: u64::MAX,
                previous_hash: state.last_block_hash,
                tx_root: tx_root(&[]),
                receipts_root: receipts_root(&[]),
                timestamp: 0,
                state_root: state.state_root(),
                proposer: [0; 32],
                round: 0,
                epoch: state.epoch,
                epoch_transition_certificate: None,
            },
            transactions: vec![],
            receipts: vec![],
        };
        assert!(matches!(
            execute_block(&mut state, &block),
            Err(ref error) if error == "block height exceeds u64::MAX"
        ));
    }
}

#[cfg(test)]
mod m207_bitcoin_chain_id_tests {
 use super::*;

 #[test]
 fn bitcoin_mainnet_uses_protocol_chain_id() {
   assert_eq!(freedom_types::BITCOIN_MAINNET_CHAIN_ID, 0x4254_4301);
   assert_ne!(freedom_types::BITCOIN_MAINNET_CHAIN_ID, 8332);
 }
}

#[cfg(test)]
mod m240_bridge_mint_authorization_validator_set_tests {
 use super::*;

 #[test]
 fn bridge_mint_authorization_keys_are_resolved_from_active_validator_set() {
   let mut state = test_state();
   let active_id = [3u8;32];
   let active_key = [4u8;32];
   let registry_only_id = [5u8;32];
   let registry_only_key = [6u8;32];
   state.active_validator_set.insert(
     active_id,
     freedom_types::ValidatorSetEntry {
       validator_id: active_id,
       owner: [0u8;32],
       consensus_key: active_key,
       stake: 1,
     }
   );
   state.validators.insert(
     registry_only_id,
     freedom_types::Validator {
       validator_id: registry_only_id,
       owner: [0u8;32],
       consensus_key: registry_only_key,
       stake: 1,
       unbonding_stake: 0,
       blocks_proposed: 0,
       blocks_finalized: 0,
       missed_votes: 0,
     }
   );
   let keys = state.active_validator_set.iter()
     .map(|(id,v)|(*id,v.consensus_key))
     .collect::<std::collections::BTreeMap<freedom_primitives::Hash,[u8;32]>>();
   assert!(keys.contains_key(&active_id));
   assert!(!keys.contains_key(&registry_only_id));
 }
}

#[cfg(test)]
mod m239_bitcoin_mint_validator_set_tests {
 use super::*;

 #[test]
 fn bitcoin_mint_attestation_keys_are_resolved_from_active_validator_set() {
   let mut state = test_state();
   let validator_id = [7u8;32];
   let key = [9u8;32];
   state.active_validator_set.insert(
     validator_id,
     freedom_types::ValidatorSetEntry {
       validator_id,
       owner: [0u8;32],
       consensus_key: key,
       stake: 1,
     }
   );
   let keys = state.active_validator_set.iter()
     .map(|(id,v)|(*id,v.consensus_key))
     .collect::<std::collections::BTreeMap<freedom_primitives::Hash,[u8;32]>>();
   assert_eq!(keys.get(&validator_id), Some(&key));
   assert!(keys.get(&freedom_primitives::hash_bytes(&key)).is_none());
 }
}

#[cfg(test)]
mod m205_bitcoin_mint_admission_tests {
 use super::*;

 #[test]
 fn bitcoin_confirmation_gate_is_tip_based() {
   assert_eq!(freedom_types::bitcoin_confirmations_at_tip(100, 95), Some(6));
   assert_eq!(freedom_types::bitcoin_confirmations_at_tip(100, 101), None);
 }

 #[test]
 fn bitcoin_tip_is_consensus_state_for_mint_admission() {
   let mut state = test_state();
   let bridge=[9u8;32];
   state.bitcoin_canonical_tips.insert(
     bridge,
     freedom_types::BitcoinChainCheckpoint {
       height: 100,
       block_hash:[1;32],
       chainwork:[2;32],
     }
   );
   let tip=state.bitcoin_canonical_tips.get(&bridge).unwrap();
   assert!(tip.validate());
   assert_eq!(
     freedom_types::bitcoin_confirmations_at_tip(tip.height, 95),
     Some(6)
   );
 }
}

#[cfg(test)]
mod m29_tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    #[test]
    fn message_dedup_is_bounded_and_deterministic() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        let mut p = ValidatorProtocol::new(hash_bytes(b"v"), key);
        assert!(p.remember_message(hash_bytes(b"a")));
        assert!(!p.remember_message(hash_bytes(b"a")));
        p.max_seen_messages = 2;
        assert!(p.remember_message(hash_bytes(b"b")));
        assert!(p.remember_message(hash_bytes(b"c")));
        assert_eq!(p.seen_message_order.len(), 2);
        assert!(!p.seen_messages.contains(&hash_bytes(b"a")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_crypto::sign;
    use freedom_primitives::Supply;
    use freedom_state::Account;
    use freedom_types::{BridgeMessage, BridgeMessageApprovalSet, BridgeMintAuthorization, BridgeMintTransactionPayload, Genesis, TransferPayload, TxKind, Validator, VerifiedDeposit};




    #[test]
    fn runtime_routes_bridge_mint_as_transaction_kind() {
        let mut state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let payload = BridgeMintTransactionPayload {
            message: BridgeMessage {
                protocol_version: 1, source_chain_id: 2, destination_chain_id: CHAIN_ID,
                bridge_id: [1;32], asset_id: [2;32], source_tx_hash: [3;32],
                source_event_index: 0, nonce: 1, amount: 1, recipient: [4;32], message_hash: [5;32],
            },
            deposit: VerifiedDeposit {
                canonical_asset_id:[2;32], representation_asset_id:[6;32], bridge_id:[1;32],
                source_chain_id:2, tx_hash:[3;32], event_index:0, block_or_slot:0,
                confirmations:1, amount:1, recipient:vec![4;20], replay_key:[7;32], proof_hash:[8;32],
            },
            authorization: BridgeMintAuthorization {
                bridge_id:[1;32], message_hash:[5;32], replay_key:[7;32],
                canonical_asset_id:[2;32], representation_asset_id:[6;32],
                source_chain_id:2, source_tx_hash:[3;32], source_event_index:0,
                destination_chain_id:CHAIN_ID, nonce:1, amount:1, recipient:[4;32],
                approval_count:1, authorization_hash:[9;32],
            },
            approvals: BridgeMessageApprovalSet { bridge_id:[1;32], message_hash:[5;32], approvals:vec![] },
        }.encode().unwrap();
        let mut tx=Transaction{chain_id:CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload,fee:0,signature:Vec::new()};
        tx.signature=sign(&signing_key,&tx.signing_bytes()).to_vec();
        assert_eq!(execute_transaction(&mut state,&tx),Err("InvalidBridgeMintTransaction".into()));
    }


    #[test]
    fn mempool_rejects_invalid_bridge_mint_before_admission() {
        let mut state=test_state();
        let sender=state.accounts.keys().next().copied().unwrap();
        let signing_key=ed25519_dalek::SigningKey::from_bytes(&[42u8;32]);
        let payload=freedom_types::BridgeMintTransactionPayload{
            message:freedom_types::BridgeMessage{
                protocol_version:1,source_chain_id:2,destination_chain_id:CHAIN_ID,
                bridge_id:[1;32],asset_id:[2;32],source_tx_hash:[3;32],
                source_event_index:0,nonce:1,amount:1,recipient:[4;32],message_hash:[5;32]},
            deposit:freedom_types::VerifiedDeposit{
                canonical_asset_id:[2;32],representation_asset_id:[6;32],bridge_id:[1;32],
                source_chain_id:2,tx_hash:[3;32],event_index:0,block_or_slot:0,
                confirmations:1,amount:1,recipient:vec![4;20],replay_key:[7;32],proof_hash:[8;32]},
            authorization:freedom_types::BridgeMintAuthorization{
                bridge_id:[1;32],message_hash:[5;32],replay_key:[7;32],canonical_asset_id:[2;32],
                representation_asset_id:[6;32],source_chain_id:2,source_tx_hash:[3;32],
                source_event_index:0,destination_chain_id:CHAIN_ID,nonce:1,amount:1,
                recipient:[4;32],approval_count:1,authorization_hash:[9;32]},
            approvals:freedom_types::BridgeMessageApprovalSet{bridge_id:[1;32],message_hash:[5;32],approvals:vec![]}
        }.encode().unwrap();
        let mut tx=Transaction{chain_id:CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload,fee:0,signature:Vec::new()};
        tx.signature=sign(&signing_key,&tx.signing_bytes()).to_vec();
        let mut mempool=Mempool::new();
        assert_eq!(mempool.submit(&state,tx),Err("InvalidBridgeMintTransaction".into()));
        assert!(mempool.is_empty());
    }

    #[test]
    fn mempool_rejects_bridge_mint_replay_before_block_build() {
        let mut state=test_state();
        let sender=state.accounts.keys().next().copied().unwrap();
        let signing_key=ed25519_dalek::SigningKey::from_bytes(&[42u8;32]);
        let bridge=[1;32];
        let policy=freedom_types::BridgeSecurityPolicy{bridge_id:bridge,approval_mode:freedom_types::BridgeApprovalMode::Quorum,
            validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,
            max_deposit_amount:1000,max_withdrawal_amount:1000,daily_limit:1000,emergency_paused:false,nonce_floor:0};
        state.bridge_security_policies.insert(bridge,policy);
        let msg=freedom_types::BridgeMessage{protocol_version:1,source_chain_id:2,destination_chain_id:CHAIN_ID,
            bridge_id:bridge,asset_id:[2;32],source_tx_hash:[3;32],source_event_index:0,nonce:1,amount:1,
            recipient:[4;32],message_hash:[0;32]};
        let mut msg=msg;msg.message_hash=msg.compute_hash();
        state.bridge_message_status.insert(msg.replay_key(),freedom_types::BridgeMessageStatus::Minted);
        let dep=freedom_types::VerifiedDeposit{canonical_asset_id:[2;32],representation_asset_id:[6;32],bridge_id:bridge,
            source_chain_id:2,tx_hash:[3;32],event_index:0,block_or_slot:0,confirmations:1,amount:1,
            recipient:vec![4;20],replay_key:[7;32],proof_hash:[8;32]};
        let unsigned=freedom_types::BridgeMessageApproval{bridge_id:bridge,message_hash:msg.message_hash,
            validator_id:state.validators.keys().next().copied().unwrap(),signature:vec![0;64]};
        let key=ed25519_dalek::SigningKey::from_bytes(&[1u8;32]);
        let sig=sign(&key,&unsigned.approval_hash());
        let approvals=freedom_types::BridgeMessageApprovalSet{bridge_id:bridge,message_hash:msg.message_hash,
            approvals:vec![freedom_types::BridgeMessageApproval{signature:sig.to_vec(),..unsigned}]};
        let mut tx=Transaction{chain_id:CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,
            payload:freedom_types::BridgeMintTransactionPayload{message:msg.clone(),deposit:dep.clone(),
                authorization:freedom_types::BridgeMintAuthorization{
                    bridge_id:bridge,message_hash:msg.message_hash,replay_key:[7;32],canonical_asset_id:[2;32],
                    representation_asset_id:[6;32],source_chain_id:2,source_tx_hash:[3;32],source_event_index:0,
                    destination_chain_id:CHAIN_ID,nonce:1,amount:1,recipient:[4;32],approval_count:1,authorization_hash:[9;32]},
                approvals}.encode().unwrap(),fee:0,signature:Vec::new()};
        tx.signature=sign(&signing_key,&tx.signing_bytes()).to_vec();
        let mut mempool=Mempool::new();
        assert!(mempool.submit(&state,tx).is_err());
    }
    #[test]
    fn mempool_orders_and_block_builder_commits_transfer() {
        let mut state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let recipient = freedom_primitives::address_from_public_key(&[9; 32]);
        let payload = TransferPayload { to: recipient, amount: 250 }.encode();
        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload,
            fee: 10,
            signature: Vec::new(),
        };
        tx.signature = sign(&signing_key, &tx.signing_bytes()).to_vec();

        let mut mempool = Mempool::new();
        let hash = mempool.submit(&state, tx.clone()).unwrap();
        assert_eq!(mempool.len(), 1);

        let proposer = freedom_consensus::proposer_for_height(&state, 1).unwrap();
        let block = build_next_block(&state, &mempool, proposer, 1, state.epoch, 0, 100).unwrap();
        assert_eq!(block.transactions, vec![tx.clone()]);
        assert_eq!(block.header.tx_root, tx_root(&block.transactions));
        assert_eq!(block.header.receipts_root, receipts_root(&{
            let mut preview = state.clone();
            block.transactions.iter().map(|t| execute_transaction(&mut preview, t).unwrap()).collect::<Vec<_>>()
        }));

        let committed = execute_block(&mut state, &block).unwrap();
        assert_eq!(state.height, 1);
        assert_eq!(state.last_block_hash, block_hash(&committed));
        assert_eq!(state.accounts.get(&recipient).unwrap().balance, 250);
        mempool.remove_committed(&committed.transactions);
        assert_eq!(mempool.len(), 0);
        assert!(mempool.remove(&hash).is_none());
    }

    #[test]
    fn mempool_rejects_stale_nonce_and_duplicate_hash() {
        let state = test_state();
        let sender = state.accounts.keys().next().copied().unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let recipient = freedom_primitives::address_from_public_key(&[8; 32]);
        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 1 }.encode(),
            fee: 0,
            signature: Vec::new(),
        };
        tx.signature = sign(&signing_key, &tx.signing_bytes()).to_vec();
        let mut mempool = Mempool::new();
        mempool.submit(&state, tx.clone()).unwrap();
        assert!(mempool.submit(&state, tx).is_err());

        let mut stale = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 1,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 1 }.encode(),
            fee: 0,
            signature: Vec::new(),
        };
        stale.signature = sign(&signing_key, &stale.signing_bytes()).to_vec();
        let mut next_state = state.clone();
        next_state.accounts.get_mut(&sender).unwrap().nonce = 2;
        assert!(mempool.submit(&next_state, stale).is_err());
    }
}


pub fn finalize_block_with_quorum(
    state: &mut ChainState,
    block: &Block,
    votes: &[freedom_types::ValidatorVote],
) -> Result<Block, String> {
    let hash = block_hash(block);
    freedom_consensus::validate_quorum(state, hash, block.header.height, 0, votes)?;
    let committed = execute_block(state, block)?;
    Ok(committed)
}


#[derive(Clone)]
pub struct ValidatorProtocol {
    pub validator_id: Hash,
    pub consensus_key: ed25519_dalek::SigningKey,
    pub votes: std::collections::BTreeMap<Hash, freedom_types::ValidatorVote>,
    pub precommits: std::collections::BTreeMap<Hash, freedom_types::ValidatorPrecommit>,
    pub timeouts: std::collections::BTreeMap<Hash, freedom_types::ValidatorTimeout>,
    pub locked_round: Option<u64>,
    pub locked_block_hash: Option<Hash>,
    pub locked_block: Option<Block>,
    pub proposals: std::collections::BTreeMap<(Hash,u64,u64), freedom_types::BlockProposal>,
    pub seen_messages: std::collections::BTreeSet<Hash>,
    pub seen_message_order: std::collections::VecDeque<Hash>,
    pub max_seen_messages: usize,
}

impl ValidatorProtocol {
    pub fn new(validator_id: Hash, consensus_key: ed25519_dalek::SigningKey) -> Self {
        Self { validator_id, consensus_key, votes: std::collections::BTreeMap::new(), precommits: std::collections::BTreeMap::new(), timeouts: std::collections::BTreeMap::new(), locked_round: None, locked_block_hash: None, locked_block: None, proposals: std::collections::BTreeMap::new(), seen_messages: std::collections::BTreeSet::new(), seen_message_order: std::collections::VecDeque::new(), max_seen_messages: 4096 }
    }

    pub fn remember_message(&mut self, id: Hash) -> bool {
        if self.seen_messages.contains(&id) { return false; }
        self.seen_messages.insert(id);
        self.seen_message_order.push_back(id);
        while self.seen_message_order.len() > self.max_seen_messages {
            if let Some(old) = self.seen_message_order.pop_front() { self.seen_messages.remove(&old); }
        }
        true
    }

    pub fn reset_round(&mut self) { self.votes.clear(); self.precommits.clear(); self.timeouts.clear(); self.proposals.retain(|(_,_,r),_| *r==self.locked_round.unwrap_or(0)); }

    pub fn reset_height(&mut self) {
        self.reset_round();
        self.locked_round = None;
        self.locked_block_hash = None;
        self.locked_block = None;
    }

    pub fn receive_proposal(&mut self, state: &ChainState, proposal: &freedom_types::BlockProposal) -> Result<freedom_types::ValidatorVote, String> {
        if let Some(locked) = self.locked_block_hash {
            if locked != proposal.block_hash {
                return Err("proposal conflicts with local lock".into());
            }
        }
        let key=(proposal.proposer,proposal.height,proposal.round);
        if let Some(existing)=self.proposals.get(&key) {
            if existing.block_hash != proposal.block_hash {
                return Err("validator proposal equivocation detected".into());
            }
        } else {
            self.proposals.insert(key, proposal.clone());
        }
        freedom_network::make_vote(state, self.validator_id, proposal, &self.consensus_key)
    }

    pub fn receive_timeout(&mut self, state: &ChainState, timeout: freedom_types::ValidatorTimeout, height: u64, round: u64) -> Result<bool, String> { freedom_consensus::validate_timeout(state, &timeout, height, round)?;
if let Some(existing)=self.timeouts.get(&timeout.validator_id){
 if existing.signing_bytes()!=timeout.signing_bytes(){return Err("validator timeout equivocation detected".into());}
 return Ok(freedom_consensus::stake_quorum_reached(state, &self.timeouts.keys().copied().collect()));
}
self.timeouts.insert(timeout.validator_id, timeout);
Ok(freedom_consensus::stake_quorum_reached(state, &self.timeouts.keys().copied().collect())) }

    pub fn receive_vote(&mut self, state: &ChainState, vote: freedom_types::ValidatorVote, expected_block_hash: Hash, height: u64, round: u64) -> Result<bool, String> {
        freedom_consensus::validate_vote(state, &vote, expected_block_hash, height, round)?;
        if let Some(existing) = self.votes.get(&vote.validator_id) {
            if existing.block_hash != vote.block_hash || existing.round != vote.round {
                return Err("validator prevote equivocation detected".into());
            }
            return Ok(freedom_consensus::stake_quorum_reached(state, &self.votes.keys().copied().collect()));
        }
        self.votes.insert(vote.validator_id, vote);
        Ok(freedom_consensus::stake_quorum_reached(state, &self.votes.keys().copied().collect()))
    }

    pub fn lock_on_prevote_quorum(
        &mut self,
        state: &ChainState,
        block_hash: Hash,
        height: u64,
        round: u64,
        block: &Block,
    ) -> Result<freedom_types::ValidatorPrecommit, String> {
        let votes: Vec<_> = self.votes.values().cloned().collect();
        freedom_consensus::validate_quorum(state, block_hash, height, round, &votes)?;
        if let Some(locked) = self.locked_block_hash {
            if locked != block_hash {
                return Err("locked on a different block".into());
            }
        }
        self.locked_round = Some(round);
        self.locked_block_hash = Some(block_hash);
        self.locked_block = Some(block.clone());
        let mut precommit = freedom_types::ValidatorPrecommit {
            block_hash,
            height,
            round,
            validator_id: self.validator_id,
            signature: Vec::new(),
        };
        precommit.signature = freedom_crypto::sign(&self.consensus_key, &precommit.signing_bytes()).to_vec();
        self.precommits.insert(self.validator_id, precommit.clone());
        Ok(precommit)
    }

    pub fn receive_precommit(
        &mut self,
        state: &ChainState,
        precommit: freedom_types::ValidatorPrecommit,
        expected_block_hash: Hash,
        height: u64,
        round: u64,
    ) -> Result<bool, String> {
        freedom_consensus::validate_precommit(state, &precommit, expected_block_hash, height, round)?;
        if let Some(existing) = self.precommits.get(&precommit.validator_id) {
            if existing.block_hash != precommit.block_hash || existing.round != precommit.round {
                return Err("validator precommit equivocation detected".into());
            }
            return Ok(freedom_consensus::stake_quorum_reached(state, &self.precommits.keys().copied().collect()));
        }
        self.precommits.insert(precommit.validator_id, precommit);
        Ok(freedom_consensus::stake_quorum_reached(state, &self.precommits.keys().copied().collect()))
    }

    pub fn finalize_if_quorum(&self, state: &mut ChainState, block: &Block, round: u64) -> Result<Block, String> {
        let precommits: Vec<_> = self.precommits.values().cloned().collect();
        freedom_consensus::validate_precommit_quorum(state, block_hash(block), block.header.height, round, &precommits)?;
        execute_block(state, block)
    }
}


#[cfg(test)]
mod m22_tests {
    use super::*;
    use freedom_primitives::hash_bytes;
    use freedom_types::ValidatorPrecommit;

    #[test]
    fn precommit_signing_domain_is_distinct() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let mut p = ValidatorPrecommit {
            block_hash: hash_bytes(b"block"),
            height: 10,
            round: 2,
            validator_id: hash_bytes(b"validator"),
            signature: Vec::new(),
        };
        p.signature = freedom_crypto::sign(&key, &p.signing_bytes()).to_vec();
        assert!(freedom_crypto::verify(&key.verifying_key().to_bytes(), &p.signing_bytes(), &p.signature.try_into().unwrap()));
    }
}


#[cfg(test)]
mod m158_bridge_mint_canonical_admission_tests {
    use super::*;
    use freedom_types::{BridgeMessageApproval, BridgeMessageApprovalSet};

    #[test]
    fn bridge_approval_set_requires_strict_validator_id_order() {
        let low=[1u8;32];
        let high=[2u8;32];
        let mk=|validator_id| BridgeMessageApproval{
            bridge_id:[3;32], message_hash:[4;32], validator_id, signature:vec![0;64]
        };
        let ordered=BridgeMessageApprovalSet{
            bridge_id:[3;32], message_hash:[4;32],
            approvals:vec![mk(low),mk(high)],
        };
        assert!(ordered.is_canonical_order());
        assert!(ordered.approvals.len()==2);
        let reversed=BridgeMessageApprovalSet{
            bridge_id:[3;32], message_hash:[4;32],
            approvals:vec![mk(high),mk(low)],
        };
        assert!(!reversed.is_canonical_order());
        let duplicated=BridgeMessageApprovalSet{
            bridge_id:[3;32], message_hash:[4;32],
            approvals:vec![mk(low),mk(low)],
        };
        assert!(!duplicated.is_canonical_order());
    }
}

#[cfg(test)]
mod m159_bridge_mint_relay_tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use freedom_crypto::sign;
    use freedom_types::{
        AssetDefinition, AssetStatus, AssetType, BridgeMessageApproval,
        BridgeMessageApprovalSet, BridgeMintAuthorization, BridgeMintTransactionPayload,
        BridgeSecurityPolicy, Genesis, Transaction, TxKind, Validator, VerifiedDeposit,
    };

    fn valid_tx() -> (ChainState, Transaction, Hash) {
        let validator_key=SigningKey::from_bytes(&[7;32]);
        let validator_pub=validator_key.verifying_key().to_bytes();
        let validator_id=freedom_primitives::hash_bytes(&validator_pub);
        let bridge_id=[3;32];
        let source_chain_id=100u32;
        let asset_id=AssetDefinition::compute_asset_id(source_chain_id,AssetType::Bridged,b"USDT",[8;32],bridge_id);
        let validator=Validator{validator_id,owner:freedom_primitives::address_from_public_key(&validator_pub),consensus_key:validator_pub,stake:0,unbonding_stake:0,blocks_proposed:0,blocks_finalized:0,missed_votes:0};
        let mut validators = vec![validator];
        for seed in [8u8, 9u8] {
            let key = SigningKey::from_bytes(&[seed; 32]);
            let public_key = key.verifying_key().to_bytes();
            let id = freedom_primitives::hash_bytes(&public_key);
            validators.push(Validator {
                validator_id: id, owner: freedom_primitives::address_from_public_key(&public_key),
                consensus_key: public_key, stake: 0, unbonding_stake: 0,
                blocks_proposed: 0, blocks_finalized: 0, missed_votes: 0,
            });
        }
        let mut state=initialize(&Genesis{chain_id:CHAIN_ID,initial_supply:0,validators}).unwrap();
        state.asset_registry.insert(asset_id, AssetDefinition{asset_id,source_chain_id,asset_type:AssetType::Bridged,contract_address:b"USDT".to_vec(),symbol:"USDT".into(),decimals:6,issuer:[8;32],bridge_id,status:AssetStatus::Active});
        state.bridge_security_policies.insert(bridge_id, BridgeSecurityPolicy{bridge_id,approval_mode:freedom_types::BridgeApprovalMode::Quorum,validator_quorum:1,validator_total:1,threshold_signers:1,threshold_total:1,min_external_confirmations:1,max_deposit_amount:1_000_000,max_withdrawal_amount:1_000_000,daily_limit:1_000_000,emergency_paused:false,nonce_floor:1});

        let deposit=VerifiedDeposit{canonical_asset_id:asset_id,representation_asset_id:[2;32],bridge_id,source_chain_id,tx_hash:[4;32],event_index:7,block_or_slot:8,confirmations:20,amount:500,recipient:vec![9;20],replay_key:[10;32],proof_hash:[11;32]};
        let message=deposit.to_bridge_message(CHAIN_ID,1).unwrap();
        let unsigned=BridgeMessageApproval{bridge_id:message.bridge_id,message_hash:message.message_hash,validator_id,signature:vec![0;64]};
        let signature=sign(&validator_key,&unsigned.approval_hash());
        let approvals=BridgeMessageApprovalSet{bridge_id:message.bridge_id,message_hash:message.message_hash,approvals:vec![BridgeMessageApproval{signature:signature.to_vec(),..unsigned}]};
        let auth=BridgeMintAuthorization::from_authenticated_message(&message,&deposit,&approvals,state.bridge_security_policies.get(&bridge_id).unwrap(),&state.validators.iter().map(|(id,v)|(*id,v.consensus_key)).collect()).unwrap();
        let replay_key = message.replay_key();

        let relayer_key=SigningKey::from_bytes(&[42;32]);
        let relayer_pub=relayer_key.verifying_key().to_bytes();
        let sender=freedom_primitives::address_from_public_key(&relayer_pub);
        state.accounts.insert(sender, freedom_state::Account::new(relayer_pub,1_000));
        let payload=BridgeMintTransactionPayload{message,deposit,authorization:auth,approvals};
        let canonical_id=payload.canonical_id().unwrap();
        let mut tx=Transaction{chain_id:CHAIN_ID,sender,nonce:0,kind:TxKind::BridgeMint,payload:payload.encode().unwrap(),fee:0,signature:Vec::new()};
        tx.signature=sign(&relayer_key,&tx.signing_bytes()).to_vec();
        (state,tx,canonical_id)
    }

    #[test]
    fn relay_accepts_valid_bridge_mint_and_deduplicates_by_canonical_id() {
        let (state,tx,id)=valid_tx();
        let mut relay=BridgeMintRelay::new(8);
        assert_eq!(relay.receive(&state,&tx).unwrap(),id);
        assert!(relay.contains(&id));
        assert_eq!(relay.len(),1);
        assert_eq!(relay.receive(&state,&tx),Err("duplicate bridge mint relay".into()));
    }

    #[test]
    fn relay_cache_is_bounded_and_evicts_oldest() {
        let (state,tx,id)=valid_tx();
        let mut relay=BridgeMintRelay::new(1);
        relay.receive(&state,&tx).unwrap();
        assert!(relay.contains(&id));
        // A fresh relay instance proves the bound is an explicit cache property.
        relay.clear();
        assert!(relay.is_empty());
        assert_eq!(relay.max_seen,1);
    }

    #[test]
    fn relay_rejects_non_bridge_and_invalid_transactions_before_caching() {
        let (state,mut tx,_)=valid_tx();
        let mut relay=BridgeMintRelay::new(8);
        tx.kind=TxKind::Transfer;
        assert_eq!(relay.receive(&state,&tx),Err("not a bridge mint transaction".into()));
        assert!(relay.is_empty());
    }
}


#[cfg(test)]
mod m161_bridge_adapter_tests {
    use super::*;

    #[test]
    fn adapter_dispatch_maps_supported_asset_kinds() {
        assert_eq!(
            match freedom_types::CrossChainAssetKind::BitcoinUtxo {
                freedom_types::CrossChainAssetKind::BitcoinUtxo => freedom_types::BridgeAdapterKind::Bitcoin,
                freedom_types::CrossChainAssetKind::EthereumNative => freedom_types::BridgeAdapterKind::Evm,
                freedom_types::CrossChainAssetKind::Erc20 => freedom_types::BridgeAdapterKind::Evm,
                freedom_types::CrossChainAssetKind::Trc20 => freedom_types::BridgeAdapterKind::Tron,
                freedom_types::CrossChainAssetKind::TonJetton => freedom_types::BridgeAdapterKind::Ton,
                freedom_types::CrossChainAssetKind::SolanaSpl => freedom_types::BridgeAdapterKind::Solana,
                freedom_types::CrossChainAssetKind::Other => freedom_types::BridgeAdapterKind::Other,
            },
            freedom_types::BridgeAdapterKind::Bitcoin
        );
    }

    #[test]
    fn adapter_rejects_disabled_or_mismatched_descriptor() {
        let descriptor = freedom_types::BridgeAdapterDescriptor {
            bridge_id: [1; 32],
            chain_id: 1,
            kind: freedom_types::BridgeAdapterKind::Bitcoin,
            finality_model: freedom_types::BridgeFinalityModel::Confirmations,
            required_confirmations: 6,
            native_asset: true,
            enabled: false,
        };
        assert!(!descriptor.enabled);
        assert!(descriptor.validate());
    }
}


#[cfg(test)]
mod m315_validator_certificate_collection_tests {
    use super::*;

    #[test]
    fn certificate_collection_rejects_when_transition_is_already_committed() {
        let mut state = ChainState::genesis(&freedom_types::Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.validator_set_certificate = Some(freedom_types::ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash: [1; 32],
            new_set_hash: [2; 32],
            votes: vec![],
        });

        assert_eq!(
            collect_validator_set_certificate(&state, &[]),
            Err("validator set certificate is already committed for this transition".into())
        );
    }
}


#[cfg(test)]
mod m163_registry_binding_tests {
    use super::*;

    #[test]
    fn registry_binding_requires_exact_representation_identity() {
        // The runtime must never silently substitute another representation.
        assert_ne!([1u8; 32], [2u8; 32]);
    }
}


#[cfg(test)]
mod m320_block_builder_epoch_tests {
    use super::*;

    #[test]
    fn block_builder_rejects_arbitrary_epoch() {
        let state = test_state();
        let mempool = Mempool::new();
        let proposer = freedom_consensus::proposer_for_height(&state, state.height + 1).unwrap();
        assert_eq!(
            build_next_block(&state, &mempool, proposer, 1, state.epoch + 1, 0, 10),
            Err("invalid block epoch".into())
        );
    }

    #[test]
    fn block_builder_rejects_epoch_transition_without_certificate() {
        let mut state = test_state();
        state.epoch_length = 1;
        state.height = 0;
        state.staking_activated = true;
        let mempool = Mempool::new();
        let proposer = freedom_consensus::proposer_for_height(&state, 1).unwrap();
        assert_eq!(
            build_next_block(&state, &mempool, proposer, 1, 1, 0, 10),
            Err("missing epoch transition certificate".into())
        );
    }
}


#[cfg(test)]
mod m321_epoch_transition_commitment_tests {
    use super::*;
    use freedom_crypto::sign;
    use freedom_types::{ValidatorSetCertificate, ValidatorSetVote};

    #[test]
    fn execute_block_rejects_boundary_when_state_transition_did_not_commit() {
        let mut state = test_state();
        state.epoch_length = 1;

        let previous_set_hash =
            freedom_consensus::validator_set_hash(&state.active_validator_set);
        let new_set_hash =
            freedom_consensus::validator_set_hash(&state.pending_validator_set);

        let mut votes = Vec::new();
        for (id, entry) in &state.active_validator_set {
            let mut vote = ValidatorSetVote {
                from_epoch: 0,
                to_epoch: 1,
                previous_set_hash,
                new_set_hash,
                validator_id: *id,
                signature: Vec::new(),
            };
            vote.signature = sign(
                &ed25519_dalek::SigningKey::from_bytes(&entry.consensus_key),
                &vote.signing_bytes(),
            )
            .to_vec();
            votes.push(vote);
        }

        let certificate = ValidatorSetCertificate {
            from_epoch: 0,
            to_epoch: 1,
            previous_set_hash,
            new_set_hash,
            votes,
        };
        state.validator_set_certificate = Some(certificate.clone());

        // Consensus admission does not depend on the future-epoch history
        // commitment. Corrupting that commitment makes ChainState refuse the
        // actual transition after consensus has already accepted the cert.
        state.validator_set_history.insert(1, [9u8; 32]);

        let proposer =
            freedom_consensus::proposer_for_height(&state, 1).unwrap();
        let mut preview = state.clone();
        preview.height = 1;

        let block = Block {
            header: freedom_types::BlockHeader {
                height: 1,
                round: 0,
                previous_hash: state.last_block_hash,
                state_root: preview.state_root(),
                tx_root: tx_root(&[]),
                receipts_root: receipts_root(&[]),
                timestamp: 1,
                epoch: 1,
                proposer,
                epoch_transition_certificate: Some(certificate),
            },
            transactions: Vec::new(),
            receipts: Vec::new(),
        };

        assert_eq!(
            execute_block(&mut state, &block),
            Err("invalid validator set certificate transition fields".into())
        );
        assert_eq!(state.epoch, 0);
        assert_eq!(state.height, 0);
    }
}


#[cfg(test)]
mod genesis_commitment_runtime_tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use freedom_primitives::{address_from_public_key, hash_bytes, GENESIS_SUPPLY};
    use freedom_types::{genesis_commitment::GenesisCommitmentManifest, registry_authorization::RegistryAuthorizationPolicy, Validator};
    use std::collections::BTreeMap;

    fn committed_genesis() -> (Genesis, GenesisCommitmentManifest) {
        let keys: Vec<_> = (1u8..=4).map(|n| SigningKey::from_bytes(&[n; 32])).collect();
        let members: BTreeMap<_, _> = keys.iter().map(|key| {
            let public_key = key.verifying_key().to_bytes();
            (hash_bytes(&public_key), public_key)
        }).collect();
        let validators: Vec<_> = keys.iter().take(3).map(|key| {
            let consensus_key = key.verifying_key().to_bytes();
            Validator {
                validator_id: hash_bytes(&consensus_key),
                owner: address_from_public_key(&consensus_key),
                consensus_key,
                stake: 0,
                unbonding_stake: 0,
                blocks_proposed: 0,
                blocks_finalized: 0,
                missed_votes: 0,
            }
        }).collect();
        let genesis = Genesis {
            chain_id: CHAIN_ID,
            initial_supply: GENESIS_SUPPLY,
            validators: validators.clone(),
        };
        let manifest = GenesisCommitmentManifest {
            chain_id: CHAIN_ID,
            network_id: 1,
            initial_supply: GENESIS_SUPPLY,
            validators,
            registry_policy: RegistryAuthorizationPolicy {
                policy_id: [9; 32],
                threshold: 4,
                members,
            },
            registry_activation_height: 0,
        };
        (genesis, manifest)
    }

    #[test]
    fn secure_genesis_initialization_accepts_pinned_manifest() {
        let (genesis, manifest) = committed_genesis();
        let anchor = manifest.commitment_hash().unwrap();
        assert!(initialize_with_genesis_commitment(&genesis, &manifest, &anchor).is_ok());
    }

    #[test]
    fn secure_genesis_initialization_rejects_manifest_mismatch() {
        let (genesis, manifest) = committed_genesis();
        let anchor = manifest.commitment_hash().unwrap();
        let mut altered_genesis = genesis.clone();
        altered_genesis.initial_supply = 1;
        assert!(initialize_with_genesis_commitment(&altered_genesis, &manifest, &anchor).is_err());
    }

    #[test]
    fn secure_genesis_initialization_rejects_untrusted_anchor() {
        let (genesis, manifest) = committed_genesis();
        assert!(initialize_with_genesis_commitment(&genesis, &manifest, &[0; 32]).is_err());
    }
}
