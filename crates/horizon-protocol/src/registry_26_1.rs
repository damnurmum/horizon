//! Protocol ID mappings from Java Edition 26.1 to 26.3.
//! Generated from ViaVersion/Mappings mapping JSON (mappings/), freely reusable per its README by tools/generate_registry_maps.py.
//! Source data is not included in the repository.
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

// 29873 matching IDs in 87 runs.
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
        last: 24686,
        delta: 3357,
    },
    Range {
        first: 24687,
        last: 25312,
        delta: 5830,
    },
    Range {
        first: 25313,
        last: 25314,
        delta: 5834,
    },
    Range {
        first: 25315,
        last: 25315,
        delta: 5837,
    },
    Range {
        first: 25316,
        last: 25316,
        delta: 5835,
    },
    Range {
        first: 25317,
        last: 25317,
        delta: 5833,
    },
    Range {
        first: 25318,
        last: 25318,
        delta: 5831,
    },
    Range {
        first: 25319,
        last: 25319,
        delta: 5841,
    },
    Range {
        first: 25320,
        last: 25320,
        delta: 5839,
    },
    Range {
        first: 25321,
        last: 25321,
        delta: 5837,
    },
    Range {
        first: 25322,
        last: 25322,
        delta: 5835,
    },
    Range {
        first: 25323,
        last: 25323,
        delta: 5841,
    },
    Range {
        first: 25324,
        last: 25324,
        delta: 5839,
    },
    Range {
        first: 25325,
        last: 25325,
        delta: 5837,
    },
    Range {
        first: 25326,
        last: 25326,
        delta: 5835,
    },
    Range {
        first: 25327,
        last: 25406,
        delta: 6078,
    },
    Range {
        first: 25407,
        last: 25486,
        delta: 5918,
    },
    Range {
        first: 25487,
        last: 25566,
        delta: 5758,
    },
    Range {
        first: 25567,
        last: 25646,
        delta: 5598,
    },
    Range {
        first: 25647,
        last: 25652,
        delta: 6176,
    },
    Range {
        first: 25653,
        last: 25658,
        delta: 6164,
    },
    Range {
        first: 25659,
        last: 25664,
        delta: 6152,
    },
    Range {
        first: 25665,
        last: 25670,
        delta: 6140,
    },
    Range {
        first: 25671,
        last: 25671,
        delta: 5472,
    },
    Range {
        first: 25672,
        last: 25672,
        delta: 5473,
    },
    Range {
        first: 25673,
        last: 25673,
        delta: 5471,
    },
    Range {
        first: 25674,
        last: 25674,
        delta: 5472,
    },
    Range {
        first: 25675,
        last: 25675,
        delta: 5481,
    },
    Range {
        first: 25676,
        last: 25676,
        delta: 5479,
    },
    Range {
        first: 25677,
        last: 25677,
        delta: 5477,
    },
    Range {
        first: 25678,
        last: 25678,
        delta: 5475,
    },
    Range {
        first: 25679,
        last: 25758,
        delta: 6046,
    },
    Range {
        first: 25759,
        last: 25838,
        delta: 5886,
    },
    Range {
        first: 25839,
        last: 25918,
        delta: 5726,
    },
    Range {
        first: 25919,
        last: 25998,
        delta: 5566,
    },
    Range {
        first: 25999,
        last: 26004,
        delta: 5848,
    },
    Range {
        first: 26005,
        last: 26010,
        delta: 5836,
    },
    Range {
        first: 26011,
        last: 26016,
        delta: 5824,
    },
    Range {
        first: 26017,
        last: 26022,
        delta: 5812,
    },
    Range {
        first: 26023,
        last: 26150,
        delta: 5830,
    },
    Range {
        first: 26151,
        last: 26214,
        delta: 5894,
    },
    Range {
        first: 26215,
        last: 26278,
        delta: 5766,
    },
    Range {
        first: 26279,
        last: 26406,
        delta: 5830,
    },
    Range {
        first: 26407,
        last: 26470,
        delta: 5894,
    },
    Range {
        first: 26471,
        last: 26534,
        delta: 5766,
    },
    Range {
        first: 26535,
        last: 26662,
        delta: 5830,
    },
    Range {
        first: 26663,
        last: 26726,
        delta: 5894,
    },
    Range {
        first: 26727,
        last: 26790,
        delta: 5766,
    },
    Range {
        first: 26791,
        last: 26918,
        delta: 5830,
    },
    Range {
        first: 26919,
        last: 26982,
        delta: 5894,
    },
    Range {
        first: 26983,
        last: 27046,
        delta: 5766,
    },
    Range {
        first: 27047,
        last: 27734,
        delta: 5830,
    },
    Range {
        first: 27735,
        last: 27754,
        delta: 5831,
    },
    Range {
        first: 27755,
        last: 27755,
        delta: 5810,
    },
    Range {
        first: 27756,
        last: 29872,
        delta: 5850,
    },
];
pub fn block_state(old: i32) -> Option<i32> {
    map(BLOCK_STATE, old)
}

