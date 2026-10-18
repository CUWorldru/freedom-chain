//! Bitcoin Script execution validation backed by Bitcoin Core's consensus
//! implementation through libbitcoinconsensus.
//!
//! This crate validates an input against authenticated previous-output data.
//! It does not establish that the previous outputs exist, remain unspent, or
//! belong to Bitcoin's canonical chain; those checks must be performed by the
//! caller before invoking this verifier.

/// Consensus verification failure for a specific input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitcoinScriptVerificationError {
    /// The requested input index does not fit the verifier's index type.
    InputIndexOutOfRange,
    /// Taproot validation requires authenticated previous-output data for all inputs.
    MissingSpentOutputs,
    /// The selected input has no corresponding previous output.
    MissingInputPrevout,
    /// A previous-output script is too large for libbitcoinconsensus' API.
    PrevoutScriptTooLong,
    /// A previous-output value does not fit Bitcoin Core's signed amount type.
    PrevoutValueOutOfRange,
    /// Bitcoin Core's consensus script engine rejected the input.
    ScriptRejected,
}

/// Authenticated previous-output data, in the same order as transaction inputs.
///
/// The caller is responsible for ensuring every entry corresponds to the
/// respective input's actual outpoint and is sourced from authenticated UTXO
/// state. This type is borrowed to keep script buffers alive while the FFI
/// verifier reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitcoinPrevout<'a> {
    pub script_pubkey: &'a [u8],
    pub value_sats: u64,
}

/// Verify one input using pre-Taproot Bitcoin Core consensus script rules.
///
/// `raw_transaction` must be the complete serialized spending transaction.
/// The previous-output script and value must come from authenticated UTXO
/// state, not from untrusted transaction metadata. `flags` must be chosen
/// deliberately for the relevant Bitcoin network.
///
/// Taproot flags are rejected here because Taproot verification requires the
/// previous outputs for every transaction input. Use
/// `verify_input_with_prevouts` when those data are available.
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
    if flags & bitcoinconsensus::VERIFY_TAPROOT != 0 {
        return Err(BitcoinScriptVerificationError::MissingSpentOutputs);
    }

    // Bound the index to a stable, conservative API range before FFI.
    let input_index = u32::try_from(input_index)
        .map_err(|_| BitcoinScriptVerificationError::InputIndexOutOfRange)?;

    bitcoinconsensus::verify_with_flags(
        previous_output_script_pubkey,
        previous_output_value_sats,
        raw_transaction,
        None,
        input_index as usize,
        flags,
    )
    .map_err(|_| BitcoinScriptVerificationError::ScriptRejected)
}

/// Verify one input with Bitcoin Core's full pre-Taproot and Taproot flag set.
///
/// `spent_outputs` must contain authenticated previous outputs for every input
/// in `raw_transaction`, ordered exactly as the transaction inputs. The
/// selected input's script and value are taken from that slice so they cannot
/// accidentally disagree with the entry passed to the Taproot verifier.
pub fn verify_input_all_flags(
    raw_transaction: &[u8],
    input_index: usize,
    spent_outputs: &[BitcoinPrevout<'_>],
) -> Result<(), BitcoinScriptVerificationError> {
    let input_index_u32 = u32::try_from(input_index)
        .map_err(|_| BitcoinScriptVerificationError::InputIndexOutOfRange)?;
    let selected = spent_outputs
        .get(input_index)
        .ok_or(BitcoinScriptVerificationError::MissingInputPrevout)?;

    let mut utxos = Vec::with_capacity(spent_outputs.len());
    for prevout in spent_outputs {
        let script_pubkey_len = u32::try_from(prevout.script_pubkey.len())
            .map_err(|_| BitcoinScriptVerificationError::PrevoutScriptTooLong)?;
        let value = i64::try_from(prevout.value_sats)
            .map_err(|_| BitcoinScriptVerificationError::PrevoutValueOutOfRange)?;
        utxos.push(bitcoinconsensus::Utxo {
            script_pubkey: prevout.script_pubkey.as_ptr(),
            script_pubkey_len,
            value,
        });
    }

    bitcoinconsensus::verify_with_flags(
        selected.script_pubkey,
        selected.value_sats,
        raw_transaction,
        Some(&utxos),
        input_index_u32 as usize,
        bitcoinconsensus::VERIFY_ALL_PRE_TAPROOT | bitcoinconsensus::VERIFY_TAPROOT,
    )
    .map_err(|_| BitcoinScriptVerificationError::ScriptRejected)
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

    #[test]
    fn refuses_taproot_flags_without_all_spent_outputs() {
        let result = verify_input(
            &[0x51],
            0,
            &[],
            0,
            bitcoinconsensus::VERIFY_TAPROOT,
        );
        assert_eq!(result, Err(BitcoinScriptVerificationError::MissingSpentOutputs));
    }

    #[test]
    fn full_flag_verifier_rejects_missing_input_prevout() {
        let result = verify_input_all_flags(&[], 0, &[]);
        assert_eq!(result, Err(BitcoinScriptVerificationError::MissingInputPrevout));
    }

    #[test]
    fn full_flag_verifier_rejects_index_that_cannot_fit_consensus_api() {
        let prevout = BitcoinPrevout {
            script_pubkey: &[0x51],
            value_sats: 0,
        };
        let result = verify_input_all_flags(&[], usize::MAX, &[prevout]);
        assert_eq!(result, Err(BitcoinScriptVerificationError::InputIndexOutOfRange));
    }
}
