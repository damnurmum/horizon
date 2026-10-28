//! Java Edition 26.2 (protocol 776) packet adapter for the 26.3 client.
use crate::{ids_26_2, registry_26_2};

pub const PROTOCOL_VERSION: i32 = 776;
const OLD_VERSION: &str = "26.2";
const BRICK_ITEM_ID: i32 = 1054;
const SPECTATOR_ACTION_NAME: &str = "spectator_action";
const LOGIN_FINISHED_HAS_SESSION: bool = true;
const PLAY_LOGIN_HAS_ONLINE_MODE: bool = true;

include!("translate_legacy.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_actions_use_26_2_enum_ordinals() {
        use crate::packets::play::{PlayerAction, Serverbound};
        for (current, old) in [(4, 3), (5, 4), (6, 5), (7, 6), (8, 7)] {
            let body = Serverbound::PlayerAction(PlayerAction {
                action: current,
                pos: [0; 3],
                direction: 0,
                sequence: 0,
            })
            .encode();
            let translated = one(State::Play, Direction::Serverbound, &body);
            let mut reader = Reader::new(&translated);
            let id = reader.varint().unwrap();
            assert_eq!(
                ids_26_2::name(State::Play, Direction::Serverbound, id),
                Some("player_action")
            );
            assert_eq!(reader.varint().unwrap(), old);
        }
        let change = Serverbound::PlayerAction(PlayerAction {
            action: 1,
            pos: [0; 3],
            direction: 0,
            sequence: 0,
        })
        .encode();
        assert!(
            Translator
                .translate(State::Play, Direction::Serverbound, &change)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn dropped_item_metadata_maps_its_stack() {
        let mut old = Writer::new();
        old.varint(123); // entity id
        old.u8(8); // ItemEntity.DATA_ITEM
        old.varint(serializer::ITEM_STACK);
        old.varint(1); // count
        old.varint(72); // 26.2 item id
        old.varint(0); // added components
        old.varint(0); // removed components
        old.u8(0xff);
        let converted = convert_entity_data(&old.data).unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.varint().unwrap(), 123);
        let entries = EntityData::read(&mut reader).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert!(matches!(
            &entries[..],
            [(8, serializer::ITEM_STACK, Value::Item(Some(stack)))] if stack.item == 73
        ));
    }

    #[test]
    fn removed_26_2_item_components_leave_patches_aligned() {
        let mut old = Writer::new();
        old.varint(1238); // piglin spawn egg in 26.2
        old.varint(1); // count
        old.varint(2); // added components
        old.varint(2); // removed components
        old.varint(45); // map_color
        old.i32(0x123456);
        old.varint(40); // swing_animation
        old.varint(1); // type
        old.varint(6); // duration
        old.varint(45);
        old.varint(40);
        let mut reader = Reader::new(&old.data);
        let item = old_template(&mut reader).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert!(item.added.is_empty());
        assert!(item.removed.is_empty());

        let mut old_slot_bytes = Writer::new();
        old_slot_bytes.varint(1); // count before item in a slot
        old_slot_bytes.varint(1238);
        let mut patch_reader = Reader::new(&old.data);
        patch_reader.varint().unwrap();
        patch_reader.varint().unwrap();
        old_slot_bytes.data.extend_from_slice(patch_reader.rest());
        let mut reader = Reader::new(&old_slot_bytes.data);
        let slot = old_slot(&mut reader).unwrap().unwrap();
        assert_eq!(reader.remaining(), 0);
        assert!(slot.added.is_empty());
        assert!(slot.removed.is_empty());
    }

    #[test]
    fn old_pot_decorations_become_optional_templates() {
        let mut old = Writer::new();
        old.varint(74); // 26.2 pot_decorations component
        old.varint(4);
        for item in [1238, 1054, 1238, 1054] {
            old.varint(item);
        }
        let mut reader = Reader::new(&old.data);
        let (kind, value) = old_component(&mut reader).unwrap().unwrap();
        assert_eq!(kind, 76);
        assert_eq!(reader.remaining(), 0);
        let Component::Raw(bytes) = value else {
            panic!("expected converted pot decorations");
        };
        let mut reader = Reader::new(&bytes);
        assert!(reader.bool().unwrap());
        assert_eq!(Template::read(&mut reader).unwrap().item, 1359);
        assert!(!reader.bool().unwrap());
        assert!(reader.bool().unwrap());
        assert_eq!(Template::read(&mut reader).unwrap().item, 1359);
        assert!(!reader.bool().unwrap());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn recipe_tag_and_item_ids_decode_in_current_book() {
        use crate::packets::recipes::{HolderSet, RecipeBookAdd, RecipeDisplay, SlotDisplay};
        let mut old = Writer::new();
        old.varint(1); // recipe count
        old.varint(42); // display id
        old.varint(0); // shapeless
        old.varint(1); // ingredient count
        old.varint(6); // old tag display
        old.string("minecraft:logs");
        old.varint(4); // result item display
        old.varint(72); // old item id
        old.varint(4); // station item display
        old.varint(0);
        old.varint(0); // group
        old.varint(0); // category
        old.bool(false); // requirements
        old.i8(0); // flags
        old.bool(true); // replace
        let converted = convert_recipe_book(&old.data).unwrap();
        let mut reader = Reader::new(&converted);
        let book = RecipeBookAdd::read(&mut reader).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert!(book.replace);
        let RecipeDisplay::Shapeless {
            ingredients,
            result,
            ..
        } = &book.entries[0].display
        else {
            panic!("expected shapeless recipe");
        };
        assert_eq!(
            ingredients,
            &vec![SlotDisplay::Tag(HolderSet::Tag("minecraft:logs".into()))]
        );
        assert_eq!(result, &SlotDisplay::Item(73));
    }

    #[test]
    fn item_slots_map_both_directions() {
        let mut old = Writer::new();
        old.varint(0);
        old.varint(1);
        old.i16(2);
        old.varint(3);
        old.varint(72);
        old.varint(0);
        old.varint(0);
        let converted = convert_item_packet(&old.data, "container_set_slot", true).unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.varint().unwrap(), 0);
        assert_eq!(reader.varint().unwrap(), 1);
        assert_eq!(reader.i16().unwrap(), 2);
        assert_eq!(Slot::read(&mut reader).unwrap().unwrap().item, 73);
        assert_eq!(reader.remaining(), 0);

        let mut current = Writer::new();
        current.i16(2);
        Slot::write(&Some(Stack::new(73, 3)), &mut current);
        let converted =
            convert_item_packet(&current.data, "set_creative_mode_slot", false).unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.i16().unwrap(), 2);
        assert_eq!(Slot::read(&mut reader).unwrap().unwrap().item, 72);

        // Via's backwards data maps new items to older substitutes. Exercise
        // both an explicit item override and a whole-word material fallback.
        for (current_item, old_item) in [(212, 163), (72, 65)] {
            let mut current = Writer::new();
            current.i16(2);
            Slot::write(&Some(Stack::new(current_item, 1)), &mut current);
            let converted =
                convert_item_packet(&current.data, "set_creative_mode_slot", false).unwrap();
            let mut reader = Reader::new(&converted);
            assert_eq!(reader.i16().unwrap(), 2);
            assert_eq!(Slot::read(&mut reader).unwrap().unwrap().item, old_item);
        }
    }

    #[test]
    fn every_current_item_has_a_26_2_id_or_fallback() {
        for current in 0..1658 {
            let old = registry_26_2::item_to_old(current).expect("missing 26.2 item fallback");
            assert!((0..1537).contains(&old));
        }
    }

    #[test]
    fn advancement_positions_move_after_requirements() {
        use crate::packets::play::UpdateAdvancements;
        use horizon_world::nbt::Tag;
        let mut old = Writer::new();
        old.bool(true); // reset
        old.varint(1); // added count
        old.string("minecraft:test");
        old.bool(false); // no parent
        old.bool(true); // display
        old.nbt(&Tag::String("Title".into()));
        old.nbt(&Tag::String("Description".into()));
        old.varint(72); // old icon item
        old.varint(1); // count
        old.varint(0); // added components
        old.varint(0); // removed components
        old.varint(0); // frame
        old.i32(0); // flags
        old.f32(1.5); // old display position
        old.f32(2.5);
        old.varint(0); // requirements
        old.bool(false); // telemetry
        old.varint(0); // removed count
        old.varint(0); // progress count
        old.bool(false); // show toast
        let converted = convert_advancements(&old.data).unwrap();
        let mut reader = Reader::new(&converted);
        let packet = UpdateAdvancements::read(&mut reader).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(packet.added[0].x, 1.5);
        assert_eq!(packet.added[0].y, 2.5);
        assert_eq!(packet.added[0].display.as_ref().unwrap().icon.item, 73);
    }

    #[test]
    fn explosion_adds_sound_flag_and_maps_nested_ids() {
        let mut old = Writer::new();
        for value in [1.0, 2.0, 3.0] {
            old.f64(value);
        }
        old.f32(4.0);
        old.i32(0);
        old.bool(false); // no knockback
        old.varint(1); // block particle
        old.varint(15046); // old block state
        old.varint(494); // old sound holder id 493 + 1
        old.varint(0); // no secondary particles
        let converted = convert_explode(&old.data).unwrap();
        let mut reader = Reader::new(&converted);
        let explosion = Explode::read(&mut reader).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(
            explosion.particle.data,
            Some(ParticleData::State(15046 + 3357))
        );
        assert_eq!(explosion.sound.0, registry_26_2::sound_event(493).unwrap());
        assert!(explosion.play_sound);
    }

    fn one(state: State, direction: Direction, body: &[u8]) -> Vec<u8> {
        Translator
            .translate(state, direction, body)
            .unwrap()
            .remove(0)
    }

    #[test]
    fn handshake_uses_26_2_protocol() {
        let original = handshake::Serverbound::Intention(handshake::Intention {
            protocol: crate::PROTOCOL_VERSION,
            host: "example.org".into(),
            port: 25565,
            intent: handshake::LOGIN,
        })
        .encode();
        let translated = one(State::Handshake, Direction::Serverbound, &original);
        let mut reader = Reader::new(&translated);
        let id = reader.varint().unwrap();
        let handshake::Serverbound::Intention(packet) =
            handshake::Serverbound::decode(id, reader.rest()).unwrap()
        else {
            panic!("expected handshake intention");
        };
        assert_eq!(packet.protocol, PROTOCOL_VERSION);
        assert_eq!(packet.host, "example.org");
    }

    #[test]
    fn remaps_configuration_and_play_ids_in_both_directions() {
        // In 26.3, post_effects shifts later clientbound packet ids.
        let old =
            ids_26_2::id(State::Configuration, Direction::Clientbound, "store_cookie").unwrap();
        let new = ids::configuration::clientbound::STORE_COOKIE;
        assert_ne!(old, new);
        assert_eq!(
            one(
                State::Configuration,
                Direction::Clientbound,
                &[old as u8, 7]
            ),
            vec![new as u8, 7]
        );
        let old = ids_26_2::id(State::Play, Direction::Serverbound, "resource_pack").unwrap();
        let new = ids::play::serverbound::RESOURCE_PACK;
        assert_ne!(old, new);
        assert_eq!(
            one(State::Play, Direction::Serverbound, &[new as u8, 8]),
            vec![old as u8, 8]
        );
    }

    #[test]
    fn teleport_ack_and_punch_use_26_2_shapes() {
        let mut packet = Writer::new();
        packet.varint(ids::play::serverbound::ACCEPT_TELEPORTATION);
        packet.varint(42);
        packet.data.extend_from_slice(&[0; 32]);
        assert_eq!(
            one(State::Play, Direction::Serverbound, &packet.data),
            vec![
                ids_26_2::id(State::Play, Direction::Serverbound, "accept_teleportation").unwrap()
                    as u8,
                42
            ]
        );
        assert_eq!(
            one(
                State::Play,
                Direction::Serverbound,
                &[ids::play::serverbound::PUNCH as u8]
            ),
            vec![
                ids_26_2::id(State::Play, Direction::Serverbound, "swing").unwrap() as u8,
                0
            ]
        );
    }

    #[test]
    fn old_light_bitsets_become_byte_bitsets() {
        let mut old = Writer::new();
        old.varint(4); // chunk x
        old.varint(-2); // chunk z
        old.long_array(&[0x0000_0001_0000_0001]);
        old.long_array(&[2]);
        old.long_array(&[]);
        old.long_array(&[0x100]);
        Layers::write(&vec![vec![0x0f; 2048]], &mut old);
        Layers::write(&vec![vec![0xf0; 2048]], &mut old);
        let converted = convert_light_payload(&old.data, false).unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.varint().unwrap(), 4);
        assert_eq!(reader.varint().unwrap(), -2);
        let light = LightData::read(&mut reader).unwrap();
        assert_eq!(light.sky_mask, vec![1, 0, 0, 0, 1]);
        assert_eq!(light.block_mask, vec![2]);
        assert_eq!(light.empty_sky_mask, Vec::<u8>::new());
        assert_eq!(light.empty_block_mask, vec![0, 1]);
        assert_eq!(light.sky[0], vec![0x0f; 2048]);
        assert_eq!(light.block[0], vec![0xf0; 2048]);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn chunk_palette_uses_current_block_state_ids() {
        let old_state = 15046;
        let old_section = Section {
            non_empty: 4096,
            fluids: 0,
            states: Paletted::from_values(&vec![old_state; BLOCKS], Strategy::Blocks, 15),
            biomes: Paletted::from_values(&vec![0; 64], Strategy::Biomes, 7),
        };
        let mut old = Writer::new();
        old.i32(1);
        old.i32(-1);
        ChunkData::write(
            &ChunkData {
                heightmaps: vec![],
                buffer: ChunkData::from_sections(&[old_section]),
                block_entities: vec![],
            },
            &mut old,
        );
        for _ in 0..4 {
            old.long_array(&[]);
        }
        Layers::write(&Vec::new(), &mut old);
        Layers::write(&Vec::new(), &mut old);
        let converted = convert_light_payload(&old.data, true).unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.i32().unwrap(), 1);
        assert_eq!(reader.i32().unwrap(), -1);
        let chunk = ChunkData::read(&mut reader).unwrap();
        let sections = chunk.sections(1, 16, 7).unwrap();
        assert_eq!(sections[0].states.get(0), old_state + 3357);
        let light = LightData::read(&mut reader).unwrap();
        assert!(light.sky.is_empty());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn old_login_game_modes_decode_as_current_spawn_info() {
        use crate::packets::play::Login;
        let mut old = Writer::new();
        old.i32(42);
        old.bool(false);
        old.varint(1);
        old.string("minecraft:overworld");
        for value in [2, 12, 8] {
            old.varint(value);
        }
        for _ in 0..3 {
            old.bool(false);
        }
        old.varint(0); // dimension type
        old.string("minecraft:overworld");
        old.i64(0); // seed
        old.i8(0); // survival
        old.i8(-1); // no previous mode
        old.bool(false); // debug
        old.bool(false); // flat
        old.bool(false); // no last death
        old.varint(0); // portal cooldown
        old.varint(63); // sea level
        old.bool(false); // offline
        old.bool(false); // unsigned chat
        let converted = convert_spawn_info(&old.data, true).unwrap();
        let mut reader = Reader::new(&converted);
        let login = Login::read(&mut reader).unwrap();
        assert_eq!(login.player_id, 42);
        assert_eq!(login.spawn.game_type, 0);
        assert_eq!(login.spawn.previous_game_type, None);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn old_entity_movement_decodes_as_current() {
        use crate::packets::play::{EntityPositionSync, MoveEntityPosRot, PositionPath, VecDelta};
        let mut old = Writer::new();
        old.varint(7);
        for delta in [100, -20, 0] {
            old.i16(delta);
        }
        old.i8(32);
        old.i8(-16);
        old.bool(true);
        let converted = convert_movement(&old.data, "move_entity_pos_rot").unwrap();
        let mut reader = Reader::new(&converted);
        let moved = MoveEntityPosRot::read(&mut reader).unwrap().0;
        assert_eq!(moved.id, 7);
        assert_eq!(moved.delta, Some(VecDelta::Linear([100, -20, 0])));
        assert_eq!(moved.rotation, Some((32, -16)));
        assert!(moved.on_ground);
        assert_eq!(reader.remaining(), 0);

        let mut old = Writer::new();
        old.varint(7);
        for value in [1.0, 2.0, 3.0] {
            old.f64(value);
        }
        for value in [0.1, 0.2, 0.3] {
            old.f64(value);
        }
        old.f32(90.0);
        old.f32(45.0);
        old.bool(false);
        let converted = convert_movement(&old.data, "entity_position_sync").unwrap();
        let mut reader = Reader::new(&converted);
        let moved = EntityPositionSync::read(&mut reader).unwrap();
        assert_eq!(moved.path, PositionPath::Linear([1.0, 2.0, 3.0]));
        assert_eq!(moved.yaw, 90.0);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn block_updates_use_current_state_ids() {
        let mut old = Writer::new();
        old.i64(0x1234);
        old.varint(2);
        old.varlong((15046_i64 << 12) | 0x123);
        old.varlong((27_i64 << 12) | 0x456);
        let converted = convert_block_update(&old.data, "section_blocks_update").unwrap();
        let mut reader = Reader::new(&converted);
        assert_eq!(reader.i64().unwrap(), 0x1234);
        assert_eq!(reader.varint().unwrap(), 2);
        assert_eq!(reader.varlong().unwrap(), ((15046 + 3357) << 12) | 0x123);
        assert_eq!(reader.varlong().unwrap(), ((27 + 1) << 12) | 0x456);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn every_old_packet_name_has_a_current_counterpart() {
        for state in [
            State::Handshake,
            State::Status,
            State::Login,
            State::Configuration,
            State::Play,
        ] {
            for direction in [Direction::Clientbound, Direction::Serverbound] {
                let mut id = 0;
                while let Some(name) = ids_26_2::name(state, direction, id) {
                    if name != "swing" {
                        assert!(
                            current_id(state, direction, name).is_some(),
                            "missing {state:?} {direction:?} {name}"
                        );
                    }
                    id += 1;
                }
            }
        }
    }
}
