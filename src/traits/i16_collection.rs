// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any collection of `i16` values.
///
/// Implementors provide three required methods — [`len`], [`contains`],
/// and [`iter`] — and get a rich set of defaulted query methods for free,
/// following the same pattern as Rust's [`Iterator`] trait.
pub trait I16Collection {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if the collection contains the value.
    fn contains(&self, value: i16) -> bool;

    /// Returns an iterator over the elements.
    fn iter(&self) -> impl Iterator<Item = i16> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the collection is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all elements as a Vec.
    fn to_vec(&self) -> Vec<i16> {
        self.iter().collect()
    }

    /// Calls the given function for each element.
    fn for_each(&self, mut f: impl FnMut(i16)) {
        for v in self.iter() {
            f(v);
        }
    }

    /// Returns true if any element satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(i16) -> bool) -> bool {
        self.iter().any(predicate)
    }

    /// Returns true if all elements satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(i16) -> bool) -> bool {
        self.iter().all(predicate)
    }

    /// Returns true if no element satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(i16) -> bool) -> bool {
        !self.iter().any(predicate)
    }

    /// Returns the count of elements satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(i16) -> bool) -> usize {
        self.iter().filter(|&v| predicate(v)).count()
    }

    /// Returns the first element satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(i16) -> bool) -> Option<i16> {
        self.iter().find(|&v| predicate(v))
    }

    /// Returns the minimum element, or None if empty.
    fn min_value(&self) -> Option<i16> {
        self.iter().min()
    }

    /// Returns the maximum element, or None if empty.
    fn max_value(&self) -> Option<i16> {
        self.iter().max()
    }

    /// Returns elements satisfying the predicate as a Vec.
    fn select(&self, predicate: impl Fn(i16) -> bool) -> Vec<i16> {
        self.iter().filter(|&v| predicate(v)).collect()
    }

    /// Returns elements NOT satisfying the predicate as a Vec.
    fn reject(&self, predicate: impl Fn(i16) -> bool) -> Vec<i16> {
        self.iter().filter(|&v| !predicate(v)).collect()
    }

    /// Folds all elements using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, i16) -> R) -> R {
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

/// Mutable collection trait extending I16Collection.
pub trait I16MutableCollection: I16Collection {
    /// Removes all elements.
    fn clear(&mut self);
}

// ── Category traits — mirror Java's IntList / IntSet / IntBag / IntStack ──
//
// These distinguish *what kind of collection* is required without naming a
// concrete type. `fn process_list<L: I16List>(l: &L)` accepts ArrayList and
// ImmutableArrayList but not HashSet.

/// Read-only ordered list of `i16` values with positional access.
pub trait I16List: I16Collection {
    /// Returns the element at `index`, or None if out of bounds.
    fn get(&self, index: usize) -> Option<i16>;

    /// Returns the index of the first occurrence of `value`, or None if not found.
    fn index_of(&self, value: i16) -> Option<usize>;
}

/// Mutable ordered list extending I16List + I16MutableCollection.
pub trait I16MutableList: I16List + I16MutableCollection {
    /// Appends a value to the end of the list.
    fn push(&mut self, value: i16);
    /// Replaces the element at `index`. Returns the previous value.
    fn set(&mut self, index: usize, value: i16) -> i16;
}

/// Read-only set of `i16` values (no positional access; uniqueness implied).
pub trait I16Set: I16Collection {}

/// Mutable set extending I16Set + I16MutableCollection.
pub trait I16MutableSet: I16Set + I16MutableCollection {
    /// Inserts a value. Returns true if the value was not already present.
    fn add(&mut self, value: i16) -> bool;
}

/// Read-only multiset (bag) of `i16` values with occurrence counts.
pub trait I16Bag: I16Collection {
    /// Returns the number of times `value` occurs in the bag.
    fn occurrences_of(&self, value: i16) -> usize;
    /// Returns the number of *distinct* values (ignoring multiplicity).
    fn size_distinct(&self) -> usize;
}

/// Mutable bag extending I16Bag + I16MutableCollection.
pub trait I16MutableBag: I16Bag + I16MutableCollection {
    /// Adds one occurrence of `value`.
    fn add(&mut self, value: i16);
}

/// Read-only LIFO stack of `i16` values.
pub trait I16Stack: I16Collection {
    /// Returns the top of the stack without removing it.
    fn peek(&self) -> Option<i16>;
}

/// Mutable LIFO stack extending I16Stack + I16MutableCollection.
pub trait I16MutableStack: I16Stack + I16MutableCollection {
    /// Pushes a value onto the top of the stack.
    fn push(&mut self, value: i16);
    /// Pops and returns the top of the stack, or None if empty.
    fn pop(&mut self) -> Option<i16>;
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_collection<T: I16Collection>() {}
    fn _assert_mutable<T: I16MutableCollection>() {}
    fn _assert_list<T: I16List>() {}
    fn _assert_mutable_list<T: I16MutableList>() {}
    fn _assert_set<T: I16Set>() {}
    fn _assert_mutable_set<T: I16MutableSet>() {}
    fn _assert_bag<T: I16Bag>() {}
    fn _assert_mutable_bag<T: I16MutableBag>() {}
    fn _assert_stack<T: I16Stack>() {}
    fn _assert_mutable_stack<T: I16MutableStack>() {}

