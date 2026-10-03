use serde::{Deserialize, Serialize};

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

pub const CHAIN_ID: u32 = 1;
pub const GENESIS_SUPPLY: u128 = 0;

/// User-facing network identity. Consensus and chain ID are intentionally unchanged.
pub const NETWORK_NAME: &str = "321MORE Network";
pub const NATIVE_ASSET_SYMBOL: &str = "321M";
pub const GAS_ASSET_SYMBOL: &str = NATIVE_ASSET_SYMBOL;
pub const NATIVE_ASSET_SCALE: u128 = 100_000;

pub fn hash_bytes(data: &[u8]) -> Hash {
    *blake3::hash(data).as_bytes()
}

pub fn address_from_public_key(public_key: &[u8]) -> Address {
    let mut input = b"FREEDOM-ADDRESS-V1".to_vec();
    input.extend_from_slice(public_key);
    hash_bytes(&input)
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Supply {
    pub total: u128,
    pub circulating: u128,
    pub locked: u128,
    pub staked: u128,
    pub treasury: u128,
}

impl Supply {
    pub fn genesis() -> Self {
        Self::default()
    }

    pub fn is_consistent(&self) -> bool {
        self.total == self.circulating + self.locked + self.staked + self.treasury
    }
}


pub const STATE_LEAF_DOMAIN: &[u8] = b"FREEDOM-STATE-LEAF-V1";
pub const STATE_ROOT_DOMAIN: &[u8] = b"FREEDOM-STATE-ROOT-V1";
pub const MERKLE_NODE_DOMAIN: &[u8] = b"FREEDOM-MERKLE-NODE-V1";

pub fn domain_hash(domain: &[u8], parts: &[&[u8]]) -> Hash {
    let mut input = Vec::with_capacity(domain.len() + parts.iter().map(|p| p.len()).sum::<usize>());
    input.extend_from_slice(domain);
    for part in parts { input.extend_from_slice(part); }
    hash_bytes(&input)
}

pub fn merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() { return domain_hash(STATE_ROOT_DOMAIN, &[b"EMPTY"]); }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        let mut i = 0;
        while i < level.len() {
            let left = level[i];
            let right = if i + 1 < level.len() { level[i + 1] } else { left };
            next.push(domain_hash(MERKLE_NODE_DOMAIN, &[&left, &right]));
            i += 2;
        }
        level = next;
    }
    domain_hash(STATE_ROOT_DOMAIN, &[&level[0]])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof { pub index: usize, pub siblings: Vec<Hash> }

pub fn merkle_proof(leaves: &[Hash], index: usize) -> Option<MerkleProof> {
    if index >= leaves.len() { return None; }
    let mut idx = index;
    let mut level = leaves.to_vec();
    let mut siblings = Vec::new();
    while level.len() > 1 {
        let sibling = if idx % 2 == 0 {
            if idx + 1 < level.len() { level[idx + 1] } else { level[idx] }
        } else { level[idx - 1] };
        siblings.push(sibling);
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        let mut i = 0;
        while i < level.len() {
            let left = level[i];
            let right = if i + 1 < level.len() { level[i + 1] } else { left };
            next.push(domain_hash(MERKLE_NODE_DOMAIN, &[&left, &right]));
            i += 2;
        }
        idx /= 2;
        level = next;
    }
    Some(MerkleProof { index, siblings })
}

pub fn verify_merkle_proof(root: Hash, leaf: Hash, proof: &MerkleProof) -> bool {
    let mut hash = leaf;
    let mut idx = proof.index;
    for sibling in &proof.siblings {
        hash = if idx % 2 == 0 {
            domain_hash(MERKLE_NODE_DOMAIN, &[&hash, sibling])
        } else {
            domain_hash(MERKLE_NODE_DOMAIN, &[sibling, &hash])
        };
        idx /= 2;
    }
    root == domain_hash(STATE_ROOT_DOMAIN, &[&hash])
}
