use ed25519_dalek::SigningKey;
use freedom_primitives::{address_from_public_key, hash_bytes, Supply, CHAIN_ID};
use freedom_runtime::{build_next_block, execute_block, initialize, Mempool};
use freedom_state::{Account, ChainState};
use freedom_storage::ChainStore;
use freedom_types::{Genesis, Transaction, TxKind, TransferPayload, Validator};

fn genesis() -> (Genesis, Vec<SigningKey>) {
    let keys: Vec<SigningKey> = (1u8..=3)
        .map(|i| SigningKey::from_bytes(&[i; 32]))
        .collect();
    let validators = keys.iter().map(|key| {
        let public_key = key.verifying_key().to_bytes();
        Validator {
            validator_id: hash_bytes(&public_key),
            owner: address_from_public_key(&public_key),
            consensus_key: public_key,
            stake: 0,
            unbonding_stake: 0,
            blocks_proposed: 0,
            blocks_finalized: 0,
            missed_votes: 0,
        }
    }).collect();
    (Genesis { chain_id: CHAIN_ID, initial_supply: 0, validators }, keys)
}

fn proposer(state: &ChainState, keys: &[SigningKey]) -> [u8; 32] {
    let (_, validator) = state.active_validator_set.iter().next().unwrap();
    assert!(keys.iter().any(|key| key.verifying_key().to_bytes() == validator.consensus_key));
    validator.owner
}

#[test]
fn signed_transfer_survives_restart_and_recovered_chain_continues() {
    let dir = tempfile::tempdir().unwrap();
    let (genesis_config, validator_keys) = genesis();
    let mut state = initialize(&genesis_config).unwrap();

    let sender_key = SigningKey::from_bytes(&[42; 32]);
    let sender_public = sender_key.verifying_key().to_bytes();
    let sender = address_from_public_key(&sender_public);
    let recipient_key = SigningKey::from_bytes(&[43; 32]);
    let recipient = address_from_public_key(&recipient_key.verifying_key().to_bytes());

    state.accounts.insert(sender, Account::new(sender_public, 1_000));
    state.supply = Supply { total: 1_000, circulating: 1_000, locked: 0, staked: 0, treasury: 0 };
    state.assert_invariants();

    {
        let store = ChainStore::open(dir.path()).unwrap();
        store.save_snapshot(&state, [0; 32]).unwrap();

        let mut tx = Transaction {
            chain_id: CHAIN_ID,
            sender,
            nonce: 0,
            kind: TxKind::Transfer,
            payload: TransferPayload { to: recipient, amount: 250 }.encode(),
            fee: 0,
            signature: Vec::new(),
        };
        tx.signature = freedom_crypto::sign(&sender_key, &tx.signing_bytes()).to_vec();

        let mut mempool = Mempool::new();
        mempool.submit(&state, tx).unwrap();

        let owner = proposer(&state, &validator_keys);
        let block = build_next_block(&state, &mempool, owner, 1, state.epoch, 0, 10).unwrap();
        execute_block(&mut state, &block).unwrap();
        store.put_block(&block).unwrap();

        assert_eq!(state.accounts.get(&sender).unwrap().balance, 750);
        assert_eq!(state.accounts.get(&recipient).unwrap().balance, 250);
        assert_eq!(state.accounts.get(&sender).unwrap().nonce, 1);

        let empty_pool = Mempool::new();
        let block_two = build_next_block(&state, &empty_pool, owner, 2, state.epoch, 0, 10).unwrap();
        execute_block(&mut state, &block_two).unwrap();
        store.put_block(&block_two).unwrap();
    }

    let store = ChainStore::open(dir.path()).unwrap();
    let stored_height = store.latest_height().unwrap();
    let snapshot = store.latest_snapshot_at_or_below(stored_height).unwrap().unwrap();
    let mut recovered = snapshot.state;
    let suffix = store.range(snapshot.height + 1, stored_height).unwrap();
    for block in &suffix {
        execute_block(&mut recovered, block).unwrap();
    }

    assert_eq!(recovered.height, 2);
    assert_eq!(recovered.accounts.get(&sender).unwrap().balance, 750);
    assert_eq!(recovered.accounts.get(&recipient).unwrap().balance, 250);
    assert_eq!(recovered.accounts.get(&sender).unwrap().nonce, 1);
    assert_eq!(recovered.state_root(), state.state_root());
    assert_eq!(recovered.last_block_hash, store.latest_hash().unwrap());

    let owner = proposer(&recovered, &validator_keys);
    let next = build_next_block(&recovered, &Mempool::new(), owner, 3, recovered.epoch, 0, 10).unwrap();
    execute_block(&mut recovered, &next).unwrap();
    store.put_block(&next).unwrap();
    assert_eq!(store.latest_height().unwrap(), 3);
    store.validate_chain_through(3).unwrap();
}
