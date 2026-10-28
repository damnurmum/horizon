// Shared 26.1/26.2 -> 26.3 payload conversion. Included by the version-specific modules.
use crate::{
    Direction, Error, Result, State,
    buf::{Codec, Ident, Reader, Writer},
    chunk::{BLOCKS, ChunkData, Layers, LightData, Paletted, Section, Strategy},
    connection::PacketTranslator,
    entity_data::{EntityData, Particle, ParticleData, Value, read_value, serializer},
    ids,
    item::{Component, Slot, Stack, Template},
    packets::{
        handshake,
        login,
        play::{Explode, SoundHolder},
    },
};

/// Native, per-connection packet adapter. Pass a fresh instance to
/// `horizon_client::net::Session::connect_translated`.
pub struct Translator;

impl PacketTranslator for Translator {
    fn translate(&self, state: State, direction: Direction, body: &[u8]) -> Result<Vec<Vec<u8>>> {
        if state == State::Handshake && direction == Direction::Serverbound {
            let mut reader = Reader::new(body);
            let id = reader.varint()?;
            let packet = handshake::Serverbound::decode(id, reader.rest())?;
            let handshake::Serverbound::Intention(mut intention) = packet else {
                return Err(Error::new("expected handshake intention"));
            };
            intention.protocol = PROTOCOL_VERSION;
            return Ok(vec![handshake::Serverbound::Intention(intention).encode()]);
        }

        let mut reader = Reader::new(body);
        let id = reader.varint()?;
        let payload = reader.rest();
        let name = if direction == Direction::Clientbound {
            ids_26_2::name(state, direction, id)
        } else {
            ids::name(state, direction, id)
        }
        .ok_or_else(|| Error::new(format!("unknown {state:?} {direction:?} packet {id}")))?;

        let target_id = if direction == Direction::Clientbound {
            current_id(state, direction, name)
        } else {
            old_packet_id(state, direction, name)
        };
        let (target_id, payload) = match (state, direction, name) {
            (State::Login, Direction::Clientbound, "login_finished")
                if !LOGIN_FINISHED_HAS_SESSION =>
            {
                let mut reader = Reader::new(payload);
                login::GameProfile::read(&mut reader)?;
                if reader.remaining() != 0 {
                    return Err(Error::new(format!("{OLD_VERSION} login_finished has trailing bytes")));
                }
                let mut converted = payload.to_vec();
                converted.extend_from_slice(&0_u128.to_be_bytes()); // session UUID absent in 26.1
                (
                    target_id.ok_or_else(|| Error::new("26.3 has no login_finished packet"))?,
                    converted,
                )
            }
            (State::Play, Direction::Serverbound, "punch") => {
                // 26.3's empty Punch replaces older version's hand-valued Swing.
                if !payload.is_empty() {
                    return Err(Error::new("punch packet has a payload"));
                }
                (
                    ids_26_2::id(State::Play, Direction::Serverbound, "swing")
                        .ok_or_else(|| Error::new(format!("{OLD_VERSION} has no swing packet")))?,
                    vec![0],
                )
            }
            (State::Play, Direction::Serverbound, "accept_teleportation") => {
                // Older version's acknowledgement contains only the teleport id.
                let mut reader = Reader::new(payload);
                reader.varint()?;
                (
                    target_id.ok_or_else(|| Error::new(format!("{OLD_VERSION} has no teleport acknowledgement")))?,
                    payload[..reader.position()].to_vec(),
                )
            }
            (State::Play, Direction::Serverbound, "player_action") => {
                let mut reader = Reader::new(payload);
                let action = reader.varint()?;
                if action == 1 {
                    // 26.3's CHANGE_DESTROY_DIRECTION has no earlier counterpart.
                    return Ok(Vec::new());
                }
                let old_action = match action {
                    0 => 0,
                    2..=8 => action - 1,
                    _ => return Err(Error::new(format!("unknown player action {action}"))),
                };
                let mut writer = Writer::new();
                writer.varint(old_action);
                writer.data.extend_from_slice(reader.rest());
                (
                    target_id.ok_or_else(|| Error::new(format!("{OLD_VERSION} has no player action packet")))?,
                    writer.data,
                )
            }
            (State::Play, Direction::Clientbound, "level_chunk_with_light") => (
                target_id.ok_or_else(|| Error::new("26.3 has no chunk packet"))?,
                convert_light_payload(payload, true)?,
            ),
            (State::Play, Direction::Clientbound, "light_update") => (
                target_id.ok_or_else(|| Error::new("26.3 has no light packet"))?,
                convert_light_payload(payload, false)?,
            ),
            (State::Play, Direction::Clientbound, "login") => (
                target_id.ok_or_else(|| Error::new("26.3 has no play login packet"))?,
                convert_play_login(payload)?,
            ),
            (State::Play, Direction::Clientbound, "respawn") => (
                target_id.ok_or_else(|| Error::new("26.3 has no respawn packet"))?,
                convert_spawn_info(payload, false)?,
            ),
            (
                State::Play,
                Direction::Clientbound,
                "move_entity_pos"
                | "move_entity_pos_rot"
                | "move_entity_rot"
                | "entity_position_sync",
            ) => (
                target_id.ok_or_else(|| Error::new("26.3 has no entity movement packet"))?,
                convert_movement(payload, name)?,
            ),
            (
                State::Play,
                Direction::Clientbound,
                "block_update" | "section_blocks_update" | "level_event",
            ) => (
                target_id.ok_or_else(|| Error::new("26.3 has no block update packet"))?,
                convert_block_update(payload, name)?,
            ),
            (State::Play, Direction::Clientbound, "block_entity_data") => {
                let mut reader = Reader::new(payload);
                let mut writer = Writer::new();
                writer.i64(reader.i64()?);
                writer.varint(
                    registry_26_2::block_entity_type(reader.varint()?)
                        .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown block entity type")))?,
                );
                writer.data.extend_from_slice(reader.rest());
                (
                    target_id.ok_or_else(|| Error::new("26.3 has no block entity packet"))?,
                    writer.data,
                )
            }
            (State::Play, Direction::Clientbound, "add_entity") => {
                let mut reader = Reader::new(payload);
                let mut writer = Writer::new();
                writer.varint(reader.varint()?); // entity id
                writer.data.extend_from_slice(reader.bytes(16)?); // UUID
                let old_type = reader.varint()?;
                let mapped = registry_26_2::entity_type(old_type).ok_or_else(|| {
                    Error::new(format!("{OLD_VERSION} sent unknown entity type {old_type}"))
                })?;
                writer.varint(mapped);
                writer.data.extend_from_slice(reader.rest());
                (
                    target_id.ok_or_else(|| Error::new("26.3 has no entity packet"))?,
                    writer.data,
                )
            }
            (State::Play, Direction::Clientbound, "set_entity_data") => (
                target_id.ok_or_else(|| Error::new("26.3 has no entity data packet"))?,
                convert_entity_data(payload)?,
            ),
            (State::Play, Direction::Clientbound, "level_particles") => {
                let mut reader = Reader::new(payload);
                let mut writer = Writer::new();
                old_particle(&mut reader)?.write(&mut writer);
                writer.data.extend_from_slice(reader.rest());
                (
                    target_id.ok_or_else(|| Error::new("26.3 has no particle packet"))?,
                    writer.data,
                )
            }
            (State::Play, Direction::Clientbound, "sound" | "sound_entity") => {
                let mut reader = Reader::new(payload);
                let mut writer = Writer::new();
                let mut sound = SoundHolder::read(&mut reader)?;
                if sound.0 >= 0 {
                    sound.0 = registry_26_2::sound_event(sound.0)
                        .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown sound event")))?;
                }
                SoundHolder::write(&sound, &mut writer);
                writer.data.extend_from_slice(reader.rest());
                (
                    target_id.ok_or_else(|| Error::new("26.3 has no sound packet"))?,
                    writer.data,
                )
            }
            (State::Play, Direction::Clientbound, "explode") => (
                target_id.ok_or_else(|| Error::new("26.3 has no explosion packet"))?,
                convert_explode(payload)?,
            ),
            (
                State::Play,
                Direction::Clientbound,
                "container_set_content"
                | "container_set_slot"
                | "set_cursor_item"
                | "set_equipment"
                | "set_player_inventory",
            ) => (
                target_id.ok_or_else(|| Error::new("26.3 has no item packet"))?,
                convert_item_packet(payload, name, true)?,
            ),
            (State::Play, Direction::Serverbound, "set_creative_mode_slot") => (
                target_id.ok_or_else(|| Error::new(format!("{OLD_VERSION} has no creative slot packet")))?,
                convert_item_packet(payload, name, false)?,
            ),
            (State::Play, Direction::Clientbound, "update_advancements") => (
                target_id.ok_or_else(|| Error::new("26.3 has no advancements packet"))?,
                convert_advancements(payload)
                    .map_err(|e| Error::new(format!("{OLD_VERSION} advancements: {e}")))?,
            ),
            (State::Play, Direction::Clientbound, "recipe_book_add") => (
                target_id.ok_or_else(|| Error::new("26.3 has no recipe book packet"))?,
                convert_recipe_book(payload)
                    .map_err(|e| Error::new(format!("{OLD_VERSION} recipe book: {e}")))?,
            ),
            _ => (
                target_id.ok_or_else(|| {
                    Error::new(format!("{OLD_VERSION} has no {state:?} {direction:?} packet {name}"))
                })?,
                payload.to_vec(),
            ),
        };
        let mut writer = Writer::new();
        writer.varint(target_id);
        writer.data.extend_from_slice(&payload);
        Ok(vec![writer.data])
    }
}

