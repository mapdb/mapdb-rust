// Copyright (c) 2026 Jan Kotek.
// Derived from Eclipse Collections (Copyright (c) Goldman Sachs and others).
// Licensed under the Eclipse Public License v1.0 and Eclipse Distribution License v1.0.
// See LICENSE-EPL-1.0.txt and LICENSE-EDL-1.0.txt.
// USE AT YOUR OWN RISK — THIS SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND.

// ═══════════════════════════════════════════════════════════════════════
// Pet Kata — Eclipse Collections for Rust
//
// This is a learning exercise. Each test has a TODO where you need to
// write code using the object collection API. The assertions are already
// written — your job is to make them pass.
//
// Run:  cargo test object::pet_kata -- --nocapture
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use crate::object::{
        ArrayList, Bag, Collection, HashBag, HashSet, MutableBag, MutableList, MutableSet,
    };

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum PetType {
        Cat,
        Dog,
        Hamster,
        Turtle,
        Bird,
        Snake,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Pet {
        name: String,
        pet_type: PetType,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Person {
        first_name: String,
        last_name: String,
        pets: ArrayList<Pet>,
    }

    impl Person {
        fn new(first: &str, last: &str, pets: Vec<Pet>) -> Self {
            Person {
                first_name: first.to_string(),
                last_name: last.to_string(),
                pets: ArrayList::of(pets),
            }
        }
        fn has_pet_type(&self, pt: &PetType) -> bool {
            self.pets.any_satisfy(|p| &p.pet_type == pt)
        }
        fn full_name(&self) -> String {
            format!("{} {}", self.first_name, self.last_name)
        }
    }

    fn pet(name: &str, pt: PetType) -> Pet {
        Pet {
            name: name.to_string(),
            pet_type: pt,
        }
    }

    fn setup_people() -> ArrayList<Person> {
        ArrayList::of(vec![
            Person::new("Mary", "Smith", vec![pet("Tabby", PetType::Cat)]),
            Person::new(
                "Bob",
                "Smith",
                vec![pet("Dolly", PetType::Dog), pet("Spot", PetType::Dog)],
            ),
            Person::new(
                "Ted",
                "Smith",
                vec![pet("Spike", PetType::Dog), pet("Serpy", PetType::Snake)],
            ),
            Person::new(
                "Jake",
                "Snake",
                vec![pet("Speedy", PetType::Turtle), pet("Tweety", PetType::Bird)],
            ),
            Person::new(
                "Barry",
                "Jones",
                vec![
                    pet("Fluffy", PetType::Cat),
                    pet("Crunchie", PetType::Hamster),
                ],
            ),
            Person::new(
                "Terry",
                "Schneider",
                vec![pet("Cozy", PetType::Cat), pet("Rumple", PetType::Hamster)],
            ),
            Person::new("Harry", "Harrison", vec![]),
        ])
    }

    // Exercise 1: Do any people have cats?
    #[test]
    fn do_any_people_have_cats() {
        let people = setup_people();
        let _ = &people; // TODO: replace false with people.any_satisfy(...)
        let has_cats = false;
        assert!(has_cats, "expected someone to have a cat");
    }

    // Exercise 2: Do all people have pets?
    #[test]
    fn do_all_people_have_pets() {
        let people = setup_people();
        let _ = &people; // TODO: replace true with people.all_satisfy(...)
        let all_have_pets = true;
        assert!(!all_have_pets, "Harry has no pets");
    }

    // Exercise 3: Does nobody have snakes?
    #[test]
    fn does_nobody_have_snakes() {
        let people = setup_people();
        let _ = &people; // TODO: replace true with people.none_satisfy(...)
        let no_snakes = true;
        assert!(!no_snakes, "Ted has a snake");
    }

    // Exercise 4: How many people have cats?
    #[test]
    fn how_many_people_have_cats() {
        let people = setup_people();
        let _ = &people; // TODO: replace 0 with people.count_where(...)
        let count = 0;
        assert_eq!(count, 3);
    }

    // Exercise 5: Get the people who have cats.
    #[test]
    fn get_people_with_cats() {
        let people = setup_people();
        let _ = &people; // TODO: replace with people.select(...)
        let cat_people: Vec<Person> = vec![];
        assert_eq!(cat_people.len(), 3);
        let names: Vec<_> = cat_people.iter().map(|p| p.first_name.as_str()).collect();
        assert!(names.contains(&"Mary"));
        assert!(names.contains(&"Barry"));
        assert!(names.contains(&"Terry"));
    }

    // Exercise 6: Get the people who do NOT have cats.
    #[test]
    fn get_people_without_cats() {
        let people = setup_people();
        let _ = &people; // TODO: replace with people.reject(...)
        let no_cat_people: Vec<Person> = vec![];
        assert_eq!(no_cat_people.len(), 4);
    }

    // Exercise 7: Find Mary Smith.
    #[test]
    fn find_mary_smith() {
        let people = setup_people();
        let _ = &people; // TODO: replace with people.detect(...)
        let mary: Option<&Person> = None;
        assert!(mary.is_some(), "should find Mary Smith");
        let mary = mary.unwrap();
        assert_eq!(mary.full_name(), "Mary Smith");
        assert_eq!(mary.pets.len(), 1);
    }

    // Exercise 8: Collect all pet names.
    #[test]
    fn collect_all_pet_names() {
        let people = setup_people();
        let mut pet_names = ArrayList::<String>::new();
        let _ = &people; // TODO: iterate people and their pets, push each name

        assert_eq!(pet_names.len(), 11);
        assert!(pet_names.contains(&"Tabby".to_string()));
        assert!(pet_names.contains(&"Tweety".to_string()));
    }

    // Exercise 9: Count pet types with a HashBag.
    #[test]
    fn count_pet_types() {
        let people = setup_people();
        let mut bag = HashBag::<PetType>::new();
        let _ = &people; // TODO: iterate all pets, bag.add(pet_type)

        assert_eq!(bag.occurrences_of(&PetType::Cat), 3);
        assert_eq!(bag.occurrences_of(&PetType::Dog), 3);
        assert_eq!(bag.occurrences_of(&PetType::Hamster), 2);
        assert_eq!(bag.len(), 11);
        let top = bag.top_occurrences(2);
        assert_eq!(top[0].1, 3);
    }

    // Exercise 10: Collect all unique pet types with a HashSet.
    #[test]
    fn unique_pet_types() {
        let people = setup_people();
        let mut types = HashSet::<PetType>::new();
        let _ = &people; // TODO: iterate all pets, types.add(pet_type)

        assert_eq!(types.len(), 6);
        assert!(types.contains(&PetType::Cat));
        assert!(types.contains(&PetType::Snake));
    }

    // Exercise 11: Total pet count via inject_into.
    #[test]
    fn total_pet_count() {
        let people = setup_people();
        let _ = &people; // TODO: replace 0 with people.inject_into(...)
        let total: usize = 0;
        assert_eq!(total, 11);
    }

    // Exercise 12: Detect a person who doesn't exist.
    #[test]
    fn detect_not_found() {
        let people = setup_people();
        let _ = &people; // TODO: replace with people.detect(...)
        let nobody: Option<&Person> = Some(&Person::new("x", "x", vec![]));
        assert!(nobody.is_none());
    }
}
