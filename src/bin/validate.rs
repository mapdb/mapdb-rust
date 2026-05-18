// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Cross-language validation runner. Reads a JSON scenario file, runs the
//! described operations through Rust collections, and prints the assertion
//! outputs in the canonical per-line `<key>: <value>` format consumed by
//! the cross-language validation harness.
//!
//! Routed through the generic collections (OpenHashMap, OpenHashSet, Vec,
//! BTreeMap, BTreeSet) — same observable behaviour as the old per-primitive
//! types but a single algorithm body.

use mapdb_collections::{HashableF32, OpenHashMap, OpenHashSet};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

fn parse_f32(v: &Value) -> f32 {
    if let Some(s) = v.as_str() {
        parse_f32_label(s)
    } else if let Some(n) = v.as_f64() {
        n as f32
    } else {
        panic!("expected f32 value, got {:?}", v);
    }
}

fn format_f32(v: f32) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v == f32::INFINITY {
        "Infinity".to_string()
    } else if v == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else if v == 0.0 && v.is_sign_negative() {
        // ±0 are bit-pattern-distinct in this project; preserve the sign.
        "-0.0".to_string()
    } else if v == v.trunc() && v.abs() < 1e16 {
        // Match Java/Go's "3.0" rendering for integer-valued floats.
        format!("{}.0", v as i64)
    } else {
        format!("{}", v)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: validate <scenario.json>");
        std::process::exit(1);
    }
    let path = &args[1];
    let text = fs::read_to_string(path).expect("failed to read scenario file");
    let scenario: Value = serde_json::from_str(&text).expect("failed to parse JSON");

    let name = scenario["name"].as_str().expect("missing name");
    let collection = scenario["collection"].as_str().expect("missing collection");
    let operations = scenario["operations"]
        .as_array()
        .expect("missing operations");
    let assertions = scenario["assertions"]
        .as_object()
        .expect("missing assertions");

    println!("=== scenario: {} ===", name);

    match collection {
        "HashMap<i32, i32>" => run_hashmap(operations, assertions),
        "ArrayList<i32>" => run_arraylist(operations, assertions),
        "HashSet<i32>" => run_hashset(operations, assertions, &scenario),
        "HashBag<i32>" => run_hashbag(operations, assertions),
        "TreeSet<i32>" => run_treeset(operations, assertions),
        "TreeMap<i32, i32>" => run_treemap(operations, assertions),
        "HashMap<f32, i32>" => run_f32_hashmap(operations, assertions),
        "HashSet<f32>" => run_f32_hashset(operations, assertions),
        "ArrayList<f32>" => run_f32_arraylist(operations, assertions),
        other => {
            eprintln!("unsupported collection type: {}", other);
            std::process::exit(1);
        }
    }
}

fn format_array(v: &[i32]) -> String {
    let parts: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

// ---- HashMap<i32, i32> ---------------------------------------------------

fn run_hashmap(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut map: OpenHashMap<i32, i32> = OpenHashMap::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = op["key"].as_i64().unwrap() as i32;
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = op["key"].as_i64().unwrap() as i32;
                map.remove(&k);
            }
            "addToValue" => {
                let k = op["key"].as_i64().unwrap() as i32;
                let delta = op["delta"].as_i64().unwrap() as i32;
                let cur = map.get(&k).copied().unwrap_or(0);
                map.insert(k, cur.wrapping_add(delta));
            }
            "clear" => map.clear(),
            other => panic!("unknown hashmap op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue; // Scenario authors use "comment" for doc strings; skip.
        }
        println!("{}: {}", key, eval_map_assertion(key, &map));
    }
}

