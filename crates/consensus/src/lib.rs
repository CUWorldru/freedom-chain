use freedom_primitives::Hash;
use freedom_state::ChainState;
use freedom_types::Block;

#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    pub min_validators: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self { min_validators: 3 }
    }
}

pub fn validate_genesis(state: &ChainState, config: &ConsensusConfig) -> Result<(), String> {
    if state.validators.len() < config.min_validators {
        return Err(format!(
            "M0 requires at least {} validators",
            config.min_validators
        ));
    }
    if state.supply.total != 0 {
        return Err("genesis supply must be zero".into());
    }
    Ok(())
}

pub fn block_hash(block: &Block) -> Hash {
    let bytes = format!("{:?}", block).into_bytes();
    *blake3::hash(&bytes).as_bytes()
}
