//! Select only implemented, release-to-release packet adapters.
use std::collections::VecDeque;

use horizon_protocol::{Direction, State, connection::PacketTranslator};

const MAX_PACKETS: usize = 16;
const MAX_PACKET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Adapter {
    V26_1,
    V26_2,
}

#[derive(Clone, Copy)]
struct Edge {
    from: &'static str,
    to: &'static str,
    adapter: Adapter,
}

// Each edge must have a real adapter for these exact source and target
// protocols. A registry table alone does not make an edge usable.
const EDGES: &[Edge] = &[
    Edge {
        from: "26.3",
        to: "26.2",
        adapter: Adapter::V26_2,
    },
    Edge {
        from: "26.3",
        to: "26.1",
        adapter: Adapter::V26_1,
    },
];

pub(crate) fn route_to(target: &str) -> Option<Vec<Adapter>> {
    shortest_path(horizon_protocol::GAME_VERSION, target, EDGES)
}

fn available_targets() -> Vec<&'static str> {
    let mut queue = VecDeque::from([horizon_protocol::GAME_VERSION]);
    let mut seen = vec![horizon_protocol::GAME_VERSION];
    while let Some(version) = queue.pop_front() {
        for edge in EDGES.iter().filter(|edge| edge.from == version) {
            if !seen.contains(&edge.to) {
                seen.push(edge.to);
                queue.push_back(edge.to);
            }
        }
    }
    let mut targets = seen.into_iter().skip(1).collect::<Vec<_>>();
    targets.sort_unstable_by_key(|version| {
        version
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    });
    targets
}

fn shortest_path(start: &str, target: &str, edges: &[Edge]) -> Option<Vec<Adapter>> {
    if start == target {
        return Some(Vec::new());
    }
    let mut queue = VecDeque::from([start]);
    let mut visited = vec![start];
    let mut parents = Vec::new();
    while let Some(version) = queue.pop_front() {
        for edge in edges.iter().filter(|edge| edge.from == version) {
            if visited.contains(&edge.to) {
                continue;
            }
            visited.push(edge.to);
            parents.push((edge.from, edge.to, edge.adapter));
            if edge.to == target {
                let mut path = Vec::new();
                let mut at = target;
                while at != start {
                    let (from, _, adapter) = parents.iter().find(|(_, to, _)| *to == at)?;
                    path.push(*adapter);
                    at = from;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(edge.to);
        }
    }
    None
}

fn stages(route: &[Adapter], direction: Direction) -> Vec<Adapter> {
    let mut result = route.to_vec();
    if direction == Direction::Clientbound {
        result.reverse();
    }
    result
}

pub(crate) fn translate(
    route: &[Adapter],
    phase: State,
    direction: Direction,
    body: Vec<u8>,
) -> Result<Vec<Vec<u8>>, String> {
    let mut packets = vec![body];
    for adapter in stages(route, direction) {
        let mut next = Vec::new();
        for packet in packets {
            let converted = match adapter {
                Adapter::V26_1 => horizon_protocol::translate_26_1::Translator
                    .translate(phase, direction, &packet),
                Adapter::V26_2 => horizon_protocol::translate_26_2::Translator
                    .translate(phase, direction, &packet),
            }
            .map_err(|e| e.to_string())?;
            if next.len() + converted.len() > MAX_PACKETS
                || converted
                    .iter()
                    .any(|packet| packet.len() > MAX_PACKET_BYTES)
            {
                return Err("version bridge exceeded the packet output limit".into());
            }
            next.extend(converted);
        }
        packets = next;
    }
    Ok(packets)
}

pub(crate) fn unsupported_message(version: &str) -> String {
    if version.split('.').count() >= 2
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        let available = available_targets().join(", ");
        format!("Java release {version} is not supported yet; available: {available}")
    } else {
        "Version Bridge supports stable Java releases only; Beta, Alpha, Classic, snapshots and special versions are outside its scope".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_only_implemented_release_routes() {
        assert_eq!(route_to("26.1"), Some(vec![Adapter::V26_1]));
        assert_eq!(route_to("26.2"), Some(vec![Adapter::V26_2]));
        assert_eq!(route_to("1.20.6"), None);
        assert_eq!(route_to("b1.8.1"), None);
        assert_eq!(available_targets(), vec!["26.1", "26.2"]);
    }

    #[test]
    fn selects_shortest_chain_and_reverses_incoming_stages() {
        let edges = [
            Edge {
                from: "26.3",
                to: "26.2",
                adapter: Adapter::V26_2,
            },
            Edge {
                from: "26.2",
                to: "26.1",
                adapter: Adapter::V26_1,
            },
            Edge {
                from: "26.1",
                to: "26.3",
                adapter: Adapter::V26_1,
            },
        ];
        let route = shortest_path("26.3", "26.1", &edges).unwrap();
        assert_eq!(route, vec![Adapter::V26_2, Adapter::V26_1]);
        assert_eq!(stages(&route, Direction::Serverbound), route);
        assert_eq!(
            stages(&route, Direction::Clientbound),
            vec![Adapter::V26_1, Adapter::V26_2]
        );
    }

    #[test]
    fn distinguishes_future_releases_from_excluded_versions() {
        assert!(unsupported_message("1.20.6").contains("not supported yet"));
        for version in ["b1.8.1", "a1.2.6", "c0.30", "24w14a", "25w14craftmine"] {
            assert!(unsupported_message(version).contains("outside its scope"));
        }
    }
}
