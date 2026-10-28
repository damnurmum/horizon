//! Local-only smoke probe for a release-version bridge. Run through tools/test_version_bridge_local.py.
use horizon_client::net::{Event, Session};
use horizon_mods::{Mods, Status};
use horizon_protocol::{
    entity_data::Value,
    item::Stack,
    packets::{
        Empty,
        play::{self, Clientbound, Serverbound},
    },
    registry_26_1,
    text::Language,
};
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let mut args = std::env::args().skip(1);
    let address = args.next().expect("server address");
    let version = args.next().expect("target release");
    let package = PathBuf::from(args.next().expect("version bridge hmod"));
    let mut mods = Mods::load(&[package], |_| true, |_| false);
    assert_eq!(mods.entries()[0].status, Status::Running);
    let translator = mods
        .network_translator(&address, Some(&version))
        .unwrap()
        .unwrap();
    let session = Session::connect_translated(
        &address,
        "BridgeProbe",
        None,
        3,
        Arc::new(Language::new()),
        translator,
    );
    let item_entity_type = registry_26_1::entity_type(71).unwrap();
    let (mut joined, mut creative, mut start) = (false, false, None);
    let (mut moved, mut sent_break, mut sent_slot, mut sent_drop) = (false, false, false, false);
    let (mut break_pos, mut break_ack, mut block_removed) = (None, false, false);
    let (mut drop_entity, mut drop_stack) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(40);
    let mut last_tick = Instant::now();
    let mut move_time = Instant::now();
    let mut slot_time = Instant::now();
    let mut position_time = Instant::now();
    while Instant::now() < deadline {
        for event in session.poll() {
            match event {
                Event::Joined { .. } => joined = true,
                Event::Packet(packet) => match *packet {
                    Clientbound::Login(ref p) => creative = p.spawn.game_type == 1,
                    Clientbound::PlayerPosition(ref p) => {
                        if moved {
                            panic!("server corrected movement to {:?}", p.position);
                        }
                        start = Some(p.position);
                        println!(
                            "START_POS {} {} {}",
                            p.position[0], p.position[1], p.position[2]
                        );
                        position_time = Instant::now();
                        session.send(Serverbound::AcceptTeleportation(
                            play::AcceptTeleportation {
                                id: p.teleport,
                                x: p.position[0],
                                y: p.position[1],
                                z: p.position[2],
                                yaw: p.yaw,
                                pitch: p.pitch,
                            },
                        ));
                        session.send(Serverbound::PlayerLoaded(Empty));
                    }
                    Clientbound::AddEntity(ref p) if p.kind == item_entity_type => {
                        drop_entity = Some(p.id);
                    }
                    Clientbound::BlockChangedAck(ref p) if sent_break && p.sequence == 1 => {
                        break_ack = true;
                    }
                    Clientbound::BlockUpdate(ref p)
                        if sent_break && Some(p.pos) == break_pos && p.state == 0 =>
                    {
                        block_removed = true;
                    }
                    Clientbound::SetEntityData(ref p) if Some(p.id) == drop_entity => {
                        for (_, _, value) in &p.values {
                            if let Value::Item(Some(stack)) = value {
                                drop_stack = Some((stack.item, stack.count));
                            }
                        }
                    }
                    _ => {}
                },
                Event::DecodeError(error) => panic!("packet decode error: {error}"),
                Event::Disconnected { reason, .. } => panic!("disconnected: {reason}"),
                _ => {}
            }
        }
        if joined
            && creative
            && let Some([x, y, z]) = start
        {
            if !moved && position_time.elapsed() > Duration::from_secs(1) {
                session.send(Serverbound::MovePlayerPos(play::MovePlayerPos {
                    x: x + 0.2,
                    y,
                    z,
                    flags: 1,
                }));
                session.send(Serverbound::Punch(Empty));
                moved = true;
                move_time = Instant::now();
                println!("sent movement and arm swing");
            }
            if moved && !sent_slot && move_time.elapsed() > Duration::from_millis(500) {
                let pos = [x.floor() as i32, y.floor() as i32 - 1, z.floor() as i32];
                session.send(Serverbound::PlayerAction(play::PlayerAction {
                    action: 0,
                    pos,
                    direction: 1,
                    sequence: 1,
                }));
                break_pos = Some(pos);
                sent_break = true;
                session.send(Serverbound::SetCarriedItem(play::SetCarriedItem {
                    slot: 0,
                }));
                session.send(Serverbound::SetCreativeModeSlot(
                    play::SetCreativeModeSlot {
                        slot: 36,
                        item: Some(Stack::new(1012, 2)),
                    },
                ));
                sent_slot = true;
                slot_time = Instant::now();
            }
            if sent_slot && !sent_drop && slot_time.elapsed() > Duration::from_millis(800) {
                session.send(Serverbound::PlayerAction(play::PlayerAction {
                    action: 5,
                    pos: [0; 3],
                    direction: 0,
                    sequence: 0,
                }));
                sent_drop = true;
            }
        }
        if joined && last_tick.elapsed() >= Duration::from_millis(50) {
            session.send(Serverbound::ClientTickEnd(Empty));
            last_tick = Instant::now();
        }
        if moved && break_ack && block_removed && drop_stack == Some((1012, 1)) {
            println!("PASS {version}: movement, block break, and dropped diamond decoded");
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "timeout: joined={joined} creative={creative} start={start:?} moved={moved} sent_break={sent_break} break_pos={break_pos:?} break_ack={break_ack} block_removed={block_removed} sent_slot={sent_slot} sent_drop={sent_drop} drop_entity={drop_entity:?} drop_stack={drop_stack:?}"
    );
}