fn eval_map_assertion(key: &str, map: &OpenHashMap<i32, i32>) -> String {
    match key {
        "size" => map.len().to_string(),
        "is_empty" => map.is_empty().to_string(),
        "sorted_keys" => {
            let mut keys: Vec<i32> = map.iter().map(|(k, _)| *k).collect();
            keys.sort();
            format_array(&keys)
        }
        "sorted_values" => {
            let mut vals: Vec<i32> = map.iter().map(|(_, v)| *v).collect();
            vals.sort();
            format_array(&vals)
        }
        "min" => {
            let mut keys: Vec<i32> = map.iter().map(|(k, _)| *k).collect();
            keys.sort();
            keys.first()
                .map(|k| k.to_string())
                .unwrap_or_else(|| "null".into())
        }
        "max" => {
            let mut keys: Vec<i32> = map.iter().map(|(k, _)| *k).collect();
            keys.sort();
            keys.last()
                .map(|k| k.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("get_") => {
            let k: i32 = key[4..].parse().unwrap();
            map.get(&k)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("contains_") => {
            let k: i32 = key[9..].parse().unwrap();
            map.contains_key(&k).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- ArrayList<i32> -------------------------------------------------------

fn run_arraylist(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut list: Vec<i32> = Vec::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => list.push(op["value"].as_i64().unwrap() as i32),
            "add_at" => {
                let idx = op["index"].as_u64().unwrap() as usize;
                let v = op["value"].as_i64().unwrap() as i32;
                list.insert(idx, v);
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                if let Some(i) = list.iter().position(|x| *x == v) {
                    list.remove(i);
                }
            }
            "clear" => list.clear(),
            other => panic!("unknown arraylist op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        println!("{}: {}", key, eval_list_assertion(key, &list));
    }
}

fn eval_list_assertion(key: &str, list: &Vec<i32>) -> String {
    match key {
        "size" => list.len().to_string(),
        "is_empty" => list.is_empty().to_string(),
        "sum" => {
            // Wrapping i32 sum — matches Java/Go behaviour. The cross-language
            // assertions rely on this; see scenarios/06-overflow/i32_sum_overflow.json.
            let mut acc: i32 = 0;
            for &v in list {
                acc = acc.wrapping_add(v);
            }
            acc.to_string()
        }
        "inject_into_wrapping_product" | "product" => {
            let mut acc: i32 = 1;
            for &v in list {
                acc = acc.wrapping_mul(v);
            }
            acc.to_string()
        }
        "max_minus_min" => match (list.iter().min(), list.iter().max()) {
            (Some(min), Some(max)) => max.wrapping_sub(*min).to_string(),
            _ => "null".to_string(),
        },
        "min" => list
            .iter()
            .min()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into()),
        "max" => list
            .iter()
            .max()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into()),
        "to_sorted_array" => {
            let mut v = list.clone();
            v.sort();
            format_array(&v)
        }
        "inject_into_sum" => list.iter().fold(0i64, |a, &v| a + v as i64).to_string(),
        "inject_into_product" => list.iter().fold(1i64, |a, &v| a * v as i64).to_string(),
        _ if key.starts_with("get_at_") => {
            let idx: usize = key[7..].parse().unwrap();
            list.get(idx)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            list.contains(&v).to_string()
        }
        _ if key.starts_with("select_gt_") => {
            let t: i32 = key[10..].parse().unwrap();
            let mut v: Vec<i32> = list.iter().copied().filter(|x| *x > t).collect();
            v.sort();
            format_array(&v)
        }
        _ if key.starts_with("reject_gt_") => {
            let t: i32 = key[10..].parse().unwrap();
            let mut v: Vec<i32> = list.iter().copied().filter(|x| *x <= t).collect();
            v.sort();
            format_array(&v)
        }
        _ if key.starts_with("detect_gt_") => {
            let t: i32 = key[10..].parse().unwrap();
            list.iter()
                .find(|&&v| v > t)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("count_gt_") => {
            let t: i32 = key[9..].parse().unwrap();
            list.iter().filter(|&&v| v > t).count().to_string()
        }
        _ if key.starts_with("count_lt_") => {
            let t: i32 = key[9..].parse().unwrap();
            list.iter().filter(|&&v| v < t).count().to_string()
        }
        "count_even" => list.iter().filter(|&&v| v % 2 == 0).count().to_string(),
        "count_odd" => list.iter().filter(|&&v| v % 2 != 0).count().to_string(),
        _ if key.starts_with("any_satisfy_gt_") => {
            let t: i32 = key[15..].parse().unwrap();
            list.iter().any(|&v| v > t).to_string()
        }
        _ if key.starts_with("all_satisfy_gt_") => {
            let t: i32 = key[15..].parse().unwrap();
            list.iter().all(|&v| v > t).to_string()
        }
        _ if key.starts_with("none_satisfy_gt_") => {
            let t: i32 = key[16..].parse().unwrap();
            (!list.iter().any(|&v| v > t)).to_string()
        }
        _ if key.starts_with("none_satisfy_lt_") => {
            let t: i32 = key[16..].parse().unwrap();
            (!list.iter().any(|&v| v < t)).to_string()
        }
        "any_satisfy_even" => list.iter().any(|&v| v % 2 == 0).to_string(),
        "all_satisfy_even" => list.iter().all(|&v| v % 2 == 0).to_string(),
        "none_satisfy_odd" => (!list.iter().any(|&v| v % 2 != 0)).to_string(),
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- HashSet<i32> ---------------------------------------------------------

fn run_hashset(
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario: &Value,
) {
    let mut set: OpenHashSet<i32> = OpenHashSet::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.add(op["value"].as_i64().unwrap() as i32);
            }
            "remove" => {
                set.remove(&(op["value"].as_i64().unwrap() as i32));
            }
            "clear" => set.clear(),
            other => panic!("unknown hashset op: {}", other),
        }
    }
    let other_set = scenario.get("other").map(|spec| {
        let mut other: OpenHashSet<i32> = OpenHashSet::new();
        if let Some(ops) = spec["operations"].as_array() {
            for op in ops {
                if let "add" = op["op"].as_str().unwrap() {
                    other.add(op["value"].as_i64().unwrap() as i32);
                }
            }
        }
        other
    });
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        println!(
            "{}: {}",
            key,
            eval_set_assertion(key, &set, other_set.as_ref())
        );
    }
}

fn eval_set_assertion(
    key: &str,
    set: &OpenHashSet<i32>,
    other: Option<&OpenHashSet<i32>>,
) -> String {
    match key {
        "size" => set.len().to_string(),
        "is_empty" => set.is_empty().to_string(),
        "to_sorted_array" => {
            let mut v: Vec<i32> = set.iter().copied().collect();
            v.sort();
            format_array(&v)
        }
        "union_sorted" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set.iter().chain(o.iter()).copied().collect();
            v.sort();
            v.dedup();
            format_array(&v)
        }
        "intersect_sorted" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set.iter().copied().filter(|x| o.contains(x)).collect();
            v.sort();
            format_array(&v)
        }
        "difference_sorted" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set.iter().copied().filter(|x| !o.contains(x)).collect();
            v.sort();
            format_array(&v)
        }
        "symmetric_difference_sorted" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set
                .iter()
                .copied()
                .filter(|x| !o.contains(x))
                .chain(o.iter().copied().filter(|x| !set.contains(x)))
                .collect();
            v.sort();
            format_array(&v)
        }
        "union_size" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set.iter().chain(o.iter()).copied().collect();
            v.sort();
            v.dedup();
            v.len().to_string()
        }
        "intersect_size" if other.is_some() => {
            let o = other.unwrap();
            set.iter().filter(|x| o.contains(x)).count().to_string()
        }
        "difference_size" if other.is_some() => {
            let o = other.unwrap();
            set.iter().filter(|x| !o.contains(x)).count().to_string()
        }
        "symmetric_difference_size" if other.is_some() => {
            let o = other.unwrap();
            let mut v: Vec<i32> = set
                .iter()
                .copied()
                .filter(|x| !o.contains(x))
                .chain(o.iter().copied().filter(|x| !set.contains(x)))
                .collect();
            v.sort();
            v.dedup();
            v.len().to_string()
        }
        "other_size" if other.is_some() => other.unwrap().len().to_string(),
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            set.contains(&v).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- HashBag<i32>  → modelled as OpenHashMap<i32, usize> -----------------

