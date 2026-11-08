//! Integration tests compile `freedom-types` as a normal dependency.
//! Unlike unit tests inside the crate, the library itself is built without
//! `cfg(test)`, so these assertions exercise the production PoW limit.

use freedom_types::{
    bitcoin_target_from_bits, BitcoinBlockHeader, BitcoinBlockHeaderVerification,
};

#[test]
fn production_target_parser_rejects_regtest_limit() {
    assert!(bitcoin_target_from_bits(0x207f_ffff).is_none());
    assert!(bitcoin_target_from_bits(0x1d00_ffff).is_some());
}

#[test]
fn production_header_parser_rejects_regtest_limit() {
    let header = BitcoinBlockHeader {
        version: 1,
        previous_block_hash: [0; 32],
        merkle_root: [1; 32],
        timestamp: 1,
        bits: 0x207f_ffff,
        nonce: 0,
    };

    assert!(BitcoinBlockHeader::target_from_bits(header.bits).is_none());
    assert!(!header.validate_pow());
    assert!(BitcoinBlockHeaderVerification::verify(&header).is_err());
}
