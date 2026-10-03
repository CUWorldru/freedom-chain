use freedom_consensus::{validate_genesis, ConsensusConfig};
use freedom_state::ChainState;
use freedom_types::Genesis;

pub fn initialize(genesis: &Genesis) -> Result<ChainState, String> {
    let state = ChainState::genesis(genesis);
    validate_genesis(&state, &ConsensusConfig::default())?;
    state.assert_invariants();
    Ok(state)
}
