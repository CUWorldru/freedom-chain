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

pub fn format_mfdm(amount: u128) -> String {
    let whole = amount / freedom_primitives::FDM_SCALE;
    let frac = amount % freedom_primitives::FDM_SCALE;
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
    fn mfdm_formatting_is_integer_only() {
        assert_eq!(format_mfdm(100_000), "1");
        assert_eq!(format_mfdm(10_000), "0.1");
        assert_eq!(format_mfdm(1), "0.00001");
    }
}
