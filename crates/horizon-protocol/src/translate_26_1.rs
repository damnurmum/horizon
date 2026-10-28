//! Java Edition 26.1 (protocol 775) packet adapter for the 26.3 client.
use crate::{ids_26_1 as ids_26_2, registry_26_1 as registry_26_2};

pub const PROTOCOL_VERSION: i32 = 775;
const OLD_VERSION: &str = "26.1";
const BRICK_ITEM_ID: i32 = 1026;
const SPECTATOR_ACTION_NAME: &str = "spectate_entity";

include!("translate_legacy.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn one(state: State, direction: Direction, body: &[u8]) -> Vec<u8> {
        let packets = Translator.translate(state, direction, body).unwrap();
        assert_eq!(packets.len(), 1);
        packets.into_iter().next().unwrap()
    }

    #[test]
    fn handshake_uses_26_1_protocol() {
        let intention = handshake::Serverbound::Intention(handshake::Intention {
            protocol: crate::PROTOCOL_VERSION,
            host: "localhost".into(),
            port: 25565,
            intent: handshake::LOGIN,
        });
        let packet = one(
            State::Handshake,
            Direction::Serverbound,
            &intention.encode(),
        );
        let mut reader = Reader::new(&packet);
        assert_eq!(reader.varint().unwrap(), 0);
        assert_eq!(reader.varint().unwrap(), PROTOCOL_VERSION);
    }

    #[test]
    fn renamed_spectator_packet_keeps_26_1_id() {
        let current = ids::play::serverbound::NAMES
            .iter()
            .position(|name| *name == "spectator_action")
            .unwrap() as i32;
        let mut writer = Writer::new();
        writer.varint(current);
        writer.varint(0);
        let packet = one(State::Play, Direction::Serverbound, &writer.data);
        let mut reader = Reader::new(&packet);
        assert_eq!(
            ids_26_2::name(
                State::Play,
                Direction::Serverbound,
                reader.varint().unwrap()
            ),
            Some("spectate_entity")
        );
        assert_eq!(reader.varint().unwrap(), 0);
    }

    #[test]
    fn item_and_block_ids_use_26_1_tables() {
        let old_item = 1026;
        let current_item = registry_26_2::item(old_item).unwrap();
        assert_eq!(current_item, crate::registry_26_2::item(1054).unwrap());
        let mut outgoing = Stack::new(current_item, 1);
        remap_stack(&mut outgoing, false).unwrap();
        assert_eq!(outgoing.item, old_item);
        let old_state = 24687; // calcite in 26.1, sulfur in 26.2
        assert_ne!(
            registry_26_2::block_state(old_state),
            crate::registry_26_2::block_state(old_state)
        );
        let mut writer = Writer::new();
        writer.i64(0);
        writer.varint(old_state);
        let converted = convert_block_update(&writer.data, "block_update").unwrap();
        let mut reader = Reader::new(&converted);
        reader.i64().unwrap();
        assert_eq!(
            reader.varint().unwrap(),
            registry_26_2::block_state(old_state).unwrap()
        );
    }
}