fn convert_play_login(payload: &[u8]) -> Result<Vec<u8>> {
    let mut converted = convert_spawn_info(payload, true)?;
    if !PLAY_LOGIN_HAS_ONLINE_MODE {
        let secure_chat = converted
            .pop()
            .ok_or_else(|| Error::new(format!("{OLD_VERSION} play login is missing secure-chat flag")))?;
        if secure_chat > 1 {
            return Err(Error::new(format!("{OLD_VERSION} play login has an invalid secure-chat flag")));
        }
        converted.push(0); // 26.1 did not send the new online-mode field.
        converted.push(secure_chat);
        let mut reader = Reader::new(&converted);
        crate::packets::play::Login::read(&mut reader)?;
        if reader.remaining() != 0 {
            return Err(Error::new(format!("{OLD_VERSION} play login has trailing bytes")));
        }
    }
    Ok(converted)
}

fn old_packet_id(state: State, direction: Direction, name: &str) -> Option<i32> {
    let old_name = if state == State::Play
        && direction == Direction::Serverbound
        && name == "spectator_action"
    {
        SPECTATOR_ACTION_NAME
    } else {
        name
    };
    ids_26_2::id(state, direction, old_name)
}

fn old_particle(reader: &mut Reader) -> Result<Particle> {
    let old_id = reader.varint()?;
    let id = registry_26_2::particle_type(old_id)
        .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown particle type")))?;
    if id == 57 {
        return Ok(Particle {
            id,
            data: Some(ParticleData::Item(old_template(reader)?)),
        });
    }
    let mut particle = Particle::read_with_id(id, reader)?;
    match &mut particle.data {
        Some(ParticleData::State(state)) => {
            *state = registry_26_2::block_state(*state)
                .ok_or_else(|| Error::new(format!("{OLD_VERSION} particle has unknown block state")))?;
        }
        Some(ParticleData::Geyser(state)) | Some(ParticleData::GeyserBase(state, _)) => {
            *state = registry_26_2::block_state(*state)
                .ok_or_else(|| Error::new(format!("{OLD_VERSION} geyser has unknown block state")))?;
        }
        _ => {}
    }
    Ok(particle)
}