// 1506 matching IDs in 43 runs.
const ITEM: &[Range] = &[
    Range {
        first: 0,
        last: 25,
        delta: 0,
    },
    Range {
        first: 26,
        last: 44,
        delta: 27,
    },
    Range {
        first: 45,
        last: 57,
        delta: 28,
    },
    Range {
        first: 58,
        last: 91,
        delta: 29,
    },
    Range {
        first: 92,
        last: 94,
        delta: 36,
    },
    Range {
        first: 95,
        last: 97,
        delta: 26,
    },
    Range {
        first: 98,
        last: 101,
        delta: 33,
    },
    Range {
        first: 102,
        last: 105,
        delta: 37,
    },
    Range {
        first: 106,
        last: 109,
        delta: 41,
    },
    Range {
        first: 110,
        last: 113,
        delta: 45,
    },
    Range {
        first: 114,
        last: 117,
        delta: 10,
    },
    Range {
        first: 118,
        last: 121,
        delta: 17,
    },
    Range {
        first: 122,
        last: 125,
        delta: 21,
    },
    Range {
        first: 126,
        last: 129,
        delta: 25,
    },
    Range {
        first: 130,
        last: 142,
        delta: 29,
    },
    Range {
        first: 143,
        last: 156,
        delta: 30,
    },
    Range {
        first: 157,
        last: 167,
        delta: 31,
    },
    Range {
        first: 168,
        last: 179,
        delta: 32,
    },
    Range {
        first: 180,
        last: 190,
        delta: 33,
    },
    Range {
        first: 191,
        last: 204,
        delta: 36,
    },
    Range {
        first: 205,
        last: 228,
        delta: 37,
    },
    Range {
        first: 229,
        last: 249,
        delta: 69,
    },
    Range {
        first: 250,
        last: 279,
        delta: 70,
    },
    Range {
        first: 280,
        last: 313,
        delta: 71,
    },
    Range {
        first: 314,
        last: 353,
        delta: 72,
    },
    Range {
        first: 354,
        last: 450,
        delta: 73,
    },
    Range {
        first: 451,
        last: 630,
        delta: 74,
    },
    Range {
        first: 631,
        last: 760,
        delta: 106,
    },
    Range {
        first: 761,
        last: 776,
        delta: 107,
    },
    Range {
        first: 777,
        last: 789,
        delta: 108,
    },
    Range {
        first: 790,
        last: 810,
        delta: 109,
    },
    Range {
        first: 811,
        last: 830,
        delta: 110,
    },
    Range {
        first: 831,
        last: 881,
        delta: 111,
    },
    Range {
        first: 882,
        last: 997,
        delta: 113,
    },
    Range {
        first: 998,
        last: 1009,
        delta: 114,
    },
    Range {
        first: 1010,
        last: 1024,
        delta: 115,
    },
    Range {
        first: 1025,
        last: 1053,
        delta: 116,
    },
    Range {
        first: 1054,
        last: 1102,
        delta: 132,
    },
    Range {
        first: 1103,
        last: 1105,
        delta: 133,
    },
    Range {
        first: 1106,
        last: 1166,
        delta: 149,
    },
    Range {
        first: 1167,
        last: 1312,
        delta: 150,
    },
    Range {
        first: 1313,
        last: 1420,
        delta: 151,
    },
    Range {
        first: 1421,
        last: 1505,
        delta: 152,
    },
];
pub fn item(old: i32) -> Option<i32> {
    map(ITEM, old)
}

