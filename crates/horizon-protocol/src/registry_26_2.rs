//! Protocol ID mappings from Java Edition 26.2 to 26.3.
//! Generated from local vanilla datagen reports by tools/generate_registry_maps.py.
//! The reports are not included in the repository.
#![allow(missing_docs)]

#[derive(Clone, Copy)]
struct Range {
    first: i32,
    last: i32,
    delta: i32,
}
fn map(ranges: &[Range], old: i32) -> Option<i32> {
    let index = ranges.partition_point(|range| range.last < old);
    let range = ranges.get(index)?;
    (old >= range.first).then_some(old + range.delta)
}

// 32366 matching IDs in 33 runs.
const BLOCK_STATE: &[Range] = &[
    Range {
        first: 0,
        last: 26,
        delta: 0,
    },
    Range {
        first: 27,
        last: 84,
        delta: 1,
    },
    Range {
        first: 85,
        last: 162,
        delta: 3,
    },
    Range {
        first: 163,
        last: 197,
        delta: 6,
    },
    Range {
        first: 198,
        last: 224,
        delta: 9,
    },
    Range {
        first: 225,
        last: 251,
        delta: 12,
    },
    Range {
        first: 252,
        last: 503,
        delta: 15,
    },
    Range {
        first: 504,
        last: 2186,
        delta: 99,
    },
    Range {
        first: 2187,
        last: 2251,
        delta: 115,
    },
    Range {
        first: 2252,
        last: 2308,
        delta: 116,
    },
    Range {
        first: 2309,
        last: 3239,
        delta: 1492,
    },
    Range {
        first: 3240,
        last: 5622,
        delta: 1556,
    },
    Range {
        first: 5623,
        last: 5898,
        delta: 1588,
    },
    Range {
        first: 5899,
        last: 6610,
        delta: 1596,
    },
    Range {
        first: 6611,
        last: 6746,
        delta: 1660,
    },
    Range {
        first: 6747,
        last: 6878,
        delta: 1668,
    },
    Range {
        first: 6879,
        last: 7689,
        delta: 1670,
    },
    Range {
        first: 7690,
        last: 9492,
        delta: 1734,
    },
    Range {
        first: 9493,
        last: 10638,
        delta: 1742,
    },
    Range {
        first: 10639,
        last: 10890,
        delta: 1743,
    },
    Range {
        first: 10891,
        last: 12371,
        delta: 1767,
    },
    Range {
        first: 12372,
        last: 13383,
        delta: 1847,
    },
    Range {
        first: 13384,
        last: 13401,
        delta: 1853,
    },
    Range {
        first: 13402,
        last: 13407,
        delta: 1920,
    },
    Range {
        first: 13408,
        last: 13473,
        delta: 1847,
    },
    Range {
        first: 13474,
        last: 13479,
        delta: 3214,
    },
    Range {
        first: 13480,
        last: 13480,
        delta: 1841,
    },
    Range {
        first: 13481,
        last: 13739,
        delta: 1847,
    },
    Range {
        first: 13740,
        last: 14027,
        delta: 1879,
    },
    Range {
        first: 14028,
        last: 14571,
        delta: 1911,
    },
    Range {
        first: 14572,
        last: 14712,
        delta: 1975,
    },
    Range {
        first: 14713,
        last: 15045,
        delta: 1981,
    },
    Range {
        first: 15046,
        last: 32365,
        delta: 3357,
    },
];
pub fn block_state(old: i32) -> Option<i32> {
    map(BLOCK_STATE, old)
}

// 1537 matching IDs in 27 runs.
const ITEM: &[Range] = &[
    Range {
        first: 0,
        last: 71,
        delta: 0,
    },
    Range {
        first: 72,
        last: 84,
        delta: 1,
    },
    Range {
        first: 85,
        last: 169,
        delta: 2,
    },
    Range {
        first: 170,
        last: 183,
        delta: 3,
    },
    Range {
        first: 184,
        last: 194,
        delta: 4,
    },
    Range {
        first: 195,
        last: 206,
        delta: 5,
    },
    Range {
        first: 207,
        last: 217,
        delta: 6,
    },
    Range {
        first: 218,
        last: 231,
        delta: 9,
    },
    Range {
        first: 232,
        last: 255,
        delta: 10,
    },
    Range {
        first: 256,
        last: 276,
        delta: 42,
    },
    Range {
        first: 277,
        last: 306,
        delta: 43,
    },
    Range {
        first: 307,
        last: 340,
        delta: 44,
    },
    Range {
        first: 341,
        last: 380,
        delta: 45,
    },
    Range {
        first: 381,
        last: 477,
        delta: 46,
    },
    Range {
        first: 478,
        last: 657,
        delta: 47,
    },
    Range {
        first: 658,
        last: 787,
        delta: 79,
    },
    Range {
        first: 788,
        last: 803,
        delta: 80,
    },
    Range {
        first: 804,
        last: 816,
        delta: 81,
    },
    Range {
        first: 817,
        last: 837,
        delta: 82,
    },
    Range {
        first: 838,
        last: 857,
        delta: 83,
    },
    Range {
        first: 858,
        last: 908,
        delta: 84,
    },
    Range {
        first: 909,
        last: 1024,
        delta: 86,
    },
    Range {
        first: 1025,
        last: 1036,
        delta: 87,
    },
    Range {
        first: 1037,
        last: 1081,
        delta: 88,
    },
    Range {
        first: 1082,
        last: 1130,
        delta: 104,
    },
    Range {
        first: 1131,
        last: 1133,
        delta: 105,
    },
    Range {
        first: 1134,
        last: 1536,
        delta: 121,
    },
];
pub fn item(old: i32) -> Option<i32> {
    map(ITEM, old)
}