fn convert_entity_data(payload: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let entity = reader.varint()?;
    let mut entries = Vec::new();
    for _ in 0..=u8::MAX {
        let index = reader.u8()?;
        if index == 0xff {
            if reader.remaining() != 0 {
                return Err(Error::new(format!("{OLD_VERSION} entity data has trailing bytes")));
            }
            let mut writer = Writer::new();
            writer.varint(entity);
            EntityData::write(&entries, &mut writer);
            return Ok(writer.data);
        }
        let kind = reader.varint()?;
        let value = match kind {
            serializer::ITEM_STACK => Value::Item(old_slot(&mut reader)?),
            serializer::BLOCK_STATE => Value::Id(
                registry_26_2::block_state(reader.varint()?)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} entity data has unknown block state")))?,
            ),
            serializer::OPTIONAL_BLOCK_STATE => {
                let old = reader.varint()?;
                Value::Id(if old == 0 {
                    0
                } else {
                    registry_26_2::block_state(old - 1)
                        .ok_or_else(|| Error::new(format!("{OLD_VERSION} entity data has unknown block state")))?
                        + 1
                })
            }
            serializer::PARTICLE => Value::Particle(old_particle(&mut reader)?),
            serializer::PARTICLES => {
                let count = reader.count(65536)?;
                Value::Particles(
                    (0..count)
                        .map(|_| old_particle(&mut reader))
                        .collect::<Result<_>>()?,
                )
            }
            _ => match read_value(kind, &mut reader)? {
                Some(value) => value,
                // Horizon's decoder also stops at an unknown serializer.
                // Leave that packet intact so the known earlier entries are
                // still available instead of closing the connection.
                None => return Ok(payload.to_vec()),
            },
        };
        entries.push((index, kind, value));
    }
    Err(Error::new(format!("{OLD_VERSION} entity data has too many entries")))
}

