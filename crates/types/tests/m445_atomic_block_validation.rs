use freedom_types::{
    BitcoinBlockHeader, BitcoinSpendScanBlock, BitcoinUtxoBlockCommitment,
    BitcoinUtxoSmtSnapshot, BitcoinUtxoStateEntry,
};

fn snapshot() -> BitcoinUtxoSmtSnapshot {
    BitcoinUtxoSmtSnapshot {
        entries: vec![BitcoinUtxoStateEntry {
            tx_hash: [7u8; 32],
            vout: 0,
            amount_sat: 1_000,
            script_pubkey: vec![0x51],
        }],
    }
}

fn candidate(raw_transactions: Vec<Vec<u8>>) -> BitcoinSpendScanBlock {
    BitcoinSpendScanBlock {
        height: 1,
        header: BitcoinBlockHeader {
            version: 1,
            previous_block_hash: [1u8; 32],
            merkle_root: [0u8; 32],
            timestamp: 1,
            bits: 0,
            nonce: 0,
        },
        raw_transactions,
    }
}

#[test]
fn m445_compatibility_builder_failure_does_not_mutate_snapshot() {
    let previous = snapshot();
    let before = previous.clone();

    let result = BitcoinUtxoBlockCommitment::build(
        &previous,
        &candidate(vec![vec![0u8; 4]]),
    );

    assert!(result.is_err());
    assert_eq!(previous, before);
}

#[test]
fn m445_script_validated_builder_failure_does_not_mutate_snapshot() {
    let previous = snapshot();
    let before = previous.clone();

    let result = BitcoinUtxoBlockCommitment::build_with_script_flags(
        &previous,
        &candidate(vec![vec![0u8; 4]]),
        0,
    );

    assert!(result.is_err());
    assert_eq!(previous, before);
}

#[test]
fn m445_empty_candidate_is_rejected_without_mutating_snapshot() {
    let previous = snapshot();
    let before = previous.clone();

    let result = BitcoinUtxoBlockCommitment::build_with_script_flags(
        &previous,
        &candidate(Vec::new()),
        0,
    );

    assert!(result.is_err());
    assert_eq!(previous, before);
}
