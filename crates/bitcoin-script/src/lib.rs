//! Bitcoin Script execution validation backed by Bitcoin Core's consensus
//! implementation through libbitcoinconsensus.
//!
//! This crate validates an input against the exact previous-output script and
//! value supplied by the caller. It does not establish that the supplied
//! previous output exists, is unspent, or belongs to the canonical Bitcoin
//! chain; those checks must be performed by the caller before invoking this
//! verifier.

/// Consensus verification failure for a specific input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitcoinScriptVerificationError {
    /// The requested input index does not fit the verifier's index type.
    InputIndexOutOfRange,
    /// Bitcoin Core's consensus script engine rejected the input.
    ScriptRejected,
}

/// Verify one input using Bitcoin Core consensus script rules.
///
/// raw_transaction must be the complete serialized spending transaction.
/// The previous-output script and value must come from authenticated UTXO
/// state, not from untrusted transaction metadata. flags must be chosen
/// deliberately for the relevant Bitcoin network.
///
/// This does not verify UTXO existence, transaction inclusion, block validity,
/// or chain reorganization safety.
pub fn verify_input(
    previous_output_script_pubkey: &[u8],
    previous_output_value_sats: u64,
    raw_transaction: &[u8],
    input_index: usize,
    flags: u32,
) -> Result<(), BitcoinScriptVerificationError> {
    // libbitcoinconsensus uses an unsigned 32-bit input index.
    let input_index = u32::try_from(input_index)
        .map_err(|_| BitcoinScriptVerificationError::InputIndexOutOfRange)?;

    bitcoinconsensus::verify(
        previous_output_script_pubkey,
        previous_output_value_sats,
        raw_transaction,
        input_index as usize,
        flags,
    )
    .map_err(|_| BitcoinScriptVerificationError::ScriptRejected)
}

/// Verify one input using the consensus crate's full supported flag set.
/// The caller must supply authenticated prevout data and select the correct
/// network-specific policy for consensus activation.
pub fn verify_input_all_flags(
    previous_output_script_pubkey: &[u8],
    previous_output_value_sats: u64,
    raw_transaction: &[u8],
    input_index: usize,
) -> Result<(), BitcoinScriptVerificationError> {
    verify_input(
        previous_output_script_pubkey,
        previous_output_value_sats,
        raw_transaction,
        input_index,
        bitcoinconsensus::VERIFY_ALL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_index_that_cannot_fit_consensus_api() {
        let result = verify_input(&[0x51], 0, &[], usize::MAX, 0);
        assert_eq!(result, Err(BitcoinScriptVerificationError::InputIndexOutOfRange));
    }

    #[test]
    fn rejects_malformed_transaction() {
        let result = verify_input(&[0x51], 0, &[], 0, 0);
        assert_eq!(result, Err(BitcoinScriptVerificationError::ScriptRejected));
    }
}