fn convert_explode(payload: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    for _ in 0..3 {
        writer.f64(reader.f64()?);
    } // centre
    writer.f32(reader.f32()?); // radius
    writer.i32(reader.i32()?); // block count
    let knockback = reader.bool()?;
    writer.bool(knockback);
    if knockback {
        for _ in 0..3 {
            writer.f64(reader.f64()?);
        }
    }
    old_particle(&mut reader)
        .map_err(|e| Error::new(format!("explosion centre: {e}")))?
        .write(&mut writer);
    let mut sound = SoundHolder::read(&mut reader)?;
    if sound.0 >= 0 {
        sound.0 = registry_26_2::sound_event(sound.0)
            .ok_or_else(|| Error::new(format!("{OLD_VERSION} explosion has unknown sound")))?;
    }
    SoundHolder::write(&sound, &mut writer);
    let count = reader.count(4096)?;
    writer.varint(count as i32);
    for _ in 0..count {
        old_particle(&mut reader)
            .map_err(|e| Error::new(format!("explosion scatter: {e}")))?
            .write(&mut writer);
        writer.f32(reader.f32()?); // scaling
        writer.f32(reader.f32()?); // speed
        writer.varint(reader.varint()?); // weight
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!("{OLD_VERSION} explosion has trailing bytes")));
    }
    writer.bool(true); // 26.3 added play_sound
    let mut check = Reader::new(&writer.data);
    Explode::read(&mut check).map_err(|e| Error::new(format!("translated explosion: {e}")))?;
    if check.remaining() != 0 {
        return Err(Error::new("translated explosion has trailing bytes"));
    }
    Ok(writer.data)
}

/// Both components were removed in 26.3. Read their older stream values so the
/// rest of the patch stays aligned, then omit them from the translated item.
fn old_component_kind(old_kind: i32) -> Result<Option<i32>> {
    match old_kind {
        40 | 45 => Ok(None), // swing_animation, map_color
        _ => registry_26_2::data_component_type(old_kind)
            .map(Some)
            .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown component {old_kind}"))),
    }
}

fn old_component(reader: &mut Reader) -> Result<Option<(i32, Component)>> {
    let old_kind = reader.varint()?;
    match old_kind {
        40 => {
            reader.varint()?; // SwingAnimationType.STREAM_CODEC
            reader.varint()?; // duration
            Ok(None)
        }
        45 => {
            reader.i32()?; // MapItemColor.STREAM_CODEC
            Ok(None)
        }
        74 => {
            // PotDecorations changed from a list of four item IDs to four
            // optional ItemStackTemplates in 26.3. Brick means an empty side.
            let count = reader.count(4)?;
            let mut writer = Writer::new();
            for side in 0..4 {
                let old_item = (side < count).then(|| reader.varint()).transpose()?;
                let old_item = old_item.filter(|id| *id != BRICK_ITEM_ID); // brick in the older version
                writer.bool(old_item.is_some());
                if let Some(old_item) = old_item {
                    let item = registry_26_2::item(old_item).ok_or_else(|| {
                        Error::new(format!("{OLD_VERSION} pot decoration has unknown item {old_item}"))
                    })?;
                    Template::write(
                        &Stack {
                            item,
                            count: 1,
                            added: Vec::new(),
                            removed: Vec::new(),
                        },
                        &mut writer,
                    );
                }
            }
            Ok(Some((76, Component::Raw(writer.data))))
        }
        _ => {
            let kind = registry_26_2::data_component_type(old_kind)
                .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown component {old_kind}")))?;
            let mut value = Component::read(kind, reader).map_err(|e| {
                Error::new(format!("{OLD_VERSION} component {old_kind} (current {kind}): {e}"))
            })?;
            remap_component(&mut value, true)?;
            Ok(Some((kind, value)))
        }
    }
}

fn old_template(reader: &mut Reader) -> Result<Stack> {
    let item = registry_26_2::item(reader.varint()?)
        .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown template item")))?;
    let count = reader.varint()?;
    let positive = reader.count(65536)?;
    let negative = reader.count(65536)?;
    let mut added = Vec::with_capacity(positive.min(64));
    for _ in 0..positive {
        if let Some(component) = old_component(reader)? {
            added.push(component);
        }
    }
    let mut removed = Vec::with_capacity(negative.min(64));
    for _ in 0..negative {
        if let Some(kind) = old_component_kind(reader.varint()?)? {
            removed.push(kind);
        }
    }
    Ok(Stack {
        item,
        count,
        added,
        removed,
    })
}

