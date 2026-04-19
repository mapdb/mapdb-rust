// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! nanprobe inserts IEEE 754 edge cases (NaN, -0.0, +0.0, +Inf, -Inf) into
//! float-keyed maps and float element sets, then reports observed behavior
//! in a canonical per-line format so outputs can be diffed across languages.

use mapdb_collections::hashmap::f32_i32_hash_map::F32I32HashMap;
use mapdb_collections::hashset::f32_hash_set::F32HashSet;

fn main() {
    println!("lang: rust");
    probe_map_nan();
    probe_map_neg_zero();
    probe_map_infinity();
    probe_set_nan();
    probe_set_neg_zero();
    probe_set_mixed();
}

fn probe_map_nan() {
    let mut m = F32I32HashMap::new();
    m.insert(f32::NAN, 1);
    println!("map_nan_size_after_put1: {}", m.len());

    m.insert(f32::NAN, 2);
    println!("map_nan_size_after_put2: {}", m.len());

    m.insert(f32::NAN, 3);
    println!("map_nan_size_after_put3: {}", m.len());

    let v = m.get(f32::NAN);
    println!("map_nan_get_found: {}", v.is_some());
    println!("map_nan_get_value: {}", v.unwrap_or(0));

    println!("map_nan_contains_key: {}", m.contains_key(f32::NAN));

    let removed = m.remove(f32::NAN);
    println!("map_nan_remove_found: {}", removed.is_some());
    println!("map_nan_size_after_remove: {}", m.len());
}

fn probe_map_neg_zero() {
    let mut m = F32I32HashMap::new();
    m.insert(0.0f32, 100);
    m.insert(-0.0f32, 200);

    println!("map_zero_size: {}", m.len());

    let v1 = m.get(0.0f32).unwrap_or(0);
    let v2 = m.get(-0.0f32).unwrap_or(0);
    println!("map_zero_get_pos: {}", v1);
    println!("map_zero_get_neg: {}", v2);

    // Which zero is stored? Check first iterated key's sign bit.
    let first_key = m.keys().next().unwrap_or(0.0f32);
    let bits = first_key.to_bits();
    let sign_bit_set = (bits & (1u32 << 31)) != 0;
    println!("map_zero_stored_negative: {}", sign_bit_set);
}

fn probe_map_infinity() {
    let mut m = F32I32HashMap::new();
    m.insert(f32::INFINITY, 111);
    m.insert(f32::NEG_INFINITY, 222);

    println!("map_inf_size: {}", m.len());

    let v1 = m.get(f32::INFINITY).unwrap_or(0);
    let v2 = m.get(f32::NEG_INFINITY).unwrap_or(0);
    println!("map_pinf_get: {}", v1);
    println!("map_ninf_get: {}", v2);

    println!("map_pinf_contains: {}", m.contains_key(f32::INFINITY));
    println!("map_ninf_contains: {}", m.contains_key(f32::NEG_INFINITY));
}

fn probe_set_nan() {
    let mut s = F32HashSet::new();
    s.add(f32::NAN);
    s.add(f32::NAN);
    s.add(f32::NAN);
    println!("set_nan_size: {}", s.len());
    println!("set_nan_contains: {}", s.contains(f32::NAN));
}

fn probe_set_neg_zero() {
    let mut s = F32HashSet::new();
    s.add(0.0f32);
    s.add(-0.0f32);
    println!("set_zero_size: {}", s.len());
    println!("set_pos_zero_contains: {}", s.contains(0.0f32));
    println!("set_neg_zero_contains: {}", s.contains(-0.0f32));
}

fn probe_set_mixed() {
    let mut s = F32HashSet::new();
    s.add(1.0f32);
    s.add(f32::NAN);
    s.add(f32::INFINITY);
    s.add(f32::NEG_INFINITY);
    s.add(0.0f32);
    println!("set_mixed_size: {}", s.len());
}
