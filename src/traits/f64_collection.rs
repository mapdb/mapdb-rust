// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any collection of `f64` values.
///
/// Implementors provide three required methods — [`len`], [`contains`],
/// and [`iter`] — and get a rich set of defaulted query methods for free,
/// following the same pattern as Rust's [`Iterator`] trait.
pub trait F64Collection {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if the collection contains the value.
    fn contains(&self, value: f64) -> bool;

    /// Returns an iterator over the elements.
    fn iter(&self) -> impl Iterator<Item = f64> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the collection is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all elements as a Vec.
    fn to_vec(&self) -> Vec<f64> {
        self.iter().collect()
    }

    /// Calls the given function for each element.
    fn for_each(&self, mut f: impl FnMut(f64)) {
        for v in self.iter() {
            f(v);
        }
    }

    /// Returns true if any element satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.iter().any(predicate)
    }

    /// Returns true if all elements satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        self.iter().all(predicate)
    }

    /// Returns true if no element satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(f64) -> bool) -> bool {
        !self.iter().any(predicate)
    }

    /// Returns the count of elements satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(f64) -> bool) -> usize {
        self.iter().filter(|&v| predicate(v)).count()
    }

    /// Returns the first element satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(f64) -> bool) -> Option<f64> {
        self.iter().find(|&v| predicate(v))
    }

    /// Returns the minimum element, or None if empty.
    fn min_value(&self) -> Option<f64> {
        self.iter().min_by(|a, b| a.total_cmp(&b))
    }

    /// Returns the maximum element, or None if empty.
    fn max_value(&self) -> Option<f64> {
        self.iter().max_by(|a, b| a.total_cmp(&b))
    }

    /// Returns elements satisfying the predicate as a Vec.
    fn select(&self, predicate: impl Fn(f64) -> bool) -> Vec<f64> {
        self.iter().filter(|&v| predicate(v)).collect()
    }

    /// Returns elements NOT satisfying the predicate as a Vec.
    fn reject(&self, predicate: impl Fn(f64) -> bool) -> Vec<f64> {
        self.iter().filter(|&v| !predicate(v)).collect()
    }

    /// Folds all elements using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, f64) -> R) -> R {
        let mut acc = initial;
        for v in self.iter() {
            acc = f(acc, v);
        }
        acc
    }

    /// Returns the sum of all elements.
    fn sum(&self) -> f64 {
        self.iter().sum()
    }
}

/// Mutable collection trait extending F64Collection.
pub trait F64MutableCollection: F64Collection {
    /// Removes all elements.
    fn clear(&mut self);
}

// ── Category traits — mirror Java's IntList / IntSet / IntBag / IntStack ──
//
// These distinguish *what kind of collection* is required without naming a
// concrete type. `fn process_list<L: F64List>(l: &L)` accepts ArrayList and
// ImmutableArrayList but not HashSet.

/// Read-only ordered list of `f64` values with positional access.
pub trait F64List: F64Collection {
    /// Returns the element at `index`, or None if out of bounds.
    fn get(&self, index: usize) -> Option<f64>;

    /// Returns the index of the first occurrence of `value`, or None if not found.
    fn index_of(&self, value: f64) -> Option<usize>;
}

/// Mutable ordered list extending F64List + F64MutableCollection.
pub trait F64MutableList: F64List + F64MutableCollection {
    /// Appends a value to the end of the list.
    fn push(&mut self, value: f64);
    /// Replaces the element at `index`. Returns the previous value.
    fn set(&mut self, index: usize, value: f64) -> f64;
}

/// Read-only set of `f64` values (no positional access; uniqueness implied).
pub trait F64Set: F64Collection {}

/// Mutable set extending F64Set + F64MutableCollection.
pub trait F64MutableSet: F64Set + F64MutableCollection {
    /// Inserts a value. Returns true if the value was not already present.
    fn add(&mut self, value: f64) -> bool;
}

/// Read-only multiset (bag) of `f64` values with occurrence counts.
pub trait F64Bag: F64Collection {
    /// Returns the number of times `value` occurs in the bag.
    fn occurrences_of(&self, value: f64) -> usize;
    /// Returns the number of *distinct* values (ignoring multiplicity).
    fn size_distinct(&self) -> usize;
}

/// Mutable bag extending F64Bag + F64MutableCollection.
pub trait F64MutableBag: F64Bag + F64MutableCollection {
    /// Adds one occurrence of `value`.
    fn add(&mut self, value: f64);
}

/// Read-only LIFO stack of `f64` values.
pub trait F64Stack: F64Collection {
    /// Returns the top of the stack without removing it.
    fn peek(&self) -> Option<f64>;
}

/// Mutable LIFO stack extending F64Stack + F64MutableCollection.
pub trait F64MutableStack: F64Stack + F64MutableCollection {
    /// Pushes a value onto the top of the stack.
    fn push(&mut self, value: f64);
    /// Pops and returns the top of the stack, or None if empty.
    fn pop(&mut self) -> Option<f64>;
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_collection<T: F64Collection>() {}
    fn _assert_mutable<T: F64MutableCollection>() {}
    fn _assert_list<T: F64List>() {}
    fn _assert_mutable_list<T: F64MutableList>() {}
    fn _assert_set<T: F64Set>() {}
    fn _assert_mutable_set<T: F64MutableSet>() {}
    fn _assert_bag<T: F64Bag>() {}
    fn _assert_mutable_bag<T: F64MutableBag>() {}
    fn _assert_stack<T: F64Stack>() {}
    fn _assert_mutable_stack<T: F64MutableStack>() {}

