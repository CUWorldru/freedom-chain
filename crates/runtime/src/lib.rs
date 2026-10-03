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

    Ok(Receipt {
        tx_hash,
        success: true,
        gas_used: 1,
    })
}

fn format_state_error(error: StateError) -> String {
    match error {
        StateError::AccountNotFound => "sender account not found".into(),
        StateError::InvalidNonce { expected, actual } =>
            format!("invalid nonce: expected {expected}, got {actual}"),
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

    #[test]
    fn signed_transfer_executes_once() {
        let key = generate_keypair();
        let public_key = key.verifying_key().to_bytes();
        let sender = address_from_public_key(&public_key);

        let recipient_key = generate_keypair();
        let recipient = address_from_public_key(&recipient_key.verifying_key().to_bytes());

        let mut state = ChainState::genesis(&Genesis {
            chain_id: CHAIN_ID,
            initial_supply: 0,
            validators: vec![],
        });
        state.insert_account(Account::new(public_key, 1_000));
        state.supply.total = 1_000;
        state.supply.circulating = 1_000;

        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 250 }.encode(),
            fee: 10,
            signature: vec![],
        };
        tx.signature = sign(&key, &tx.signing_bytes()).to_vec();

        let receipt = execute_transaction(&mut state, &tx).unwrap();

        assert!(receipt.success);
        assert_eq!(state.accounts.get(&sender).unwrap().balance, 740);
        assert_eq!(state.accounts.get(&sender).unwrap().nonce, 1);
        assert_eq!(state.accounts.get(&recipient).unwrap().balance, 250);
        assert_eq!(state.supply.circulating, 990);
        assert_eq!(state.supply.treasury, 10);
        assert_eq!(state.supply.total, 1_000);

        let second = execute_transaction(&mut state, &tx);
        assert!(second.is_err());
    }
}
