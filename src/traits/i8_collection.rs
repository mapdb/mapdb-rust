// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any collection of `i8` values.
///
/// Implementors provide three required methods — [`len`], [`contains`],
/// and [`iter`] — and get a rich set of defaulted query methods for free,
/// following the same pattern as Rust's [`Iterator`] trait.
pub trait I8Collection {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if the collection contains the value.
    fn contains(&self, value: i8) -> bool;

    /// Returns an iterator over the elements.
    fn iter(&self) -> impl Iterator<Item = i8> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the collection is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all elements as a Vec.
    fn to_vec(&self) -> Vec<i8> {
        self.iter().collect()
    }

    /// Calls the given function for each element.
    fn for_each(&self, mut f: impl FnMut(i8)) {
        for v in self.iter() {
            f(v);
        }
    }

    /// Returns true if any element satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.iter().any(predicate)
    }

    /// Returns true if all elements satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        self.iter().all(predicate)
    }

    /// Returns true if no element satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(i8) -> bool) -> bool {
        !self.iter().any(predicate)
    }

    /// Returns the count of elements satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(i8) -> bool) -> usize {
        self.iter().filter(|&v| predicate(v)).count()
    }

    /// Returns the first element satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(i8) -> bool) -> Option<i8> {
        self.iter().find(|&v| predicate(v))
    }

    /// Returns the minimum element, or None if empty.
    fn min_value(&self) -> Option<i8> {
        self.iter().min()
    }

    /// Returns the maximum element, or None if empty.
    fn max_value(&self) -> Option<i8> {
        self.iter().max()
    }

    /// Returns elements satisfying the predicate as a Vec.
    fn select(&self, predicate: impl Fn(i8) -> bool) -> Vec<i8> {
        self.iter().filter(|&v| predicate(v)).collect()
    }

    /// Returns elements NOT satisfying the predicate as a Vec.
    fn reject(&self, predicate: impl Fn(i8) -> bool) -> Vec<i8> {
        self.iter().filter(|&v| !predicate(v)).collect()
    }

    /// Folds all elements using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, i8) -> R) -> R {
        let mut acc = initial;
        for v in self.iter() {
            acc = f(acc, v);
        }
        acc
    }

    /// Returns the sum of all elements.
    fn sum(&self) -> i64 {
        self.iter().map(|v| v as i64).sum()
    }
}

/// Mutable collection trait extending I8Collection.
pub trait I8MutableCollection: I8Collection {
    /// Removes all elements.
    fn clear(&mut self);
}

// ── Category traits — mirror Java's IntList / IntSet / IntBag / IntStack ──
//
// These distinguish *what kind of collection* is required without naming a
// concrete type. `fn process_list<L: I8List>(l: &L)` accepts ArrayList and
// ImmutableArrayList but not HashSet.

/// Read-only ordered list of `i8` values with positional access.
pub trait I8List: I8Collection {
    /// Returns the element at `index`, or None if out of bounds.
    fn get(&self, index: usize) -> Option<i8>;

    /// Returns the index of the first occurrence of `value`, or None if not found.
    fn index_of(&self, value: i8) -> Option<usize>;
}

/// Mutable ordered list extending I8List + I8MutableCollection.
pub trait I8MutableList: I8List + I8MutableCollection {
    /// Appends a value to the end of the list.
    fn push(&mut self, value: i8);
    /// Replaces the element at `index`. Returns the previous value.
    fn set(&mut self, index: usize, value: i8) -> i8;
}

/// Read-only set of `i8` values (no positional access; uniqueness implied).
pub trait I8Set: I8Collection {}

/// Mutable set extending I8Set + I8MutableCollection.
pub trait I8MutableSet: I8Set + I8MutableCollection {
    /// Inserts a value. Returns true if the value was not already present.
    fn add(&mut self, value: i8) -> bool;
}

/// Read-only multiset (bag) of `i8` values with occurrence counts.
pub trait I8Bag: I8Collection {
    /// Returns the number of times `value` occurs in the bag.
    fn occurrences_of(&self, value: i8) -> usize;
    /// Returns the number of *distinct* values (ignoring multiplicity).
    fn size_distinct(&self) -> usize;
}

/// Mutable bag extending I8Bag + I8MutableCollection.
pub trait I8MutableBag: I8Bag + I8MutableCollection {
    /// Adds one occurrence of `value`.
    fn add(&mut self, value: i8);
}

/// Read-only LIFO stack of `i8` values.
pub trait I8Stack: I8Collection {
    /// Returns the top of the stack without removing it.
    fn peek(&self) -> Option<i8>;
}

/// Mutable LIFO stack extending I8Stack + I8MutableCollection.
pub trait I8MutableStack: I8Stack + I8MutableCollection {
    /// Pushes a value onto the top of the stack.
    fn push(&mut self, value: i8);
    /// Pops and returns the top of the stack, or None if empty.
    fn pop(&mut self) -> Option<i8>;
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_collection<T: I8Collection>() {}
    fn _assert_mutable<T: I8MutableCollection>() {}
    fn _assert_list<T: I8List>() {}
    fn _assert_mutable_list<T: I8MutableList>() {}
    fn _assert_set<T: I8Set>() {}
    fn _assert_mutable_set<T: I8MutableSet>() {}
    fn _assert_bag<T: I8Bag>() {}
    fn _assert_mutable_bag<T: I8MutableBag>() {}
    fn _assert_stack<T: I8Stack>() {}
    fn _assert_mutable_stack<T: I8MutableStack>() {}

