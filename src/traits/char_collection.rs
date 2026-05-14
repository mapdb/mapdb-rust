// AUTO-GENERATED. DO NOT EDIT.

/// Read-only trait for any collection of `char` values.
///
/// Implementors provide three required methods — [`len`], [`contains`],
/// and [`iter`] — and get a rich set of defaulted query methods for free,
/// following the same pattern as Rust's [`Iterator`] trait.
pub trait CharCollection {
    // ── Required methods ────────────────────────────────────────────

    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if the collection contains the value.
    fn contains(&self, value: char) -> bool;

    /// Returns an iterator over the elements.
    fn iter(&self) -> impl Iterator<Item = char> + '_;

    // ── Defaulted methods ───────────────────────────────────────────

    /// Returns true if the collection is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all elements as a Vec.
    fn to_vec(&self) -> Vec<char> {
        self.iter().collect()
    }

    /// Calls the given function for each element.
    fn for_each(&self, mut f: impl FnMut(char)) {
        for v in self.iter() {
            f(v);
        }
    }

    /// Returns true if any element satisfies the predicate.
    fn any_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.iter().any(predicate)
    }

    /// Returns true if all elements satisfy the predicate.
    fn all_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        self.iter().all(predicate)
    }

    /// Returns true if no element satisfies the predicate.
    fn none_satisfy(&self, predicate: impl Fn(char) -> bool) -> bool {
        !self.iter().any(predicate)
    }

    /// Returns the count of elements satisfying the predicate.
    fn count_where(&self, predicate: impl Fn(char) -> bool) -> usize {
        self.iter().filter(|&v| predicate(v)).count()
    }

    /// Returns the first element satisfying the predicate, or None.
    fn detect(&self, predicate: impl Fn(char) -> bool) -> Option<char> {
        self.iter().find(|&v| predicate(v))
    }

    /// Returns the minimum element, or None if empty.
    fn min_value(&self) -> Option<char> {
        self.iter().min()
    }

    /// Returns the maximum element, or None if empty.
    fn max_value(&self) -> Option<char> {
        self.iter().max()
    }

    /// Returns elements satisfying the predicate as a Vec.
    fn select(&self, predicate: impl Fn(char) -> bool) -> Vec<char> {
        self.iter().filter(|&v| predicate(v)).collect()
    }

    /// Returns elements NOT satisfying the predicate as a Vec.
    fn reject(&self, predicate: impl Fn(char) -> bool) -> Vec<char> {
        self.iter().filter(|&v| !predicate(v)).collect()
    }

    /// Folds all elements using the given function and initial value.
    fn inject_into<R>(&self, initial: R, mut f: impl FnMut(R, char) -> R) -> R {
        let mut acc = initial;
        for v in self.iter() {
            acc = f(acc, v);
        }
        acc
    }
}

/// Mutable collection trait extending CharCollection.
pub trait CharMutableCollection: CharCollection {
    /// Removes all elements.
    fn clear(&mut self);
}

// ── Category traits — mirror Java's IntList / IntSet / IntBag / IntStack ──
//
// These distinguish *what kind of collection* is required without naming a
// concrete type. `fn process_list<L: CharList>(l: &L)` accepts ArrayList and
// ImmutableArrayList but not HashSet.

/// Read-only ordered list of `char` values with positional access.
pub trait CharList: CharCollection {
    /// Returns the element at `index`, or None if out of bounds.
    fn get(&self, index: usize) -> Option<char>;

    /// Returns the index of the first occurrence of `value`, or None if not found.
    fn index_of(&self, value: char) -> Option<usize>;
}

/// Mutable ordered list extending CharList + CharMutableCollection.
pub trait CharMutableList: CharList + CharMutableCollection {
    /// Appends a value to the end of the list.
    fn push(&mut self, value: char);
    /// Replaces the element at `index`. Returns the previous value.
    fn set(&mut self, index: usize, value: char) -> char;
}

/// Read-only set of `char` values (no positional access; uniqueness implied).
pub trait CharSet: CharCollection {}

/// Mutable set extending CharSet + CharMutableCollection.
pub trait CharMutableSet: CharSet + CharMutableCollection {
    /// Inserts a value. Returns true if the value was not already present.
    fn add(&mut self, value: char) -> bool;
}

/// Read-only multiset (bag) of `char` values with occurrence counts.
pub trait CharBag: CharCollection {
    /// Returns the number of times `value` occurs in the bag.
    fn occurrences_of(&self, value: char) -> usize;
    /// Returns the number of *distinct* values (ignoring multiplicity).
    fn size_distinct(&self) -> usize;
}

/// Mutable bag extending CharBag + CharMutableCollection.
pub trait CharMutableBag: CharBag + CharMutableCollection {
    /// Adds one occurrence of `value`.
    fn add(&mut self, value: char);
}

/// Read-only LIFO stack of `char` values.
pub trait CharStack: CharCollection {
    /// Returns the top of the stack without removing it.
    fn peek(&self) -> Option<char>;
}

/// Mutable LIFO stack extending CharStack + CharMutableCollection.
pub trait CharMutableStack: CharStack + CharMutableCollection {
    /// Pushes a value onto the top of the stack.
    fn push(&mut self, value: char);
    /// Pops and returns the top of the stack, or None if empty.
    fn pop(&mut self) -> Option<char>;
}