    /// Compile-time verification that every concrete collection type for
    /// `i16` implements the appropriate read-only and mutable traits.
    /// If any implementation is missing this test fails to compile.
    #[test]
    fn types_implement_traits() {
        // Mutable collections — base Collection / MutableCollection trait
        _assert_collection::<crate::arraylist::i16_array_list::I16ArrayList>();
        _assert_mutable::<crate::arraylist::i16_array_list::I16ArrayList>();
        _assert_collection::<crate::hashset::i16_hash_set::I16HashSet>();
        _assert_mutable::<crate::hashset::i16_hash_set::I16HashSet>();
        _assert_collection::<crate::bag::i16_hash_bag::I16HashBag>();
        _assert_mutable::<crate::bag::i16_hash_bag::I16HashBag>();
        _assert_collection::<crate::bag::i16_tree_bag::I16TreeBag>();
        _assert_mutable::<crate::bag::i16_tree_bag::I16TreeBag>();
        _assert_collection::<crate::stack::i16_array_stack::I16ArrayStack>();
        _assert_mutable::<crate::stack::i16_array_stack::I16ArrayStack>();
        _assert_collection::<crate::treeset::i16_tree_set::I16TreeSet>();
        _assert_mutable::<crate::treeset::i16_tree_set::I16TreeSet>();

        // Immutable collections — read-only trait only
        _assert_collection::<crate::immutable::immutable_i16_array_list::ImmutableI16ArrayList>();
        _assert_collection::<crate::immutable::immutable_i16_hash_set::ImmutableI16HashSet>();
        _assert_collection::<crate::immutable::immutable_i16_hash_bag::ImmutableI16HashBag>();
        _assert_collection::<crate::immutable::immutable_i16_array_stack::ImmutableI16ArrayStack>();

        // Category traits — Lists
        _assert_list::<crate::arraylist::i16_array_list::I16ArrayList>();
        _assert_mutable_list::<crate::arraylist::i16_array_list::I16ArrayList>();
        _assert_list::<crate::immutable::immutable_i16_array_list::ImmutableI16ArrayList>();

        // Category traits — Sets
        _assert_set::<crate::hashset::i16_hash_set::I16HashSet>();
        _assert_mutable_set::<crate::hashset::i16_hash_set::I16HashSet>();
        _assert_set::<crate::treeset::i16_tree_set::I16TreeSet>();
        _assert_mutable_set::<crate::treeset::i16_tree_set::I16TreeSet>();
        _assert_set::<crate::immutable::immutable_i16_hash_set::ImmutableI16HashSet>();

        // Category traits — Bags
        _assert_bag::<crate::bag::i16_hash_bag::I16HashBag>();
        _assert_mutable_bag::<crate::bag::i16_hash_bag::I16HashBag>();
        _assert_bag::<crate::bag::i16_tree_bag::I16TreeBag>();
        _assert_mutable_bag::<crate::bag::i16_tree_bag::I16TreeBag>();
        _assert_bag::<crate::immutable::immutable_i16_hash_bag::ImmutableI16HashBag>();

        // Category traits — Stacks
        _assert_stack::<crate::stack::i16_array_stack::I16ArrayStack>();
        _assert_mutable_stack::<crate::stack::i16_array_stack::I16ArrayStack>();
        _assert_stack::<crate::immutable::immutable_i16_array_stack::ImmutableI16ArrayStack>();

        // Interval — read-only Collection (integer types only)
        _assert_collection::<crate::interval::i16_interval::I16Interval>();
    }

    /// Proves the trait contract is reachable from outside the codegen.
    /// A minimal third-party type implementing I16MutableList, with no
    /// dependency on generated code beyond the trait definitions.
    struct FakeI16List {
        items: Vec<i16>,
    }

    impl I16Collection for FakeI16List {
        fn len(&self) -> usize {
            self.items.len()
        }
        fn contains(&self, value: i16) -> bool {
            self.items.contains(&value)
        }
        fn iter(&self) -> impl Iterator<Item = i16> + '_ {
            self.items.iter().copied()
        }
    }
    impl I16MutableCollection for FakeI16List {
        fn clear(&mut self) {
            self.items.clear();
        }
    }
    impl I16List for FakeI16List {
        fn get(&self, index: usize) -> Option<i16> {
            self.items.get(index).copied()
        }
        fn index_of(&self, value: i16) -> Option<usize> {
            self.items.iter().position(|&v| v == value)
        }
    }
    impl I16MutableList for FakeI16List {
        fn push(&mut self, value: i16) {
            self.items.push(value);
        }
        fn set(&mut self, index: usize, value: i16) -> i16 {
            let old = self.items[index];
            self.items[index] = value;
            old
        }
    }

    #[test]
    fn third_party_impl_satisfies_traits() {
        _assert_mutable_list::<FakeI16List>();
        let mut fake = FakeI16List { items: Vec::new() };
        fake.push(1);
        assert_eq!(fake.len(), 1);
        assert!(fake.contains(1));
        // Verify defaulted methods work via trait dispatch
        assert!(!fake.is_empty());
        assert_eq!(fake.to_vec().len(), 1);
    }
}