fn run_hashbag(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut bag: OpenHashMap<i32, usize> = OpenHashMap::new();
    let mut total: usize = 0;
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                let v = op["value"].as_i64().unwrap() as i32;
                let next = bag.get(&v).copied().unwrap_or(0) + 1;
                bag.insert(v, next);
                total += 1;
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                if let Some(&cur) = bag.get(&v) {
                    if cur <= 1 {
                        bag.remove(&v);
                    } else {
                        bag.insert(v, cur - 1);
                    }
                    total -= 1;
                }
            }
            "clear" => {
                bag.clear();
                total = 0;
            }
            other => panic!("unknown hashbag op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        println!("{}: {}", key, eval_bag_assertion(key, &bag, total));
    }
}

fn eval_bag_assertion(key: &str, bag: &OpenHashMap<i32, usize>, total: usize) -> String {
    match key {
        "size" => total.to_string(),
        "size_distinct" => bag.len().to_string(),
        "is_empty" => (total == 0).to_string(),
        "sorted_distinct" => {
            let mut keys: Vec<i32> = bag.iter().map(|(k, _)| *k).collect();
            keys.sort();
            format_array(&keys)
        }
        "to_sorted_array" => {
            // Flatten the bag back to a sorted array including duplicates.
            let mut flat: Vec<i32> = Vec::with_capacity(total);
            for (&k, &count) in bag.iter() {
                for _ in 0..count {
                    flat.push(k);
                }
            }
            flat.sort();
            format_array(&flat)
        }
        _ if key.starts_with("occurrences_") => {
            let v: i32 = key[12..].parse().unwrap();
            bag.get(&v).copied().unwrap_or(0).to_string()
        }
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            bag.contains_key(&v).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- TreeSet<i32> ---------------------------------------------------------

fn run_treeset(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut set: BTreeSet<i32> = BTreeSet::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.insert(op["value"].as_i64().unwrap() as i32);
            }
            "remove" => {
                set.remove(&(op["value"].as_i64().unwrap() as i32));
            }
            "clear" => set.clear(),
            other => panic!("unknown treeset op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => set.len().to_string(),
            "is_empty" => set.is_empty().to_string(),
            "min" => set
                .iter()
                .next()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
            "max" => set
                .iter()
                .next_back()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
            "to_sorted_array" => {
                let v: Vec<i32> = set.iter().copied().collect();
                format_array(&v)
            }
            _ if key.starts_with("contains_") => {
                let k: i32 = key[9..].parse().unwrap();
                set.contains(&k).to_string()
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        println!("{}: {}", key, v);
    }
}

// ---- TreeMap<i32, i32> ----------------------------------------------------

fn run_treemap(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut map: BTreeMap<i32, i32> = BTreeMap::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = op["key"].as_i64().unwrap() as i32;
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = op["key"].as_i64().unwrap() as i32;
                map.remove(&k);
            }
            "clear" => map.clear(),
            other => panic!("unknown treemap op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => map.len().to_string(),
            "is_empty" => map.is_empty().to_string(),
            "min" => map
                .iter()
                .next()
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "null".into()),
            "max" => map
                .iter()
                .next_back()
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "null".into()),
            "sorted_keys" => {
                let v: Vec<i32> = map.keys().copied().collect();
                format_array(&v)
            }
            "sorted_values" => {
                let v: Vec<i32> = map.values().copied().collect();
                format_array(&v)
            }
            _ if key.starts_with("get_") => {
                let k: i32 = key[4..].parse().unwrap();
                map.get(&k)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".into())
            }
            _ if key.starts_with("contains_") => {
                let k: i32 = key[9..].parse().unwrap();
                map.contains_key(&k).to_string()
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        println!("{}: {}", key, v);
    }
}

