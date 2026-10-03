use freedom_consensus::block_hash;
use freedom_crypto::{sign, verify};
use freedom_primitives::{Address, Hash};
use freedom_state::ChainState;
use freedom_types::{BlockProposal, ValidatorVote};
use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify, ping,
    swarm::NetworkBehaviour,
    PeerId, Swarm, SwarmBuilder,
};
use serde::{Deserialize, Serialize};

pub const NETWORK_PROTOCOL: &str = "/freedom/1";
pub const PROPOSAL_TOPIC: &str = "freedom/1/proposals";
pub const VOTE_TOPIC: &str = "freedom/1/votes";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMessage {
    Proposal(BlockProposal),
    Vote(ValidatorVote),
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "FreedomEvent")]
pub struct FreedomBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
}

pub enum FreedomEvent {
    Gossipsub(gossipsub::Event),
    Identify(identify::Event),
    Ping(ping::Event),
}

impl From<gossipsub::Event> for FreedomEvent {
    fn from(event: gossipsub::Event) -> Self { Self::Gossipsub(event) }
}
impl From<identify::Event> for FreedomEvent {
    fn from(event: identify::Event) -> Self { Self::Identify(event) }
}
impl From<ping::Event> for FreedomEvent {
    fn from(event: ping::Event) -> Self { Self::Ping(event) }
}

pub fn sign_proposal(proposal: &mut BlockProposal, consensus_key: &ed25519_dalek::SigningKey) {
    proposal.signature = sign(consensus_key, &proposal.signing_bytes()).to_vec();
}

pub fn validate_proposal(state: &ChainState, proposal: &BlockProposal) -> Result<(), String> {
    if proposal.height != proposal.block.header.height { return Err("proposal height mismatch".into()); }
    if proposal.proposer != proposal.block.header.proposer { return Err("proposal proposer mismatch".into()); }
    if block_hash(&proposal.block) != proposal.block_hash { return Err("proposal block hash mismatch".into()); }
    let validator = state.validators.values().find(|v| v.owner == proposal.proposer)
        .ok_or_else(|| "unknown proposal proposer".to_string())?;
    if proposal.signature.len() != 64 { return Err("invalid proposal signature length".into()); }
    let mut sig = [0u8;64]; sig.copy_from_slice(&proposal.signature);
    if !verify(&validator.consensus_key, &proposal.signing_bytes(), &sig) {
        return Err("invalid proposal signature".into());
    }
    Ok(())
}

pub fn make_vote(state: &ChainState, validator_id: Hash, proposal: &BlockProposal, consensus_key: &ed25519_dalek::SigningKey) -> Result<ValidatorVote, String> {
    validate_proposal(state, proposal)?;
    if !state.validators.contains_key(&validator_id) { return Err("unknown voting validator".into()); }
    let mut vote = ValidatorVote {
        block_hash: proposal.block_hash,
        height: proposal.height,
        validator_id,
        signature: Vec::new(),
    };
    vote.signature = sign(consensus_key, &vote.signing_bytes()).to_vec();
    Ok(vote)
}

pub fn proposal_topic() -> IdentTopic {
    IdentTopic::new(PROPOSAL_TOPIC)
}

pub fn vote_topic() -> IdentTopic {
    IdentTopic::new(VOTE_TOPIC)
}

pub fn build_swarm() -> Result<Swarm<FreedomBehaviour>, Box<dyn std::error::Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let mut config = gossipsub::ConfigBuilder::default();
            config.validation_mode(ValidationMode::Strict);
            let config = config.build().map_err(|e| format!("gossipsub config: {e}"))?;
            let mut gossipsub = gossipsub::Behaviour::new(
                MessageAuthenticity::Signed(key.clone()),
                config,
            ).map_err(|e| format!("gossipsub init: {e}"))?;
            gossipsub.subscribe(&proposal_topic())?;
            gossipsub.subscribe(&vote_topic())?;
            Ok(FreedomBehaviour {
                gossipsub,
                identify: identify::Behaviour::new(
                    identify::Config::new(NETWORK_PROTOCOL.to_string(), key.public()),
                ),
                ping: ping::Behaviour::default(),
            })
        })?
        .build();

    Ok(swarm)
}

pub fn local_peer_id(swarm: &Swarm<FreedomBehaviour>) -> PeerId {
    *swarm.local_peer_id()
}

pub fn encode_message(message: &NetworkMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(message)
}

pub fn decode_message(bytes: &[u8]) -> Result<NetworkMessage, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn publish(
    swarm: &mut Swarm<FreedomBehaviour>,
    message: &NetworkMessage,
) -> Result<gossipsub::MessageId, Box<dyn std::error::Error>> {
    let (topic, bytes) = match message {
        NetworkMessage::Proposal(_) => (proposal_topic(), encode_message(message)?),
        NetworkMessage::Vote(_) => (vote_topic(), encode_message(message)?),
    };
    Ok(swarm.behaviour_mut().gossipsub.publish(topic, bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use freedom_primitives::hash_bytes;

    #[test]
    fn network_messages_round_trip() {
        let block = freedom_types::Block {
            header: freedom_types::BlockHeader {
                height: 7, previous_hash: [0;32], state_root: [1;32], tx_root: [2;32],
                receipts_root: [3;32], timestamp: 7, epoch: 7, proposer: [3;32],
            },
            transactions: vec![],
            receipts: vec![],
        };
        let mut proposal = BlockProposal {
            block_hash: hash_bytes(b"block"),
            height: 7,
            proposer: [3; 32],
            block,
            signature: Vec::new(),
        };
        let bytes = encode_message(&NetworkMessage::Proposal(proposal.clone())).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        assert_eq!(decoded, NetworkMessage::Proposal(proposal));
    }

    #[test]
    fn swarm_has_authenticated_peer_identity() {
        let swarm = build_swarm().unwrap();
        assert!(!local_peer_id(&swarm).to_bytes().is_empty());
    }
}
