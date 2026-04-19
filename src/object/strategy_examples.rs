// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

//! Real-world examples demonstrating [`HashingStrategy`], [`Comparator`],
//! [`TreeMap`], and [`TreeSet`]. These run as regular tests but double as
//! usage documentation.

#[cfg(test)]
mod tests {
    use crate::object::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // ── Example 1: Case-insensitive HTTP headers ──────────────────────────
    //
    // HTTP header names are case-insensitive per RFC 7230. A plain map would
    // treat "Content-Type" and "content-type" as different keys. Using a
    // case_insensitive_hashing_strategy fixes this.
    #[test]
    fn example_http_headers() {
        let mut headers: HashMapWithStrategy<String, String> =
            HashMapWithStrategy::new(case_insensitive_hashing_strategy());

        headers.insert(
            String::from("Content-Type"),
            String::from("application/json"),
        );
        headers.insert(String::from("Content-Length"), String::from("42"));
        headers.insert(String::from("Authorization"), String::from("Bearer xyz"));

        // Case-insensitive lookup.
        assert_eq!(
            headers.get(&String::from("content-type")),
            Some(&String::from("application/json")),
        );
        assert_eq!(
            headers.get(&String::from("AUTHORIZATION")),
            Some(&String::from("Bearer xyz")),
        );

        // Overwriting with different case.
        headers.insert(String::from("content-TYPE"), String::from("text/html"));
        assert_eq!(headers.len(), 3);
        assert_eq!(
            headers.get(&String::from("Content-Type")),
            Some(&String::from("text/html")),
        );
    }

    // ── Example 2: Deduplicating users by email ───────────────────────────
    //
    // Real-world scenario: you're processing a stream of user records from
    // multiple sources. The same user may appear with different casing in
    // email or different metadata. You want unique users by email only.
    #[derive(Debug, Clone)]
    struct User {
        email: String,
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        source: String,
        #[allow(dead_code)]
        login_count: i32,
    }

    #[test]
    fn example_deduplicate_users() {
        // Two users are "the same" if their emails match case-insensitively.
        // Build the email strategy by composing the case-insensitive string
        // strategy on the email field. We need two instances because the
        // closures move the strategy.
        let ci_hash = case_insensitive_hashing_strategy();
        let ci_eq = case_insensitive_hashing_strategy();
        let email_strategy: HashingStrategy<User> = HashingStrategy::new(
            Box::new(move |u: &User| ci_hash.hash_code(&u.email)),
            Box::new(move |a: &User, b: &User| ci_eq.equals(&a.email, &b.email)),
        );

        let mut unique = HashSetWithStrategy::new(email_strategy);

        // Duplicate-ish records from multiple sources.
        unique.add(User {
            email: String::from("alice@example.com"),
            name: String::from("Alice"),
            source: String::from("source-a"),
            login_count: 5,
        });
        unique.add(User {
            email: String::from("ALICE@example.com"),
            name: String::from("Alice A."),
            source: String::from("source-b"),
            login_count: 10,
        });
        unique.add(User {
            email: String::from("bob@example.com"),
            name: String::from("Bob"),
            source: String::from("source-a"),
            login_count: 3,
        });
        unique.add(User {
            email: String::from("Alice@Example.Com"),
            name: String::from("Alice"),
            source: String::from("source-c"),
            login_count: 0,
        });

        assert_eq!(unique.len(), 2, "expected 2 unique users (alice, bob)");
    }

    // ── Example 3: Log lines sorted by timestamp, then severity ───────────
    #[derive(Debug, Clone)]
    struct LogLine {
        timestamp: i64,
        severity: i32, // 0=debug, 1=info, 2=warn, 3=error
        message: String,
    }

    #[test]
    fn example_log_sorting() {
        // Sort: timestamp ascending, then severity descending (errors first
        // within the same timestamp).
        let by_timestamp = comparator_by_field(|l: &LogLine| l.timestamp);
        let by_severity_desc: Comparator<LogLine> =
            Comparator::new(Box::new(|a: &LogLine, b: &LogLine| {
                b.severity.cmp(&a.severity)
            }));
        let cmp = then_comparing(by_timestamp, by_severity_desc);

        let mut logs: TreeSet<LogLine> = TreeSet::new(cmp);
        logs.add(LogLine {
            timestamp: 100,
            severity: 1,
            message: String::from("info first"),
        });
        logs.add(LogLine {
            timestamp: 100,
            severity: 3,
            message: String::from("error same time"),
        });
        logs.add(LogLine {
            timestamp: 50,
            severity: 0,
            message: String::from("debug earliest"),
        });
        logs.add(LogLine {
            timestamp: 200,
            severity: 2,
            message: String::from("warn latest"),
        });

        let ordered: Vec<&str> = logs.iter().map(|l| l.message.as_str()).collect();

        // Expected order:
        //   t=50  severity=0 "debug earliest"
        //   t=100 severity=3 "error same time"  (higher severity first)
        //   t=100 severity=1 "info first"
        //   t=200 severity=2 "warn latest"
        assert_eq!(
            ordered,
            vec![
                "debug earliest",
                "error same time",
                "info first",
                "warn latest"
            ],
        );
    }

    // ── Example 4: Leaderboard with TreeMap ───────────────────────────────
    //
    // Use a TreeMap keyed by score (descending) to get sorted leaderboards.
    #[test]
    fn example_leaderboard() {
        // Key: score, Value: player name.
        // Higher scores first → reverse comparator.
        let mut board: TreeMap<i32, String> = TreeMap::new(reverse_comparator::<i32>());

        board.insert(100, String::from("Alice"));
        board.insert(250, String::from("Bob"));
        board.insert(175, String::from("Charlie"));
        board.insert(50, String::from("Dave"));

        // Top player — Min under reverse = highest score.
        let (top_score, top_player) = board.min().expect("leaderboard is non-empty");
        assert_eq!(*top_score, 250);
        assert_eq!(top_player, "Bob");

        // Iterate in rank order.
        let mut ranked: Vec<String> = Vec::new();
        board.for_each(|_, name| ranked.push(name.clone()));
        assert_eq!(
            ranked,
            vec![
                String::from("Bob"),
                String::from("Charlie"),
                String::from("Alice"),
                String::from("Dave"),
            ],
        );
    }

    // ── Example 5: Grouping by normalized name ────────────────────────────
    //
    // Merging data from external systems where names have inconsistent
    // whitespace or casing ("New York", "new york", "NEW  YORK" are the same).
    fn normalize_name(s: &str) -> String {
        s.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn hash_str(s: &str) -> u64 {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }

    #[test]
    fn example_normalized_grouping() {
        let norm_strategy: HashingStrategy<String> = HashingStrategy::new(
            Box::new(|s: &String| hash_str(&normalize_name(s))),
            Box::new(|a: &String, b: &String| normalize_name(a) == normalize_name(b)),
        );

        let mut m: HashMapWithStrategy<String, i32> = HashMapWithStrategy::new(norm_strategy);
        m.insert(String::from("New York"), 1);
        m.insert(String::from("new york"), 2); // merges with above
        m.insert(String::from("NEW  YORK"), 3); // merges with above
        m.insert(String::from("Boston"), 10);

        assert_eq!(m.len(), 2, "expected 2 distinct cities");
    }
}