    /// Compile-time verification that every concrete collection type for
    /// `f64` implements the appropriate read-only and mutable traits.
    /// If any implementation is missing this test fails to compile.
    #[test]
    fn types_implement_traits() {
        // Mutable collections — base Collection / MutableCollection trait
        _assert_collection::<crate::arraylist::f64_array_list::F64ArrayList>();
        _assert_mutable::<crate::arraylist::f64_array_list::F64ArrayList>();
        _assert_collection::<crate::hashset::f64_hash_set::F64HashSet>();
        _assert_mutable::<crate::hashset::f64_hash_set::F64HashSet>();
        _assert_collection::<crate::bag::f64_hash_bag::F64HashBag>();
        _assert_mutable::<crate::bag::f64_hash_bag::F64HashBag>();
        _assert_collection::<crate::bag::f64_tree_bag::F64TreeBag>();
        _assert_mutable::<crate::bag::f64_tree_bag::F64TreeBag>();
        _assert_collection::<crate::stack::f64_array_stack::F64ArrayStack>();
        _assert_mutable::<crate::stack::f64_array_stack::F64ArrayStack>();
        _assert_collection::<crate::treeset::f64_tree_set::F64TreeSet>();
        _assert_mutable::<crate::treeset::f64_tree_set::F64TreeSet>();

        // Immutable collections — read-only trait only
        _assert_collection::<crate::immutable::immutable_f64_array_list::ImmutableF64ArrayList>();
        _assert_collection::<crate::immutable::immutable_f64_hash_set::ImmutableF64HashSet>();
        _assert_collection::<crate::immutable::immutable_f64_hash_bag::ImmutableF64HashBag>();
        _assert_collection::<crate::immutable::immutable_f64_array_stack::ImmutableF64ArrayStack>();

        // Category traits — Lists
        _assert_list::<crate::arraylist::f64_array_list::F64ArrayList>();
        _assert_mutable_list::<crate::arraylist::f64_array_list::F64ArrayList>();
        _assert_list::<crate::immutable::immutable_f64_array_list::ImmutableF64ArrayList>();

        // Category traits — Sets
        _assert_set::<crate::hashset::f64_hash_set::F64HashSet>();
        _assert_mutable_set::<crate::hashset::f64_hash_set::F64HashSet>();
        _assert_set::<crate::treeset::f64_tree_set::F64TreeSet>();
        _assert_mutable_set::<crate::treeset::f64_tree_set::F64TreeSet>();
        _assert_set::<crate::immutable::immutable_f64_hash_set::ImmutableF64HashSet>();

        // Category traits — Bags
        _assert_bag::<crate::bag::f64_hash_bag::F64HashBag>();
        _assert_mutable_bag::<crate::bag::f64_hash_bag::F64HashBag>();
        _assert_bag::<crate::bag::f64_tree_bag::F64TreeBag>();
        _assert_mutable_bag::<crate::bag::f64_tree_bag::F64TreeBag>();
        _assert_bag::<crate::immutable::immutable_f64_hash_bag::ImmutableF64HashBag>();

        // Category traits — Stacks
        _assert_stack::<crate::stack::f64_array_stack::F64ArrayStack>();
        _assert_mutable_stack::<crate::stack::f64_array_stack::F64ArrayStack>();
        _assert_stack::<crate::immutable::immutable_f64_array_stack::ImmutableF64ArrayStack>();
    }

    /// Proves the trait contract is reachable from outside the codegen.
    /// A minimal third-party type implementing F64MutableList, with no
    /// dependency on generated code beyond the trait definitions.
    struct FakeF64List {
        items: Vec<f64>,
    }

    impl F64Collection for FakeF64List {
        fn len(&self) -> usize {
            self.items.len()
        }
        fn contains(&self, value: f64) -> bool {
            self.items.contains(&value)
        }
        fn iter(&self) -> impl Iterator<Item = f64> + '_ {
            self.items.iter().copied()
        }
    }
    impl F64MutableCollection for FakeF64List {
        fn clear(&mut self) {
            self.items.clear();
        }
    }
    impl F64List for FakeF64List {
        fn get(&self, index: usize) -> Option<f64> {
            self.items.get(index).copied()
        }
        fn index_of(&self, value: f64) -> Option<usize> {
            self.items.iter().position(|&v| v == value)
        }
    }
    impl F64MutableList for FakeF64List {
        fn push(&mut self, value: f64) {
            self.items.push(value);
        }
        fn set(&mut self, index: usize, value: f64) -> f64 {
            let old = self.items[index];
            self.items[index] = value;
            old
        }
    }

    #[test]
    fn third_party_impl_satisfies_traits() {
        _assert_mutable_list::<FakeF64List>();
        let mut fake = FakeF64List { items: Vec::new() };
        fake.push(1.0f64);
        assert_eq!(fake.len(), 1);
        assert!(fake.contains(1.0f64));
        // Verify defaulted methods work via trait dispatch
        assert!(!fake.is_empty());
        assert_eq!(fake.to_vec().len(), 1);
    }
}
