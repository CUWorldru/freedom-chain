use freedom_primitives::{address_from_public_key, Address, Hash};
use freedom_crypto::generate_keypair;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wallet {
    pub address: Address,
    pub public_key: [u8; 32],
}

pub fn create_wallet() -> Wallet {
    let key = generate_keypair();
    let public_key = key.verifying_key().to_bytes();
    Wallet { address: address_from_public_key(&public_key), public_key }
}

pub fn wallet_id(address: &Address) -> Hash {
    freedom_primitives::hash_bytes(&[b"FREEDOM-WALLET-V1".as_slice(), address].concat())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletSnapshot {
    pub address: Address,
    pub balance_321m_units: u128,
    pub balance_321m: String,
}

pub fn snapshot(state: &freedom_state::ChainState, address: &Address) -> Option<WalletSnapshot> {
    let account = state.accounts.get(address)?;
    Some(WalletSnapshot { address:*address, balance_321m_units:account.balance, balance_321m:format_321m(account.balance) })
}

pub fn history(state: &freedom_state::ChainState, address: &Address) -> Vec<freedom_state::LedgerEntry> {
    state.ledger.iter().filter(|e| &e.sender == address).cloned().collect()
}

pub fn format_321m(amount: u128) -> String {
    let whole = amount / freedom_primitives::NATIVE_ASSET_SCALE;
    let frac = amount % freedom_primitives::NATIVE_ASSET_SCALE;
    if frac == 0 { return whole.to_string(); }
    format!("{whole}.{:05}", frac).trim_end_matches('0').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wallet_address_is_deterministic_from_public_key() {
        let w = create_wallet();
        assert_eq!(w.address, address_from_public_key(&w.public_key));
        assert_ne!(wallet_id(&w.address), [0u8;32]);
    }

    #[test]
    fn snapshot_reads_chain_balance() {
        let w = create_wallet();
        let mut state = freedom_state::ChainState::genesis(&freedom_types::Genesis { chain_id: freedom_primitives::CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.accounts.insert(w.address, freedom_state::Account::new(w.public_key, 12345));
        let s = snapshot(&state, &w.address).unwrap();
        assert_eq!(s.balance_321m_units, 12345);
        assert_eq!(s.balance_321m, "0.12345");
    }

    #[test]
    fn native_asset_formatting_is_integer_only() {
        assert_eq!(format_321m(100_000), "1");
        assert_eq!(format_321m(10_000), "0.1");
        assert_eq!(format_321m(1), "0.00001");
    }
#[deprecated(note="use format_321m; legacy FDM unit name retained for compatibility")]
pub fn format_mfdm(amount:u128)->String{format_321m(amount)}

}
