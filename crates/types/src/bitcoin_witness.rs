//! Bitcoin SegWit witness-commitment verification (BIP141).
//!
//! This validates the commitment carried by a coinbase transaction against
//! the witness transaction IDs supplied as raw serialized transactions. It
//! does not validate Bitcoin Script execution or full block consensus rules.

use crate::{bitcoin_sha256d, BitcoinTransaction, BitcoinTransactionParseError, Hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitcoinWitnessCommitmentError {
    EmptyBlock,
    InvalidTransaction(BitcoinTransactionParseError),
    MissingCoinbase,
    MultipleCoinbaseTransactions,
    MissingWitnessReservedValue,
    InvalidWitnessReservedValue,
    MissingWitnessCommitment,
    WitnessCommitmentMismatch,
    MerkleTreeSizeOverflow,
}

const COMMITMENT_PREFIX: [u8; 4] = [0xaa, 0x21, 0xa9, 0xed];

fn is_coinbase(tx: &BitcoinTransaction) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].previous_txid == [0; 32]
        && tx.inputs[0].previous_vout == u32::MAX
}

fn has_witness(tx: &BitcoinTransaction) -> bool {
    tx.inputs.iter().any(|input| !input.witness.is_empty())
}

/// Computes the BIP141 witness Merkle root. The coinbase leaf is always zero;
/// all other leaves are each transaction's wtxid in the hash byte order used
/// by this crate. Odd-width levels duplicate their final hash.
pub fn bitcoin_witness_merkle_root(wtxids: &[Hash]) -> Result<Hash, BitcoinWitnessCommitmentError> {
    if wtxids.is_empty() {
        return Err(BitcoinWitnessCommitmentError::EmptyBlock);
    }
    let mut level = Vec::with_capacity(wtxids.len());
    level.push([0; 32]);
    level.extend(wtxids.iter().skip(1).copied());
    while level.len() > 1 {
        let next_len = level.len().checked_add(1)
            .ok_or(BitcoinWitnessCommitmentError::MerkleTreeSizeOverflow)? / 2;
        let mut next = Vec::with_capacity(next_len);
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { pair[1] } else { pair[0] };
            let mut preimage = [0u8; 64];
            preimage[..32].copy_from_slice(&pair[0]);
            preimage[32..].copy_from_slice(&right);
            next.push(bitcoin_sha256d(&preimage));
        }
        level = next;
    }
    Ok(level[0])
}

