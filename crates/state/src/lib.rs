use std::collections::BTreeMap;
use freedom_primitives::{Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, Validator};

#[derive(Clone, Debug)]
pub struct Account {
    pub address: Address,
    pub nonce: u64,
    pub balance: u128,
    pub locked: u128,
    pub staked: u128,
}

#[derive(Clone, Debug)]
pub struct ChainState {
    pub height: u64,
    pub last_block_hash: Hash,
    pub accounts: BTreeMap<Address, Account>,
    pub validators: BTreeMap<Hash, Validator>,
    pub supply: Supply,
}

impl ChainState {
    pub fn genesis(genesis: &Genesis) -> Self {
        assert_eq!(genesis.initial_supply, GENESIS_SUPPLY, "Genesis FDM must be zero");

        let validators = genesis.validators
            .iter()
            .cloned()
            .map(|v| (v.validator_id, v))
            .collect();

        Self {
            height: 0,
            last_block_hash: [0u8; 32],
            accounts: BTreeMap::new(),
            validators,
            supply: Supply::genesis(),
        }
    }

    pub fn assert_invariants(&self) {
        assert!(self.supply.is_consistent());
        assert_eq!(self.supply.total, 0, "M0 must not mint FDM");
    }
}
