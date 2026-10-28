//! Per-connection 26.1/26.2 protocol translator using the network mod API.
use horizon_mod_sdk::{Mod, NetworkDirection, NetworkPhase, Registration};
use horizon_protocol::{Direction, State, connection::PacketTranslator};

struct VersionBridge {
    target: Target,
}

enum Target {
    V26_1,
    V26_2,
}

impl Mod for VersionBridge {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Self {
                target: Target::V26_2,
            },
            Registration::new(),
        ))
    }

    fn network_start(_address: &str, target_version: Option<&str>) -> Result<Option<Self>, String> {
        let target = match target_version {
            Some("26.1") => Target::V26_1,
            Some("26.2") => Target::V26_2,
            _ => return Ok(None),
        };
        Ok(Some(Self { target }))
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
        let result = match self.target {
            Target::V26_1 => {
                horizon_protocol::translate_26_1::Translator.translate(phase, direction, &body)
            }
            Target::V26_2 => {
                horizon_protocol::translate_26_2::Translator.translate(phase, direction, &body)
            }
        };
        result.map_err(|e| e.to_string())
    }
}

horizon_mod_sdk::export_mod!(VersionBridge);
