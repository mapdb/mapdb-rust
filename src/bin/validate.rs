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

use mapdb_collections::bulk::DuplicatePolicy;
use mapdb_collections::multimap::{Multimap, SetMultimap};
use mapdb_collections::object::ArrayList;
use mapdb_collections::object::Collection as ObjectCollection;
use mapdb_collections::object::TreeMap as ObjectTreeMap;
use mapdb_collections::object::{natural_comparator, TreeSet};
use mapdb_collections::object::{MutableCollection, MutableList};
use mapdb_collections::{HashableF32, OpenHashMap, OpenHashSet};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

// Set whenever any assertion mismatches. The process exits non-zero at the
// end so the harness treats assertion failures as the primary pass/fail.
static ANY_FAIL: AtomicBool = AtomicBool::new(false);

/// Emit a computed assertion: skip unrecognised keys silently (per the
/// README unknown-assertion-skip rule — no print, no UNKNOWN_ASSERTION
/// line), otherwise print the canonical `key: value` line and compare
/// against the expected JSON value.
fn emit(scenario: &str, key: &str, computed: &str, expected: &Value, float_mode: FloatMode) {
    if computed.starts_with("UNKNOWN_ASSERTION:") {
        return; // unrecognised key -> skip (do not print, do not fail)
    }
    println!("{}: {}", key, computed);
    let expected_str = render_expected(expected, key, float_mode);
    if computed != expected_str && !loose_nan_match(expected, float_mode, computed) {
        println!(
            "FAIL {} {}: expected={} got={}",
            scenario, key, expected_str, computed
        );
        ANY_FAIL.store(true, Ordering::Relaxed);
    }
}

/// Loose-NaN scalar match. When the EXPECTED operand is a bare NaN *label*
/// (`"NaN"`/`"+NaN"`/`"-NaN"`) — NOT a `{"bits":"0x.."}` object and NOT an
/// array element — the assertion passes against ANY NaN the runner computed,
/// regardless of sign/payload. This covers impl/arch-defined arithmetic NaNs
/// such as (+Inf)+(-Inf), whose bits differ across x86 vs ARM. `{"bits"}`
/// operands stay bitwise-exact and array elements stay exact/positional (the
/// render_expected path is unchanged for both). See
/// cross-language-validation/README.md §"Float operand encoding".
fn loose_nan_match(expected: &Value, mode: FloatMode, computed: &str) -> bool {
    if mode == FloatMode::None {
        return false;
    }
    // Expected must be a bare string label that parses to a NaN.
    let Some(s) = expected.as_str() else {
        return false;
    };
    if !parse_f32_label(s).is_nan() {
        return false;
    }
    // Computed must itself be a NaN bit pattern (canonical "0x........").
    computed
        .strip_prefix("0x")
        .or_else(|| computed.strip_prefix("0X"))
        .and_then(|b| u32::from_str_radix(b, 16).ok())
        .map(|bits| f32::from_bits(bits).is_nan())
        .unwrap_or(false)
}

#[derive(Copy, Clone, PartialEq)]
enum FloatMode {
    /// Integer collections: arrays/scalars render as plain JSON i32/bool/null.
    None,
    /// f32 map/set keyed by floats but whose *scalar* assertions (size,
    /// get_N, contains_N) are i32/bool — only float *arrays* (sorted_keys,
    /// sorted_values) render as quoted float labels ("NaN").
    F32Keyed,
    /// f32 ArrayList: scalar assertions (sum, min, max) are floats and the
    /// `sorted` array renders each element unquoted (NaN).
    F32List,
}