fn convert_advancements(payload: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    writer.bool(reader.bool()?); // reset
    let count = reader.count(4096)?;
    writer.varint(count as i32);
    for _ in 0..count {
        writer.string(&reader.string(32767)?); // id
        let parent = reader.bool()?;
        writer.bool(parent);
        if parent {
            writer.string(&reader.string(32767)?);
        }
        let display = reader.bool()?;
        writer.bool(display);
        let mut position = (0.0, 0.0);
        if display {
            writer.nbt(&reader.nbt()?); // title
            writer.nbt(&reader.nbt()?); // description
            Template::write(&old_template(&mut reader)?, &mut writer);
            writer.varint(reader.varint()?); // frame
            let flags = reader.i32()?;
            writer.i32(flags);
            if flags & 1 != 0 {
                writer.string(&reader.string(32767)?);
            }
            position = (reader.f32()?, reader.f32()?);
        }
        let groups = reader.count(4096)?;
        writer.varint(groups as i32);
        for _ in 0..groups {
            let entries = reader.count(4096)?;
            writer.varint(entries as i32);
            for _ in 0..entries {
                writer.string(&reader.string(32767)?);
            }
        }
        writer.bool(reader.bool()?); // telemetry
        writer.f32(position.0);
        writer.f32(position.1);
    }
    // Removed ids, progress and toast flag use the same layout.
    writer.data.extend_from_slice(reader.rest());
    Ok(writer.data)
}

fn old_holder_set(reader: &mut Reader, writer: &mut Writer) -> Result<()> {
    let encoded = reader.varint()?;
    if !(0..=4097).contains(&encoded) {
        return Err(Error::new(format!("{OLD_VERSION} recipe holder set is too large")));
    }
    writer.varint(encoded);
    if encoded == 0 {
        writer.string(&reader.string(32767)?);
    } else {
        for _ in 0..(encoded - 1) {
            writer.varint(
                registry_26_2::item(reader.varint()?)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} recipe has unknown item")))?,
            );
        }
    }
    Ok(())
}

fn old_slot_display(reader: &mut Reader, writer: &mut Writer, depth: u8) -> Result<()> {
    if depth > 32 {
        return Err(Error::new(format!("{OLD_VERSION} slot displays nest too deep")));
    }
    let id = reader.varint()?;
    writer.varint(id);
    match id {
        0 | 1 => {}
        2 => old_slot_display(reader, writer, depth + 1)?,
        3 => {
            old_slot_display(reader, writer, depth + 1)?;
            writer.varint(
                registry_26_2::data_component_type(reader.varint()?)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} recipe has unknown component")))?,
            );
        }
        4 => writer.varint(
            registry_26_2::item(reader.varint()?)
                .ok_or_else(|| Error::new(format!("{OLD_VERSION} recipe has unknown item")))?,
        ),
        5 => Template::write(&old_template(reader)?, writer),
        6 => {
            // TagSlotDisplay was a TagKey in older versions; 26.3 uses a HolderSet.
            writer.varint(0);
            writer.string(&reader.string(32767)?);
        }
        7 | 9 => {
            old_slot_display(reader, writer, depth + 1)?;
            old_slot_display(reader, writer, depth + 1)?;
        }
        8 => {
            old_slot_display(reader, writer, depth + 1)?;
            old_slot_display(reader, writer, depth + 1)?;
            let pattern = reader.varint()?;
            writer.varint(pattern);
            if pattern == 0 {
                writer.string(&reader.string(32767)?);
                writer.nbt(&reader.nbt()?);
                writer.bool(reader.bool()?);
            }
        }
        10 => {
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                old_slot_display(reader, writer, depth + 1)?;
            }
        }
        _ => return Err(Error::new(format!("{OLD_VERSION} has unknown slot display {id}"))),
    }
    Ok(())
}

fn old_recipe_display(reader: &mut Reader, writer: &mut Writer) -> Result<()> {
    let id = reader.varint()?;
    writer.varint(id);
    let slots = match id {
        0 => {
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                old_slot_display(reader, writer, 0)?;
            }
            2
        }
        1 => {
            writer.varint(reader.varint()?); // width
            writer.varint(reader.varint()?); // height
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                old_slot_display(reader, writer, 0)?;
            }
            2
        }
        2 => 4,
        3 => 3,
        4 => 5,
        _ => return Err(Error::new(format!("{OLD_VERSION} has unknown recipe display {id}"))),
    };
    for _ in 0..slots {
        old_slot_display(reader, writer, 0)?;
    }
    if id == 2 {
        writer.varint(reader.varint()?); // duration
        writer.f32(reader.f32()?); // experience
    }
    Ok(())
}