    /// Compile-time verification that every concrete collection type for
    /// `i8` implements the appropriate read-only and mutable traits.
    /// If any implementation is missing this test fails to compile.
    #[test]
    fn types_implement_traits() {
        // Mutable collections — base Collection / MutableCollection trait
        _assert_collection::<crate::arraylist::i8_array_list::I8ArrayList>();
        _assert_mutable::<crate::arraylist::i8_array_list::I8ArrayList>();
        _assert_collection::<crate::hashset::i8_hash_set::I8HashSet>();
        _assert_mutable::<crate::hashset::i8_hash_set::I8HashSet>();
        _assert_collection::<crate::bag::i8_hash_bag::I8HashBag>();
        _assert_mutable::<crate::bag::i8_hash_bag::I8HashBag>();
        _assert_collection::<crate::bag::i8_tree_bag::I8TreeBag>();
        _assert_mutable::<crate::bag::i8_tree_bag::I8TreeBag>();
        _assert_collection::<crate::stack::i8_array_stack::I8ArrayStack>();
        _assert_mutable::<crate::stack::i8_array_stack::I8ArrayStack>();
        _assert_collection::<crate::treeset::i8_tree_set::I8TreeSet>();
        _assert_mutable::<crate::treeset::i8_tree_set::I8TreeSet>();

        // Immutable collections — read-only trait only
        _assert_collection::<crate::immutable::immutable_i8_array_list::ImmutableI8ArrayList>();
        _assert_collection::<crate::immutable::immutable_i8_hash_set::ImmutableI8HashSet>();
        _assert_collection::<crate::immutable::immutable_i8_hash_bag::ImmutableI8HashBag>();
        _assert_collection::<crate::immutable::immutable_i8_array_stack::ImmutableI8ArrayStack>();

        // Category traits — Lists
        _assert_list::<crate::arraylist::i8_array_list::I8ArrayList>();
        _assert_mutable_list::<crate::arraylist::i8_array_list::I8ArrayList>();
        _assert_list::<crate::immutable::immutable_i8_array_list::ImmutableI8ArrayList>();

        // Category traits — Sets
        _assert_set::<crate::hashset::i8_hash_set::I8HashSet>();
        _assert_mutable_set::<crate::hashset::i8_hash_set::I8HashSet>();
        _assert_set::<crate::treeset::i8_tree_set::I8TreeSet>();
        _assert_mutable_set::<crate::treeset::i8_tree_set::I8TreeSet>();
        _assert_set::<crate::immutable::immutable_i8_hash_set::ImmutableI8HashSet>();

        // Category traits — Bags
        _assert_bag::<crate::bag::i8_hash_bag::I8HashBag>();
        _assert_mutable_bag::<crate::bag::i8_hash_bag::I8HashBag>();
        _assert_bag::<crate::bag::i8_tree_bag::I8TreeBag>();
        _assert_mutable_bag::<crate::bag::i8_tree_bag::I8TreeBag>();
        _assert_bag::<crate::immutable::immutable_i8_hash_bag::ImmutableI8HashBag>();

        // Category traits — Stacks
        _assert_stack::<crate::stack::i8_array_stack::I8ArrayStack>();
        _assert_mutable_stack::<crate::stack::i8_array_stack::I8ArrayStack>();
        _assert_stack::<crate::immutable::immutable_i8_array_stack::ImmutableI8ArrayStack>();

        // Interval — read-only Collection (integer types only)
        _assert_collection::<crate::interval::i8_interval::I8Interval>();
    }

    /// Proves the trait contract is reachable from outside the codegen.
    /// A minimal third-party type implementing I8MutableList, with no
    /// dependency on generated code beyond the trait definitions.
    struct FakeI8List {
        items: Vec<i8>,
    }

    impl I8Collection for FakeI8List {
        fn len(&self) -> usize {
            self.items.len()
        }
        fn contains(&self, value: i8) -> bool {
            self.items.contains(&value)
        }
        fn iter(&self) -> impl Iterator<Item = i8> + '_ {
            self.items.iter().copied()
        }
    }
    impl I8MutableCollection for FakeI8List {
        fn clear(&mut self) {
            self.items.clear();
        }
    }
    impl I8List for FakeI8List {
        fn get(&self, index: usize) -> Option<i8> {
            self.items.get(index).copied()
        }
        fn index_of(&self, value: i8) -> Option<usize> {
            self.items.iter().position(|&v| v == value)
        }
    }
    impl I8MutableList for FakeI8List {
        fn push(&mut self, value: i8) {
            self.items.push(value);
        }
        fn set(&mut self, index: usize, value: i8) -> i8 {
            let old = self.items[index];
            self.items[index] = value;
            old
        }
    }

    #[test]
    fn third_party_impl_satisfies_traits() {
        _assert_mutable_list::<FakeI8List>();
        let mut fake = FakeI8List { items: Vec::new() };
        fake.push(1);
        assert_eq!(fake.len(), 1);
        assert!(fake.contains(1));
        // Verify defaulted methods work via trait dispatch
        assert!(!fake.is_empty());
        assert_eq!(fake.to_vec().len(), 1);
    }
}