// ---- HashMap<f32, i32> ----------------------------------------------------

fn run_f32_hashmap(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut map: OpenHashMap<HashableF32, i32> = OpenHashMap::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = HashableF32(parse_f32(&op["key"]));
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = HashableF32(parse_f32(&op["key"]));
                map.remove(&k);
            }
            "clear" => map.clear(),
            other => panic!("unknown f32-hashmap op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        let val = match key.as_str() {
            "size" => map.len().to_string(),
            "is_empty" => map.is_empty().to_string(),
            k if k.starts_with("get_") => {
                let raw = &k[4..];
                let probe = HashableF32(parse_f32_label(raw));
                map.get(&probe)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".into())
            }
            k if k.starts_with("contains_") => {
                let raw = &k[9..];
                let probe = HashableF32(parse_f32_label(raw));
                map.contains_key(&probe).to_string()
            }
            "sorted_keys" => {
                let mut keys: Vec<HashableF32> = map.iter().map(|(k, _)| *k).collect();
                keys.sort();
                let parts: Vec<String> = keys
                    .into_iter()
                    .map(|x| format!("\"{}\"", format_f32(x.0)))
                    .collect();
                format!("[{}]", parts.join(","))
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        println!("{}: {}", key, val);
    }
}

fn parse_f32_label(s: &str) -> f32 {
    match s {
        "NaN" => f32::NAN,
        "Infinity" | "+Infinity" => f32::INFINITY,
        "-Infinity" => f32::NEG_INFINITY,
        "pos_zero" => 0.0_f32,
        "neg_zero" => -0.0_f32,
        other => other.parse::<f32>().expect("invalid f32 literal in key"),
    }
}