fn convert_recipe_book(payload: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    let count = reader.count(65536)?;
    writer.varint(count as i32);
    for _ in 0..count {
        writer.varint(reader.varint()?); // display id
        old_recipe_display(&mut reader, &mut writer)?;
        writer.varint(reader.varint()?); // group
        writer.varint(reader.varint()?); // category
        let requirements = reader.bool()?;
        writer.bool(requirements);
        if requirements {
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                old_holder_set(&mut reader, &mut writer)?;
            }
        }
        writer.i8(reader.i8()?); // flags
    }
    writer.bool(reader.bool()?); // replace
    if reader.remaining() != 0 {
        return Err(Error::new(format!("{OLD_VERSION} recipe book has trailing bytes")));
    }
    Ok(writer.data)
}

fn remap_stack(stack: &mut Stack, to_current: bool) -> Result<()> {
    let map_item = if to_current {
        registry_26_2::item
    } else {
        registry_26_2::item_to_old
    };
    let map_component = if to_current {
        registry_26_2::data_component_type
    } else {
        registry_26_2::data_component_type_to_old
    };
    stack.item = map_item(stack.item).ok_or_else(|| Error::new("item absent in target version"))?;
    for (kind, value) in &mut stack.added {
        *kind =
            map_component(*kind).ok_or_else(|| Error::new("component absent in target version"))?;
        remap_component(value, to_current)?;
    }
    for kind in &mut stack.removed {
        *kind =
            map_component(*kind).ok_or_else(|| Error::new("component absent in target version"))?;
    }
    Ok(())
}

fn remap_component(value: &mut Component, to_current: bool) -> Result<()> {
    let map_component = if to_current {
        registry_26_2::data_component_type
    } else {
        registry_26_2::data_component_type_to_old
    };
    match value {
        Component::Stacks(stacks) => {
            for stack in stacks {
                remap_stack(stack, to_current)?;
            }
        }
        Component::TooltipDisplay { hidden, .. } => {
            for kind in hidden {
                *kind = map_component(*kind)
                    .ok_or_else(|| Error::new("hidden component absent in target version"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn old_slot(reader: &mut Reader) -> Result<Option<Stack>> {
    let count = reader.varint()?;
    if count <= 0 {
        return Ok(None);
    }
    let item = registry_26_2::item(reader.varint()?)
        .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown item")))?;
    let positive = reader.count(65536)?;
    let negative = reader.count(65536)?;
    let mut added = Vec::with_capacity(positive.min(64));
    for _ in 0..positive {
        if let Some(component) = old_component(reader)? {
            added.push(component);
        }
    }
    let mut removed = Vec::with_capacity(negative.min(64));
    for _ in 0..negative {
        if let Some(kind) = old_component_kind(reader.varint()?)? {
            removed.push(kind);
        }
    }
    Ok(Some(Stack {
        item,
        count,
        added,
        removed,
    }))
}

fn convert_item_packet(payload: &[u8], name: &str, to_current: bool) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    match name {
        "container_set_content" => {
            writer.varint(reader.varint()?); // container
            writer.varint(reader.varint()?); // state
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                Slot::write(&old_slot(&mut reader)?, &mut writer);
            }
            Slot::write(&old_slot(&mut reader)?, &mut writer); // carried
        }
        "container_set_slot" => {
            writer.varint(reader.varint()?); // container
            writer.varint(reader.varint()?); // state
            writer.i16(reader.i16()?); // slot
            Slot::write(&old_slot(&mut reader)?, &mut writer);
        }
        "set_cursor_item" => Slot::write(&old_slot(&mut reader)?, &mut writer),
        "set_equipment" => {
            writer.varint(reader.varint()?); // entity id
            loop {
                let slot = reader.u8()?;
                writer.u8(slot);
                Slot::write(&old_slot(&mut reader)?, &mut writer);
                if slot & 128 == 0 {
                    break;
                }
            }
        }
        "set_player_inventory" => {
            writer.varint(reader.varint()?); // slot
            Slot::write(&old_slot(&mut reader)?, &mut writer);
        }
        "set_creative_mode_slot" if !to_current => {
            writer.i16(reader.i16()?);
            let mut stack = Slot::read(&mut reader)?;
            if let Some(stack) = &mut stack {
                remap_stack(stack, false)?;
            }
            Slot::write(&stack, &mut writer);
        }
        _ => return Err(Error::new("unsupported item packet")),
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!("{name} has trailing bytes")));
    }
    Ok(writer.data)
}

fn convert_block_update(payload: &[u8], name: &str) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    match name {
        "block_update" => {
            writer.i64(reader.i64()?); // block position
            let old = reader.varint()?;
            writer.varint(
                registry_26_2::block_state(old)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown block state")))?,
            );
        }
        "section_blocks_update" => {
            writer.i64(reader.i64()?); // section position
            let count = reader.count(4096)?;
            writer.varint(count as i32);
            for _ in 0..count {
                let change = reader.varlong()?;
                let old = (change as u64 >> 12) as i32;
                let mapped = registry_26_2::block_state(old)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} sent unknown section block state")))?;
                writer.varlong((i64::from(mapped) << 12) | (change & 0xfff));
            }
        }
        "level_event" => {
            let event = reader.i32()?;
            writer.i32(event);
            writer.i64(reader.i64()?); // block position
            let data = reader.i32()?;
            writer.i32(if event == 2001 {
                registry_26_2::block_state(data)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} block break event has unknown state")))?
            } else {
                data
            });
            writer.bool(reader.bool()?);
        }
        _ => return Err(Error::new("unsupported block update packet")),
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!("{OLD_VERSION} {name} has trailing bytes")));
    }
    Ok(writer.data)
}

