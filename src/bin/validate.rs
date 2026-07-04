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

use mapdb_collections::bloom::Bloom;
use mapdb_collections::bounded_lru::{BoundedLruMap, EvictionCause};
use mapdb_collections::bulk::DuplicatePolicy;
use mapdb_collections::count_min::CountMin;
use mapdb_collections::fenwick::FenwickTree;
use mapdb_collections::hash;
use mapdb_collections::hyperloglog::HyperLogLog;
use mapdb_collections::multimap::{Multimap, SetMultimap};
use mapdb_collections::object::ArrayList;
use mapdb_collections::object::Collection as ObjectCollection;
use mapdb_collections::object::TreeMap as ObjectTreeMap;
use mapdb_collections::object::{natural_comparator, DynTreeSet, TreeSet};
use mapdb_collections::object::{MutableCollection, MutableList};
use mapdb_collections::range::{BoundType, Range};
use mapdb_collections::roaring::RoaringU32;
use mapdb_collections::space_saving::SpaceSaving;
use mapdb_collections::{
    HashableF32, ImmutableSortedMap, ImmutableSortedSet, OpenHashMap, OpenHashSet, RangeMap,
    RangeSet,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::rc::Rc;
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
            if mode == FloatMode::None {
                // Plain string scalar (e.g. Range lower_bound_type: "closed").
                s.clone()
            } else {
                // Float label scalar (e.g. sum: "NaN", max: "NaN").
                format_f32(parse_f32_label(s))
            }
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
        "TreeSet<i32>" => run_treeset(name, operations, assertions, &scenario),
        "TreeMap<i32, i32>" => run_treemap(name, operations, assertions, &scenario, construction),
        "HashMap<f32, i32>" => run_f32_hashmap(name, operations, assertions),
        "HashSet<f32>" => run_f32_hashset(name, operations, assertions),
        "TreeSet<f32>" => run_f32_treeset(name, operations, assertions),
        "ArrayList<f32>" => run_f32_arraylist(name, operations, assertions),
        "Range<i32>" => run_range(name, operations, assertions, &scenario),
        "RangeSet<i32>" => run_range_set(name, operations, assertions, &scenario),
        "RangeMap<i32, i32>" => run_range_map(name, operations, assertions, &scenario),
        "ImmutableSortedMap<i32, i32>" => {
            run_immutable_sorted_map(name, operations, assertions, &scenario)
        }
        "ImmutableSortedSet<i32>" => {
            run_immutable_sorted_set(name, operations, assertions, &scenario)
        }
        "HashPipeline" => run_hash_pipeline(name, operations, assertions),
        "Bloom" => run_bloom(name, operations, assertions, &scenario),
        "HyperLogLog" => run_hyperloglog(name, operations, assertions, &scenario),
        "CountMin" => run_count_min(name, operations, assertions),
        "SpaceSaving" => run_space_saving(name, operations, assertions),
        "FenwickTree" => run_fenwick(name, operations, assertions),
        "RoaringU32" => run_roaring(name, operations, assertions, &scenario),
        "BoundedLruMap<i32, i32>" => run_bounded_lru(name, operations, assertions, &scenario),
        other => {
            // Forward-compat (README "unknown collection kinds skip"): a runner
            // that does not understand a collection kind must SKIP, not fail, so
            // newer scenarios never break an older runner. Mirrors the
            // unknown-assertion-key skip in `emit`.
            eprintln!(
                "skip: unsupported collection kind (forward-compat): {}",
                other
            );
            return;
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

// ---- HashPipeline (spec/features/hash-pipeline.md) ------------------------
//
// A stateless probe (not a stored collection): exactly ONE hash op carries the
// input + seed under test; the assertions read the deterministic hash output.
// Outputs are serialized as fixed-width, lower-case, `0x`-prefixed hex strings
// (8 digits for a u32, 16 for a u64) so a 64-bit hash survives the JSON `2^53`
// ceiling and every port's consensus diff is byte-identical. `positions` is an
// int[] in derivation order (NOT sorted). Unknown ops/keys SKIP (forward-compat).

/// Parse a `0x`-prefixed hex word operand to a `u64` (used for `word` operands;
/// the caller narrows to `u32` where the op needs a 32-bit word).
fn parse_hex_word(v: &Value) -> u64 {
    let s = v
        .as_str()
        .expect("hash-pipeline `word` must be a 0x-hex string");
    let body = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or_else(|| panic!("hash-pipeline word must start with 0x: {:?}", s));
    u64::from_str_radix(body, 16).unwrap_or_else(|_| panic!("invalid hex word: {:?}", s))
}

/// Parse a `seed` operand: a DECIMAL STRING parsed straight to u64 (never via
/// f64), reusing the i64-suite's decimal-string discipline. A bare JSON number
/// is also accepted for small seeds.
fn parse_seed(v: &Value) -> u64 {
    if let Some(s) = v.as_str() {
        s.parse::<u64>()
            .unwrap_or_else(|_| panic!("invalid u64 decimal-string seed: {:?}", s))
    } else if let Some(n) = v.as_u64() {
        n
    } else {
        panic!("expected u64 seed (decimal string or number), got {:?}", v);
    }
}

/// Parse a `0x`-hex byte string (e.g. `"0x01020304"`) to a byte vector.
fn parse_hex_bytes(v: &Value) -> Vec<u8> {
    let s = v
        .as_str()
        .expect("hash-pipeline `bytes` must be a 0x-hex string");
    let body = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or_else(|| panic!("hash-pipeline bytes must start with 0x: {:?}", s));
    assert!(
        body.len() % 2 == 0,
        "hash-pipeline bytes must have an even hex-digit count: {:?}",
        s
    );
    (0..body.len() / 2)
        .map(|i| {
            u8::from_str_radix(&body[2 * i..2 * i + 2], 16)
                .unwrap_or_else(|_| panic!("invalid hex byte in {:?}", s))
        })
        .collect()
}

/// The probe a hash op builds. `Word32`/`Word64` carry an already-encoded input
/// word (so only the matching hash width is meaningful); `I32`/`Bytes` carry the
/// logical input so EITHER the 32- or 64-bit form (incl. lanes) can be asserted;
/// `Positions` carries the derived-position array.
enum HashProbe {
    Word32(u32),
    Word64(u64),
    I32 { value: i32, seed: u64 },
    Bytes { bytes: Vec<u8>, seed: u64 },
    Positions(Vec<u32>),
}

fn run_hash_pipeline(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    // Authoring rule: exactly ONE hash op. Zero or multiple => malformed =>
    // SKIP (like the sorted-table `from_sorted` rule). Forward-compat: an
    // unrecognised op kind also makes the scenario un-runnable here => SKIP.
    if operations.len() != 1 {
        eprintln!(
            "skip: hash-pipeline scenario must have exactly one op (forward-compat): got {}",
            operations.len()
        );
        return;
    }
    let op = &operations[0];
    let probe = match op["op"].as_str().unwrap_or("") {
        "hash_word32" => {
            let raw = parse_hex_word(&op["word"]);
            assert!(
                raw <= u32::MAX as u64,
                "hash_word32 `word` exceeds 32 bits: {:#x}",
                raw
            );
            let word = raw as u32;
            let seed = parse_seed(&op["seed"]);
            HashProbe::Word32(hash::hash32(word, seed))
        }
        "hash_word64" => {
            let word = parse_hex_word(&op["word"]);
            let seed = parse_seed(&op["seed"]);
            HashProbe::Word64(hash::hash64(word, seed))
        }
        "hash_i32" => {
            let value = op["value"].as_i64().expect("hash_i32 needs i32 value") as i32;
            let seed = parse_seed(&op["seed"]);
            HashProbe::I32 { value, seed }
        }
        "hash_bytes" => {
            let bytes = parse_hex_bytes(&op["bytes"]);
            let seed = parse_seed(&op["seed"]);
            HashProbe::Bytes { bytes, seed }
        }
        "positions" => {
            let value = op["value"].as_i64().expect("positions needs i32 value") as i32;
            let m = op["m"].as_u64().expect("positions needs m") as u32;
            let k = op["k"].as_u64().expect("positions needs k") as u32;
            // The byte encoding of an i32 element drives positions: encode the
            // i32 to its little-endian 4-byte form (the byte path the sketches
            // use), then derive. No op-level seed (the scheme fixes 0 / SALT2).
            let bytes = (value as u32).to_le_bytes();
            HashProbe::Positions(hash::positions(&bytes, m, k))
        }
        other => {
            eprintln!("skip: unknown hash-pipeline op (forward-compat): {}", other);
            return;
        }
    };

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = probe.eval(key);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

impl HashProbe {
    fn eval(&self, key: &str) -> String {
        match self {
            HashProbe::Word32(h) => eval_h32_only(*h, key),
            HashProbe::Word64(h) => eval_h64_only(*h, key),
            HashProbe::Positions(p) => eval_positions(p, key),
            HashProbe::I32 { value, seed } => match key {
                "hash32" => eval_h32_only(hash::hash32_i32(*value, *seed), "hash32"),
                "hash64" | "hash64_hi" | "hash64_lo" => {
                    eval_h64_only(hash::hash64_i32(*value, *seed), key)
                }
                _ => format!("UNKNOWN_ASSERTION:{}", key),
            },
            HashProbe::Bytes { bytes, seed } => match key {
                "hash32" => eval_h32_only(hash::hash32_bytes(bytes, *seed), "hash32"),
                "hash64" | "hash64_hi" | "hash64_lo" => {
                    eval_h64_only(hash::hash64_bytes(bytes, *seed), key)
                }
                _ => format!("UNKNOWN_ASSERTION:{}", key),
            },
        }
    }
}

fn eval_h32_only(h: u32, key: &str) -> String {
    match key {
        "hash32" => format!("0x{:08x}", h),
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

fn eval_h64_only(h: u64, key: &str) -> String {
    match key {
        "hash64" => format!("0x{:016x}", h),
        "hash64_hi" => format!("0x{:08x}", (h >> 32) as u32),
        "hash64_lo" => format!("0x{:08x}", h as u32),
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

fn eval_positions(p: &[u32], key: &str) -> String {
    match key {
        // Emitted in DERIVATION order (p_0 … p_{k-1}), NOT sorted.
        "positions" => {
            let parts: Vec<String> = p.iter().map(|x| x.to_string()).collect();
            format!("[{}]", parts.join(","))
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- Bloom (spec/features/bloom.md) --------------------------------------
//
// Approximate set membership riding the hash pipeline. Exactly ONE `with_params`
// op (explicit m,k — never `optimal`, the float trap) builds the filter; the rest
// are `add` ops. A `union` scenario carries a second filter in the top-level
// `"other"` block (same with_params+add shape). Outputs: `bytes`/`union_bytes`
// are `0x`-hex strings (LSB-first within each byte, ascending bytes, length
// ceil(m/8)); `set_bits`/`union_set_bits` are sorted-ascending int arrays;
// `contains_<v>`/`union_contains_<v>` are bools. Unknown ops/keys SKIP
// (forward-compat); malformed (0 or >1 with_params) SKIPs like `from_sorted`.

/// Render a byte slice as the canonical lower-case `0x`-prefixed hex string,
/// byte 0 first (the serialized bit-array form).
fn bloom_bytes_hex(bytes: &[u8]) -> String {
    let mut s = String::from("0x");
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Build a `Bloom` from a scenario `operations` array: exactly one `with_params`
/// op (explicit m,k) followed by `add` ops. Returns `None` if the operations are
/// malformed — the caller then SKIPs (forward-compat), exactly like the
/// sorted-table `from_sorted` rule. "Malformed" here is anything an older/newer
/// runner cannot faithfully execute: zero or multiple `with_params`; an unknown
/// op; `m == 0` (a construction trap, kept native-test-only — never a shared
/// scenario); or an `m`/`k`/`value` operand outside its declared integer range.
/// Guarding these here means the shared runner never panics on a malformed or
/// forward-incompatible Bloom scenario; it SKIPs.
fn build_bloom(operations: &[Value]) -> Option<Bloom> {
    // Find the single with_params; reject zero or multiple.
    let mut params: Option<(u32, u32)> = None;
    for op in operations {
        if op["op"].as_str() == Some("with_params") {
            if params.is_some() {
                return None; // multiple with_params => malformed
            }
            // m/k must be exact u32 (a wider value would wrap under `as u32`).
            let m = u32::try_from(op["m"].as_u64()?).ok()?;
            let k = u32::try_from(op["k"].as_u64()?).ok()?;
            // m == 0 is a construction trap (native-test-only); never a shared
            // scenario — treat as malformed so the runner SKIPs, not panics.
            if m == 0 {
                return None;
            }
            params = Some((m, k));
        }
    }
    let (m, k) = params?; // zero with_params => malformed
    let mut b = Bloom::with_params(m, k);
    for op in operations {
        match op["op"].as_str().unwrap_or("") {
            "with_params" => {} // already consumed
            "add" => {
                // value must be an exact i32 (a wider value would wrap).
                let v = i32::try_from(op["value"].as_i64()?).ok()?;
                b.add(v);
            }
            // Forward-compat: an unknown op makes the scenario un-runnable here.
            _ => return None,
        }
    }
    Some(b)
}

fn run_bloom(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    full: &Value,
) {
    let Some(b) = build_bloom(operations) else {
        eprintln!("skip: malformed/unknown Bloom scenario (forward-compat): {scenario}");
        return;
    };

    // Optional second filter for union scenarios (top-level "other" block).
    let other: Option<Bloom> = full
        .get("other")
        .and_then(|o| o.get("operations"))
        .and_then(|ops| ops.as_array())
        .and_then(|ops| build_bloom(ops));
    // Compute the union only when the partners' params actually match. A
    // mismatched union is a native-test-only trap, never a shared scenario, so
    // here a mismatch simply leaves `union` unavailable (any `union_*` assertion
    // then SKIPs via UNKNOWN_ASSERTION) rather than panicking.
    let union: Option<Bloom> = other.as_ref().and_then(|o| {
        if o.m_bits() == b.m_bits() && o.k() == b.k() {
            Some(b.union(o))
        } else {
            None
        }
    });

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_bloom(&b, union.as_ref(), key);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_bloom(b: &Bloom, union: Option<&Bloom>, key: &str) -> String {
    match key {
        "m_bits" => b.m_bits().to_string(),
        "k" => b.k().to_string(),
        "bit_count" => b.bit_count().to_string(),
        "is_empty" => b.is_empty().to_string(),
        "set_bits" => format_u32_array(&b.set_bits()),
        "bytes" => bloom_bytes_hex(&b.to_bytes()),
        // Union assertions: require the "other" block (union computed in caller).
        "union_bit_count" => match union {
            Some(u) => u.bit_count().to_string(),
            None => format!("UNKNOWN_ASSERTION:{}", key),
        },
        "union_set_bits" => match union {
            Some(u) => format_u32_array(&u.set_bits()),
            None => format!("UNKNOWN_ASSERTION:{}", key),
        },
        "union_bytes" => match union {
            Some(u) => bloom_bytes_hex(&u.to_bytes()),
            None => format!("UNKNOWN_ASSERTION:{}", key),
        },
        // contains_<v> / union_contains_<v>: signed i32 suffix.
        _ if key.starts_with("union_contains_") => match union {
            Some(u) => match key["union_contains_".len()..].parse::<i32>() {
                Ok(v) => u.might_contain(v).to_string(),
                Err(_) => format!("UNKNOWN_ASSERTION:{}", key),
            },
            None => format!("UNKNOWN_ASSERTION:{}", key),
        },
        _ if key.starts_with("contains_") => match key["contains_".len()..].parse::<i32>() {
            Ok(v) => b.might_contain(v).to_string(),
            Err(_) => format!("UNKNOWN_ASSERTION:{}", key),
        },
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

fn format_u32_array(v: &[u32]) -> String {
    let parts: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    format!("[{}]", parts.join(","))
}

// ---- HyperLogLog (spec/features/hyperloglog.md) --------------------------
//
// A stored cardinality sketch. The cross-language oracle is the INTEGER register
// array (via `register_hex` / `nonzero_registers` / `max_register` /
// `register_at_N`) — NEVER the float `estimate` (float-quarantine Rule Q1; there
// is deliberately NO `estimate` assertion key here). Exactly one builder op,
// first: either a `with_precision(p)` (then zero or more `add`/`merge`) OR a
// single `from_bytes`. Zero/two builders or an `add` before the builder =>
// malformed => SKIP. A `merge` consumes the scenario's `other` HyperLogLog.
// Unknown ops/keys/kinds SKIP (forward-compat).

/// Build a HyperLogLog from an op list (used for the primary and the `other`
/// block). Returns `None` (=> caller SKIPs) when the op list is malformed for
/// the harness: not starting with exactly one builder, an `add`/`merge` before
/// the builder, an out-of-range `with_precision`, or a bad `from_bytes`.
fn build_hll(operations: &[Value], other: Option<&Value>) -> Option<HyperLogLog> {
    let first = operations.first()?;
    let first_op = first["op"].as_str().unwrap_or("");
    let mut hll = match first_op {
        "with_precision" => {
            let p = first["p"].as_u64()? as u8;
            // Out-of-range p is a construction error -> SKIP (the harness cannot
            // build the probe). The native tests pin the error path itself.
            HyperLogLog::with_precision(p).ok()?
        }
        "from_bytes" => {
            // `from_bytes` is the SOLE op when present (full state replacement,
            // first op or malformed). Reject any trailing ops.
            if operations.len() != 1 {
                eprintln!("skip: from_bytes must be the only op (forward-compat)");
                return None;
            }
            let bytes = parse_hex_bytes(&first["bytes"]);
            HyperLogLog::from_bytes(&bytes).ok()?
        }
        _ => {
            eprintln!("skip: HyperLogLog first op must be a builder (forward-compat)");
            return None;
        }
    };
    for op in &operations[1..] {
        match op["op"].as_str().unwrap_or("") {
            "add" => {
                let v = op["value"].as_i64()? as i32;
                hll.add(v);
            }
            "merge" => {
                // Merge the scenario's `other` HyperLogLog (built by its own
                // op list) by element-wise register max.
                let other_spec = other?;
                let other_ops = other_spec["operations"].as_array()?;
                let other_hll = build_hll(other_ops, None)?;
                hll.merge(&other_hll).ok()?;
            }
            other_op => {
                eprintln!(
                    "skip: unknown HyperLogLog op (forward-compat): {}",
                    other_op
                );
                return None;
            }
        }
    }
    Some(hll)
}

fn run_hyperloglog(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let other = scenario_obj.get("other");
    let hll = match build_hll(operations, other) {
        Some(h) => h,
        None => {
            eprintln!("skip: malformed HyperLogLog scenario (forward-compat)");
            return;
        }
    };
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_hll_assertion(key, &hll);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_hll_assertion(key: &str, hll: &HyperLogLog) -> String {
    match key {
        // The PRIMARY integer oracle: the full serialized form (HLL1 + p +
        // register bytes) as a lower-case, 0x-prefixed hex string.
        "register_hex" => {
            let mut s = String::from("0x");
            for b in hll.to_bytes() {
                s.push_str(&format!("{:02x}", b));
            }
            s
        }
        "nonzero_registers" => hll.nonzero_registers().to_string(),
        "max_register" => hll.max_register().to_string(),
        // NOTE: there is deliberately NO `estimate` key (float-quarantine Q1).
        _ if key.starts_with("register_at_") => {
            // Spot-check a specific register index.
            match key["register_at_".len()..].parse::<usize>() {
                Ok(n) if n < hll.register_count() => hll.registers()[n].to_string(),
                _ => format!("UNKNOWN_ASSERTION:{}", key),
            }
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- CountMin (spec/features/count-min.md) -------------------------------
//
// A `d×w` integer counter matrix. Built by exactly ONE `with_params` op (zero
// or multiple => malformed => SKIP, like `from_sorted`/`HashPipeline`); never
// `optimal` (the float-derivation trap is kept out of the shared suite — an
// `optimal`/`epsilon`/`delta` op is unknown here => SKIP). Subsequent `add` ops
// carry an i32 `value` and a `count` DECIMAL STRING (omitted => 1; may exceed
// 2^53). Counters / `estimate_<v>` / `total` are u64 DECIMAL STRINGS (the 2^64
// range exceeds JSON-safe 2^53); `depth`/`width` are plain ints. `counters` is
// the row-major (explicit-order, NOT sorted) primary oracle. Unknown ops/keys
// SKIP (forward-compat).

/// Parse a `count` operand: a DECIMAL STRING parsed straight to u64 (never via
/// f64), reusing the i64-suite's wide-integer discipline. A bare JSON number is
/// also accepted for small counts. Returns `None` if malformed (negative,
/// non-numeric, or exceeding `u64::MAX`) so the caller can SKIP the scenario.
fn parse_count_opt(v: &Value) -> Option<u64> {
    if v.is_null() {
        return Some(1); // count omitted => 1 (the add_one shape)
    }
    if let Some(s) = v.as_str() {
        s.parse::<u64>().ok()
    } else {
        v.as_u64()
    }
}

fn run_count_min(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    // Authoring rule: exactly ONE `with_params` op, first. The remaining ops are
    // `add`. Anything else (e.g. `optimal`) makes the scenario un-runnable here
    // => SKIP (forward-compat / float-trap quarantine).
    let with_params: Vec<&Value> = operations
        .iter()
        .filter(|op| op["op"].as_str() == Some("with_params"))
        .collect();
    if with_params.len() != 1 || operations.first().map(|o| &o["op"]) != Some(&with_params[0]["op"])
    {
        eprintln!(
            "skip: CountMin scenario needs exactly one leading `with_params` op (forward-compat)"
        );
        return;
    }
    let ctor = with_params[0];
    let d = ctor["d"].as_u64().expect("with_params needs d") as u32;
    let w = ctor["w"].as_u64().expect("with_params needs w") as u32;
    let mut cms = CountMin::with_params(d, w);

    for op in &operations[1..] {
        match op["op"].as_str().unwrap_or("") {
            "add" => {
                let value = op["value"].as_i64().expect("add needs i32 value") as i32;
                let count = match parse_count_opt(&op["count"]) {
                    Some(c) => c,
                    None => {
                        eprintln!("skip: CountMin add `count` is not a 0..=u64::MAX integer");
                        return;
                    }
                };
                cms.add(value, count);
            }
            other => {
                // Forward-compat: an unknown op makes the scenario un-runnable.
                eprintln!("skip: unknown CountMin op (forward-compat): {}", other);
                return;
            }
        }
    }

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_count_min(key, &cms);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_count_min(key: &str, cms: &CountMin) -> String {
    match key {
        // Row-major counter matrix, each u64 as a decimal string; explicit order.
        "counters" => {
            let parts: Vec<String> = cms
                .to_counters()
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect();
            format!("[{}]", parts.join(","))
        }
        // Scalar u64 decimal strings: emitted UNQUOTED (the expected JSON string
        // "8" renders via render_expected to the bare `8` in FloatMode::None,
        // matching the hash-pipeline hex-string convention). Only the `counters`
        // ARRAY elements keep their JSON quotes.
        "total" => cms.total().to_string(),
        "depth" => cms.depth().to_string(),
        "width" => cms.width().to_string(),
        // estimate_<v>: signed i32 suffix (matches ^estimate_(-?[0-9]+)$).
        _ if estimate_key(key).is_some() => cms.estimate(estimate_key(key).unwrap()).to_string(),
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

/// Recognise an `estimate_<v>` assertion: `<v>` is a SIGNED base-10 i32
/// (exact `^estimate_(-?[0-9]+)$`, full i32 range incl. negatives). A leading
/// `+` is rejected so the recogniser matches the documented regex.
fn estimate_key(key: &str) -> Option<i32> {
    let rest = key.strip_prefix("estimate_")?;
    let digits = rest.strip_prefix('-').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

// ---- SpaceSaving (spec/features/count-min.md) ----------------------------
//
// A bounded heavy-hitters summary. Built by exactly ONE `with_capacity` op,
// first (zero or multiple => SKIP). Subsequent `add` ops are applied IN LISTED
// ORDER (Space-Saving is order-dependent — a runner MUST NOT reorder). `value`
// is an i32; `count` is a u64 decimal string (omitted => 1). `monitored_set` /
// `top_k_<k>` are explicit-order arrays of `[item, count_str, error_str]`
// triples in canonical order (count DESC, signed item ASC). count/error are u64
// decimal strings (2^64 range); size/capacity plain ints. Unknown ops/keys SKIP.

fn run_space_saving(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
) {
    let with_capacity: Vec<&Value> = operations
        .iter()
        .filter(|op| op["op"].as_str() == Some("with_capacity"))
        .collect();
    if with_capacity.len() != 1
        || operations.first().map(|o| &o["op"]) != Some(&with_capacity[0]["op"])
    {
        eprintln!(
            "skip: SpaceSaving scenario needs exactly one leading `with_capacity` op (forward-compat)"
        );
        return;
    }
    let m = with_capacity[0]["m"]
        .as_u64()
        .expect("with_capacity needs m") as u32;
    let mut ss = SpaceSaving::with_capacity(m);

    for op in &operations[1..] {
        match op["op"].as_str().unwrap_or("") {
            "add" => {
                let value = op["value"].as_i64().expect("add needs i32 value") as i32;
                let count = match parse_count_opt(&op["count"]) {
                    Some(c) => c,
                    None => {
                        eprintln!("skip: SpaceSaving add `count` is not a 0..=u64::MAX integer");
                        return;
                    }
                };
                ss.add(value, count);
            }
            other => {
                eprintln!("skip: unknown SpaceSaving op (forward-compat): {}", other);
                return;
            }
        }
    }

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_space_saving(key, &ss);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

/// Render a `(item, count, error)` triple list as a JSON array of
/// `[item, "count", "error"]` (item int, count/error u64 decimal strings).
fn format_ss_triples(triples: &[(i32, u64, u64)]) -> String {
    let parts: Vec<String> = triples
        .iter()
        .map(|(item, count, error)| format!("[{},\"{}\",\"{}\"]", item, count, error))
        .collect();
    format!("[{}]", parts.join(","))
}

fn eval_space_saving(key: &str, ss: &SpaceSaving) -> String {
    match key {
        "monitored_set" => format_ss_triples(&ss.monitored_set()),
        "size" => ss.size().to_string(),
        "capacity" => ss.capacity().to_string(),
        // top_k_<k>: non-negative int suffix (matches ^top_k_([0-9]+)$).
        _ if top_k_key(key).is_some() => format_ss_triples(&ss.top_k(top_k_key(key).unwrap())),
        // count_<v> / error_<v>: signed i32 suffix; scalar u64 decimal strings
        // emitted UNQUOTED (the `monitored_set`/`top_k` triple ARRAYS keep the
        // count/error quotes; bare scalars do not, matching render_expected).
        _ if ss_signed_key(key, "count_").is_some() => {
            ss.count(ss_signed_key(key, "count_").unwrap()).to_string()
        }
        _ if ss_signed_key(key, "error_").is_some() => {
            ss.error(ss_signed_key(key, "error_").unwrap()).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

/// Recognise a `top_k_<k>` assertion: `<k>` is a NON-NEGATIVE base-10 int
/// (exact `^top_k_([0-9]+)$`).
fn top_k_key(key: &str) -> Option<u32> {
    let rest = key.strip_prefix("top_k_")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// Recognise a `<prefix><v>` assertion whose `<v>` is a SIGNED base-10 i32
/// (full range incl. negatives; leading `+` rejected). Used for
/// `count_<v>`/`error_<v>`.
fn ss_signed_key(key: &str, prefix: &str) -> Option<i32> {
    let rest = key.strip_prefix(prefix)?;
    let digits = rest.strip_prefix('-').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

// ---- FenwickTree (spec/features/fenwick.md) -------------------------------
//
// A fixed-size i32-element / i64-accumulator Binary Indexed Tree. Construction
// is EXACTLY ONE op (`with_size` or `from_values`) first, then any number of
// `update`/`set` point ops (all indices in-range; out-of-range traps are
// native-test-only). Sum-returning assertions (`total`, `get_<i>`,
// `prefix_sum_<i>`, `range_sum_<lo>_<hi>`, and each `tree` element) are i64 and
// wire-encoded as DECIMAL STRINGS (parsed straight to i64, never via f64); the
// runner accepts a bare JSON number too. `tree` is the canonical 1-based BIT
// array in 1-based index order — an explicit-order key, NOT sorted. Unknown
// ops / kinds / assertion keys SKIP (forward-compat).

// NOTE on i64 wire encoding: the runner emits every Fenwick i64 result via
// `i64::to_string()` (a plain decimal). The JSON `assertions` carry the
// authoritative i64 values as DECIMAL STRINGS (or bare numbers when small);
// `emit`/`render_expected` compare them as strings under FloatMode::None, so a
// decimal-string expected (`"total": "8589934588"`) and the runner's decimal
// output match without any f64 round-trip. Element operands (`delta`/`value`)
// stay plain JSON numbers (they are i32).

fn run_fenwick(scenario: &str, operations: &[Value], assertions: &serde_json::Map<String, Value>) {
    // Authoring rule: the FIRST op MUST be exactly one construction op
    // (`with_size` OR `from_values`); a missing/late/duplicate construction op
    // is a malformed scenario => SKIP (forward-compat), like the hash-pipeline
    // single-op and sorted-table from_sorted rules.
    if operations.is_empty() {
        eprintln!("skip: fenwick scenario must begin with a construction op (forward-compat)");
        return;
    }
    let first = operations[0]["op"].as_str().unwrap_or("");
    let mut tree = match first {
        "with_size" => {
            let n = operations[0]["n"].as_i64().unwrap_or(-1);
            if n < 0 {
                eprintln!("skip: fenwick with_size negative n (malformed): {}", n);
                return;
            }
            FenwickTree::with_size(n as usize)
        }
        "from_values" => {
            let vals: Vec<i32> = operations[0]["values"]
                .as_array()
                .expect("from_values needs values array")
                .iter()
                .map(|v| v.as_i64().expect("from_values element must be i32") as i32)
                .collect();
            FenwickTree::from_values(&vals)
        }
        other => {
            eprintln!(
                "skip: fenwick first op must be with_size/from_values (forward-compat): {}",
                other
            );
            return;
        }
    };
    // Any subsequent construction op is malformed => SKIP.
    for op in &operations[1..] {
        match op["op"].as_str().unwrap_or("") {
            "update" => {
                let i = op["index"].as_u64().expect("update needs index") as usize;
                let delta = op["delta"].as_i64().expect("update needs i32 delta") as i32;
                tree.update(i, delta);
            }
            "set" => {
                let i = op["index"].as_u64().expect("set needs index") as usize;
                let value = op["value"].as_i64().expect("set needs i32 value") as i32;
                tree.set(i, value);
            }
            "with_size" | "from_values" => {
                eprintln!("skip: fenwick has a non-first construction op (malformed)");
                return;
            }
            other => {
                eprintln!("skip: unknown fenwick op (forward-compat): {}", other);
                return;
            }
        }
    }

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_fenwick_assertion(key, &tree);
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_fenwick_assertion(key: &str, tree: &FenwickTree) -> String {
    match key {
        "size" => tree.len().to_string(),
        "is_empty" => tree.is_empty().to_string(),
        "total" => tree.total().to_string(),
        // Canonical 1-based BIT array, in 1-based index order (NOT sorted).
        "tree" => {
            let parts: Vec<String> = tree
                .canonical_tree()
                .iter()
                .map(|v| v.to_string())
                .collect();
            format!("[{}]", parts.join(","))
        }
        _ if key.starts_with("get_") => match key[4..].parse::<usize>() {
            Ok(i) => tree.get(i).to_string(),
            Err(_) => format!("UNKNOWN_ASSERTION:{}", key),
        },
        _ if key.starts_with("prefix_sum_") => match key[11..].parse::<usize>() {
            Ok(i) => tree.prefix_sum(i).to_string(),
            Err(_) => format!("UNKNOWN_ASSERTION:{}", key),
        },
        _ if key.starts_with("range_sum_") => {
            // ^range_sum_([0-9]+)_([0-9]+)$
            let rest = &key[10..];
            match rest.split_once('_') {
                Some((lo_s, hi_s)) => match (lo_s.parse::<usize>(), hi_s.parse::<usize>()) {
                    (Ok(lo), Ok(hi)) => tree.range_sum(lo, hi).to_string(),
                    _ => format!("UNKNOWN_ASSERTION:{}", key),
                },
                None => format!("UNKNOWN_ASSERTION:{}", key),
            }
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- RoaringU32 (spec/features/roaring-u32.md) ---------------------------
//
// A mutable, sparse, compressed u32 set. Values are i32 in the JSON suite,
// REINTERPRETED to u32 (not sign-extended). Ordering is UNSIGNED u32 ascending
// throughout (to_sorted_array, min, max, serialized chunk order). The byte
// oracle is serialized_hex (+ the four set-algebra hex keys); container_types /
// chunk_count / to_sorted_array localize a failure.
//
// Ops: add / remove / clear / add_range / remove_range / deserialize. A
// reversed range (from_u32 > to_u32 after reinterpret) is a MALFORMED scenario
// -> SKIP the whole scenario. A `deserialize` op must be the ONLY op in its
// scenario; mixing it with add/remove is malformed -> SKIP. Unknown ops SKIP
// (forward-compat).

/// Build a `RoaringU32` by applying the scenario's operation list. Returns
/// `None` if the scenario is malformed (reversed range, deserialize mixed with
/// other ops, bad deserialize hex) — the caller SKIPs.
fn build_roaring(operations: &[Value]) -> Option<RoaringU32> {
    // A single `deserialize` op builds the set from a literal hex image and
    // must be the only op.
    let has_deserialize = operations
        .iter()
        .any(|op| op["op"].as_str() == Some("deserialize"));
    if has_deserialize {
        if operations.len() != 1 {
            eprintln!("skip: `deserialize` op must be the only op (malformed)");
            return None;
        }
        let hex = operations[0]["bytes"]
            .as_str()
            .expect("deserialize op needs `bytes`");
        // Syntactically bad hex is ALSO a malformed scenario -> SKIP (not a
        // panic), matching the README "unparseable/non-canonical deserialize
        // image is SKIP" rule.
        let bytes = match parse_roaring_hex(hex) {
            Some(b) => b,
            None => {
                eprintln!("skip: malformed deserialize hex: {:?}", hex);
                return None;
            }
        };
        return match RoaringU32::deserialize(&bytes) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("skip: deserialize failed (malformed image): {}", e);
                None
            }
        };
    }

    let mut set = RoaringU32::new();
    for op in operations {
        match op["op"].as_str().unwrap_or("") {
            "add" => {
                let v = op["value"].as_i64().expect("add needs i32 value") as i32;
                set.add(v as u32);
            }
            "remove" => {
                let v = op["value"].as_i64().expect("remove needs i32 value") as i32;
                set.remove(v as u32);
            }
            "clear" => set.clear(),
            "add_range" | "remove_range" => {
                let from = op["from"].as_i64().expect("range needs `from`") as i32 as u32;
                let to = op["to"].as_i64().expect("range needs `to`") as i32 as u32;
                // Reversed range (unsigned) is a malformed scenario -> SKIP.
                if from > to {
                    eprintln!(
                        "skip: reversed range from={:#010x} > to={:#010x} (malformed)",
                        from, to
                    );
                    return None;
                }
                let add = op["op"].as_str() == Some("add_range");
                // Inclusive [from, to]; `to` may be u32::MAX so iterate carefully.
                let mut v = from;
                loop {
                    if add {
                        set.add(v);
                    } else {
                        set.remove(v);
                    }
                    if v == to {
                        break;
                    }
                    v += 1;
                }
            }
            // Forward-compat: an unknown op makes the scenario un-runnable here.
            other => {
                eprintln!("skip: unknown roaring op (forward-compat): {}", other);
                return None;
            }
        }
    }
    Some(set)
}

/// Parse a `0x`-prefixed hex byte string for a `deserialize` op. Returns `None`
/// for syntactically malformed hex (missing `0x`, odd digit count, bad digit) —
/// a malformed scenario the caller SKIPs (never a panic).
fn parse_roaring_hex(s: &str) -> Option<Vec<u8>> {
    let body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))?;
    if body.len() % 2 != 0 {
        return None;
    }
    (0..body.len() / 2)
        .map(|i| u8::from_str_radix(&body[2 * i..2 * i + 2], 16).ok())
        .collect()
}

/// Lower-case `0x`-prefixed hex string of a byte image (the serialized oracle).
fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// `to_sorted_array` in UNSIGNED u32 ascending order, emitted as i32 (explicit-
/// order key — NOT re-sorted signed).
fn roaring_sorted_array(set: &RoaringU32) -> String {
    let parts: Vec<String> = set
        .to_sorted_vec()
        .iter()
        .map(|v| (*v as i32).to_string())
        .collect();
    format!("[{}]", parts.join(","))
}

fn run_roaring(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let set = match build_roaring(operations) {
        Some(s) => s,
        None => return, // malformed / forward-compat skip
    };

    // The `other` collection (a second RoaringU32) drives set-algebra keys.
    let other = scenario_obj
        .get("other")
        .and_then(|o| o["operations"].as_array())
        .and_then(|ops| build_roaring(ops));

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_roaring_assertion(key, &set, other.as_ref());
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

fn eval_roaring_assertion(key: &str, set: &RoaringU32, other: Option<&RoaringU32>) -> String {
    match key {
        "cardinality" => set.cardinality().to_string(),
        "is_empty" => set.is_empty().to_string(),
        "chunk_count" => set.chunk_count().to_string(),
        "serialized_len" => set.serialize().len().to_string(),
        "serialized_hex" => bytes_to_hex(&set.serialize()),
        "to_sorted_array" => roaring_sorted_array(set),
        "min" => set
            .min()
            .map(|v| (v as i32).to_string())
            .unwrap_or_else(|| "null".into()),
        "max" => set
            .max()
            .map(|v| (v as i32).to_string())
            .unwrap_or_else(|| "null".into()),
        "container_types" => {
            let parts: Vec<String> = set
                .container_types()
                .iter()
                .map(|t| format!("\"{}\"", t))
                .collect();
            format!("[{}]", parts.join(","))
        }
        // Set-algebra byte oracle + cardinalities (require `other`).
        "union_serialized_hex" if other.is_some() => {
            bytes_to_hex(&set.or(other.unwrap()).serialize())
        }
        "intersect_serialized_hex" if other.is_some() => {
            bytes_to_hex(&set.and(other.unwrap()).serialize())
        }
        "and_not_serialized_hex" if other.is_some() => {
            bytes_to_hex(&set.and_not(other.unwrap()).serialize())
        }
        "xor_serialized_hex" if other.is_some() => {
            bytes_to_hex(&set.xor(other.unwrap()).serialize())
        }
        "union_cardinality" if other.is_some() => set.or(other.unwrap()).cardinality().to_string(),
        "intersect_cardinality" if other.is_some() => {
            set.and(other.unwrap()).cardinality().to_string()
        }
        "and_not_cardinality" if other.is_some() => {
            set.and_not(other.unwrap()).cardinality().to_string()
        }
        "xor_cardinality" if other.is_some() => set.xor(other.unwrap()).cardinality().to_string(),
        _ if key.starts_with("contains_") => {
            // Signed i32 suffix, reinterpreted to u32.
            let v: i32 = key[9..].parse().unwrap();
            set.contains(v as u32).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
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

// NavigableSet result-log: poll/remove_range return values recorded in
// execution order while applying operations (see README §NavigableMap).
#[derive(Default)]
struct NavLog {
    poll_first_keys: Vec<Option<i32>>,
    poll_last_keys: Vec<Option<i32>>,
    poll_first_values: Vec<Option<i32>>,
    poll_last_values: Vec<Option<i32>>,
    remove_range_counts: Vec<i32>,
}

fn opt_array(v: &[Option<i32>]) -> String {
    let parts: Vec<String> = v
        .iter()
        .map(|x| x.map(|n| n.to_string()).unwrap_or_else(|| "null".into()))
        .collect();
    format!("[{}]", parts.join(","))
}

fn run_treeset(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let mut set: BTreeSet<i32> = BTreeSet::new();
    let mut log = NavLog::default();
    for op in operations {
        match op["op"].as_str().unwrap() {
            "add" => {
                set.insert(op["value"].as_i64().unwrap() as i32);
            }
            "remove" => {
                set.remove(&(op["value"].as_i64().unwrap() as i32));
            }
            "clear" => set.clear(),
            "poll_first" => {
                let e = set.iter().next().copied();
                if let Some(x) = e {
                    set.remove(&x);
                }
                log.poll_first_keys.push(e);
            }
            "poll_last" => {
                let e = set.iter().next_back().copied();
                if let Some(x) = e {
                    set.remove(&x);
                }
                log.poll_last_keys.push(e);
            }
            "remove_range" => {
                let range = build_range_obj(&op["range"]);
                let victims: Vec<i32> =
                    set.iter().copied().filter(|x| range.contains(*x)).collect();
                let count = victims.len() as i32;
                for x in &victims {
                    set.remove(x);
                }
                log.remove_range_counts.push(count);
            }
            // Forward-compat: an unknown op must not crash an older/newer
            // runner mix; skip it (mirrors unknown-collection/assertion skip).
            _ => {}
        }
    }
    let query = scenario_obj.get("query").map(build_range_obj);
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => set.len().to_string(),
            "is_empty" => set.is_empty().to_string(),
            "min" | "first" => set
                .iter()
                .next()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
            "max" | "last" => set
                .iter()
                .next_back()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
            "to_sorted_array" => {
                let v: Vec<i32> = set.iter().copied().collect();
                format_array(&v)
            }
            "descending_elements" => {
                let v: Vec<i32> = set.iter().rev().copied().collect();
                format_array(&v)
            }
            "range_elements" => match &query {
                Some(r) => format_array(
                    &set.iter()
                        .copied()
                        .filter(|x| r.contains(*x))
                        .collect::<Vec<i32>>(),
                ),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_elements_desc" => match &query {
                Some(r) => format_array(
                    &set.iter()
                        .rev()
                        .copied()
                        .filter(|x| r.contains(*x))
                        .collect::<Vec<i32>>(),
                ),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_size" => match &query {
                Some(r) => set.iter().filter(|x| r.contains(**x)).count().to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "poll_first_keys" => opt_array(&log.poll_first_keys),
            "poll_last_keys" => opt_array(&log.poll_last_keys),
            "remove_range_counts" => format_array(&log.remove_range_counts),
            _ if nav_key_prefix(key).is_some() => {
                let (kind, n) = nav_key_prefix(key).unwrap();
                opt_i32_str(set_nav(&set, kind, n))
            }
            _ if rank_key(key).is_some() => set_rank(&set, rank_key(key).unwrap()).to_string(),
            _ if select_index(key).is_some() => {
                opt_i32_str(set_select(&set, select_index(key).unwrap()))
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

/// Recognise a `floor_<k>`/`ceiling_<k>`/`lower_<k>`/`higher_<k>` assertion
/// key. `<k>` is parsed as a SIGNED base-10 i32 (leading `-` and the full
/// i32 range allowed). Returns `(prefix, key)` on a match.
fn nav_key_prefix(key: &str) -> Option<(&'static str, i32)> {
    for prefix in ["floor_", "ceiling_", "lower_", "higher_"] {
        if let Some(rest) = key.strip_prefix(prefix) {
            // Must parse as signed i32 — otherwise it is not a nav key.
            if let Ok(n) = rest.parse::<i32>() {
                let p = match prefix {
                    "floor_" => "floor",
                    "ceiling_" => "ceiling",
                    "lower_" => "lower",
                    _ => "higher",
                };
                return Some((p, n));
            }
        }
    }
    None
}

/// Recognise a `rank_<k>` order-statistic assertion: `<k>` is a SIGNED
/// base-10 i32 (exact `^rank_(-?[0-9]+)$`, full i32 range incl. negatives).
/// Returns the parsed key on a match.
fn rank_key(key: &str) -> Option<i32> {
    let rest = key.strip_prefix("rank_")?;
    // Exact grammar `-?[0-9]+`: reject a leading `+` (which `i32::parse` would
    // otherwise accept) so the recogniser matches the documented regex.
    let digits = rest.strip_prefix('-').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// Recognise a `select_<i>` order-statistic assertion: `<i>` is a
/// NON-NEGATIVE base-10 index (exact `^select_([0-9]+)$`). Returns the
/// parsed index on a match. This must NOT match the functional predicate
/// keys (`select_gt_N`, `select_even`, …) — `parse::<usize>` rejects them
/// since they are not all-digits.
fn select_index(key: &str) -> Option<usize> {
    let rest = key.strip_prefix("select_")?;
    // Exact grammar `[0-9]+`: all-digits (rejects a leading `+`/`-` and the
    // functional `select_gt_N`/`select_even` predicate keys).
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// `rank` over a sorted-int oracle: count of elements strictly less than `k`.
fn set_rank(set: &BTreeSet<i32>, k: i32) -> usize {
    set.range(..k).count()
}

/// `select(i)`: i-th smallest element (0-based), or `None` if `i >= len`.
fn set_select(set: &BTreeSet<i32>, i: usize) -> Option<i32> {
    set.iter().nth(i).copied()
}

fn set_nav(set: &BTreeSet<i32>, kind: &str, k: i32) -> Option<i32> {
    match kind {
        "floor" => set.range(..=k).next_back().copied(),
        "ceiling" => set.range(k..).next().copied(),
        "lower" => set.range(..k).next_back().copied(),
        "higher" => set
            .range((std::ops::Bound::Excluded(k), std::ops::Bound::Unbounded))
            .next()
            .copied(),
        _ => None,
    }
}

// ---- TreeMap<i32, i32> ----------------------------------------------------

fn run_treemap(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
    construction: Option<&str>,
) {
    let mut map: BTreeMap<i32, i32> = BTreeMap::new();
    let mut log = NavLog::default();
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
                "poll_first" => {
                    let e = map.iter().next().map(|(k, v)| (*k, *v));
                    if let Some((k, _)) = e {
                        map.remove(&k);
                    }
                    log.poll_first_keys.push(e.map(|(k, _)| k));
                    log.poll_first_values.push(e.map(|(_, v)| v));
                }
                "poll_last" => {
                    let e = map.iter().next_back().map(|(k, v)| (*k, *v));
                    if let Some((k, _)) = e {
                        map.remove(&k);
                    }
                    log.poll_last_keys.push(e.map(|(k, _)| k));
                    log.poll_last_values.push(e.map(|(_, v)| v));
                }
                "remove_range" => {
                    let range = build_range_obj(&op["range"]);
                    let victims: Vec<i32> =
                        map.keys().copied().filter(|k| range.contains(*k)).collect();
                    let count = victims.len() as i32;
                    for k in &victims {
                        map.remove(k);
                    }
                    log.remove_range_counts.push(count);
                }
                // Forward-compat: skip unknown ops.
                _ => {}
            }
        }
    }
    let query = scenario_obj.get("query").map(build_range_obj);
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => map.len().to_string(),
            "is_empty" => map.is_empty().to_string(),
            "min" | "first_key" => map
                .iter()
                .next()
                .map(|(k, _)| k.to_string())
                .unwrap_or_else(|| "null".into()),
            "max" | "last_key" => map
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
            "descending_keys" => {
                let v: Vec<i32> = map.keys().rev().copied().collect();
                format_array(&v)
            }
            "range_keys" => match &query {
                Some(r) => format_array(
                    &map.keys()
                        .copied()
                        .filter(|k| r.contains(*k))
                        .collect::<Vec<i32>>(),
                ),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_keys_desc" => match &query {
                Some(r) => format_array(
                    &map.keys()
                        .rev()
                        .copied()
                        .filter(|k| r.contains(*k))
                        .collect::<Vec<i32>>(),
                ),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_size" => match &query {
                Some(r) => map.keys().filter(|k| r.contains(**k)).count().to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "poll_first_keys" => opt_array(&log.poll_first_keys),
            "poll_last_keys" => opt_array(&log.poll_last_keys),
            "poll_first_values" => opt_array(&log.poll_first_values),
            "poll_last_values" => opt_array(&log.poll_last_values),
            "remove_range_counts" => format_array(&log.remove_range_counts),
            _ if nav_key_prefix(key).is_some() => {
                let (kind, n) = nav_key_prefix(key).unwrap();
                opt_i32_str(map_nav(&map, kind, n))
            }
            _ if rank_key(key).is_some() => {
                let k = rank_key(key).unwrap();
                map.range(..k).count().to_string()
            }
            _ if select_index(key).is_some() => {
                opt_i32_str(map.keys().nth(select_index(key).unwrap()).copied())
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

fn map_nav(map: &BTreeMap<i32, i32>, kind: &str, k: i32) -> Option<i32> {
    match kind {
        "floor" => map.range(..=k).next_back().map(|(k, _)| *k),
        "ceiling" => map.range(k..).next().map(|(k, _)| *k),
        "lower" => map.range(..k).next_back().map(|(k, _)| *k),
        "higher" => map
            .range((std::ops::Bound::Excluded(k), std::ops::Bound::Unbounded))
            .next()
            .map(|(k, _)| *k),
        _ => None,
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
    let mut set: DynTreeSet<HashableF32> =
        TreeSet::with_comparator(natural_comparator::<HashableF32>());
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

// ---- Range<i32> ----------------------------------------------------------

// The Bound/Range value model (spec/features/bound-range.md). Exactly ONE
// constructor op builds the range under test; an optional "other" block (same
// single-builder shape) supplies the second range for binary ops. Routed
// through the production Range<i32> — every assertion is proved against the
// real cut algebra, not re-derived here.
fn build_range(ops: &[Value]) -> Range<i32> {
    let constructors: Vec<&Value> = ops.iter().collect();
    if constructors.len() != 1 {
        panic!("Range<i32> scenario must have exactly one constructor op");
    }
    build_range_obj(constructors[0])
}

/// Build a `Range<i32>` from a single range-builder object (the `10-range`
/// op shape). Shared by the `Range<i32>` runner and the NavigableMap/Set
/// `range`/`query` fields.
fn build_range_obj(op: &Value) -> Range<i32> {
    let lower = || op["lower"].as_i64().expect("missing lower") as i32;
    let upper = || op["upper"].as_i64().expect("missing upper") as i32;
    match op["op"].as_str().unwrap() {
        "closed" => Range::closed(lower(), upper()),
        "open" => Range::open(lower(), upper()),
        "closed_open" => Range::closed_open(lower(), upper()),
        "open_closed" => Range::open_closed(lower(), upper()),
        "at_least" => Range::at_least(lower()),
        "greater_than" => Range::greater_than(lower()),
        "at_most" => Range::at_most(upper()),
        "less_than" => Range::less_than(upper()),
        "all" => Range::all(),
        "singleton" => Range::singleton(op["value"].as_i64().expect("missing value") as i32),
        other => panic!("unknown range op: {}", other),
    }
}

fn bound_type_str(bt: Option<BoundType>) -> String {
    match bt {
        Some(BoundType::Open) => "open".to_string(),
        Some(BoundType::Closed) => "closed".to_string(),
        None => "null".to_string(),
    }
}

fn opt_i32_str(v: Option<i32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "null".into())
}

fn run_range(
    scenario_name: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario: &Value,
) {
    let range = build_range(operations);
    let other = scenario.get("other").map(|spec| {
        let ops = spec["operations"].as_array().expect("other.operations");
        build_range(ops)
    });

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_range_assertion(key, &range, other.as_ref());
        emit(scenario_name, key, &computed, expected, FloatMode::None);
    }
}

fn eval_range_assertion(key: &str, range: &Range<i32>, other: Option<&Range<i32>>) -> String {
    match key {
        "is_empty" => range.is_empty().to_string(),
        "has_lower_bound" => range.has_lower_bound().to_string(),
        "has_upper_bound" => range.has_upper_bound().to_string(),
        "lower_bound_type" => bound_type_str(range.lower_bound_type()),
        "upper_bound_type" => bound_type_str(range.upper_bound_type()),
        "lower_endpoint" => opt_i32_str(range.lower_endpoint()),
        "upper_endpoint" => opt_i32_str(range.upper_endpoint()),
        _ if key.starts_with("contains_") => {
            let n: i32 = key[9..].parse().expect("contains_<N> integer");
            range.contains(n).to_string()
        }
        // ---- binary ops: require "other" -------------------------------
        "encloses_other" if other.is_some() => range.encloses(other.unwrap()).to_string(),
        "is_connected_other" if other.is_some() => range.is_connected(other.unwrap()).to_string(),
        "span_lower" if other.is_some() => opt_i32_str(range.span(other.unwrap()).lower_endpoint()),
        "span_upper" if other.is_some() => opt_i32_str(range.span(other.unwrap()).upper_endpoint()),
        "span_lower_type" if other.is_some() => {
            bound_type_str(range.span(other.unwrap()).lower_bound_type())
        }
        "span_upper_type" if other.is_some() => {
            bound_type_str(range.span(other.unwrap()).upper_bound_type())
        }
        "intersection_is_none" if other.is_some() => {
            range.intersection(other.unwrap()).is_none().to_string()
        }
        "intersection_is_empty" if other.is_some() => range
            .intersection(other.unwrap())
            .map(|i| i.is_empty())
            .unwrap_or(false)
            .to_string(),
        "intersection_lower" if other.is_some() => opt_i32_str(
            range
                .intersection(other.unwrap())
                .and_then(|i| i.lower_endpoint()),
        ),
        "intersection_upper" if other.is_some() => opt_i32_str(
            range
                .intersection(other.unwrap())
                .and_then(|i| i.upper_endpoint()),
        ),
        "intersection_lower_type" if other.is_some() => bound_type_str(
            range
                .intersection(other.unwrap())
                .and_then(|i| i.lower_bound_type()),
        ),
        "intersection_upper_type" if other.is_some() => bound_type_str(
            range
                .intersection(other.unwrap())
                .and_then(|i| i.upper_bound_type()),
        ),
        "intersection_has_lower_bound" if other.is_some() => range
            .intersection(other.unwrap())
            .map(|i| i.has_lower_bound())
            .unwrap_or(false)
            .to_string(),
        "intersection_has_upper_bound" if other.is_some() => range
            .intersection(other.unwrap())
            .map(|i| i.has_upper_bound())
            .unwrap_or(false)
            .to_string(),
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}

// ---- ImmutableSortedMap<i32, i32> / ImmutableSortedSet<i32> --------------
//
// The compact immutable sorted map/set (spec/features/sorted-table-map.md).
// Routed through the PRODUCTION ImmutableSortedMap/Set — every assertion is
// proved against the real packed-array binary-search code, not re-derived.
//
// Construction is a SINGLE `from_sorted` bulk op (no incremental put/add):
//   map: {"op":"from_sorted","keys":[...],"values":[...]}  (strictly ascending)
//   set: {"op":"from_sorted","elements":[...]}             (strictly ascending)
// Authoring rule (spec §"Cross-language test scenarios"): exactly ONE
// `from_sorted` op. Zero or multiple is a MALFORMED scenario -> SKIP it (do
// not silently apply the first, do not fail), pinning the behaviour so runner
// authors do not each invent their own. Scenarios in the suite are authored
// strictly-ascending, so production never traps here.

/// The single `from_sorted` op from a well-formed sorted-table scenario, or
/// `None` (malformed) -> the caller SKIPs. A sorted-table collection is built
/// by EXACTLY ONE bulk `from_sorted` op: the `operations` array must be that
/// one op and nothing else. Any other shape — zero ops, multiple ops, or a
/// `from_sorted` mixed with a stray `put`/`add`/unknown op — is malformed and
/// is skipped (never partially applied), so runner authors cannot diverge on
/// how to treat the extras.
fn single_from_sorted(operations: &[Value]) -> Option<&Value> {
    match operations {
        [only] if only["op"].as_str() == Some("from_sorted") => Some(only),
        _ => None,
    }
}

fn i32_array(v: &Value) -> Vec<i32> {
    v.as_array()
        .expect("from_sorted: expected array")
        .iter()
        .map(|e| e.as_i64().expect("i32 array element") as i32)
        .collect()
}

fn run_immutable_sorted_map(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let Some(op) = single_from_sorted(operations) else {
        // Malformed (zero or multiple from_sorted) -> SKIP, do not fail.
        eprintln!("skip: malformed sorted-table scenario (expected exactly one from_sorted)");
        return;
    };
    let keys = i32_array(&op["keys"]);
    let values = i32_array(&op["values"]);
    let map: ImmutableSortedMap<i32, i32> = ImmutableSortedMap::from_sorted(&keys, &values);
    let query = scenario_obj.get("query").map(build_range_obj);

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => map.len().to_string(),
            "is_empty" => map.is_empty().to_string(),
            "min" | "first_key" => opt_i32_str(map.first_key().copied()),
            "max" | "last_key" => opt_i32_str(map.last_key().copied()),
            "sorted_keys" => format_array(&map.keys().copied().collect::<Vec<i32>>()),
            // values() iterates in ascending-KEY order; the suite's
            // `sorted_values` means "all values sorted ascending" (it cannot
            // pin key-order pairing — that is a native test), so sort a copy.
            "sorted_values" => {
                let mut vs: Vec<i32> = map.values().copied().collect();
                vs.sort();
                format_array(&vs)
            }
            "descending_keys" => format_array(&map.descending_keys()),
            "range_keys" => match &query {
                Some(r) => format_array(&map.range_keys(*r)),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_keys_desc" => match &query {
                Some(r) => format_array(&map.descending_range_keys(*r)),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_size" => match &query {
                Some(r) => map.range_keys(*r).len().to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            _ if nav_key_prefix(key).is_some() => {
                let (kind, n) = nav_key_prefix(key).unwrap();
                let r = match kind {
                    "floor" => map.floor_key(&n),
                    "ceiling" => map.ceiling_key(&n),
                    "lower" => map.lower_key(&n),
                    _ => map.higher_key(&n),
                };
                opt_i32_str(r.copied())
            }
            _ if rank_key(key).is_some() => map.rank(&rank_key(key).unwrap()).to_string(),
            _ if select_index(key).is_some() => {
                opt_i32_str(map.select_key(select_index(key).unwrap()).copied())
            }
            _ if key.starts_with("get_") => {
                let k: i32 = key[4..].parse().unwrap();
                opt_i32_str(map.get(&k).copied())
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

fn run_immutable_sorted_set(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let Some(op) = single_from_sorted(operations) else {
        eprintln!("skip: malformed sorted-table scenario (expected exactly one from_sorted)");
        return;
    };
    let elements = i32_array(&op["elements"]);
    let set: ImmutableSortedSet<i32> = ImmutableSortedSet::from_sorted(&elements);
    let query = scenario_obj.get("query").map(build_range_obj);

    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "size" => set.len().to_string(),
            "is_empty" => set.is_empty().to_string(),
            "min" | "first" => opt_i32_str(set.first().copied()),
            "max" | "last" => opt_i32_str(set.last().copied()),
            "to_sorted_array" => format_array(&set.elements().copied().collect::<Vec<i32>>()),
            "descending_elements" => format_array(&set.descending_elements()),
            "range_elements" => match &query {
                Some(r) => format_array(&set.range_elements(*r)),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_elements_desc" => match &query {
                Some(r) => format_array(&set.descending_range_elements(*r)),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "range_size" => match &query {
                Some(r) => set.range_elements(*r).len().to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            _ if nav_key_prefix(key).is_some() => {
                let (kind, n) = nav_key_prefix(key).unwrap();
                let r = match kind {
                    "floor" => set.floor(&n),
                    "ceiling" => set.ceiling(&n),
                    "lower" => set.lower(&n),
                    _ => set.higher(&n),
                };
                opt_i32_str(r.copied())
            }
            _ if rank_key(key).is_some() => set.rank(&rank_key(key).unwrap()).to_string(),
            _ if select_index(key).is_some() => {
                opt_i32_str(set.select(select_index(key).unwrap()).copied())
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

// ---- RangeSet<i32> / RangeMap<i32, i32> -----------------------------------
//
// The auto-coalescing RangeSet / piecewise RangeMap (spec/features/
// range-set-map.md). Routed through the PRODUCTION RangeSet / RangeMap — every
// assertion is proved against the real cut-algebra coalescing/split/complement
// code, not re-derived here.
//
// A RangeSet/RangeMap is a STATEFUL structure built by a sequence of mutating
// ops, each naming a `range` via the shared 10-range builder object:
//   RangeSet: {"op":"add","range":{...}} / {"op":"remove_range","range":{...}}
//             / {"op":"clear"}
//   RangeMap: {"op":"put","range":{...},"value":N}
//             / {"op":"put_coalescing","range":{...},"value":N}
//             / {"op":"remove_range","range":{...}} / {"op":"clear"}
// An optional top-level `query` (same builder shape) supplies the range for
// `encloses_query`/`intersects_query`/`sub_range_set_ranges`/
// `sub_range_map_entries`. Unknown ops/keys/kinds SKIP (forward-compat).
//
// The `as_ranges`/`complement_ranges`/`sub_range_set_ranges`/`as_map_of_ranges`/
// `sub_range_map_entries` arrays are EXPLICIT-ORDER (ascending by lower cut),
// each element a fixed-shape range/entry object pinning the exact cut.

/// Serialize a `Range<i32>` as the fixed-shape assertion object
/// `{"lower":..,"lower_type":..,"upper":..,"upper_type":..}` — endpoints are
/// the i32 value or `null` when unbounded; `*_type` is `"open"`/`"closed"`/null.
/// Rendered as a single compact line so it matches the JSON `to_string()` of the
/// expected array element (keys in the same `serde_json` object order).
fn range_obj_str(r: &Range<i32>) -> String {
    format!(
        "{{\"lower\":{},\"lower_type\":{},\"upper\":{},\"upper_type\":{}}}",
        opt_i32_json(r.lower_endpoint()),
        bound_type_json(r.lower_bound_type()),
        opt_i32_json(r.upper_endpoint()),
        bound_type_json(r.upper_bound_type()),
    )
}

/// Serialize a `(Range<i32>, value)` RangeMap entry: the range object plus a
/// trailing `"value":<i32>`.
fn entry_obj_str(r: &Range<i32>, value: i32) -> String {
    format!(
        "{{\"lower\":{},\"lower_type\":{},\"upper\":{},\"upper_type\":{},\"value\":{}}}",
        opt_i32_json(r.lower_endpoint()),
        bound_type_json(r.lower_bound_type()),
        opt_i32_json(r.upper_endpoint()),
        bound_type_json(r.upper_bound_type()),
        value,
    )
}

fn opt_i32_json(v: Option<i32>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "null".into())
}

fn bound_type_json(bt: Option<BoundType>) -> String {
    match bt {
        Some(BoundType::Open) => "\"open\"".to_string(),
        Some(BoundType::Closed) => "\"closed\"".to_string(),
        None => "null".to_string(),
    }
}

/// Render an explicit-order array of range objects as a compact JSON line
/// matching `serde_json::Value::to_string()` of the expected array.
fn range_array_str(ranges: &[Range<i32>]) -> String {
    let parts: Vec<String> = ranges.iter().map(range_obj_str).collect();
    format!("[{}]", parts.join(","))
}

fn entry_array_str(entries: &[(Range<i32>, i32)]) -> String {
    let parts: Vec<String> = entries.iter().map(|(r, v)| entry_obj_str(r, *v)).collect();
    format!("[{}]", parts.join(","))
}

/// Parse a signed base-10 i32 suffix (leading `-` allowed, rejects `+`) from a
/// `<prefix><N>` assertion key — the `contains_<v>` / `get_<v>` /
/// `range_containing_<v>` / `get_entry_<v>` convention.
fn signed_i32_suffix(key: &str, prefix: &str) -> Option<i32> {
    let rest = key.strip_prefix(prefix)?;
    let digits = rest.strip_prefix('-').unwrap_or(rest);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn run_range_set(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let mut set: RangeSet<i32> = RangeSet::new();
    for op in operations {
        match op["op"].as_str().unwrap_or("") {
            "add" => set.add(build_range_obj(&op["range"])),
            "remove_range" => set.remove(build_range_obj(&op["range"])),
            "clear" => set.clear(),
            // Forward-compat: unknown op kinds skip (do not crash the runner).
            _ => {}
        }
    }
    let query = scenario_obj.get("query").map(build_range_obj);
    let span = set.span();
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "is_empty" => set.is_empty().to_string(),
            "as_ranges" => range_array_str(&set.as_ranges().collect::<Vec<_>>()),
            "span_lower" => opt_i32_str(span.and_then(|r| r.lower_endpoint())),
            "span_upper" => opt_i32_str(span.and_then(|r| r.upper_endpoint())),
            "span_lower_type" => bound_type_str(span.and_then(|r| r.lower_bound_type())),
            "span_upper_type" => bound_type_str(span.and_then(|r| r.upper_bound_type())),
            "encloses_query" => match &query {
                Some(q) => set.encloses(q).to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "intersects_query" => match &query {
                Some(q) => set.intersects(q).to_string(),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            "complement_ranges" => {
                range_array_str(&set.complement().as_ranges().collect::<Vec<_>>())
            }
            "sub_range_set_ranges" => match &query {
                Some(q) => range_array_str(&set.sub_range_set(q).as_ranges().collect::<Vec<_>>()),
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            _ if signed_i32_suffix(key, "range_containing_").is_some() => {
                let n = signed_i32_suffix(key, "range_containing_").unwrap();
                match set.range_containing(n) {
                    Some(r) => range_obj_str(&r),
                    None => "null".to_string(),
                }
            }
            _ if signed_i32_suffix(key, "contains_").is_some() => {
                let n = signed_i32_suffix(key, "contains_").unwrap();
                set.contains(n).to_string()
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        emit(scenario, key, &v, expected, FloatMode::None);
    }
}

fn run_range_map(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let mut map: RangeMap<i32, i32> = RangeMap::new();
    for op in operations {
        match op["op"].as_str().unwrap_or("") {
            "put" => {
                let value = op["value"].as_i64().expect("put needs value") as i32;
                map.put(build_range_obj(&op["range"]), value);
            }
            "put_coalescing" => {
                let value = op["value"].as_i64().expect("put_coalescing needs value") as i32;
                map.put_coalescing(build_range_obj(&op["range"]), value);
            }
            "remove_range" => map.remove(build_range_obj(&op["range"])),
            "clear" => map.clear(),
            // Forward-compat: unknown op kinds skip.
            _ => {}
        }
    }
    let query = scenario_obj.get("query").map(build_range_obj);
    let span = map.span();
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let v = match key.as_str() {
            "is_empty" => map.is_empty().to_string(),
            "as_map_of_ranges" => {
                let entries: Vec<(Range<i32>, i32)> =
                    map.as_map_of_ranges().map(|(r, v)| (r, *v)).collect();
                entry_array_str(&entries)
            }
            "span_lower" => opt_i32_str(span.and_then(|r| r.lower_endpoint())),
            "span_upper" => opt_i32_str(span.and_then(|r| r.upper_endpoint())),
            "span_lower_type" => bound_type_str(span.and_then(|r| r.lower_bound_type())),
            "span_upper_type" => bound_type_str(span.and_then(|r| r.upper_bound_type())),
            "sub_range_map_entries" => match &query {
                Some(q) => {
                    let entries: Vec<(Range<i32>, i32)> = map
                        .sub_range_map(q)
                        .as_map_of_ranges()
                        .map(|(r, v)| (r, *v))
                        .collect();
                    entry_array_str(&entries)
                }
                None => format!("UNKNOWN_ASSERTION:{}", key),
            },
            _ if signed_i32_suffix(key, "get_entry_").is_some() => {
                let n = signed_i32_suffix(key, "get_entry_").unwrap();
                match map.get_entry(n) {
                    Some((r, v)) => entry_obj_str(&r, *v),
                    None => "null".to_string(),
                }
            }
            _ if signed_i32_suffix(key, "get_").is_some() => {
                let n = signed_i32_suffix(key, "get_").unwrap();
                opt_i32_str(map.get(n).copied())
            }
            _ => format!("UNKNOWN_ASSERTION:{}", key),
        };
        emit(scenario, key, &v, expected, FloatMode::None);
    }
}

// ---- BoundedLruMap<i32, i32> ---------------------------------------------
//
// The bounded LRU map (spec/features/bounded-lru.md). Routed through the
// PRODUCTION BoundedLruMap<i32> + arena/slot-index intrusive LRU list. The
// recording eviction callback (always installed by the runner) appends each
// (key, value, cause) triple to an ordered eviction LOG — the load-bearing
// cross-language oracle.
//
// Config (on the scenario object): `max_size` (required, non-negative) and
// `ttl` (a logical-tick TTL, or null/absent for a pure max-size map). `now`
// and `ttl` are decimal STRINGS if they exceed 2^53 (the i64-suite discipline),
// plain JSON numbers otherwise.
//
// Operations: put / put_at (put with `now`) / get / get_or_default /
// contains_key / remove / clear / expire_entries / snapshot_keys /
// snapshot_values / snapshot_entries, applied in listed order. The runner
// records each op's result (put_results, get_results, get_or_default_results,
// contains_results, remove_results, expired_counts) and each snapshot
// (snapshot_keys_log, snapshot_values_log) for the result-log assertions.
//
// Explicit-order assertion keys (emitted in LRU / invocation order, NEVER
// re-sorted): `lru_order_keys`, `lru_order_values`, `eviction_log`, and the
// inner arrays of `snapshot_keys_log` / `snapshot_values_log`. The
// `eviction_log` element shape is `[key, value, "cause"]` with `cause` the
// lower-case string "size"/"expired". Unknown ops / assertion keys SKIP
// (forward-compat).

/// Parse a `now`/`ttl` operand: a u64 logical tick encoded as a decimal STRING
/// (parsed straight to u64, never via f64) or a plain JSON number for small
/// values. Reuses the i64-suite's decimal-string discipline.
fn parse_u64_tick(v: &Value) -> u64 {
    if let Some(s) = v.as_str() {
        s.parse::<u64>()
            .unwrap_or_else(|_| panic!("invalid u64 decimal-string tick: {:?}", s))
    } else if let Some(n) = v.as_u64() {
        n
    } else {
        panic!("expected u64 tick (decimal string or number), got {:?}", v);
    }
}

/// Result logs accumulated while applying bounded-LRU operations, in execution
/// order — exactly as the NavigableMap runner records poll/remove_range.
#[derive(Default)]
struct LruLog {
    put_results: Vec<Option<i32>>,
    get_results: Vec<Option<i32>>,
    get_or_default_results: Vec<i32>,
    contains_results: Vec<bool>,
    remove_results: Vec<Option<i32>>,
    expired_counts: Vec<i32>,
    snapshot_keys_log: Vec<Vec<i32>>,
    snapshot_values_log: Vec<Vec<i32>>,
    /// Each recorded `snapshot_entries` op: an LRU-order array of `[key, value]`
    /// pairs — exercises the `entries()` snapshot path (key/value pairing), not
    /// just the key array.
    snapshot_entries_log: Vec<Vec<(i32, i32)>>,
}

fn run_bounded_lru(
    scenario: &str,
    operations: &[Value],
    assertions: &serde_json::Map<String, Value>,
    scenario_obj: &Value,
) {
    let max_size = scenario_obj["max_size"]
        .as_u64()
        .expect("BoundedLruMap scenario needs a non-negative max_size") as usize;
    // ttl: null/absent => pure max-size map; otherwise a u64 logical tick.
    let ttl: Option<u64> = match scenario_obj.get("ttl") {
        None | Some(Value::Null) => None,
        Some(v) => Some(parse_u64_tick(v)),
    };

    // The recording eviction callback appends each (key, value, cause) triple
    // to the shared eviction LOG — the load-bearing oracle.
    let evict_log: Rc<RefCell<Vec<(i32, i32, EvictionCause)>>> = Rc::new(RefCell::new(Vec::new()));
    let cb_log = evict_log.clone();
    let mut builder = BoundedLruMap::<i32>::builder().max_size(max_size);
    if let Some(t) = ttl {
        builder = builder.ttl(t);
    }
    let mut map = builder
        .on_evict(move |k, v, c| cb_log.borrow_mut().push((*k, v, c)))
        .build();

    let mut log = LruLog::default();

    for op in operations {
        match op["op"].as_str().unwrap_or("") {
            "put" => {
                let k = op["key"].as_i64().expect("put needs key") as i32;
                let v = op["value"].as_i64().expect("put needs value") as i32;
                // An optional `now` makes this a put_at; absent => plain put
                // (which is put_at(k, v, 0) — no hidden clock).
                let prev = match op.get("now") {
                    Some(n) if !n.is_null() => map.put_at(k, v, parse_u64_tick(n)),
                    _ => map.put(k, v),
                };
                log.put_results.push(prev);
            }
            "put_at" => {
                let k = op["key"].as_i64().expect("put_at needs key") as i32;
                let v = op["value"].as_i64().expect("put_at needs value") as i32;
                let now = parse_u64_tick(&op["now"]);
                log.put_results.push(map.put_at(k, v, now));
            }
            "get" => {
                let k = op["key"].as_i64().expect("get needs key") as i32;
                log.get_results.push(map.get(&k));
            }
            "get_or_default" => {
                let k = op["key"].as_i64().expect("get_or_default needs key") as i32;
                let d = op["default"]
                    .as_i64()
                    .expect("get_or_default needs default") as i32;
                log.get_or_default_results.push(map.get_or_default(&k, d));
            }
            "contains_key" => {
                let k = op["key"].as_i64().expect("contains_key needs key") as i32;
                log.contains_results.push(map.contains_key(&k));
            }
            "remove" => {
                let k = op["key"].as_i64().expect("remove needs key") as i32;
                log.remove_results.push(map.remove(&k));
            }
            "clear" => map.clear(),
            "expire_entries" => {
                let now = parse_u64_tick(&op["now"]);
                log.expired_counts.push(map.expire_entries(now) as i32);
            }
            // Mid-sequence read-only LRU-order snapshots: record the current
            // contents WITHOUT refreshing recency or evicting.
            "snapshot_keys" => log.snapshot_keys_log.push(map.keys()),
            "snapshot_values" => log.snapshot_values_log.push(map.values()),
            "snapshot_entries" => {
                // Exercise the entries() snapshot path directly (key/value
                // pairing in LRU order), read-only — recorded as [key, value]
                // pairs so a port with a broken entries() pairing/order fails.
                log.snapshot_entries_log.push(map.entries());
            }
            // Forward-compat: an unknown op must not crash a newer/older runner
            // mix; skip it (mirrors unknown-collection/assertion skip).
            _ => {}
        }
    }
    for (key, expected) in assertions {
        if key == "comment" {
            continue;
        }
        let computed = eval_lru_assertion(key, &map, &log, &evict_log.borrow());
        emit(scenario, key, &computed, expected, FloatMode::None);
    }
}

/// Render an `Option<i32>` array (`null` for absence) in explicit order.
fn opt_i32_array(v: &[Option<i32>]) -> String {
    let parts: Vec<String> = v
        .iter()
        .map(|x| x.map(|n| n.to_string()).unwrap_or_else(|| "null".into()))
        .collect();
    format!("[{}]", parts.join(","))
}

/// Render a bool array in explicit order.
fn bool_array(v: &[bool]) -> String {
    let parts: Vec<String> = v.iter().map(|b| b.to_string()).collect();
    format!("[{}]", parts.join(","))
}

/// Render an array-of-int-arrays (the snapshot logs), inner arrays in their
/// recorded explicit (LRU) order, NOT sorted.
fn array_of_i32_arrays(v: &[Vec<i32>]) -> String {
    let parts: Vec<String> = v.iter().map(|inner| format_array(inner)).collect();
    format!("[{}]", parts.join(","))
}

/// Render the snapshot_entries log: an array of LRU-order `[[key,value], ...]`
/// arrays, explicit order, NOT sorted.
fn array_of_pair_arrays(v: &[Vec<(i32, i32)>]) -> String {
    let outer: Vec<String> = v
        .iter()
        .map(|inner| {
            let pairs: Vec<String> = inner
                .iter()
                .map(|(k, val)| format!("[{},{}]", k, val))
                .collect();
            format!("[{}]", pairs.join(","))
        })
        .collect();
    format!("[{}]", outer.join(","))
}

fn eval_lru_assertion(
    key: &str,
    map: &BoundedLruMap<i32>,
    log: &LruLog,
    evict_log: &[(i32, i32, EvictionCause)],
) -> String {
    match key {
        "size" => map.len().to_string(),
        "is_empty" => map.is_empty().to_string(),
        // Post-sequence contents in LRU order (least-recently-used first).
        // Explicit-order keys: NEVER re-sorted.
        "lru_order_keys" => format_array(&map.keys()),
        "lru_order_values" => format_array(&map.values()),
        // The load-bearing oracle: the ordered eviction LOG, each element a
        // fixed 3-tuple [key, value, "cause"], in invocation order (NOT sorted).
        "eviction_log" => {
            let parts: Vec<String> = evict_log
                .iter()
                .map(|(k, v, c)| format!("[{},{},\"{}\"]", k, v, c.as_str()))
                .collect();
            format!("[{}]", parts.join(","))
        }
        // Per-op result logs, in execution order.
        "put_results" => opt_i32_array(&log.put_results),
        "get_results" => opt_i32_array(&log.get_results),
        "get_or_default_results" => format_array(&log.get_or_default_results),
        "contains_results" => bool_array(&log.contains_results),
        "remove_results" => opt_i32_array(&log.remove_results),
        "expired_counts" => format_array(&log.expired_counts),
        "snapshot_keys_log" => array_of_i32_arrays(&log.snapshot_keys_log),
        "snapshot_values_log" => array_of_i32_arrays(&log.snapshot_values_log),
        "snapshot_entries_log" => array_of_pair_arrays(&log.snapshot_entries_log),
        // Post-op out-of-band reads: MUST NOT refresh recency, evict, or mutate.
        // `contains_key` is read-only; `get_<k>` is computed read-only via the
        // LRU-order snapshot (NOT map.get, which WOULD refresh recency).
        _ if key.starts_with("get_") => {
            let k: i32 = key[4..].parse().expect("get_<k> integer");
            map.entries()
                .into_iter()
                .find(|(ek, _)| *ek == k)
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| "null".into())
        }
        _ if key.starts_with("contains_") => {
            let k: i32 = key[9..].parse().expect("contains_<k> integer");
            map.contains_key(&k).to_string()
        }
        _ => format!("UNKNOWN_ASSERTION:{}", key),
    }
}