/// Canonical rendering of an expected JSON value, matching runner output.
fn render_expected(v: &Value, key: &str, mode: FloatMode) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            // f32 scalars (sum/min/max) under F32List render as floats; the
            // structural `size` count stays an integer even in float mode.
            // Under F32Keyed, the key-typed scalars min/max are also f32 and
            // must render via the f32 formatter (matching TS), while other
            // scalars (size, get_N, contains_N) stay i32.
            let f32_scalar = (mode == FloatMode::F32List && key != "size")
                || (mode == FloatMode::F32Keyed && (key == "min" || key == "max"));
            if f32_scalar {
                format_f32(n.as_f64().unwrap() as f32)
            } else {
                // i32 scalar (None) or i32 map value under F32Keyed.
                n.to_string()
            }
        }
        Value::String(s) => {
            // Float label scalar (e.g. sum: "NaN", max: "NaN").
            format_f32(parse_f32_label(s))
        }
        // Bits-escape float scalar (e.g. sum: {"bits":"0xffc00000"}).
        Value::Object(_) if mode != FloatMode::None => format_f32(parse_f32(v)),
        Value::Array(arr) => {
            let parts: Vec<String> = arr
                .iter()
                .map(|e| match mode {
                    FloatMode::None => match e {
                        Value::Number(n) => n.to_string(),
                        _ => e.to_string(),
                    },
                    FloatMode::F32Keyed => format!("\"{}\"", format_f32(element_to_f32(e))),
                    FloatMode::F32List => format_f32(element_to_f32(e)),
                })
                .collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}

fn element_to_f32(e: &Value) -> f32 {
    match e {
        Value::String(s) => parse_f32_label(s),
        Value::Number(n) => n.as_f64().unwrap() as f32,
        // {"bits":"0x.."} escape inside an assertion array.
        Value::Object(_) => parse_f32(e),
        _ => panic!("unexpected float array element: {:?}", e),
    }
}

// Q4 float operand encoding (see cross-language-validation/README.md
// §"Float operand encoding"). A float operand is one of:
//   * JSON number            -> the exact value
//   * human-label string     -> "NaN"/"+NaN"/"-NaN", "Infinity"/"+Infinity"/
//                               "-Infinity", "0.0"/"+0.0"/"-0.0", or a decimal
//   * bits-escape object      -> {"bits":"0x........"} reinterpret 32 hex bits
fn parse_f32(v: &Value) -> f32 {
    if let Some(s) = v.as_str() {
        parse_f32_label(s)
    } else if let Some(obj) = v.as_object() {
        if let Some(Value::String(hex)) = obj.get("bits") {
            f32::from_bits(parse_f32_bits(hex))
        } else {
            panic!("expected {{\"bits\":\"0x..\"}} float object, got {:?}", v);
        }
    } else if let Some(n) = v.as_f64() {
        n as f32
    } else {
        panic!("expected f32 value, got {:?}", v);
    }
}

/// Parse an exact 32-bit IEEE-754 pattern from a `0x`-prefixed, 8-hex-digit
/// (case-insensitive) string. This is the canonical NaN-payload / signed-bit
/// escape and also the canonical serialization for NaN and ±0.0.
fn parse_f32_bits(hex: &str) -> u32 {
    let body = hex
        .strip_prefix("0x")
        .or_else(|| hex.strip_prefix("0X"))
        .unwrap_or_else(|| panic!("f32 bits literal must start with 0x: {:?}", hex));
    assert_eq!(
        body.len(),
        8,
        "f32 bits literal must be 8 hex digits: {:?}",
        hex
    );
    u32::from_str_radix(body, 16).unwrap_or_else(|_| panic!("invalid f32 bits literal: {:?}", hex))
}

