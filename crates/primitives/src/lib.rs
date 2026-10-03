use serde::{Deserialize, Serialize};

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

pub const CHAIN_ID: u32 = 1;
pub const GENESIS_SUPPLY: u128 = 0;
pub const FDM_SCALE: u128 = 100_000;

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