const ITEM_TO_OLD: &[Range] = &[
    Range {
        first: 0,
        last: 25,
        delta: 0,
    },
    Range {
        first: 53,
        last: 71,
        delta: -27,
    },
    Range {
        first: 73,
        last: 85,
        delta: -28,
    },
    Range {
        first: 87,
        last: 120,
        delta: -29,
    },
    Range {
        first: 121,
        last: 123,
        delta: -26,
    },
    Range {
        first: 124,
        last: 127,
        delta: -10,
    },
    Range {
        first: 128,
        last: 130,
        delta: -36,
    },
    Range {
        first: 131,
        last: 134,
        delta: -33,
    },
    Range {
        first: 135,
        last: 138,
        delta: -17,
    },
    Range {
        first: 139,
        last: 142,
        delta: -37,
    },
    Range {
        first: 143,
        last: 146,
        delta: -21,
    },
    Range {
        first: 147,
        last: 150,
        delta: -41,
    },
    Range {
        first: 151,
        last: 154,
        delta: -25,
    },
    Range {
        first: 155,
        last: 158,
        delta: -45,
    },
    Range {
        first: 159,
        last: 171,
        delta: -29,
    },
    Range {
        first: 173,
        last: 186,
        delta: -30,
    },
    Range {
        first: 188,
        last: 198,
        delta: -31,
    },
    Range {
        first: 200,
        last: 211,
        delta: -32,
    },
    Range {
        first: 213,
        last: 223,
        delta: -33,
    },
    Range {
        first: 227,
        last: 240,
        delta: -36,
    },
    Range {
        first: 242,
        last: 265,
        delta: -37,
    },
    Range {
        first: 298,
        last: 318,
        delta: -69,
    },
    Range {
        first: 320,
        last: 349,
        delta: -70,
    },
    Range {
        first: 351,
        last: 384,
        delta: -71,
    },
    Range {
        first: 386,
        last: 425,
        delta: -72,
    },
    Range {
        first: 427,
        last: 523,
        delta: -73,
    },
    Range {
        first: 525,
        last: 704,
        delta: -74,
    },
    Range {
        first: 737,
        last: 866,
        delta: -106,
    },
    Range {
        first: 868,
        last: 883,
        delta: -107,
    },
    Range {
        first: 885,
        last: 897,
        delta: -108,
    },
    Range {
        first: 899,
        last: 919,
        delta: -109,
    },
    Range {
        first: 921,
        last: 940,
        delta: -110,
    },
    Range {
        first: 942,
        last: 992,
        delta: -111,
    },
    Range {
        first: 995,
        last: 1110,
        delta: -113,
    },
    Range {
        first: 1112,
        last: 1123,
        delta: -114,
    },
    Range {
        first: 1125,
        last: 1139,
        delta: -115,
    },
    Range {
        first: 1141,
        last: 1169,
        delta: -116,
    },
    Range {
        first: 1186,
        last: 1234,
        delta: -132,
    },
    Range {
        first: 1236,
        last: 1238,
        delta: -133,
    },
    Range {
        first: 1255,
        last: 1315,
        delta: -149,
    },
    Range {
        first: 1317,
        last: 1462,
        delta: -150,
    },
    Range {
        first: 1464,
        last: 1571,
        delta: -151,
    },
    Range {
        first: 1573,
        last: 1657,
        delta: -152,
    },
];
pub fn item_to_old(new: i32) -> Option<i32> {
    map(ITEM_TO_OLD, new)
}

// 157 matching IDs in 4 runs.
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
        last: 129,
        delta: 3,
    },
    Range {
        first: 130,
        last: 156,
        delta: 4,
    },
];
pub fn entity_type(old: i32) -> Option<i32> {
    map(ENTITY_TYPE, old)
}

// 117 matching IDs in 3 runs.
const PARTICLE_TYPE: &[Range] = &[
    Range {
        first: 0,
        last: 3,
        delta: 0,
    },
    Range {
        first: 4,
        last: 35,
        delta: 7,
    },
    Range {
        first: 36,
        last: 116,
        delta: 10,
    },
];
pub fn particle_type(old: i32) -> Option<i32> {
    map(PARTICLE_TYPE, old)
}

// 108 matching IDs in 6 runs.
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
        last: 77,
        delta: 2,
    },
    Range {
        first: 78,
        last: 80,
        delta: 3,
    },
    Range {
        first: 81,
        last: 109,
        delta: 7,
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
        last: 79,
        delta: -2,
    },
    Range {
        first: 81,
        last: 83,
        delta: -3,
    },
    Range {
        first: 88,
        last: 116,
        delta: -7,
    },
];
pub fn data_component_type_to_old(new: i32) -> Option<i32> {
    map(DATA_COMPONENT_TYPE_TO_OLD, new)
}

// 48 matching IDs in 2 runs.
const BLOCK_ENTITY_TYPE: &[Range] = &[
    Range {
        first: 0,
        last: 24,
        delta: 0,
    },
    Range {
        first: 26,
        last: 48,
        delta: -1,
    },
];
pub fn block_entity_type(old: i32) -> Option<i32> {
    map(BLOCK_ENTITY_TYPE, old)
}

// 1902 matching IDs in 7 runs.
const SOUND_EVENT: &[Range] = &[
    Range {
        first: 0,
        last: 227,
        delta: 0,
    },
    Range {
        first: 228,
        last: 233,
        delta: 1,
    },
    Range {
        first: 234,
        last: 489,
        delta: 2,
    },
    Range {
        first: 490,
        last: 1030,
        delta: 6,
    },
    Range {
        first: 1031,
        last: 1075,
        delta: 7,
    },
    Range {
        first: 1076,
        last: 1604,
        delta: 8,
    },
    Range {
        first: 1605,
        last: 1901,
        delta: 14,
    },
];
pub fn sound_event(old: i32) -> Option<i32> {
    map(SOUND_EVENT, old)
}