fn convert_movement(payload: &[u8], name: &str) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let mut writer = Writer::new();
    let id = reader.varint()?;
    writer.varint(id);
    match name {
        "entity_position_sync" => {
            // Older version's PositionMoveRotation sends position and velocity; 26.3
            // sends a PositionPath instead. A linear path is one absolute
            // position, and velocity is handled by its own packet.
            let position = [reader.f64()?, reader.f64()?, reader.f64()?];
            for _ in 0..3 {
                reader.f64()?;
            }
            let yaw = reader.f32()?;
            let pitch = reader.f32()?;
            let on_ground = reader.bool()?;
            writer.varint(0); // linear path
            for coordinate in position {
                writer.f64(coordinate);
            }
            writer.f32(yaw);
            writer.f32(pitch);
            writer.bool(on_ground);
        }
        "move_entity_pos" | "move_entity_pos_rot" => {
            let delta = [reader.i16()?, reader.i16()?, reader.i16()?];
            let rotation = if name == "move_entity_pos_rot" {
                Some((reader.i8()?, reader.i8()?))
            } else {
                None
            };
            let on_ground = reader.bool()?;
            writer.varint(i32::from(on_ground)); // zero VecDelta steps
            for coordinate in delta {
                writer.i16(coordinate);
            }
            if let Some((yaw, pitch)) = rotation {
                writer.i8(yaw);
                writer.i8(pitch);
            }
        }
        "move_entity_rot" => {
            let yaw = reader.i8()?;
            let pitch = reader.i8()?;
            let on_ground = reader.bool()?;
            writer.bool(on_ground);
            writer.i8(yaw);
            writer.i8(pitch);
        }
        _ => return Err(Error::new("unsupported movement packet")),
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!("{OLD_VERSION} {name} has trailing bytes")));
    }
    Ok(writer.data)
}

/// `CommonPlayerSpawnInfo` changed its two game mode fields from signed
/// bytes in older versions to VarInts in 26.3 (the previous mode uses 0 for none).
fn convert_spawn_info(payload: &[u8], login: bool) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    if login {
        reader.i32()?; // player id
        reader.bool()?; // hardcore
        let levels = reader.count(256)?;
        for _ in 0..levels {
            Ident::read(&mut reader)?;
        }
        reader.varint()?; // max players
        reader.varint()?; // view distance
        reader.varint()?; // simulation distance
        for _ in 0..3 {
            reader.bool()?;
        }
    }
    reader.varint()?; // dimension type holder
    Ident::read(&mut reader)?; // dimension key
    reader.i64()?; // seed
    let modes_start = reader.position();
    let mode = reader.i8()?;
    let previous = reader.i8()?;
    if !(0..=3).contains(&mode) || !(-1..=3).contains(&previous) {
        return Err(Error::new(format!("{OLD_VERSION} sent an invalid game mode")));
    }
    let mut writer = Writer::new();
    writer.data.extend_from_slice(&payload[..modes_start]);
    writer.varint(i32::from(mode));
    writer.varint(i32::from(previous) + 1);
    writer.data.extend_from_slice(reader.rest());
    Ok(writer.data)
}

