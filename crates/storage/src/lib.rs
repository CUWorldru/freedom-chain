use freedom_state::{legacy_snapshot::LegacyChainStateV1, ChainState};
use freedom_primitives::{CHAIN_ID, Hash};
use freedom_types::Block;
use rocksdb::{Options, DB, WriteBatch};
use std::path::Path;

const BLOCK_PREFIX: &[u8] = b"block:";
const META_HEIGHT: &[u8] = b"meta:height";
const META_HASH: &[u8] = b"meta:last_hash";
const SNAPSHOT_PREFIX: &[u8] = b"snapshot:";
const REPUTATION_KEY: &[u8] = b"meta:peer_reputation";

// Versioned binary encoding supports consensus-state maps keyed by fixed-size
// hashes and addresses, which JSON object keys cannot represent. Legacy JSON
// snapshots remain readable for backward compatibility.
const SNAPSHOT_MAGIC: &[u8] = b"321MSNP\x01";

fn encode_snapshot(snapshot: &StateSnapshot) -> Result<Vec<u8>, StorageError> {
    // Keep writing the frozen v1 wire type until the next schema version is
    // introduced. This prevents incidental ChainState edits from silently
    // changing the bytes emitted under the v1 magic.
    let legacy = LegacyStateSnapshotV1::from(snapshot.clone());
    let payload = bincode::serialize(&legacy)
        .map_err(|e| StorageError::Codec(e.to_string()))?;
    let mut bytes = SNAPSHOT_MAGIC.to_vec();
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

fn decode_snapshot(bytes: &[u8]) -> Result<StateSnapshot, StorageError> {
    if let Some(payload) = bytes.strip_prefix(SNAPSHOT_MAGIC) {
        let legacy: LegacyStateSnapshotV1 =
            bincode::deserialize(payload).map_err(|e| StorageError::Codec(e.to_string()))?;
        Ok(legacy.into())
    } else {
        serde_json::from_slice(bytes).map_err(|e| StorageError::Codec(e.to_string()))
    }
}

/// Frozen outer snapshot envelope for the original bincode v1 format.
/// Field order and types must remain unchanged; new snapshot fields belong
/// only in a future versioned envelope.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct LegacyStateSnapshotV1 {
    #[serde(default)]
    chain_id: u64,
    height: u64,
    block_hash: [u8;32],
    state_root: [u8;32],
    state: LegacyChainStateV1,
    #[serde(default)]
    certificate: Option<freedom_types::FinalityCertificate>,
    #[serde(default)]
    finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    #[serde(default)]
    epoch_transition_certificate: Option<freedom_types::ValidatorSetCertificate>,
}

impl From<StateSnapshot> for LegacyStateSnapshotV1 {
    fn from(snapshot: StateSnapshot) -> Self {
        Self {
            chain_id: snapshot.chain_id,
            height: snapshot.height,
            block_hash: snapshot.block_hash,
            state_root: snapshot.state_root,
            state: snapshot.state.into(),
            certificate: snapshot.certificate,
            finality_validator_set: snapshot.finality_validator_set,
            epoch_transition_certificate: snapshot.epoch_transition_certificate,
        }
    }
}

impl From<LegacyStateSnapshotV1> for StateSnapshot {
    fn from(snapshot: LegacyStateSnapshotV1) -> Self {
        Self {
            chain_id: snapshot.chain_id,
            height: snapshot.height,
            block_hash: snapshot.block_hash,
            state_root: snapshot.state_root,
            state: snapshot.state.into(),
            certificate: snapshot.certificate,
            finality_validator_set: snapshot.finality_validator_set,
            epoch_transition_certificate: snapshot.epoch_transition_certificate,
        }
    }
}

#[derive(Debug)]
pub enum StorageError { Rocks(String), Codec(String), MissingBlock(u64), InvalidMetadata, InvalidSnapshot(String) }

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StorageError {}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StateSnapshot {
    #[serde(default)]
    pub chain_id: u64,
    pub height: u64,
    pub block_hash: [u8;32],
    pub state_root: [u8;32],
    pub state: ChainState,
    #[serde(default)]
    pub certificate: Option<freedom_types::FinalityCertificate>,
    #[serde(default)]
    pub finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    #[serde(default)]
    pub epoch_transition_certificate: Option<freedom_types::ValidatorSetCertificate>,
}

/// Bind the snapshot's finality signer set to the validator history committed
/// by the snapshot state root. A peer or corrupted disk must not be able to
/// replace both the certificate and its accompanying signer set independently.
fn validate_snapshot_finality_set_binding(
    snapshot: &StateSnapshot,
    checkpoint: &Block,
) -> Result<(), StorageError> {
    let finality_epoch = checkpoint.header.epoch_transition_certificate
        .as_ref()
        .map(|certificate| certificate.from_epoch)
        .unwrap_or(checkpoint.header.epoch);
    let historical_set = snapshot.state.validator_set_history_entries
        .get(&finality_epoch)
        .ok_or_else(|| StorageError::InvalidSnapshot(
            format!("snapshot has no validator history for finality epoch {finality_epoch}"),
        ))?;
    if historical_set != &snapshot.finality_validator_set {
        return Err(StorageError::InvalidSnapshot(
            "finality validator set does not match snapshot's committed validator history".into(),
        ));
    }
    Ok(())
}

pub struct ChainStore { db: DB }