#[cfg(test)]
mod verify {
    use super::*;
    fn _assert_collection<T: CharCollection>() {}
    fn _assert_mutable<T: CharMutableCollection>() {}
    fn _assert_list<T: CharList>() {}
    fn _assert_mutable_list<T: CharMutableList>() {}
    fn _assert_set<T: CharSet>() {}
    fn _assert_mutable_set<T: CharMutableSet>() {}
    fn _assert_bag<T: CharBag>() {}
    fn _assert_mutable_bag<T: CharMutableBag>() {}
    fn _assert_stack<T: CharStack>() {}
    fn _assert_mutable_stack<T: CharMutableStack>() {}

    /// Compile-time verification that every concrete collection type for
    /// `char` implements the appropriate read-only and mutable traits.
    /// If any implementation is missing this test fails to compile.
    #[test]
    fn types_implement_traits() {
        // Mutable collections — base Collection / MutableCollection trait
        _assert_collection::<crate::arraylist::char_array_list::CharArrayList>();
        _assert_mutable::<crate::arraylist::char_array_list::CharArrayList>();
        _assert_collection::<crate::hashset::char_hash_set::CharHashSet>();
        _assert_mutable::<crate::hashset::char_hash_set::CharHashSet>();
        _assert_collection::<crate::bag::char_hash_bag::CharHashBag>();
        _assert_mutable::<crate::bag::char_hash_bag::CharHashBag>();
        _assert_collection::<crate::bag::char_tree_bag::CharTreeBag>();
        _assert_mutable::<crate::bag::char_tree_bag::CharTreeBag>();
        _assert_collection::<crate::stack::char_array_stack::CharArrayStack>();
        _assert_mutable::<crate::stack::char_array_stack::CharArrayStack>();
        _assert_collection::<crate::treeset::char_tree_set::CharTreeSet>();
        _assert_mutable::<crate::treeset::char_tree_set::CharTreeSet>();

        // Immutable collections — read-only trait only
        _assert_collection::<crate::immutable::immutable_char_array_list::ImmutableCharArrayList>();
        _assert_collection::<crate::immutable::immutable_char_hash_set::ImmutableCharHashSet>();
        _assert_collection::<crate::immutable::immutable_char_hash_bag::ImmutableCharHashBag>();
        _assert_collection::<crate::immutable::immutable_char_array_stack::ImmutableCharArrayStack>(
        );

        // Category traits — Lists
        _assert_list::<crate::arraylist::char_array_list::CharArrayList>();
        _assert_mutable_list::<crate::arraylist::char_array_list::CharArrayList>();
        _assert_list::<crate::immutable::immutable_char_array_list::ImmutableCharArrayList>();

        // Category traits — Sets
        _assert_set::<crate::hashset::char_hash_set::CharHashSet>();
        _assert_mutable_set::<crate::hashset::char_hash_set::CharHashSet>();
        _assert_set::<crate::treeset::char_tree_set::CharTreeSet>();
        _assert_mutable_set::<crate::treeset::char_tree_set::CharTreeSet>();
        _assert_set::<crate::immutable::immutable_char_hash_set::ImmutableCharHashSet>();

        // Category traits — Bags
        _assert_bag::<crate::bag::char_hash_bag::CharHashBag>();
        _assert_mutable_bag::<crate::bag::char_hash_bag::CharHashBag>();
        _assert_bag::<crate::bag::char_tree_bag::CharTreeBag>();
        _assert_mutable_bag::<crate::bag::char_tree_bag::CharTreeBag>();
        _assert_bag::<crate::immutable::immutable_char_hash_bag::ImmutableCharHashBag>();

        // Category traits — Stacks
        _assert_stack::<crate::stack::char_array_stack::CharArrayStack>();
        _assert_mutable_stack::<crate::stack::char_array_stack::CharArrayStack>();
        _assert_stack::<crate::immutable::immutable_char_array_stack::ImmutableCharArrayStack>();
    }

    /// Proves the trait contract is reachable from outside the codegen.
    /// A minimal third-party type implementing CharMutableList, with no
    /// dependency on generated code beyond the trait definitions.
    struct FakeCharList {
        items: Vec<char>,
    }

    impl CharCollection for FakeCharList {
        fn len(&self) -> usize {
            self.items.len()
        }
        fn contains(&self, value: char) -> bool {
            self.items.contains(&value)
        }
        fn iter(&self) -> impl Iterator<Item = char> + '_ {
            self.items.iter().copied()
        }
    }
    impl CharMutableCollection for FakeCharList {
        fn clear(&mut self) {
            self.items.clear();
        }
    }
    impl CharList for FakeCharList {
        fn get(&self, index: usize) -> Option<char> {
            self.items.get(index).copied()
        }
        fn index_of(&self, value: char) -> Option<usize> {
            self.items.iter().position(|&v| v == value)
        }
    }
    impl CharMutableList for FakeCharList {
        fn push(&mut self, value: char) {
            self.items.push(value);
        }
        fn set(&mut self, index: usize, value: char) -> char {
            let old = self.items[index];
            self.items[index] = value;
            old
        }
    }

    #[test]
    fn third_party_impl_satisfies_traits() {
        _assert_mutable_list::<FakeCharList>();
        let mut fake = FakeCharList { items: Vec::new() };
        fake.push('a');
        assert_eq!(fake.len(), 1);
        assert!(fake.contains('a'));
        // Verify defaulted methods work via trait dispatch
        assert!(!fake.is_empty());
        assert_eq!(fake.to_vec().len(), 1);
    }
}