/// Older version's `FriendlyByteBuf.readBitSet` uses a VarInt-prefixed array of
/// big-endian longs; 26.3's `ByteBufCodecs.BIT_SET` uses a length-prefixed
/// little-endian byte array. The five other light fields keep their layout.
fn old_bitset(reader: &mut Reader) -> Result<Vec<u8>> {
    let longs = reader.long_array()?;
    let mut bytes: Vec<u8> = longs.iter().flat_map(|long| long.to_le_bytes()).collect();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    Ok(bytes)
}

fn convert_light_payload(payload: &[u8], chunk: bool) -> Result<Vec<u8>> {
    let mut reader = Reader::new(payload);
    let chunk_data = if chunk {
        let x = reader.i32()?;
        let z = reader.i32()?;
        let mut data = ChunkData::read(&mut reader)?;
        data.buffer = convert_sections(&data.buffer)?;
        for entity in &mut data.block_entities {
            entity.kind = registry_26_2::block_entity_type(entity.kind)
                .ok_or_else(|| Error::new(format!("{OLD_VERSION} chunk contains unknown block entity type")))?;
        }
        Some((x, z, data))
    } else {
        reader.varint()?;
        reader.varint()?;
        None
    };
    let prefix = reader.position();
    let light = LightData {
        sky_mask: old_bitset(&mut reader)?,
        block_mask: old_bitset(&mut reader)?,
        empty_sky_mask: old_bitset(&mut reader)?,
        empty_block_mask: old_bitset(&mut reader)?,
        sky: Layers::read(&mut reader)?,
        block: Layers::read(&mut reader)?,
    };
    if reader.remaining() != 0 {
        return Err(Error::new(format!(
            "{OLD_VERSION} light packet has {} trailing bytes",
            reader.remaining()
        )));
    }
    let mut writer = Writer::new();
    if let Some((x, z, data)) = chunk_data {
        writer.i32(x);
        writer.i32(z);
        ChunkData::write(&data, &mut writer);
    } else {
        writer.data.extend_from_slice(&payload[..prefix]);
    }
    LightData::write(&light, &mut writer);
    Ok(writer.data)
}

fn convert_sections(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Reader::new(buffer);
    let mut writer = Writer::new();
    while reader.remaining() > 0 {
        // The old global block-state palette has 15 bits (32,366 states).
        // 26.3 has 35,723 states and uses 16 bits. Biome ids are dynamic.
        let mut section = Section::read(&mut reader, 15, 7)?;
        if section.states.palette.is_empty() {
            let values = (0..BLOCKS)
                .map(|i| {
                    registry_26_2::block_state(section.states.get(i))
                        .ok_or_else(|| Error::new(format!("{OLD_VERSION} chunk contains an unknown block state")))
                })
                .collect::<Result<Vec<_>>>()?;
            section.states = Paletted::from_values(&values, Strategy::Blocks, 16);
        } else {
            for old in &mut section.states.palette {
                *old = registry_26_2::block_state(*old)
                    .ok_or_else(|| Error::new(format!("{OLD_VERSION} chunk contains an unknown block state")))?;
            }
        }
        section.write(&mut writer);
    }
    Ok(writer.data)
}

/// The 26.3 table is generated as id-to-name only; its inverse is needed for
/// clientbound older packets. Keep this match aligned with `ids::name`.
fn current_id(state: State, direction: Direction, name: &str) -> Option<i32> {
    let names: &[&str] = match (state, direction) {
        (State::Handshake, Direction::Serverbound) => ids::handshake::serverbound::NAMES,
        (State::Status, Direction::Clientbound) => ids::status::clientbound::NAMES,
        (State::Status, Direction::Serverbound) => ids::status::serverbound::NAMES,
        (State::Login, Direction::Clientbound) => ids::login::clientbound::NAMES,
        (State::Login, Direction::Serverbound) => ids::login::serverbound::NAMES,
        (State::Configuration, Direction::Clientbound) => ids::configuration::clientbound::NAMES,
        (State::Configuration, Direction::Serverbound) => ids::configuration::serverbound::NAMES,
        (State::Play, Direction::Clientbound) => ids::play::clientbound::NAMES,
        (State::Play, Direction::Serverbound) => ids::play::serverbound::NAMES,
        _ => &[],
    };
    names
        .iter()
        .position(|candidate| *candidate == name)
        .and_then(|id| i32::try_from(id).ok())
}