impl ChainStore {
    pub fn load_peer_reputation<T: serde::de::DeserializeOwned>(&self) -> Result<T, StorageError> {
        match self.db.get(REPUTATION_KEY).map_err(|e| StorageError::Rocks(e.to_string()))? {
            Some(bytes) => serde_json::from_slice(&bytes).map_err(|e| StorageError::Codec(e.to_string())),
            None => Err(StorageError::InvalidMetadata),
        }
    }

    pub fn save_peer_reputation<T: serde::Serialize>(&self, reputation: &T) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(reputation).map_err(|e| StorageError::Codec(e.to_string()))?;
        let mut batch = WriteBatch::default();
        batch.put(REPUTATION_KEY, bytes);
        self.db.write(batch).map_err(|e| StorageError::Rocks(e.to_string()))
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        Ok(Self { db: DB::open(&opts, path).map_err(|e| StorageError::Rocks(e.to_string()))? })
    }

    fn snapshot_key(height: u64) -> Vec<u8> { let mut k = SNAPSHOT_PREFIX.to_vec(); k.extend_from_slice(&height.to_be_bytes()); k }

    fn height_key(height: u64) -> Vec<u8> {
        let mut k = BLOCK_PREFIX.to_vec();
        k.extend_from_slice(&height.to_be_bytes());
        k
    }

    pub fn put_block(&self, block: &Block) -> Result<(), StorageError> {
        // Keep single-block commits subject to the exact same canonical-chain
        // continuity and metadata checks as multi-block synchronization.
        self.put_blocks_atomic(std::slice::from_ref(block))
    }

    pub fn put_blocks_atomic(&self, blocks: &[Block]) -> Result<(), StorageError> {
        if blocks.is_empty() {
            return Ok(());
        }
        // Never allow a batch to overwrite history, skip heights, or attach to a
        // metadata tip that does not match the persisted canonical block.
        self.validate_tip_metadata()?;
        let current_height = self.latest_height()?;
        let mut expected_height = current_height
            .checked_add(1)
            .ok_or(StorageError::InvalidMetadata)?;
        let mut expected_previous_hash = self.latest_hash()?;

        let mut batch = WriteBatch::default();
        for block in blocks {
            if block.header.height != expected_height
                || block.header.previous_hash != expected_previous_hash
            {
                return Err(StorageError::InvalidMetadata);
            }
            let bytes = serde_json::to_vec(block)
                .map_err(|e| StorageError::Codec(e.to_string()))?;
            batch.put(Self::height_key(block.header.height), bytes);
            expected_previous_hash = block_hash(block);
            expected_height = expected_height
                .checked_add(1)
                .ok_or(StorageError::InvalidMetadata)?;
        }

        let tip = blocks.last().expect("non-empty batch");
        batch.put(META_HEIGHT, tip.header.height.to_be_bytes());
        batch.put(META_HASH, block_hash(tip));
        self.db.write(batch).map_err(|e| StorageError::Rocks(e.to_string()))?;
        Ok(())
    }

    pub fn validate_tip_metadata(&self) -> Result<(), StorageError> {
        let height = self.latest_height()?;
        let hash = self.latest_hash()?;
        if height == 0 {
            if hash != [0; 32] {
                return Err(StorageError::InvalidMetadata);
            }
            return Ok(());
        }
        let block = self.get_block(height)?
            .ok_or(StorageError::MissingBlock(height))?;
        // The key and metadata hash alone are insufficient: corrupted storage
        // could place a differently-heighted block at the tip key and make its
        // hash agree with META_HASH. Never accept that as a canonical tip.
        if block.header.height != height || block_hash(&block) != hash {
            return Err(StorageError::InvalidMetadata);
        }
        Ok(())
    }

    pub fn get_block(&self, height: u64) -> Result<Option<Block>, StorageError> {
        let Some(bytes) = self.db.get(Self::height_key(height)).map_err(|e| StorageError::Rocks(e.to_string()))? else { return Ok(None); };
        let block: Block = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::Codec(e.to_string()))?;
        // A block's database key is part of its canonical identity. Reject a
        // record whose serialized header claims a different height, including
        // reads used by replay and peer sync serving.
        if block.header.height != height {
            return Err(StorageError::InvalidMetadata);
        }
        Ok(Some(block))
    }

    pub fn latest_height(&self) -> Result<u64, StorageError> {
        match self.db.get(META_HEIGHT).map_err(|e| StorageError::Rocks(e.to_string()))? {
            Some(b) if b.len() == 8 => { let mut x=[0u8;8]; x.copy_from_slice(&b); Ok(u64::from_be_bytes(x)) }
            Some(_) => Err(StorageError::InvalidMetadata),
            None => Ok(0),
        }
    }

    pub fn latest_hash(&self) -> Result<[u8;32], StorageError> {
        match self.db.get(META_HASH).map_err(|e| StorageError::Rocks(e.to_string()))? {
            Some(b) if b.len() == 32 => { let mut x=[0u8;32]; x.copy_from_slice(&b); Ok(x) }
            Some(_) => Err(StorageError::InvalidMetadata),
            None => Ok([0u8;32]),
        }
    }

    pub fn save_snapshot(&self, state: &ChainState, block_hash: [u8;32]) -> Result<(), StorageError> {
        self.save_snapshot_with_certificate_opt(state, block_hash, None, state.active_validator_set.clone())
    }

    pub fn save_snapshot_with_certificate(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: freedom_types::FinalityCertificate,
    ) -> Result<(), StorageError> {
        self.save_snapshot_with_certificate_and_validator_set(
            state, block_hash, certificate, state.active_validator_set.clone()
        )
    }

    pub fn save_snapshot_with_certificate_and_validator_set(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: freedom_types::FinalityCertificate,
        finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    ) -> Result<(), StorageError> {
        self.save_snapshot_with_certificate_opt(state, block_hash, Some(certificate), finality_validator_set)
    }

    fn validate_snapshot_integrity_with_checkpoint(
        &self,
        snapshot: &StateSnapshot,
        checkpoint: Option<&Block>,
    ) -> Result<(), StorageError> {
        if snapshot.chain_id != u64::from(CHAIN_ID) {
            return Err(StorageError::InvalidSnapshot("chain id mismatch".into()));
        }
        if snapshot.state.height != snapshot.height {
            return Err(StorageError::InvalidSnapshot("height mismatch".into()));
        }
        if snapshot.state.state_root() != snapshot.state_root {
            return Err(StorageError::InvalidSnapshot("state root mismatch".into()));
        }

        snapshot.state.validate_finality_state_integrity()
            .map_err(|e| StorageError::InvalidSnapshot(format!("persisted finality state integrity failure: {e:?}")))?;

        snapshot.state.validate_persisted_bridge_state_integrity()
            .map_err(|e| StorageError::InvalidSnapshot(format!("persisted bridge state integrity failure: {e:?}")))?;

        if snapshot.height == 0 {
            if snapshot.block_hash != [0; 32] || snapshot.state.last_block_hash != [0; 32] {
                return Err(StorageError::InvalidSnapshot("genesis snapshot hash linkage mismatch".into()));
            }
            if snapshot.certificate.is_some() || snapshot.epoch_transition_certificate.is_some() {
                return Err(StorageError::InvalidSnapshot("genesis snapshot contains finality certificate metadata".into()));
            }
        } else {
            if snapshot.state.last_block_hash != snapshot.block_hash {
                return Err(StorageError::InvalidSnapshot("snapshot state last hash mismatch".into()));
            }
            let checkpoint = checkpoint
                .ok_or_else(|| StorageError::InvalidSnapshot("checkpoint block missing".into()))?;
            if checkpoint.header.height != snapshot.height {
                return Err(StorageError::InvalidSnapshot("checkpoint height mismatch".into()));
            }
            if freedom_consensus::block_hash(checkpoint) != snapshot.block_hash {
                return Err(StorageError::InvalidSnapshot("snapshot checkpoint hash mismatch".into()));
            }
            if checkpoint.header.state_root != snapshot.state_root {
                return Err(StorageError::InvalidSnapshot("checkpoint state root mismatch".into()));
            }

            let cert = snapshot.certificate.as_ref()
                .ok_or_else(|| StorageError::InvalidSnapshot("snapshot has no finality certificate".into()))?;

            if cert.block_hash != snapshot.block_hash || cert.height != snapshot.height {
                return Err(StorageError::InvalidSnapshot("finality certificate does not match snapshot checkpoint".into()));
            }
            freedom_consensus::validate_finality_certificate_for_set(
                &snapshot.finality_validator_set,
                checkpoint,
                cert,
            ).map_err(StorageError::InvalidSnapshot)?;

            validate_snapshot_finality_set_binding(snapshot, checkpoint)?;

            if let Some(transition) = &snapshot.epoch_transition_certificate {
                if freedom_consensus::validator_set_hash(&snapshot.finality_validator_set) != transition.previous_set_hash {
                    return Err(StorageError::InvalidSnapshot("finality validator set does not match transition previous set".into()));
                }
            } else if freedom_consensus::validator_set_hash(&snapshot.finality_validator_set)
                != freedom_consensus::validator_set_hash(&snapshot.state.active_validator_set)
            {
                return Err(StorageError::InvalidSnapshot("finality validator set does not match active set".into()));
            }
        }

        if snapshot.epoch_transition_certificate.as_ref().map(|c| c.hash())
            != snapshot.state.validator_set_certificate.as_ref().map(|c| c.hash())
        {
            return Err(StorageError::InvalidSnapshot("epoch transition certificate mismatch".into()));
        }
        if let Some(cert) = &snapshot.epoch_transition_certificate {
            if cert.to_epoch != snapshot.state.epoch {
                return Err(StorageError::InvalidSnapshot("epoch transition certificate target epoch mismatch".into()));
            }
            if freedom_consensus::validator_set_hash(&snapshot.state.active_validator_set) != cert.new_set_hash {
                return Err(StorageError::InvalidSnapshot("epoch transition certificate new set mismatch".into()));
            }
        }
        Ok(())
    }

    fn validate_snapshot_integrity(&self, snapshot: &StateSnapshot) -> Result<(), StorageError> {
        let checkpoint = if snapshot.height == 0 {
            None
        } else {
            Some(
                self.get_block(snapshot.height)?
                    .ok_or_else(|| StorageError::InvalidSnapshot("checkpoint block missing".into()))?
            )
        };
        self.validate_snapshot_integrity_with_checkpoint(snapshot, checkpoint.as_ref())
    }

    fn build_snapshot(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: Option<freedom_types::FinalityCertificate>,
        finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    ) -> StateSnapshot {
        StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: state.height,
            block_hash,
            state_root: state.state_root(),
            state: state.clone(),
            certificate,
            finality_validator_set,
            epoch_transition_certificate: state.validator_set_certificate.clone(),
        }
    }

    fn save_snapshot_with_certificate_opt(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: Option<freedom_types::FinalityCertificate>,
        finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    ) -> Result<(), StorageError> {
        let snapshot = self.build_snapshot(state, block_hash, certificate, finality_validator_set);
        self.validate_snapshot_integrity(&snapshot)?;
        let bytes = encode_snapshot(&snapshot)?;
        self.db.put(Self::snapshot_key(state.height), bytes).map_err(|e| StorageError::Rocks(e.to_string()))?;
        Ok(())
    }

    pub fn put_block_and_snapshot(
        &self,
        checkpoint: &Block,
        state: &ChainState,
        certificate: freedom_types::FinalityCertificate,
        finality_validator_set: std::collections::BTreeMap<Hash, freedom_types::ValidatorSetEntry>,
    ) -> Result<(), StorageError> {
        // The atomic checkpoint path must enforce the same append-only rule as
        // put_block(s)_atomic; otherwise a snapshot write could overwrite history.
        self.validate_tip_metadata()?;
        let current_height = self.latest_height()?;
        // A matching tip does not prove the older canonical prefix is intact.
        // Before extending local history with a checkpoint, validate every
        // parent link from genesis so a remote snapshot cannot cement a fork
        // above an already-corrupted ancestor.
        self.validate_chain_through(current_height)?;
        let expected_height = current_height
            .checked_add(1)
            .ok_or(StorageError::InvalidMetadata)?;
        let current_hash = self.latest_hash()?;
        if checkpoint.header.height != expected_height
            || checkpoint.header.previous_hash != current_hash
            || checkpoint.header.height != state.height
        {
            return Err(StorageError::InvalidMetadata);
        }
        let checkpoint_hash = freedom_consensus::block_hash(checkpoint);
        if checkpoint_hash != state.last_block_hash {
            return Err(StorageError::InvalidSnapshot("checkpoint does not match state last hash".into()));
        }

        let snapshot = self.build_snapshot(
            state,
            checkpoint_hash,
            Some(certificate),
            finality_validator_set,
        );
        self.validate_snapshot_integrity_with_checkpoint(&snapshot, Some(checkpoint))?;

        let block_bytes = serde_json::to_vec(checkpoint)
            .map_err(|e| StorageError::Codec(e.to_string()))?;
        let snapshot_bytes = encode_snapshot(&snapshot)?;

        let mut batch = WriteBatch::default();
        batch.put(Self::height_key(checkpoint.header.height), block_bytes);
        batch.put(META_HEIGHT, checkpoint.header.height.to_be_bytes());
        batch.put(META_HASH, checkpoint_hash);
        batch.put(Self::snapshot_key(snapshot.height), snapshot_bytes);
        self.db.write(batch).map_err(|e| StorageError::Rocks(e.to_string()))?;
        Ok(())
    }

    pub fn load_snapshot(&self, height: u64) -> Result<Option<StateSnapshot>, StorageError> {
        let Some(bytes) = self.db.get(Self::snapshot_key(height)).map_err(|e| StorageError::Rocks(e.to_string()))? else { return Ok(None); };
        let snapshot = decode_snapshot(&bytes)?;
        if snapshot.height != height {
            return Err(StorageError::InvalidSnapshot("height mismatch".into()));
        }
        self.validate_snapshot_integrity(&snapshot)?;
        Ok(Some(snapshot))
    }

    pub fn latest_snapshot_at_or_below(&self, target_height: u64) -> Result<Option<StateSnapshot>, StorageError> {
        let mut candidates = Vec::new();
        for item in self.db.iterator(rocksdb::IteratorMode::Start) {
            let (key, _) = item.map_err(|e| StorageError::Rocks(e.to_string()))?;
            if key.starts_with(SNAPSHOT_PREFIX) && key.len() == SNAPSHOT_PREFIX.len() + 8 {
                let mut h=[0u8;8]; h.copy_from_slice(&key[SNAPSHOT_PREFIX.len()..]);
                let height=u64::from_be_bytes(h);
                if height <= target_height {
                    candidates.push(height);
                }
            }
        }

        // A corrupt newest snapshot must not prevent recovery from an older
        // valid checkpoint. Block replay from that checkpoint will reconstruct
        // and validate the remaining canonical history.
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        for height in candidates {
            match self.load_snapshot(height) {
                Ok(Some(snapshot)) => return Ok(Some(snapshot)),
                Ok(None) => continue,
                Err(StorageError::InvalidSnapshot(_)
                    | StorageError::Codec(_)
                    | StorageError::MissingBlock(_)
                    | StorageError::InvalidMetadata) => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub fn latest_snapshot(&self) -> Result<Option<StateSnapshot>, StorageError> {
        self.latest_snapshot_at_or_below(u64::MAX)
    }
    /// Verify the complete canonical parent-link chain from genesis through
    /// `height` without materializing the whole history in memory.
    pub fn validate_chain_through(&self, height: u64) -> Result<(), StorageError> {
        if height == 0 {
            return Ok(());
        }
        let mut expected_previous_hash = [0u8; 32];
        for current_height in 1..=height {
            let block = self.get_block(current_height)?
                .ok_or(StorageError::MissingBlock(current_height))?;
            if block.header.previous_hash != expected_previous_hash {
                return Err(StorageError::InvalidMetadata);
            }
            expected_previous_hash = block_hash(&block);
        }
        Ok(())
    }

    pub fn range(&self, from: u64, to: u64) -> Result<Vec<Block>, StorageError> {
        if from > to { return Ok(Vec::new()); }
        // Preserve the missing-genesis behavior and avoid attacker-controlled
        // inclusive range arithmetic such as `to - from + 1`.
        if from == 0 {
            return Err(StorageError::MissingBlock(0));
        }
        // Validate the prefix as well as the requested segment. Otherwise a
        // request starting after a corrupted ancestor could still serve a
        // superficially contiguous suffix to a peer.
        self.validate_chain_through(to)?;
        let mut out = Vec::new();
        for height in from..=to {
            let block = self.get_block(height)?
                .ok_or(StorageError::MissingBlock(height))?;
            out.push(block);
        }
        Ok(out)
    }
}

fn block_hash(block: &Block) -> [u8;32] {
    freedom_consensus::block_hash(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_types::{Block, BlockHeader};
    use tempfile::tempdir;

    #[test]
    fn persists_and_reads_blocks_by_height() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), block_hash(&block));
        assert_eq!(store.get_block(1).unwrap(), Some(block));
        let genesis = freedom_state::ChainState::genesis(&freedom_types::Genesis { chain_id: 1, initial_supply: 0, validators: vec![] });
        store.save_snapshot(&genesis, [0;32]).unwrap();
        let loaded = store.load_snapshot(0).unwrap().unwrap();
        assert_eq!(loaded.block_hash, [0;32]);
        assert_eq!(loaded.state.last_block_hash, [0;32]);
        assert_eq!(loaded.state.state_root(), loaded.state_root);
        // The binary snapshot codec must preserve populated fixed-size-key
        // registries; these are not serializable as ordinary JSON map keys.
        assert!(!loaded.state.asset_registry.is_empty());
        assert!(!loaded.state.cross_chain_asset_registries.is_empty());
        assert!(loaded.certificate.is_none());
        assert!(loaded.epoch_transition_certificate.is_none());
    }

    #[test]
    fn reads_legacy_json_genesis_snapshots() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        // Historical JSON snapshots could only represent empty fixed-size-key
        // maps. Preserve compatibility for those previously serializable states.
        state.asset_registry.clear();
        state.cross_chain_asset_registries.clear();
        let snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: 0,
            block_hash: [0; 32],
            state_root: state.state_root(),
            state,
            certificate: None,
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };
        let legacy_json = serde_json::to_vec(&snapshot).unwrap();
        store.db.put(ChainStore::snapshot_key(0), legacy_json).unwrap();

        let loaded = store.load_snapshot(0).unwrap().unwrap();
        assert_eq!(loaded.height, 0);
        assert_eq!(loaded.state_root, loaded.state.state_root());
    }

    #[test]
    fn frozen_bincode_v1_snapshot_migrates_through_legacy_wire_types() {
        let state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        let snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: 0,
            block_hash: [0; 32],
            state_root: state.state_root(),
            state,
            certificate: None,
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };

        // Encode with the frozen v1 wire structs, as an older node did.
        let legacy = LegacyStateSnapshotV1::from(snapshot.clone());
        let mut bytes = SNAPSHOT_MAGIC.to_vec();
        bytes.extend_from_slice(&bincode::serialize(&legacy).unwrap());

        let restored = decode_snapshot(&bytes).unwrap();
        assert_eq!(restored.height, snapshot.height);
        assert_eq!(restored.block_hash, snapshot.block_hash);
        assert_eq!(restored.state_root, snapshot.state_root);
        assert_eq!(restored.state.state_root(), snapshot.state_root);
        assert_eq!(restored.state.accounts, snapshot.state.accounts);
    }

    #[test]
    fn rejects_zero_height_block_write() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 0, round: 0, previous_hash: [0;32], state_root: [0;32], tx_root: [0;32],
                receipts_root: [0;32], timestamp: 0, epoch: 0, proposer: [0;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(matches!(store.put_block(&block), Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), 0);
        assert_eq!(store.latest_hash().unwrap(), [0;32]);
    }

    #[test]
    fn tip_metadata_matches_persisted_block() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();
        store.validate_tip_metadata().unwrap();
        store.db.put(META_HASH, [9u8;32]).unwrap();
        assert!(matches!(store.validate_tip_metadata(), Err(StorageError::InvalidMetadata)));
    }

    #[test]
    fn get_block_rejects_header_height_that_disagrees_with_storage_key() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();
        block.header.height = 0;
        store.db.put(ChainStore::height_key(1), serde_json::to_vec(&block).unwrap()).unwrap();

        assert!(matches!(store.get_block(1), Err(StorageError::InvalidMetadata)));
        assert!(matches!(store.range(1, 1), Err(StorageError::InvalidMetadata)));
    }

    #[test]
    fn tip_metadata_rejects_block_whose_header_height_disagrees_with_key() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();

        // Model internally consistent hash metadata around a block stored under
        // height 1 whose own header claims height 0.
        block.header.height = 0;
        let corrupted_hash = block_hash(&block);
        store.db.put(ChainStore::height_key(1), serde_json::to_vec(&block).unwrap()).unwrap();
        store.db.put(META_HASH, corrupted_hash).unwrap();

        assert!(matches!(store.validate_tip_metadata(), Err(StorageError::InvalidMetadata)));
        // Any append must fail before it can extend this malformed history.
        let next = Block {
            header: BlockHeader {
                height: 2, round: 0, previous_hash: corrupted_hash, state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 2, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(matches!(store.put_block(&next), Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), corrupted_hash);
        assert!(store.get_block(2).unwrap().is_none());
    }

    #[test]
    fn atomic_block_batch_rejects_non_contiguous_members_without_writing() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height| Block {
            header: BlockHeader {
                height, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        assert!(matches!(
            store.put_blocks_atomic(&[make(1), make(3)]),
            Err(StorageError::InvalidMetadata)
        ));
        assert!(store.get_block(1).unwrap().is_none());
        assert!(store.get_block(3).unwrap().is_none());
        assert_eq!(store.latest_height().unwrap(), 0);
    }

    #[test]
    fn single_block_write_rejects_overwrite_and_wrong_parent() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };

        let first = make(1, [0;32]);
        let first_hash = block_hash(&first);
        store.put_block(&first).unwrap();

        assert!(matches!(
            store.put_block(&make(1, [0;32])),
            Err(StorageError::InvalidMetadata)
        ));
        assert!(matches!(
            store.put_block(&make(2, [9;32])),
            Err(StorageError::InvalidMetadata)
        ));
        assert_eq!(store.get_block(1).unwrap(), Some(first));
        assert!(store.get_block(2).unwrap().is_none());
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), first_hash);
    }

    #[test]
    fn atomic_block_batch_rejects_disconnected_or_overwriting_history() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };

        let first = make(1, [0;32]);
        let first_hash = block_hash(&first);
        store.put_blocks_atomic(&[first.clone()]).unwrap();

        assert!(matches!(
            store.put_blocks_atomic(&[make(1, [0;32])]),
            Err(StorageError::InvalidMetadata)
        ));
        assert!(matches!(
            store.put_blocks_atomic(&[make(2, [9;32])]),
            Err(StorageError::InvalidMetadata)
        ));
        assert_eq!(store.get_block(1).unwrap(), Some(first));
        assert!(store.get_block(2).unwrap().is_none());
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), first_hash);
    }

    #[test]
    fn atomic_block_batch_rejects_invalid_member_without_writing_any_block() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let valid = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let invalid = Block {
            header: BlockHeader {
                height: 0, round: 0, previous_hash: [0;32], state_root: [0;32], tx_root: [0;32],
                receipts_root: [0;32], timestamp: 0, epoch: 0, proposer: [0;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };

        assert!(matches!(
            store.put_blocks_atomic(&[valid, invalid]),
            Err(StorageError::InvalidMetadata)
        ));
        assert!(store.get_block(1).unwrap().is_none());
        assert_eq!(store.latest_height().unwrap(), 0);
        assert_eq!(store.latest_hash().unwrap(), [0; 32]);
    }

    #[test]
    fn snapshot_lookup_falls_back_when_newest_snapshot_is_corrupt() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let genesis = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![]
        });
        store.save_snapshot(&genesis, [0;32]).unwrap();

        // Simulate a torn/corrupt latest snapshot while retaining an older
        // valid checkpoint and its block history.
        store.db.put(ChainStore::snapshot_key(1), b"{not valid json").unwrap();

        let snapshot = store.latest_snapshot_at_or_below(1).unwrap().unwrap();
        assert_eq!(snapshot.height, 0);
        assert_eq!(snapshot.block_hash, [0;32]);
        assert_eq!(snapshot.state.height, 0);
    }

    #[test]
    fn bounded_snapshot_lookup_ignores_higher_snapshot() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let genesis = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        store.save_snapshot(&genesis, [0;32]).unwrap();
        // A snapshot may survive above the canonical tip after legacy/non-atomic
        // writes. Startup lookup bounded by the tip must ignore it entirely.
        store.db.put(ChainStore::snapshot_key(10), b"{corrupt snapshot").unwrap();
        assert_eq!(
            store.latest_snapshot_at_or_below(0).unwrap().unwrap().height,
            0
        );
        // Unbounded lookup must still skip the corrupt newer entry and recover
        // the valid genesis checkpoint rather than failing closed unnecessarily.
        assert_eq!(
            store.latest_snapshot_at_or_below(10).unwrap().unwrap().height,
            0
        );
    }

    #[test]
    fn rejects_non_genesis_snapshot_without_checkpoint() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.height = 1;
        state.last_block_hash = [7; 32];
        let state_root = state.state_root();
        let snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: 1,
            block_hash: [7; 32],
            state_root,
            state,
            certificate: None,
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };
        store.db.put(ChainStore::snapshot_key(1), encode_snapshot(&snapshot).unwrap()).unwrap();
        let err = store.load_snapshot(1).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "checkpoint block missing"));
    }


    #[test]
    fn atomic_checkpoint_snapshot_rejects_mismatched_state_before_write() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let checkpoint = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.height = 1;
        state.last_block_hash = [9; 32];

        let err = store.put_block_and_snapshot(
            &checkpoint,
            &state,
            freedom_types::FinalityCertificate {
                block_hash: block_hash(&checkpoint),
                height: 1,
                round: 0,
                precommits: Vec::new(),
            },
            std::collections::BTreeMap::new(),
        ).unwrap_err();

        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "checkpoint does not match state last hash"));
        assert_eq!(store.latest_height().unwrap(), 0);
        assert_eq!(store.latest_hash().unwrap(), [0; 32]);
        assert!(store.get_block(1).unwrap().is_none());
        assert!(store.load_snapshot(1).unwrap().is_none());
    }

    #[test]
    fn rejects_snapshot_write_without_checkpoint() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.height = 1;
        state.last_block_hash = [7; 32];
        let err = store.save_snapshot(&state, [7; 32]).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "checkpoint block missing"));
        assert!(store.load_snapshot(1).unwrap().is_none());
    }

    #[test]
    fn rejects_snapshot_write_with_mismatched_checkpoint_state_root() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.height = 1;
        state.last_block_hash = block_hash(&block);
        let err = store.save_snapshot(&state, block_hash(&block)).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "checkpoint state root mismatch"));
        assert!(store.load_snapshot(1).unwrap().is_none());
    }

    #[test]
    fn rejects_snapshot_write_with_mismatched_state_hash_linkage() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        store.put_block(&block).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.height = 1;
        state.last_block_hash = [9; 32];
        let err = store.save_snapshot(&state, block_hash(&block)).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "snapshot state last hash mismatch"));
        assert!(store.load_snapshot(0).is_ok());
    }

    #[test]
    fn rejects_snapshot_with_invalid_persisted_bitcoin_bridge_state() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        state.bitcoin_canonical_tips.insert([7u8; 32], freedom_types::BitcoinChainCheckpoint {
            height: 10, block_hash: [8u8; 32], chainwork: [0u8; 32],
        });
        let err = store.save_snapshot(&state, [0; 32]).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message.contains("persisted finality state integrity failure")));
        assert!(store.load_snapshot(0).unwrap().is_none());
    }

    #[test]
    fn rejects_corrupted_genesis_snapshot_hash_linkage() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let genesis = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: 1, initial_supply: 0, validators: vec![]
        });
        let mut snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: 0,
            block_hash: [0; 32],
            state_root: genesis.state_root(),
            state: genesis,
            certificate: None,
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };
        snapshot.block_hash = [9; 32];
        store.db.put(ChainStore::snapshot_key(0), encode_snapshot(&snapshot).unwrap()).unwrap();
        let err = store.load_snapshot(0).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message == "genesis snapshot hash linkage mismatch"));
    }
    #[test]
    fn range_rejects_corrupted_ancestor_before_requested_suffix() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [height as u8;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let first = make(1, [0;32]);
        let second = make(2, block_hash(&first));
        let third = make(3, block_hash(&second));
        store.put_blocks_atomic(&[first.clone(), second.clone(), third]).unwrap();

        let mut corrupted_ancestor = first.clone();
        corrupted_ancestor.header.previous_hash = [9;32];
        store.db.put(
            ChainStore::height_key(1),
            serde_json::to_vec(&corrupted_ancestor).unwrap(),
        ).unwrap();

        assert!(matches!(store.range(3, 3), Err(StorageError::InvalidMetadata)));
        assert!(matches!(store.validate_chain_through(2), Err(StorageError::InvalidMetadata)));
    }

    #[test]
    fn range_rejects_intermediate_parent_corruption_without_mutating_tip() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [height as u8;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let first = make(1, [0;32]);
        let second = make(2, block_hash(&first));
        let third = make(3, block_hash(&second));
        store.put_blocks_atomic(&[first.clone(), second.clone(), third.clone()]).unwrap();
        let tip_height = store.latest_height().unwrap();
        let tip_hash = store.latest_hash().unwrap();

        let mut corrupted_middle = second.clone();
        corrupted_middle.header.previous_hash = [9;32];
        store.db.put(
            ChainStore::height_key(2),
            serde_json::to_vec(&corrupted_middle).unwrap(),
        ).unwrap();

        assert!(matches!(store.range(1, 3), Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), tip_height);
        assert_eq!(store.latest_hash().unwrap(), tip_hash);
        assert_eq!(store.get_block(3).unwrap(), Some(third));
    }

    #[test]
    fn range_with_maximum_endpoint_does_not_overflow_capacity_arithmetic() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        assert!(matches!(
            store.range(0, u64::MAX),
            Err(StorageError::MissingBlock(0))
        ));
    }

    #[test]
    fn peer_reputation_roundtrip_persists() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut value = std::collections::BTreeMap::<String, u64>::new();
        value.insert("peer".to_string(), 3);
        store.save_peer_reputation(&value).unwrap();
        let loaded: std::collections::BTreeMap<String, u64> = store.load_peer_reputation().unwrap();
        assert_eq!(loaded, value);
    }




    #[test]
    fn snapshot_finality_set_must_match_committed_history() {
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![],
        });
        let checkpoint = Block {
            header: freedom_types::BlockHeader {
                height: 1, round: 0, previous_hash: [0; 32], state_root: [0; 32], tx_root: [0; 32],
                receipts_root: [0; 32], timestamp: 1, epoch: 0, proposer: [0; 32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let mut snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID), height: 1, block_hash: [0; 32], state_root: [0; 32],
            state: state.clone(), certificate: None, finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };
        assert!(validate_snapshot_finality_set_binding(&snapshot, &checkpoint).is_ok());

        snapshot.finality_validator_set.insert([7; 32], freedom_types::ValidatorSetEntry {
            validator_id: [7; 32], owner: [8; 32], consensus_key: [9; 32], stake: 10,
        });
        assert!(matches!(
            validate_snapshot_finality_set_binding(&snapshot, &checkpoint),
            Err(StorageError::InvalidSnapshot(_)),
        ));
    }

    #[test]
    fn transition_checkpoint_uses_previous_epoch_finality_set() {
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![],
        });
        let previous = std::collections::BTreeMap::from([([1; 32], freedom_types::ValidatorSetEntry {
            validator_id: [1; 32], owner: [2; 32], consensus_key: [3; 32], stake: 100,
        })]);
        let next = std::collections::BTreeMap::from([([4; 32], freedom_types::ValidatorSetEntry {
            validator_id: [4; 32], owner: [5; 32], consensus_key: [6; 32], stake: 200,
        })]);
        state.epoch = 1;
        state.validator_set_history_entries.insert(0, previous.clone());
        state.validator_set_history_entries.insert(1, next);
        let transition = freedom_types::ValidatorSetCertificate {
            from_epoch: 0, to_epoch: 1, previous_set_hash: [0; 32], new_set_hash: [0; 32], votes: vec![],
        };
        let checkpoint = Block {
            header: freedom_types::BlockHeader {
                height: 10, round: 0, previous_hash: [0; 32], state_root: [0; 32], tx_root: [0; 32],
                receipts_root: [0; 32], timestamp: 10, epoch: 1, proposer: [0; 32],
                epoch_transition_certificate: Some(transition),
            },
            transactions: vec![], receipts: vec![],
        };
        let snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID), height: 10, block_hash: [0; 32], state_root: [0; 32],
            state, certificate: None, finality_validator_set: previous,
            epoch_transition_certificate: None,
        };
        assert!(validate_snapshot_finality_set_binding(&snapshot, &checkpoint).is_ok());
    }

    #[test]
    fn corrupted_replay_block_fails_without_mutating_canonical_tip_metadata() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let block = Block {
            header: BlockHeader {
                height: 1, round: 0, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 1, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let canonical_hash = block_hash(&block);
        store.put_block(&block).unwrap();

        // Simulate on-disk corruption after the canonical tip was committed.
        store.db.put(ChainStore::height_key(1), b"{corrupt block json").unwrap();

        assert!(matches!(store.range(1, 1), Err(StorageError::Codec(_))));
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), canonical_hash);
        // Reading the damaged history fails closed; it is never silently replaced.
        assert!(matches!(store.get_block(1), Err(StorageError::Codec(_))));
    }


    #[test]
    fn atomic_checkpoint_snapshot_rejects_corrupt_local_ancestor() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [height as u8;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let first = make(1, [0;32]);
        let second = make(2, block_hash(&first));
        let checkpoint = make(3, block_hash(&second));
        store.put_blocks_atomic(&[first.clone(), second.clone()]).unwrap();
        let original_height = store.latest_height().unwrap();
        let original_hash = store.latest_hash().unwrap();

        let mut corrupted_ancestor = first;
        corrupted_ancestor.header.previous_hash = [9;32];
        store.db.put(
            ChainStore::height_key(1),
            serde_json::to_vec(&corrupted_ancestor).unwrap(),
        ).unwrap();

        let mut candidate = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![]
        });
        candidate.height = 3;
        candidate.last_block_hash = block_hash(&checkpoint);
        let result = store.put_block_and_snapshot(
            &checkpoint,
            &candidate,
            freedom_types::FinalityCertificate {
                block_hash: block_hash(&checkpoint), height: 3, round: 0, precommits: Vec::new()
            },
            std::collections::BTreeMap::new(),
        );

        assert!(matches!(result, Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), original_height);
        assert_eq!(store.latest_hash().unwrap(), original_hash);
        assert!(store.get_block(3).unwrap().is_none());
        assert!(store.load_snapshot(3).unwrap().is_none());
    }

    #[test]
    fn atomic_checkpoint_snapshot_rejects_overwrite_and_wrong_parent() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let make = |height, previous_hash| Block {
            header: BlockHeader {
                height, round: 0, previous_hash, state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: height, epoch: 0, proposer: [4;32], epoch_transition_certificate: None,
            },
            transactions: vec![], receipts: vec![],
        };
        let first = make(1, [0;32]);
        store.put_block(&first).unwrap();
        let original_hash = store.latest_hash().unwrap();
        let mut candidate = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![]
        });
        candidate.height = 1;
        candidate.last_block_hash = block_hash(&first);

        let invalid = store.put_block_and_snapshot(
            &first,
            &candidate,
            freedom_types::FinalityCertificate {
                block_hash: block_hash(&first), height: 1, round: 0, precommits: Vec::new()
            },
            std::collections::BTreeMap::new(),
        );
        assert!(matches!(invalid, Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), original_hash);
        assert_eq!(store.get_block(1).unwrap(), Some(first));
        assert!(store.load_snapshot(1).unwrap().is_none());

        let wrong_parent = make(2, [9;32]);
        let mut next_state = candidate.clone();
        next_state.height = 2;
        next_state.last_block_hash = block_hash(&wrong_parent);
        let invalid = store.put_block_and_snapshot(
            &wrong_parent,
            &next_state,
            freedom_types::FinalityCertificate {
                block_hash: block_hash(&wrong_parent), height: 2, round: 0, precommits: Vec::new()
            },
            std::collections::BTreeMap::new(),
        );
        assert!(matches!(invalid, Err(StorageError::InvalidMetadata)));
        assert_eq!(store.latest_height().unwrap(), 1);
        assert_eq!(store.latest_hash().unwrap(), original_hash);
        assert!(store.get_block(2).unwrap().is_none());
    }

    #[test]
    fn rejects_snapshot_with_orphaned_bridge_withdrawal_rate_limit_period() {
        let dir = tempdir().unwrap();
        let store = ChainStore::open(dir.path()).unwrap();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis {
            chain_id: CHAIN_ID, initial_supply: 0, validators: vec![]
        });
        state.bridge_withdrawal_rate_limit_period.insert([9u8; 32], 0);
        let snapshot = StateSnapshot {
            chain_id: u64::from(CHAIN_ID),
            height: 0,
            block_hash: [0; 32],
            state_root: state.state_root(),
            state,
            certificate: None,
            finality_validator_set: std::collections::BTreeMap::new(),
            epoch_transition_certificate: None,
        };
        store.db.put(ChainStore::snapshot_key(0), encode_snapshot(&snapshot).unwrap()).unwrap();

        let err = store.load_snapshot(0).unwrap_err();
        assert!(matches!(err, StorageError::InvalidSnapshot(message) if message.contains("persisted bridge state integrity failure")));
    }

}
