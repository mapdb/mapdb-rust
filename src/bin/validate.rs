// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

use mapdb_collections::arraylist::i32_array_list::I32ArrayList;
use mapdb_collections::bag::i32_hash_bag::I32HashBag;
use mapdb_collections::hashmap::i32_i32_hash_map::I32I32HashMap;
use mapdb_collections::hashset::i32_hash_set::I32HashSet;
use mapdb_collections::treemap::i32_i32_tree_map::I32I32TreeMap;
use mapdb_collections::treeset::i32_tree_set::I32TreeSet;

use serde_json::Value;
use std::fs;

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
        "HashMap<i32, i32>" => run_hashmap(operations, assertions, &scenario),
        "ArrayList<i32>" => run_arraylist(operations, assertions),
        "HashSet<i32>" => run_hashset(operations, assertions, &scenario),
        "HashBag<i32>" => run_hashbag(operations, assertions),
        "TreeSet<i32>" => run_treeset(operations, assertions),
        "TreeMap<i32, i32>" => run_treemap(operations, assertions),
        other => {
            eprintln!("unsupported collection type: {}", other);
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// HashMap<i32, i32>
// ---------------------------------------------------------------------------

fn run_hashmap(
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario: &Value,
) {
    let mut map = I32I32HashMap::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = op["key"].as_i64().unwrap() as i32;
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = op["key"].as_i64().unwrap() as i32;
                map.remove(k);
            }
            "clear" => {
                map.clear();
            }
            other => panic!("unknown hashmap op: {}", other),
        }
    }

    for (key, _expected) in assertions {
        let val = eval_hashmap_assertion(key, &map, scenario);
        println!("{}: {}", key, val);
    }
}

fn eval_hashmap_assertion(key: &str, map: &I32I32HashMap, _scenario: &Value) -> String {
    match key {
        "size" => map.len().to_string(),
        "is_empty" => map.is_empty().to_string(),
        "sorted_keys" => {
            let mut keys: Vec<i32> = map.keys().collect();
            keys.sort();
            format_array(&keys)
        }
        "sorted_values" => {
            let mut vals: Vec<i32> = map.values().collect();
            vals.sort();
            format_array(&vals)
        }
        "min" => {
            // For treemap min is the min key; for hashmap we take min key
            let mut keys: Vec<i32> = map.keys().collect();
            keys.sort();
            match keys.first() {
                Some(k) => k.to_string(),
                None => "null".to_string(),
            }
        }
        "max" => {
            let mut keys: Vec<i32> = map.keys().collect();
            keys.sort();
            match keys.last() {
                Some(k) => k.to_string(),
                None => "null".to_string(),
            }
        }
        _ if key.starts_with("get_") => {
            let k: i32 = key[4..].parse().unwrap();
            match map.get(k) {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            }
        }
        _ if key.starts_with("contains_") => {
            let k: i32 = key[9..].parse().unwrap();
            map.contains_key(k).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// ArrayList<i32>
// ---------------------------------------------------------------------------

fn run_arraylist(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut list = I32ArrayList::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                let v = op["value"].as_i64().unwrap() as i32;
                list.push(v);
            }
            "add_at" => {
                let idx = op["index"].as_u64().unwrap() as usize;
                let v = op["value"].as_i64().unwrap() as i32;
                // I32ArrayList doesn't have insert_at, so we use to_vec, insert, rebuild
                let mut vec = list.to_vec();
                vec.insert(idx, v);
                list = I32ArrayList::of(&vec);
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                list.remove(v);
            }
            "clear" => {
                list.clear();
            }
            other => panic!("unknown arraylist op: {}", other),
        }
    }

    for (key, _expected) in assertions {
        let val = eval_arraylist_assertion(key, &list);
        println!("{}: {}", key, val);
    }
}