/// Verify a block's BIP141 witness commitment using its raw serialized
/// transactions in block order. If no transaction has witness data and no
/// commitment is present, verification succeeds. If a commitment is present,
/// the coinbase must contain exactly one 32-byte witness reserved value.
pub fn bitcoin_verify_witness_commitment(
    raw_transactions: &[Vec<u8>],
) -> Result<(), BitcoinWitnessCommitmentError> {
    if raw_transactions.is_empty() {
        return Err(BitcoinWitnessCommitmentError::EmptyBlock);
    }
    let mut txs = Vec::with_capacity(raw_transactions.len());
    for raw in raw_transactions {
        txs.push(BitcoinTransaction::parse(raw)
            .map_err(BitcoinWitnessCommitmentError::InvalidTransaction)?);
    }
    if !is_coinbase(&txs[0]) {
        return Err(BitcoinWitnessCommitmentError::MissingCoinbase);
    }
    if txs.iter().skip(1).any(is_coinbase) {
        return Err(BitcoinWitnessCommitmentError::MultipleCoinbaseTransactions);
    }

    // Per BIP141, if several matching outputs exist, the highest output index
    // is authoritative.
    let commitment = txs[0].outputs.iter().enumerate()
        .filter_map(|(index, output)| {
            let script = &output.script_pubkey;
            (script.len() >= 38
                && script[0] == 0x6a
                && script[1] == 0x24
                && script[2..6] == COMMITMENT_PREFIX)
                .then(|| (index, <[u8; 32]>::try_from(&script[6..38]).ok().unwrap()))
        })
        .max_by_key(|(index, _)| *index)
        .map(|(_, hash)| hash);

    let any_witness = txs.iter().any(has_witness);
    let Some(expected_commitment) = commitment else {
        return if any_witness {
            Err(BitcoinWitnessCommitmentError::MissingWitnessCommitment)
        } else {
            Ok(())
        };
    };

    let coinbase_witness = &txs[0].inputs[0].witness;
    if coinbase_witness.is_empty() {
        return Err(BitcoinWitnessCommitmentError::MissingWitnessReservedValue);
    }
    if coinbase_witness.len() != 1 || coinbase_witness[0].len() != 32 {
        return Err(BitcoinWitnessCommitmentError::InvalidWitnessReservedValue);
    }

    let wtxids: Vec<Hash> = txs.iter().map(|tx| tx.wtxid).collect();
    let root = bitcoin_witness_merkle_root(&wtxids)?;
    let mut preimage = [0u8; 64];
    preimage[..32].copy_from_slice(&root);
    preimage[32..].copy_from_slice(&coinbase_witness[0]);
    if bitcoin_sha256d(&preimage) != expected_commitment {
        return Err(BitcoinWitnessCommitmentError::WitnessCommitmentMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(n: usize) -> Vec<u8> {
        assert!(n < 253);
        vec![n as u8]
    }

    fn tx(coinbase: bool, witness: Option<Vec<Vec<u8>>>, outputs: Vec<Vec<u8>>) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&2i32.to_le_bytes());
        let segwit = witness.is_some();
        if segwit { raw.extend_from_slice(&[0, 1]); }
        raw.push(1);
        if coinbase {
            raw.extend_from_slice(&[0; 32]);
            raw.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            raw.extend_from_slice(&[0x11; 32]);
            raw.extend_from_slice(&0u32.to_le_bytes());
        }
        raw.push(0); // scriptSig
        raw.extend_from_slice(&u32::MAX.to_le_bytes());
        raw.push(outputs.len() as u8);
        for script in outputs {
            raw.extend_from_slice(&1u64.to_le_bytes());
            raw.extend(varint(script.len()));
            raw.extend(script);
        }
        if let Some(stack) = witness {
            raw.push(stack.len() as u8);
            for item in stack {
                raw.push(item.len() as u8);
                raw.extend(item);
            }
        }
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw
    }

    fn commitment_script(hash: Hash) -> Vec<u8> {
        let mut script = vec![0x6a, 0x24, 0xaa, 0x21, 0xa9, 0xed];
        script.extend_from_slice(&hash);
        script
    }

    fn committed_pair(reserved: Vec<u8>) -> Vec<Vec<u8>> {
        let other = tx(false, Some(vec![vec![1]]), vec![vec![0x51]]);
        let parsed_other = BitcoinTransaction::parse(&other).unwrap();
        let root = bitcoin_witness_merkle_root(&[[0; 32], parsed_other.wtxid]).unwrap();
        let mut preimage = [0u8; 64];
        preimage[..32].copy_from_slice(&root);
        preimage[32..].copy_from_slice(&reserved);
        let commitment = bitcoin_sha256d(&preimage);
        vec![
            tx(true, Some(vec![reserved]), vec![commitment_script(commitment)]),
            other,
        ]
    }

    #[test]
    fn accepts_valid_witness_commitment() {
        assert_eq!(bitcoin_verify_witness_commitment(&committed_pair(vec![0; 32])), Ok(()));
    }

    #[test]
    fn rejects_missing_commitment_when_witness_exists() {
        let block = vec![
            tx(true, None, vec![vec![0x51]]),
            tx(false, Some(vec![vec![1]]), vec![vec![0x51]]),
        ];
        assert_eq!(bitcoin_verify_witness_commitment(&block), Err(BitcoinWitnessCommitmentError::MissingWitnessCommitment));
    }

    #[test]
    fn rejects_mutated_witness_commitment() {
        let mut block = committed_pair(vec![0; 32]);
        let last = block[0].len() - 5;
        block[0][last] ^= 1;
        assert_eq!(bitcoin_verify_witness_commitment(&block), Err(BitcoinWitnessCommitmentError::WitnessCommitmentMismatch));
    }

    #[test]
    fn rejects_wrong_reserved_value_length() {
        let block = committed_pair(vec![0; 31]);
        assert_eq!(bitcoin_verify_witness_commitment(&block), Err(BitcoinWitnessCommitmentError::InvalidTransaction(BitcoinTransactionParseError::UnexpectedEof)));
    }

    #[test]
    fn allows_legacy_block_without_witness_or_commitment() {
        let block = vec![tx(true, None, vec![vec![0x51]])];
        assert_eq!(bitcoin_verify_witness_commitment(&block), Ok(()));
    }
}
