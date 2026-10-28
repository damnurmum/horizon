//! Java Edition 26.2 (protocol 776) packet adapter for the 26.3 client.
//! The version bridge mod invokes this adapter through the network mod API.
use crate::{
    Direction, Error, Result, State,
    buf::{Codec, Ident, Reader, Writer},
    chunk::{BLOCKS, ChunkData, Layers, LightData, Paletted, Section, Strategy},
    connection::PacketTranslator,
    entity_data::{EntityData, Particle, ParticleData, Value, read_value, serializer},
    ids, ids_26_2,
    item::{Component, Slot, Stack, Template},
    packets::{
        handshake,
        play::{Explode, SoundHolder},
    },
    registry_26_2,
};

pub const PROTOCOL_VERSION: i32 = 776;

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
            ids_26_2::id(state, direction, name)
        };
        let (target_id, payload) = match (state, direction, name) {
            (State::Play, Direction::Serverbound, "punch") => {
                // 26.3's empty Punch replaces 26.2's hand-valued Swing.
                if !payload.is_empty() {
                    return Err(Error::new("punch packet has a payload"));
                }
                (
                    ids_26_2::id(State::Play, Direction::Serverbound, "swing")
                        .ok_or_else(|| Error::new("26.2 has no swing packet"))?,
                    vec![0],
                )
            }
            (State::Play, Direction::Serverbound, "accept_teleportation") => {
                // 26.2's acknowledgement contains only the teleport id.
                let mut reader = Reader::new(payload);
                reader.varint()?;
                (
                    target_id.ok_or_else(|| Error::new("26.2 has no teleport acknowledgement"))?,
                    payload[..reader.position()].to_vec(),
                )
            }
            (State::Play, Direction::Serverbound, "player_action") => {
                let mut reader = Reader::new(payload);
                let action = reader.varint()?;
                if action == 1 {
                    // 26.3's CHANGE_DESTROY_DIRECTION has no 26.2 counterpart.
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
                    target_id.ok_or_else(|| Error::new("26.2 has no player action packet"))?,
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
                convert_spawn_info(payload, true)?,
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
                        .ok_or_else(|| Error::new("26.2 sent unknown block entity type"))?,
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
                    Error::new(format!("26.2 sent unknown entity type {old_type}"))
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
                        .ok_or_else(|| Error::new("26.2 sent unknown sound event"))?;
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
                target_id.ok_or_else(|| Error::new("26.2 has no creative slot packet"))?,
                convert_item_packet(payload, name, false)?,
            ),
            (State::Play, Direction::Clientbound, "update_advancements") => (
                target_id.ok_or_else(|| Error::new("26.3 has no advancements packet"))?,
                convert_advancements(payload)
                    .map_err(|e| Error::new(format!("26.2 advancements: {e}")))?,
            ),
            (State::Play, Direction::Clientbound, "recipe_book_add") => (
                target_id.ok_or_else(|| Error::new("26.3 has no recipe book packet"))?,
                convert_recipe_book(payload)
                    .map_err(|e| Error::new(format!("26.2 recipe book: {e}")))?,
            ),
            _ => (
                target_id.ok_or_else(|| {
                    Error::new(format!("26.2 has no {state:?} {direction:?} packet {name}"))
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

fn old_particle(reader: &mut Reader) -> Result<Particle> {
    let old_id = reader.varint()?;
    let id = registry_26_2::particle_type(old_id)
        .ok_or_else(|| Error::new("26.2 sent unknown particle type"))?;
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
                .ok_or_else(|| Error::new("26.2 particle has unknown block state"))?;
        }
        Some(ParticleData::Geyser(state)) | Some(ParticleData::GeyserBase(state, _)) => {
            *state = registry_26_2::block_state(*state)
                .ok_or_else(|| Error::new("26.2 geyser has unknown block state"))?;
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
                return Err(Error::new("26.2 entity data has trailing bytes"));
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
                    .ok_or_else(|| Error::new("26.2 entity data has unknown block state"))?,
            ),
            serializer::OPTIONAL_BLOCK_STATE => {
                let old = reader.varint()?;
                Value::Id(if old == 0 {
                    0
                } else {
                    registry_26_2::block_state(old - 1)
                        .ok_or_else(|| Error::new("26.2 entity data has unknown block state"))?
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
    Err(Error::new("26.2 entity data has too many entries"))
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
            .ok_or_else(|| Error::new("26.2 explosion has unknown sound"))?;
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
        return Err(Error::new("26.2 explosion has trailing bytes"));
    }
    writer.bool(true); // 26.3 added play_sound
    let mut check = Reader::new(&writer.data);
    Explode::read(&mut check).map_err(|e| Error::new(format!("translated explosion: {e}")))?;
    if check.remaining() != 0 {
        return Err(Error::new("translated explosion has trailing bytes"));
    }
    Ok(writer.data)
}

/// Both components were removed in 26.3. Read their 26.2 stream values so the
/// rest of the patch stays aligned, then omit them from the translated item.
fn old_component_kind(old_kind: i32) -> Result<Option<i32>> {
    match old_kind {
        40 | 45 => Ok(None), // swing_animation, map_color
        _ => registry_26_2::data_component_type(old_kind)
            .map(Some)
            .ok_or_else(|| Error::new(format!("26.2 sent unknown component {old_kind}"))),
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
                let old_item = old_item.filter(|id| *id != 1054); // 26.2 brick
                writer.bool(old_item.is_some());
                if let Some(old_item) = old_item {
                    let item = registry_26_2::item(old_item).ok_or_else(|| {
                        Error::new(format!("26.2 pot decoration has unknown item {old_item}"))
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
                .ok_or_else(|| Error::new(format!("26.2 sent unknown component {old_kind}")))?;
            let mut value = Component::read(kind, reader).map_err(|e| {
                Error::new(format!("26.2 component {old_kind} (current {kind}): {e}"))
            })?;
            remap_component(&mut value, true)?;
            Ok(Some((kind, value)))
        }
    }
}

fn old_template(reader: &mut Reader) -> Result<Stack> {
    let item = registry_26_2::item(reader.varint()?)
        .ok_or_else(|| Error::new("26.2 sent unknown template item"))?;
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
        return Err(Error::new("26.2 recipe holder set is too large"));
    }
    writer.varint(encoded);
    if encoded == 0 {
        writer.string(&reader.string(32767)?);
    } else {
        for _ in 0..(encoded - 1) {
            writer.varint(
                registry_26_2::item(reader.varint()?)
                    .ok_or_else(|| Error::new("26.2 recipe has unknown item"))?,
            );
        }
    }
    Ok(())
}

fn old_slot_display(reader: &mut Reader, writer: &mut Writer, depth: u8) -> Result<()> {
    if depth > 32 {
        return Err(Error::new("26.2 slot displays nest too deep"));
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
                    .ok_or_else(|| Error::new("26.2 recipe has unknown component"))?,
            );
        }
        4 => writer.varint(
            registry_26_2::item(reader.varint()?)
                .ok_or_else(|| Error::new("26.2 recipe has unknown item"))?,
        ),
        5 => Template::write(&old_template(reader)?, writer),
        6 => {
            // TagSlotDisplay was a TagKey in 26.2; 26.3 uses a HolderSet.
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
        _ => return Err(Error::new(format!("26.2 has unknown slot display {id}"))),
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
        _ => return Err(Error::new(format!("26.2 has unknown recipe display {id}"))),
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
        return Err(Error::new("26.2 recipe book has trailing bytes"));
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
        .ok_or_else(|| Error::new("26.2 sent unknown item"))?;
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
                    .ok_or_else(|| Error::new("26.2 sent unknown block state"))?,
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
                    .ok_or_else(|| Error::new("26.2 sent unknown section block state"))?;
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
                    .ok_or_else(|| Error::new("26.2 block break event has unknown state"))?
            } else {
                data
            });
            writer.bool(reader.bool()?);
        }
        _ => return Err(Error::new("unsupported block update packet")),
    }
    if reader.remaining() != 0 {
        return Err(Error::new(format!("26.2 {name} has trailing bytes")));
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
            // 26.2's PositionMoveRotation sends position and velocity; 26.3
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
        return Err(Error::new(format!("26.2 {name} has trailing bytes")));
    }
    Ok(writer.data)
}

/// `CommonPlayerSpawnInfo` changed its two game mode fields from signed
/// bytes in 26.2 to VarInts in 26.3 (the previous mode uses 0 for none).
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
        return Err(Error::new("26.2 sent an invalid game mode"));
    }
    let mut writer = Writer::new();
    writer.data.extend_from_slice(&payload[..modes_start]);
    writer.varint(i32::from(mode));
    writer.varint(i32::from(previous) + 1);
    writer.data.extend_from_slice(reader.rest());
    Ok(writer.data)
}

/// 26.2's `FriendlyByteBuf.readBitSet` uses a VarInt-prefixed array of
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
                .ok_or_else(|| Error::new("26.2 chunk contains unknown block entity type"))?;
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
            "26.2 light packet has {} trailing bytes",
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
                        .ok_or_else(|| Error::new("26.2 chunk contains an unknown block state"))
                })
                .collect::<Result<Vec<_>>>()?;
            section.states = Paletted::from_values(&values, Strategy::Blocks, 16);
        } else {
            for old in &mut section.states.palette {
                *old = registry_26_2::block_state(*old)
                    .ok_or_else(|| Error::new("26.2 chunk contains an unknown block state"))?;
            }
        }
        section.write(&mut writer);
    }
    Ok(writer.data)
}

/// The 26.3 table is generated as id-to-name only; its inverse is needed for
/// clientbound 26.2 packets. Keep this match aligned with `ids::name`.
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