// ---- HashSet<f32> ---------------------------------------------------------

fn run_f32_hashset(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut set: OpenHashSet<HashableF32> = OpenHashSet::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.add(HashableF32(parse_f32(&op["value"])));
            }
            "remove" => {
                set.remove(&HashableF32(parse_f32(&op["value"])));
            }
            "clear" => set.clear(),
            other => panic!("unknown f32-hashset op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        let val = match key.as_str() {
            "size" => set.len().to_string(),
            "is_empty" => set.is_empty().to_string(),
            k if k.starts_with("contains_") => {
                let raw = &k[9..];
                let probe = HashableF32(parse_f32_label(raw));
                set.contains(&probe).to_string()
            }
            "sorted_values" | "to_sorted_array" => {
                let mut v: Vec<HashableF32> = set.iter().copied().collect();
                v.sort();
                let parts: Vec<String> = v
                    .into_iter()
                    .map(|x| format!("\"{}\"", format_f32(x.0)))
                    .collect();
                format!("[{}]", parts.join(","))
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        println!("{}: {}", key, val);
    }
}

// ---- ArrayList<f32> -------------------------------------------------------

fn run_f32_arraylist(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut list: Vec<f32> = Vec::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => list.push(parse_f32(&op["value"])),
            "clear" => list.clear(),
            other => panic!("unknown f32-arraylist op: {}", other),
        }
    }
    for (key, _) in assertions {
        if key == "comment" {
            continue;
        }
        let val = match key.as_str() {
            "size" => list.len().to_string(),
            "is_empty" => list.is_empty().to_string(),
            "sum" => {
                let s: f32 = list.iter().copied().sum();
                format_f32(s)
            }
            "min" => list
                .iter()
                .copied()
                .min_by(|a, b| a.total_cmp(b))
                .map(format_f32)
                .unwrap_or_else(|| "null".into()),
            "max" => list
                .iter()
                .copied()
                .max_by(|a, b| a.total_cmp(b))
                .map(format_f32)
                .unwrap_or_else(|| "null".into()),
            "sorted" | "to_sorted_array" => {
                let mut v = list.clone();
                v.sort_by(|a, b| a.total_cmp(b));
                let parts: Vec<String> = v.into_iter().map(format_f32).collect();
                format!("[{}]", parts.join(","))
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        println!("{}: {}", key, val);
    }
}
