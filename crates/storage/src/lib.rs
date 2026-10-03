use freedom_state::ChainState;
use freedom_primitives::CHAIN_ID;
use freedom_types::Block;
use rocksdb::{Options, DB};
use std::path::Path;

const BLOCK_PREFIX: &[u8] = b"block:";
const META_HEIGHT: &[u8] = b"meta:height";
const META_HASH: &[u8] = b"meta:last_hash";
const SNAPSHOT_PREFIX: &[u8] = b"snapshot:";

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
    pub epoch_transition_certificate: Option<freedom_types::ValidatorSetCertificate>,
}

pub struct ChainStore { db: DB }

impl ChainStore {
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
        let bytes = serde_json::to_vec(block).map_err(|e| StorageError::Codec(e.to_string()))?;
        self.db.put(Self::height_key(block.header.height), bytes)
            .map_err(|e| StorageError::Rocks(e.to_string()))?;
        self.db.put(META_HEIGHT, block.header.height.to_be_bytes())
            .map_err(|e| StorageError::Rocks(e.to_string()))?;
        self.db.put(META_HASH, block_hash(block))
            .map_err(|e| StorageError::Rocks(e.to_string()))?;
        Ok(())
    }

    pub fn get_block(&self, height: u64) -> Result<Option<Block>, StorageError> {
        let Some(bytes) = self.db.get(Self::height_key(height)).map_err(|e| StorageError::Rocks(e.to_string()))? else { return Ok(None); };
        serde_json::from_slice(&bytes).map(Some).map_err(|e| StorageError::Codec(e.to_string()))
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
        self.save_snapshot_with_certificate_opt(state, block_hash, None)
    }

    pub fn save_snapshot_with_certificate(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: freedom_types::FinalityCertificate,
    ) -> Result<(), StorageError> {
        self.save_snapshot_with_certificate_opt(state, block_hash, Some(certificate))
    }

    fn save_snapshot_with_certificate_opt(
        &self,
        state: &ChainState,
        block_hash: [u8;32],
        certificate: Option<freedom_types::FinalityCertificate>,
    ) -> Result<(), StorageError> {
        let snapshot = StateSnapshot {
            chain_id: CHAIN_ID,
            height: state.height,
            block_hash,
            state_root: state.state_root(),
            state: state.clone(),
            certificate,
            epoch_transition_certificate: state.validator_set_certificate.clone(),
        };
        let bytes = serde_json::to_vec(&snapshot).map_err(|e| StorageError::Codec(e.to_string()))?;
        self.db.put(Self::snapshot_key(state.height), bytes).map_err(|e| StorageError::Rocks(e.to_string()))?;
        Ok(())
    }

    pub fn load_snapshot(&self, height: u64) -> Result<Option<StateSnapshot>, StorageError> {
        let Some(bytes) = self.db.get(Self::snapshot_key(height)).map_err(|e| StorageError::Rocks(e.to_string()))? else { return Ok(None); };
        let snapshot: StateSnapshot = serde_json::from_slice(&bytes).map_err(|e| StorageError::Codec(e.to_string()))?;
        if snapshot.chain_id != CHAIN_ID {
            return Err(StorageError::InvalidSnapshot("chain id mismatch".into()));
        }
        if snapshot.height != height || snapshot.state.height != height { return Err(StorageError::InvalidSnapshot("height mismatch".into())); }
        if snapshot.state.state_root() != snapshot.state_root { return Err(StorageError::InvalidSnapshot("state root mismatch".into())); }
        let checkpoint = if snapshot.height == 0 { None } else { None };
if snapshot.epoch_transition_certificate.as_ref().map(|c| c.hash())
            != snapshot.state.validator_set_certificate.as_ref().map(|c| c.hash()) {
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
        
        Ok(Some(snapshot))
    }

    pub fn latest_snapshot(&self) -> Result<Option<StateSnapshot>, StorageError> {
        let mut best = None;
        for item in self.db.iterator(rocksdb::IteratorMode::Start) {
            let (key, _) = item.map_err(|e| StorageError::Rocks(e.to_string()))?;
            if key.starts_with(SNAPSHOT_PREFIX) && key.len() == SNAPSHOT_PREFIX.len() + 8 {
                let mut h=[0u8;8]; h.copy_from_slice(&key[SNAPSHOT_PREFIX.len()..]);
                let height=u64::from_be_bytes(h);
                if best.map(|b| height>b).unwrap_or(true) { best=Some(height); }
            }
        }
        match best { Some(h)=>self.load_snapshot(h), None=>Ok(None) }
    }
    pub fn range(&self, from: u64, to: u64) -> Result<Vec<Block>, StorageError> {
        if from > to { return Ok(Vec::new()); }
        let mut out = Vec::with_capacity((to - from + 1) as usize);
        for height in from..=to {
            match self.get_block(height)? {
                Some(block) => out.push(block),
                None => return Err(StorageError::MissingBlock(height)),
            }
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
        store.save_snapshot(&genesis, [9;32]).unwrap();
        let loaded = store.load_snapshot(0).unwrap().unwrap();
        assert_eq!(loaded.block_hash, [9;32]);
        assert_eq!(loaded.state.state_root(), loaded.state_root);
        assert!(loaded.certificate.is_none());
        assert!(loaded.epoch_transition_certificate.is_none());
    }
}
