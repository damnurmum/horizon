//! Per-connection 26.2 protocol translator using the network mod API.
use horizon_mod_sdk::{Mod, NetworkDirection, NetworkPhase, Registration};
use horizon_protocol::{Direction, State, connection::PacketTranslator};

struct VersionBridge;

impl Mod for VersionBridge {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self, Registration::new()))
    }

    fn network_start(_address: &str, target_version: Option<&str>) -> Result<Option<Self>, String> {
        Ok((target_version == Some("26.2")).then_some(Self))
    }

    fn network_packet(
        &mut self,
        phase: NetworkPhase,
        direction: NetworkDirection,
        body: Vec<u8>,
    ) -> Result<Vec<Vec<u8>>, String> {
        let phase = match phase {
            NetworkPhase::Handshake => State::Handshake,
            NetworkPhase::Status => State::Status,
            NetworkPhase::Login => State::Login,
            NetworkPhase::Configuration => State::Configuration,
            NetworkPhase::Play => State::Play,
        };
        let direction = match direction {
            NetworkDirection::Clientbound => Direction::Clientbound,
            NetworkDirection::Serverbound => Direction::Serverbound,
        };
        horizon_protocol::translate_26_2::Translator
            .translate(phase, direction, &body)
            .map_err(|e| e.to_string())
    }
}

horizon_mod_sdk::export_mod!(VersionBridge);
