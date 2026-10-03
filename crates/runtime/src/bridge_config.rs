use freedom_primitives::Hash;
use freedom_types::{
    AssetStatus, AssetType, CanonicalAssetProfile, CrossChainAssetKind, ExternalAssetConfig,
    BITCOIN_MAINNET_CHAIN_ID, ETHEREUM_MAINNET_CHAIN_ID, TRON_MAINNET_CHAIN_ID,
};

pub const ETH_USDT_CONTRACT: [u8; 20] = [
    0xda, 0xc1, 0x7f, 0x95, 0x8d, 0x2e, 0xe5, 0x23, 0xa2, 0x20,
    0x62, 0x06, 0x99, 0x45, 0x97, 0xc1, 0x3d, 0x83, 0x1e, 0xc7,
];

pub const ETH_USDC_CONTRACT: [u8; 20] = [
    0xa0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1,
    0x9d, 0x4a, 0x2e, 0x9e, 0xb0, 0xce, 0x36, 0x06, 0xeb, 0x48,
];

// Canonical 20-byte TRC20 contract identity: the 20-byte payload of
// the Base58Check address TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t
// (the leading 0x41 network byte is not part of the contract identity).
pub const TRON_USDT_CONTRACT: [u8; 20] = [
    0xa6, 0x14, 0xf8, 0x03, 0xb6, 0xfd, 0x78, 0x09, 0x86, 0xa4,
    0x2c, 0x78, 0xec, 0x9c, 0x7f, 0x77, 0xe6, 0xde, 0xd1, 0x3c,
];

fn config(
    profile: CanonicalAssetProfile,
    source_chain_id: u32,
    asset_kind: CrossChainAssetKind,
    asset_type: AssetType,
    contract_address: Vec<u8>,
    issuer: Hash,
    bridge_id: Hash,
) -> ExternalAssetConfig {
    ExternalAssetConfig {
        profile,
        source_chain_id,
        asset_kind,
        asset_type,
        contract_address,
        issuer,
        bridge_id,
        status: AssetStatus::Active,
    }
}

pub fn bitcoin_mainnet(issuer: Hash, bridge_id: Hash) -> ExternalAssetConfig {
    config(
        CanonicalAssetProfile::Btc,
        BITCOIN_MAINNET_CHAIN_ID,
        CrossChainAssetKind::BitcoinUtxo,
        AssetType::Native,
        b"BTC".to_vec(),
        issuer,
        bridge_id,
    )
}

pub fn ethereum_mainnet(issuer: Hash, bridge_id: Hash) -> ExternalAssetConfig {
    config(
        CanonicalAssetProfile::Eth,
        ETHEREUM_MAINNET_CHAIN_ID,
        CrossChainAssetKind::EthereumNative,
        AssetType::Native,
        b"ETH".to_vec(),
        issuer,
        bridge_id,
    )
}

pub fn ethereum_usdt(issuer: Hash, bridge_id: Hash) -> ExternalAssetConfig {
    config(
        CanonicalAssetProfile::Usdt,
        ETHEREUM_MAINNET_CHAIN_ID,
        CrossChainAssetKind::Erc20,
        AssetType::Bridged,
        ETH_USDT_CONTRACT.to_vec(),
        issuer,
        bridge_id,
    )
}

pub fn tron_usdt(issuer: Hash, bridge_id: Hash) -> ExternalAssetConfig {
    config(
        CanonicalAssetProfile::Usdt,
        TRON_MAINNET_CHAIN_ID,
        CrossChainAssetKind::Trc20,
        AssetType::Bridged,
        TRON_USDT_CONTRACT.to_vec(),
        issuer,
        bridge_id,
    )
}

pub fn ethereum_usdc(issuer: Hash, bridge_id: Hash) -> ExternalAssetConfig {
    config(
        CanonicalAssetProfile::Usdc,
        ETHEREUM_MAINNET_CHAIN_ID,
        CrossChainAssetKind::Erc20,
        AssetType::Bridged,
        ETH_USDC_CONTRACT.to_vec(),
        issuer,
        bridge_id,
    )
}

pub fn production_profiles(issuer: Hash, bridge_id: Hash) -> Vec<ExternalAssetConfig> {
    vec![
        bitcoin_mainnet(issuer, bridge_id),
        ethereum_mainnet(issuer, bridge_id),
        ethereum_usdt(issuer, bridge_id),
        tron_usdt(issuer, bridge_id),
        ethereum_usdc(issuer, bridge_id),
    ]
}
