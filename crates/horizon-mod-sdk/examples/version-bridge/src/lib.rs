//! Per-connection release protocol translator using the network mod API.
mod routes;

use horizon_mod_sdk::{Mod, NetworkDirection, NetworkPhase, Registration};
use horizon_protocol::{Direction, State};

struct VersionBridge {
    route: Vec<routes::Adapter>,
}

impl Mod for VersionBridge {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self { route: Vec::new() }, Registration::new()))
    }

    fn network_start(_address: &str, target_version: Option<&str>) -> Result<Option<Self>, String> {
        let Some(version) = target_version else {
            return Ok(None);
        };
        if version == horizon_protocol::GAME_VERSION {
            return Ok(None);
        }
        let route =
            routes::route_to(version).ok_or_else(|| routes::unsupported_message(version))?;
        Ok(Some(Self { route }))
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
        routes::translate(&self.route, phase, direction, body)
    }
}

horizon_mod_sdk::export_mod!(VersionBridge);