fn eval_arraylist_assertion(key: &str, list: &I32ArrayList) -> String {
    match key {
        "size" => list.len().to_string(),
        "is_empty" => list.is_empty().to_string(),
        "sum" => list.sum().to_string(),
        "min" => match list.min() {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        "max" => match list.max() {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        "to_sorted_array" => {
            let mut v = list.to_vec();
            v.sort();
            format_array(&v)
        }
        "inject_into_sum" => {
            let r = list.inject_into(0i64, |acc, v| acc + v as i64);
            r.to_string()
        }
        "inject_into_product" => {
            let r = list.inject_into(1i64, |acc, v| acc * v as i64);
            r.to_string()
        }
        _ if key.starts_with("get_at_") => {
            let idx: usize = key[7..].parse().unwrap();
            match list.get(idx) {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            }
        }
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            list.contains(v).to_string()
        }
        _ if key.starts_with("select_gt_") => {
            let threshold: i32 = key[10..].parse().unwrap();
            let selected = list.select(|v| v > threshold);
            let mut v = selected.to_vec();
            v.sort();
            format_array(&v)
        }
        _ if key.starts_with("reject_gt_") => {
            let threshold: i32 = key[10..].parse().unwrap();
            let rejected = list.reject(|v| v > threshold);
            let mut v = rejected.to_vec();
            v.sort();
            format_array(&v)
        }
        _ if key.starts_with("detect_gt_") => {
            let threshold: i32 = key[10..].parse().unwrap();
            match list.detect(|v| v > threshold) {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            }
        }
        _ if key.starts_with("count_gt_") => {
            let threshold: i32 = key[9..].parse().unwrap();
            list.count(|v| v > threshold).to_string()
        }
        _ if key.starts_with("count_lt_") => {
            let threshold: i32 = key[9..].parse().unwrap();
            list.count(|v| v < threshold).to_string()
        }
        "count_even" => list.count(|v| v % 2 == 0).to_string(),
        "count_odd" => list.count(|v| v % 2 != 0).to_string(),
        _ if key.starts_with("any_satisfy_gt_") => {
            let threshold: i32 = key[15..].parse().unwrap();
            list.any_satisfy(|v| v > threshold).to_string()
        }
        _ if key.starts_with("all_satisfy_gt_") => {
            let threshold: i32 = key[15..].parse().unwrap();
            list.all_satisfy(|v| v > threshold).to_string()
        }
        _ if key.starts_with("none_satisfy_gt_") => {
            let threshold: i32 = key[16..].parse().unwrap();
            list.none_satisfy(|v| v > threshold).to_string()
        }
        _ if key.starts_with("none_satisfy_lt_") => {
            let threshold: i32 = key[16..].parse().unwrap();
            list.none_satisfy(|v| v < threshold).to_string()
        }
        "any_satisfy_even" => list.any_satisfy(|v| v % 2 == 0).to_string(),
        "all_satisfy_even" => list.all_satisfy(|v| v % 2 == 0).to_string(),
        "none_satisfy_odd" => list.none_satisfy(|v| v % 2 != 0).to_string(),
        _ if key.starts_with("any_satisfy_gt_") => {
            let threshold: i32 = key[15..].parse().unwrap();
            list.any_satisfy(|v| v > threshold).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// HashSet<i32>
// ---------------------------------------------------------------------------

fn run_hashset(
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario: &Value,
) {
    let mut set = I32HashSet::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                let v = op["value"].as_i64().unwrap() as i32;
                set.add(v);
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                set.remove(v);
            }
            "clear" => {
                set.clear();
            }
            other => panic!("unknown hashset op: {}", other),
        }
    }

    // Build "other" set if present
    let other_set = if let Some(other_spec) = scenario.get("other") {
        let mut other = I32HashSet::new();
        if let Some(ops) = other_spec["operations"].as_array() {
            for op in ops {
                match op["op"].as_str().unwrap() {
                    "add" => {
                        let v = op["value"].as_i64().unwrap() as i32;
                        other.add(v);
                    }
                    other_op => panic!("unknown hashset other op: {}", other_op),
                }
            }
        }
        Some(other)
    } else {
        None
    };

    for (key, _expected) in assertions {
        let val = eval_hashset_assertion(key, &set, other_set.as_ref());
        println!("{}: {}", key, val);
    }
}

fn eval_hashset_assertion(key: &str, set: &I32HashSet, other: Option<&I32HashSet>) -> String {
    match key {
        "size" => set.len().to_string(),
        "is_empty" => set.is_empty().to_string(),
        "to_sorted_array" => {
            let mut v = set.to_vec();
            v.sort();
            format_array(&v)
        }
        "other_size" => match other {
            Some(o) => o.len().to_string(),
            None => "null".to_string(),
        },
        "union_sorted" => {
            let o = other.expect("union_sorted requires 'other'");
            let u = set.union(o);
            let mut v = u.to_vec();
            v.sort();
            format_array(&v)
        }
        "union_size" => {
            let o = other.expect("union_size requires 'other'");
            set.union(o).len().to_string()
        }
        "intersect_sorted" => {
            let o = other.expect("intersect_sorted requires 'other'");
            let i = set.intersect(o);
            let mut v = i.to_vec();
            v.sort();
            format_array(&v)
        }
        "intersect_size" => {
            let o = other.expect("intersect_size requires 'other'");
            set.intersect(o).len().to_string()
        }
        "difference_sorted" => {
            let o = other.expect("difference_sorted requires 'other'");
            let d = set.difference(o);
            let mut v = d.to_vec();
            v.sort();
            format_array(&v)
        }
        "difference_size" => {
            let o = other.expect("difference_size requires 'other'");
            set.difference(o).len().to_string()
        }
        "symmetric_difference_sorted" => {
            let o = other.expect("symmetric_difference_sorted requires 'other'");
            let sd = set.symmetric_difference(o);
            let mut v = sd.to_vec();
            v.sort();
            format_array(&v)
        }
        "symmetric_difference_size" => {
            let o = other.expect("symmetric_difference_size requires 'other'");
            set.symmetric_difference(o).len().to_string()
        }
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            set.contains(v).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// HashBag<i32>
// ---------------------------------------------------------------------------

fn run_hashbag(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut bag = I32HashBag::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                let v = op["value"].as_i64().unwrap() as i32;
                bag.add(v);
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                bag.remove(v);
            }
            "clear" => {
                bag.clear();
            }
            other => panic!("unknown hashbag op: {}", other),
        }
    }

    for (key, _expected) in assertions {
        let val = eval_hashbag_assertion(key, &bag);
        println!("{}: {}", key, val);
    }
}