// Canonical, bit-faithful serialization. NaN (any sign/payload) and ±0.0
// render as their 0x-hex bit pattern so distinct payloads and signed zeros
// stay distinguishable and every port emits the identical string; finite and
// infinite values keep their human-readable label (all ports agree on those).
fn format_f32(v: f32) -> String {
    if v.is_nan() || v == 0.0 {
        format!("0x{:08x}", v.to_bits())
    } else if v == f32::INFINITY {
        "Infinity".to_string()
    } else if v == f32::NEG_INFINITY {
        "-Infinity".to_string()
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
    let construction = scenario.get("construction").and_then(Value::as_str);
    let operations = scenario["operations"]
        .as_array()
        .expect("missing operations");
    let assertions = scenario["assertions"]
        .as_object()
        .expect("missing assertions");

    println!("=== scenario: {} ===", name);

    match collection {
        "HashMap<i32, i32>" => run_hashmap(name, operations, assertions, construction),
        "HashMap<i64, i32>" => run_i64_hashmap(name, operations, assertions),
        "ListMultimap<i64, i32>" => {
            run_i64_list_multimap(name, operations, assertions, construction)
        }
        "SetMultimap<i64, i32>" => run_i64_set_multimap(name, operations, assertions, construction),
        "ArrayList<i32>" => run_arraylist(name, operations, assertions),
        "HashSet<i32>" => run_hashset(name, operations, assertions, &scenario),
        "HashBag<i32>" => run_hashbag(name, operations, assertions),
        "TreeSet<i32>" => run_treeset(name, operations, assertions),
        "TreeMap<i32, i32>" => run_treemap(name, operations, assertions, construction),
        "HashMap<f32, i32>" => run_f32_hashmap(name, operations, assertions),
        "HashSet<f32>" => run_f32_hashset(name, operations, assertions),
        "TreeSet<f32>" => run_f32_treeset(name, operations, assertions),
        "ArrayList<f32>" => run_f32_arraylist(name, operations, assertions),
        other => {
            eprintln!("unsupported collection type: {}", other);
            std::process::exit(1);
        }
    }

    if ANY_FAIL.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}

fn format_array(v: &[i32]) -> String {
    let parts: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

// ---- HashMap<i32, i32> ---------------------------------------------------

fn run_hashmap(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    construction: Option<&str>,
) {
    let map: OpenHashMap<i32, i32> = if construction == Some("bulkLoadExact") {
        OpenHashMap::bulk_load_exact(
            i32_pairs(operations),
            operations.len(),
            DuplicatePolicy::Error,
        )
        .expect("bulkLoadExact failed")
    } else {
        let mut map = OpenHashMap::new();
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
        map
    };
    for (key, expected) in assertions {
        if key == "comment" {
            continue; // Scenario authors use "comment" for doc strings; skip.
        }
        let computed = eval_map_assertion(key, &map);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn i32_pairs(operations: &[Value]) -> Vec<(i32, i32)> {
    operations
        .iter()
        .map(|op| {
            (
                op["key"].as_i64().unwrap() as i32,
                op["value"].as_i64().unwrap() as i32,
            )
        })
        .collect()
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

// ---- HashMap<i64, i32> ---------------------------------------------------

// Wide-integer (i64) operand encoding — see cross-language-validation/README.md
// §"Wide-integer (i64) operand encoding". An i64 KEY is a decimal STRING (small
// keys may also be bare JSON numbers); it is parsed straight to i64 (never via
// f64). The value stays an i32 JSON number. Routes through the generic
// production OpenHashMap<i64, i32> (real hash spread + i64 key identity).
fn parse_i64_operand(v: &Value) -> i64 {
    if let Some(s) = v.as_str() {
        s.parse::<i64>()
            .unwrap_or_else(|_| panic!("invalid i64 decimal-string key: {:?}", s))
    } else if let Some(n) = v.as_i64() {
        n
    } else {
        panic!("expected i64 key (decimal string or number), got {:?}", v);
    }
}

fn run_i64_hashmap(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    let mut map: OpenHashMap<i64, i32> = OpenHashMap::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "put" => {
                let k = parse_i64_operand(&op["key"]);
                let v = op["value"].as_i64().unwrap() as i32;
                map.insert(k, v);
            }
            "remove" => {
                let k = parse_i64_operand(&op["key"]);
                map.remove(&k);
            }
            "clear" => map.clear(),
            other => panic!("unknown i64-hashmap op: {}", other),
        }
    }
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_i64_map_assertion(key, &map);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_i64_map_assertion(key: &str, map: &OpenHashMap<i64, i32>) -> String {
    match key {
        "size" => map.len().to_string(),
        "is_empty" => map.is_empty().to_string(),
        "sorted_keys" => {
            // i64 keys exceed 2^53, so serialize each as a decimal STRING in a
            // quoted array, sorted numerically as i64 ascending.
            let mut keys: Vec<i64> = map.iter().map(|(k, _)| *k).collect();
            keys.sort();
            let parts: Vec<String> = keys.iter().map(|k| format!("\"{}\"", k)).collect();
            format!("[{}]", parts.join(","))
        }
        _ if key.starts_with("get_") => {
            let k: i64 = key[4..].parse().unwrap();
            map.get(&k)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("contains_") => {
            let k: i64 = key[9..].parse().unwrap();
            map.contains_key(&k).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- {List,Set}Multimap<i64, i32> ----------------------------------------

// Unlike Go/TS/Zig (whose multimaps use a stdlib/builtin hash map), Rust's
// Multimap/SetMultimap are built on the project OpenHashMap<i64, Vec<i32>>. That
// map hashes keys with std's DefaultHasher (SipHash) and uses h.finish()
// directly — no explicit high-bit fold (unlike Go/Zig's OpenHashMap), since
// SipHash already mixes all bits. This runner verifies the same full-range i64
// keys keep their identity (stay distinct and retrievable) — key identity, not
// bucket-distribution quality.
// Multimap (list) keeps duplicate values; SetMultimap dedups. Both expose the
// same insert / get(&k)->&[i32] / remove_all / contains_key / distinct_len / keys
// surface, so a macro generates the two near-identical runners.
//
// Assertions (identical to the other ports):
//   distinct_key_count -> distinct-key count (integer string)
//   sorted_keys        -> DISTINCT keys, ascending i64, quoted decimal strings
//                         (i64 keys exceed 2^53) — same as the i64-HashMap form
//   get_<k>            -> values for the key, ascending-sorted i32 array (sort a
//                         COPY); absent/removed => []
//   contains_key_<k>   -> bool
macro_rules! run_i64_multimap {
    ($fn_name:ident, $ty:ty) => {
        fn $fn_name(
            scenario: &str,
            operations: &[Value],
            assertions: &serde_json::Map<String, Value>,
            construction: Option<&str>,
        ) {
            let map: $ty = if construction == Some("fromSortedKeyValues") {
                <$ty>::from_sorted_key_values(
                    natural_comparator::<i64>(),
                    natural_comparator::<i32>(),
                    i64_pairs(operations),
                )
                .expect("fromSortedKeyValues failed")
            } else {
                let mut map: $ty = <$ty>::new();
                for op in operations {
                    match op["op"].as_str().unwrap() {
                        "put" => {
                            let k = parse_i64_operand(&op["key"]);
                            let v = op["value"].as_i64().unwrap() as i32;
                            map.insert(k, v);
                        }
                        "removeAll" => {
                            let k = parse_i64_operand(&op["key"]);
                            map.remove_all(&k);
                        }
                        other => panic!("unknown i64-multimap op: {}", other),
                    }
                }
                map
            };
            for (key, expected) in assertions {
                if key == "comment" {
                    continue;
                }
                let computed = if key == "distinct_key_count" {
                    map.distinct_len().to_string()
                } else if key == "sorted_keys" {
                    let mut ks: Vec<i64> = map.keys().copied().collect();
                    ks.sort();
                    let parts: Vec<String> = ks.iter().map(|k| format!("\"{}\"", k)).collect();
                    format!("[{}]", parts.join(","))
                } else if let Some(rest) = key.strip_prefix("get_") {
                    let k: i64 = rest.parse().unwrap();
                    let mut vals: Vec<i32> = map.get(&k).to_vec();
                    vals.sort();
                    format_array(&vals)
                } else if let Some(rest) = key.strip_prefix("contains_key_") {
                    let k: i64 = rest.parse().unwrap();
                    map.contains_key(&k).to_string()
                } else {
                    format!("UNKNOWN_ASSERTION:{}", key)
                };
                emit(scenario, key, &computed, expected, FloatMode::None);
            }
        }
    };
}

run_i64_multimap!(run_i64_list_multimap, Multimap<i64, i32>);
run_i64_multimap!(run_i64_set_multimap, SetMultimap<i64, i32>);

fn i64_pairs(operations: &[Value]) -> Vec<(i64, i32)> {
    operations
        .iter()
        .map(|op| {
            (
                parse_i64_operand(&op["key"]),
                op["value"].as_i64().unwrap() as i32,
            )
        })
        .collect()
}

// ---- ArrayList<i32> -------------------------------------------------------

fn run_arraylist(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
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
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_list_assertion(key, &list);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_list_assertion(key: &str, list: &Vec<i32>) -> String {
    match key {
        "size" => list.len().to_string(),
        "is_empty" => list.is_empty().to_string(),
        "sum" => {
            // List sum() widens into an i64 accumulator (IntList.sum(): long
            // parity) and does NOT wrap at i32 — see algorithms.md "Integer
            // overflow contract" and scenarios/06-overflow/i32_sum_overflow.json.
            // Routed through the production ArrayList::inject_into fold.
            let al = ArrayList::from_iter(list.iter().copied());
            al.inject_into(0i64, |a, &v| a + v as i64).to_string()
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
        "inject_into_sum" => {
            // injectInto with a + reduction accumulates in the i32 seed type
            // and wraps two's-complement at i32 — via the production fold.
            let al = ArrayList::from_iter(list.iter().copied());
            al.inject_into(0i32, |a, &v| a.wrapping_add(v)).to_string()
        }
        "inject_into_product" => {
            let al = ArrayList::from_iter(list.iter().copied());
            al.inject_into(1i32, |a, &v| a.wrapping_mul(v)).to_string()
        }
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
    scenario_name: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario: &Value,
) {
    let mut set: OpenHashSet<i32> = OpenHashSet::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.insert(op["value"].as_i64().unwrap() as i32);
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
                    other.insert(op["value"].as_i64().unwrap() as i32);
                }
            }
        }
        other
    });
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_set_assertion(key, &set, other_set.as_ref());
        emit(scenario_name, key, &computed, expected, FloatMode::None);
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

fn run_hashbag(scenario: &str, operations: &[Value], assertions: &serde_json::Map<String, Value>) {
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
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_bag_assertion(key, &bag, total);
        emit(scenario, key, &computed, expected, FloatMode::None);
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

fn run_treeset(scenario: &str, operations: &[Value], assertions: &serde_json::Map<String, Value>) {
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
    for (key, expected) in assertions {
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
        emit(scenario, key, &v, expected, FloatMode::None);
    }
}

// ---- TreeMap<i32, i32> ----------------------------------------------------

fn run_treemap(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    construction: Option<&str>,
) {
    let mut map: BTreeMap<i32, i32> = BTreeMap::new();
    if construction == Some("fromSorted") {
        let pumped = ObjectTreeMap::from_sorted(
            natural_comparator::<i32>(),
            i32_pairs(operations),
            DuplicatePolicy::Error,
        )
        .expect("fromSorted failed");
        for (k, v) in &pumped {
            map.insert(*k, *v);
        }
    } else {
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
    }
    for (key, expected) in assertions {
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
        emit(scenario, key, &v, expected, FloatMode::None);
    }
}

// ---- HashMap<f32, i32> ----------------------------------------------------

fn run_f32_hashmap(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
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
    for (key, expected) in assertions {
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
        emit(scenario, key, &val, expected, FloatMode::F32Keyed);
    }
}

// Parse a human-label / decimal / hex-bits float string. Used both for
// string operands and for assertion-key suffixes (get_-NaN, contains_0.0,
// contains_0x7fc00001). Canonical NaN bits: +NaN=0x7FC00000, -NaN=0xFFC00000.
fn parse_f32_label(s: &str) -> f32 {
    match s {
        "NaN" | "+NaN" => f32::from_bits(0x7FC0_0000),
        "-NaN" => f32::from_bits(0xFFC0_0000),
        "Infinity" | "+Infinity" => f32::INFINITY,
        "-Infinity" => f32::NEG_INFINITY,
        "0.0" | "+0.0" => 0.0_f32,
        "-0.0" => -0.0_f32,
        "pos_zero" => 0.0_f32,
        "neg_zero" => -0.0_f32,
        other if other.starts_with("0x") || other.starts_with("0X") => {
            f32::from_bits(parse_f32_bits(other))
        }
        other => other.parse::<f32>().expect("invalid f32 literal in key"),
    }
}

// ---- HashSet<f32> ---------------------------------------------------------

fn run_f32_hashset(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    let mut set: OpenHashSet<HashableF32> = OpenHashSet::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.insert(HashableF32(parse_f32(&op["value"])));
            }
            "remove" => {
                set.remove(&HashableF32(parse_f32(&op["value"])));
            }
            "clear" => set.clear(),
            other => panic!("unknown f32-hashset op: {}", other),
        }
    }
    for (key, expected) in assertions {
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
        emit(scenario, key, &val, expected, FloatMode::F32Keyed);
    }
}

// ---- TreeSet<f32> ---------------------------------------------------------

// Routes through the PRODUCTION object::TreeSet ordered by the natural
// comparator over HashableF32 (whose Ord is f32::total_cmp). Sorted output is
// the tree's in-order traversal — NEVER sorted in the runner — so this
// exercises the production float total-order comparator directly.
fn run_f32_treeset(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    let mut set: TreeSet<HashableF32> = TreeSet::new(natural_comparator::<HashableF32>());
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.insert(HashableF32(parse_f32(&op["value"])));
            }
            "remove" => {
                set.remove(&HashableF32(parse_f32(&op["value"])));
            }
            "clear" => set.clear(),
            other => panic!("unknown f32-treeset op: {}", other),
        }
    }
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let val = match key.as_str() {
            "size" => set.len().to_string(),
            "is_empty" => set.is_empty().to_string(),
            "min" => set
                .min()
                .map(|x| format_f32(x.0))
                .unwrap_or_else(|| "null".into()),
            "max" => set
                .max()
                .map(|x| format_f32(x.0))
                .unwrap_or_else(|| "null".into()),
            k if k.starts_with("contains_") => {
                let raw = &k[9..];
                let probe = HashableF32(parse_f32_label(raw));
                set.contains(&probe).to_string()
            }
            "sorted" | "sorted_values" | "to_sorted_array" => {
                // In-order traversal straight from the production tree.
                let parts: Vec<String> = set
                    .iter()
                    .map(|x| format!("\"{}\"", format_f32(x.0)))
                    .collect();
                format!("[{}]", parts.join(","))
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        emit(scenario, key, &val, expected, FloatMode::F32Keyed);
    }
}

// ---- ArrayList<f32> -------------------------------------------------------

fn run_f32_arraylist(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    // Route through the PRODUCTION generic list keyed by HashableF32 (whose Ord
    // is the IEEE total order). Matches the Go/Zig f32-list runners: no local
    // Vec<f32> with its own total_cmp/sum — sorted/min/max/sum are all proved
    // against the real collection code.
    let mut list: ArrayList<HashableF32> = ArrayList::new();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => list.push(HashableF32(parse_f32(&op["value"]))),
            "clear" => list.clear(),
            other => panic!("unknown f32-arraylist op: {}", other),
        }
    }
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let val = match key.as_str() {
            "size" => list.len().to_string(),
            "is_empty" => list.is_empty().to_string(),
            "sum" => {
                // f32 list sum is a per-add f32 left-fold (IEEE arithmetic),
                // NOT a widened/f64 accumulation — matches Go's
                // Float32ArrayList.Sum() and the TS per-add fround fold.
                // Driven by the production inject_into fold.
                let s = list.inject_into(0.0f32, |acc, v| acc + v.0);
                format_f32(s)
            }
            // min/max via the production total-order sort (HashableF32 Ord),
            // then take the ends — no runner-side comparator.
            "min" | "max" => {
                let mut sorted = ArrayList::from_iter(list.iter().copied());
                sorted.sort();
                let pick = if key == "min" {
                    sorted.iter().next()
                } else {
                    sorted.iter().last()
                };
                pick.map(|v| format_f32(v.0))
                    .unwrap_or_else(|| "null".into())
            }
            "sorted" | "to_sorted_array" => {
                // Sort a COPY through the production total-order Sort() so the
                // assertion proves conformance without mutating the live list.
                let mut sorted = ArrayList::from_iter(list.iter().copied());
                sorted.sort();
                let parts: Vec<String> = sorted.iter().map(|v| format_f32(v.0)).collect();
                format!("[{}]", parts.join(","))
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        emit(scenario, key, &val, expected, FloatMode::F32List);
    }
}
