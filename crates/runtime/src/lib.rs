use freedom_consensus::{validate_genesis, ConsensusConfig};
use freedom_crypto::verify;
use freedom_primitives::{CHAIN_ID, Hash};
use freedom_state::{ChainState, StateError};
use freedom_types::{Genesis, Receipt, Transaction};

pub fn initialize(genesis: &Genesis) -> Result<ChainState, String> {
    if genesis.chain_id != CHAIN_ID {
        return Err("invalid chain id".into());
    }
    let state = ChainState::genesis(genesis);
    validate_genesis(&state, &ConsensusConfig::default())?;
    state.assert_invariants();
    Ok(state)
}

pub fn execute_transaction(state: &mut ChainState, tx: &Transaction) -> Result<Receipt, String> {
    if tx.chain_id != CHAIN_ID {
        return Err("invalid chain id".into());
    }

    let account = state.accounts.get(&tx.sender)
        .ok_or_else(|| "sender account not found".to_string())?;

    if tx.signature.len() != 64 {
        return Err("invalid signature length".into());
    }

    let mut signature = [0u8; 64];
    signature.copy_from_slice(&tx.signature);

    if !verify(&account.public_key, &tx.signing_bytes(), &signature) {
        return Err("invalid transaction signature".into());
    }

    let tx_hash: Hash = tx.hash();
    state.execute_transaction(tx).map_err(format_state_error)?;
    state.assert_invariants();

    Ok(Receipt { tx_hash, success: true, gas_used: 1 })
}

fn format_state_error(error: StateError) -> String {
    match error {
        StateError::AccountNotFound => "sender account not found".into(),
        StateError::InvalidNonce { expected, actual } => format!("invalid nonce: expected {expected}, got {actual}"),
        StateError::InsufficientBalance => "insufficient balance".into(),
        StateError::InvalidTransferPayload => "invalid transfer payload".into(),
        StateError::InvalidRecipient => "invalid recipient".into(),
        StateError::ArithmeticOverflow => "arithmetic overflow".into(),
        StateError::UnsupportedTransaction => "unsupported transaction".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_crypto::{generate_keypair, sign};
    use freedom_primitives::address_from_public_key;
    use freedom_state::Account;
    use freedom_types::{TransferPayload, TxKind};

    fn setup(balance: u128) -> (ChainState, ed25519_dalek::SigningKey, [u8; 32], [u8; 32]) {
        let key = generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let sender = address_from_public_key(&public_key);
        let recipient_key = generate_keypair();
        let recipient = address_from_public_key(&recipient_key.verifying_key().to_bytes());
        let mut state = ChainState::genesis(&Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators: vec![] });
        state.insert_account(Account::new(public_key, balance));
        state.supply.total = balance;
        state.supply.circulating = balance;
        (state, key, sender, recipient)
    }

    fn make_tx(key: &ed25519_dalek::SigningKey, sender: [u8; 32], recipient: [u8; 32], nonce: u64, amount: u128, fee: u128) -> Transaction {
        let mut tx = Transaction {
            chain_id: CHAIN_ID, sender, nonce, kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount }.encode(), fee, signature: vec![],
        };
        tx.signature = sign(key, &tx.signing_bytes()).to_vec();
        tx
    }

    #[test]
    fn signed_transfer_executes_once_and_preserves_supply() {
        let (mut state, key, sender, recipient) = setup(1_000);
        let tx = make_tx(&key, sender, recipient, 0, 250, 10);
        let receipt = execute_transaction(&mut state, &tx).unwrap();
        assert!(receipt.success);
        assert_eq!(state.accounts.get(&sender).unwrap().balance, 740);
        assert_eq!(state.accounts.get(&sender).unwrap().nonce, 1);
        assert_eq!(state.accounts.get(&recipient).unwrap().balance, 250);
        assert_eq!(state.supply.circulating, 990);
        assert_eq!(state.supply.treasury, 10);
        assert_eq!(state.supply.total, 1_000);
        assert!(execute_transaction(&mut state, &tx).is_err());
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let (mut state, key, sender, recipient) = setup(1_000);
        let other = generate_keypair();
        let mut tx = make_tx(&key, sender, recipient, 0, 1, 0);
        tx.signature = sign(&other, &tx.signing_bytes()).to_vec();
        assert!(execute_transaction(&mut state, &tx).is_err());
        assert_eq!(state.accounts.get(&sender).unwrap().balance, 1_000);
    }

    #[test]
    fn invalid_nonce_is_rejected() {
        let (mut state, key, sender, recipient) = setup(1_000);
        let tx = make_tx(&key, sender, recipient, 1, 1, 0);
        assert!(execute_transaction(&mut state, &tx).is_err());
    }

    #[test]
    fn insufficient_balance_is_rejected() {
        let (mut state, key, sender, recipient) = setup(100);
        let tx = make_tx(&key, sender, recipient, 0, 101, 0);
        assert!(execute_transaction(&mut state, &tx).is_err());
        assert_eq!(state.accounts.get(&sender).unwrap().balance, 100);
    }

    #[test]
    fn arithmetic_overflow_is_rejected() {
        let (mut state, key, sender, recipient) = setup(u128::MAX);
        let tx = make_tx(&key, sender, recipient, 0, u128::MAX, 1);
        assert!(execute_transaction(&mut state, &tx).is_err());
    }

    #[test]
    fn transaction_hash_changes_when_signature_changes() {
        let (mut state, key, sender, recipient) = setup(1_000);
        let tx = make_tx(&key, sender, recipient, 0, 1, 0);
        let h1 = tx.hash();
        let other = generate_keypair();
        let mut tampered = tx.clone();
        tampered.signature = sign(&other, &tampered.signing_bytes()).to_vec();
        assert_ne!(h1, tampered.hash());
        let _ = &mut state;
    }
}