const ITEM_TO_OLD: &[Range] = &[
    Range {
        first: 0,
        last: 71,
        delta: 0,
    },
    Range {
        first: 73,
        last: 85,
        delta: -1,
    },
    Range {
        first: 87,
        last: 171,
        delta: -2,
    },
    Range {
        first: 173,
        last: 186,
        delta: -3,
    },
    Range {
        first: 188,
        last: 198,
        delta: -4,
    },
    Range {
        first: 200,
        last: 211,
        delta: -5,
    },
    Range {
        first: 213,
        last: 223,
        delta: -6,
    },
    Range {
        first: 227,
        last: 240,
        delta: -9,
    },
    Range {
        first: 242,
        last: 265,
        delta: -10,
    },
    Range {
        first: 298,
        last: 318,
        delta: -42,
    },
    Range {
        first: 320,
        last: 349,
        delta: -43,
    },
    Range {
        first: 351,
        last: 384,
        delta: -44,
    },
    Range {
        first: 386,
        last: 425,
        delta: -45,
    },
    Range {
        first: 427,
        last: 523,
        delta: -46,
    },
    Range {
        first: 525,
        last: 704,
        delta: -47,
    },
    Range {
        first: 737,
        last: 866,
        delta: -79,
    },
    Range {
        first: 868,
        last: 883,
        delta: -80,
    },
    Range {
        first: 885,
        last: 897,
        delta: -81,
    },
    Range {
        first: 899,
        last: 919,
        delta: -82,
    },
    Range {
        first: 921,
        last: 940,
        delta: -83,
    },
    Range {
        first: 942,
        last: 992,
        delta: -84,
    },
    Range {
        first: 995,
        last: 1110,
        delta: -86,
    },
    Range {
        first: 1112,
        last: 1123,
        delta: -87,
    },
    Range {
        first: 1125,
        last: 1169,
        delta: -88,
    },
    Range {
        first: 1186,
        last: 1234,
        delta: -104,
    },
    Range {
        first: 1236,
        last: 1238,
        delta: -105,
    },
    Range {
        first: 1255,
        last: 1657,
        delta: -121,
    },
];
pub fn item_to_old(new: i32) -> Option<i32> {
    map(ITEM_TO_OLD, new)
}

// 158 matching IDs in 3 runs.
const ENTITY_TYPE: &[Range] = &[
    Range {
        first: 0,
        last: 32,
        delta: 0,
    },
    Range {
        first: 33,
        last: 104,
        delta: 1,
    },
    Range {
        first: 105,
        last: 157,
        delta: 3,
    },
];
pub fn entity_type(old: i32) -> Option<i32> {
    map(ENTITY_TYPE, old)
}

// 125 matching IDs in 2 runs.
const PARTICLE_TYPE: &[Range] = &[
    Range {
        first: 0,
        last: 42,
        delta: 0,
    },
    Range {
        first: 43,
        last: 124,
        delta: 3,
    },
];
pub fn particle_type(old: i32) -> Option<i32> {
    map(PARTICLE_TYPE, old)
}

// 109 matching IDs in 5 runs.
const DATA_COMPONENT_TYPE: &[Range] = &[
    Range {
        first: 0,
        last: 39,
        delta: 0,
    },
    Range {
        first: 41,
        last: 41,
        delta: 1,
    },
    Range {
        first: 42,
        last: 44,
        delta: 3,
    },
    Range {
        first: 46,
        last: 81,
        delta: 2,
    },
    Range {
        first: 82,
        last: 110,
        delta: 6,
    },
];
pub fn data_component_type(old: i32) -> Option<i32> {
    map(DATA_COMPONENT_TYPE, old)
}

const DATA_COMPONENT_TYPE_TO_OLD: &[Range] = &[
    Range {
        first: 0,
        last: 39,
        delta: 0,
    },
    Range {
        first: 42,
        last: 42,
        delta: -1,
    },
    Range {
        first: 45,
        last: 47,
        delta: -3,
    },
    Range {
        first: 48,
        last: 83,
        delta: -2,
    },
    Range {
        first: 88,
        last: 116,
        delta: -6,
    },
];
pub fn data_component_type_to_old(new: i32) -> Option<i32> {
    map(DATA_COMPONENT_TYPE_TO_OLD, new)
}

// 49 matching IDs in 1 runs.
const BLOCK_ENTITY_TYPE: &[Range] = &[Range {
    first: 0,
    last: 48,
    delta: 0,
}];
pub fn block_entity_type(old: i32) -> Option<i32> {
    map(BLOCK_ENTITY_TYPE, old)
}

// 1968 matching IDs in 2 runs.
const SOUND_EVENT: &[Range] = &[
    Range {
        first: 0,
        last: 491,
        delta: 0,
    },
    Range {
        first: 492,
        last: 1967,
        delta: 4,
    },
];
pub fn sound_event(old: i32) -> Option<i32> {
    map(SOUND_EVENT, old)
}