fn eval_hashbag_assertion(key: &str, bag: &I32HashBag) -> String {
    match key {
        "size" => bag.size().to_string(),
        "size_distinct" => bag.size_distinct().to_string(),
        "is_empty" => bag.is_empty().to_string(),
        "to_sorted_array" => {
            let mut v = bag.to_vec();
            v.sort();
            format_array(&v)
        }
        _ if key.starts_with("occurrences_") => {
            let v: i32 = key[12..].parse().unwrap();
            bag.occurrences_of(v).to_string()
        }
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            bag.contains(v).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// TreeSet<i32>
// ---------------------------------------------------------------------------

fn run_treeset(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut set = I32TreeSet::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                let v = op["value"].as_i64().unwrap() as i32;
                set.add(v);
            }
            "remove" => {
                let v = op["value"].as_i64().unwrap() as i32;
                set.remove(v);
            }
            "clear" => {
                set.clear();
            }
            other => panic!("unknown treeset op: {}", other),
        }
    }

    for (key, _expected) in assertions {
        let val = eval_treeset_assertion(key, &set);
        println!("{}: {}", key, val);
    }
}

fn eval_treeset_assertion(key: &str, set: &I32TreeSet) -> String {
    match key {
        "size" => set.len().to_string(),
        "is_empty" => set.is_empty().to_string(),
        "to_sorted_array" => {
            let v = set.to_vec(); // TreeSet iterates in sorted order
            format_array(&v)
        }
        "min" => match set.min() {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        "max" => match set.max() {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        },
        _ if key.starts_with("contains_") => {
            let v: i32 = key[9..].parse().unwrap();
            set.contains(v).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// TreeMap<i32, i32>
// ---------------------------------------------------------------------------

fn run_treemap(operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    let mut map = I32I32TreeMap::new();

    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = op["key"].as_i64().unwrap() as i32;
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = op["key"].as_i64().unwrap() as i32;
                map.remove(k);
            }
            "clear" => {
                map.clear();
            }
            other => panic!("unknown treemap op: {}", other),
        }
    }

    for (key, _expected) in assertions {
        let val = eval_treemap_assertion(key, &map);
        println!("{}: {}", key, val);
    }
}

fn eval_treemap_assertion(key: &str, map: &I32I32TreeMap) -> String {
    match key {
        "size" => map.len().to_string(),
        "is_empty" => map.is_empty().to_string(),
        "sorted_keys" => {
            let keys: Vec<i32> = map.keys().collect(); // TreeMap keys already sorted
            format_array(&keys)
        }
        "sorted_values" => {
            // values in key-sorted order
            let vals: Vec<i32> = map.values().collect();
            format_array(&vals)
        }
        "min" => match map.min() {
            Some((k, _)) => k.to_string(),
            None => "null".to_string(),
        },
        "max" => match map.max() {
            Some((k, _)) => k.to_string(),
            None => "null".to_string(),
        },
        _ if key.starts_with("get_") => {
            let k: i32 = key[4..].parse().unwrap();
            match map.get(k) {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            }
        }
        _ if key.starts_with("contains_") => {
            let k: i32 = key[9..].parse().unwrap();
            map.contains_key(k).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_array(vals: &[i32]) -> String {
    let items: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
    format!("[{}]", items.join(", "))
}
