// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Please see the COPYING file in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Arena;
use stack_graphs::arena::SupplementalArena;

#[test]
fn can_allocate_in_arena() {
    let mut arena = Arena::new();
    let hello1 = arena.add("hello".to_string());
    let hello2 = arena.add("hello".to_string());
    let there = arena.add("there".to_string());
    assert_ne!(hello1, hello2);
    assert_ne!(hello1, there);
    assert_ne!(hello2, there);
    assert_eq!(arena.get(hello1), arena.get(hello2));
    assert_ne!(arena.get(hello1), arena.get(there));
    assert_ne!(arena.get(hello2), arena.get(there));
}

#[test]
fn can_allocate_in_supplemental_arena() {
    let mut arena = Arena::<u32>::new();
    let h1 = arena.add(1);
    let h2 = arena.add(2);
    let h3 = arena.add(3);
    let mut supplemental = SupplementalArena::<u32, String>::new();
    assert_eq!(supplemental.get(h1), None);
    assert_eq!(supplemental.get(h2), None);
    assert_eq!(supplemental.get(h3), None);
    assert_eq!(&mut supplemental[h1], ""); // &mut to force "get or create" behavior
    supplemental[h2].push_str("hiya");
    assert_eq!(supplemental.get(h2).map(String::as_str), Some("hiya"));
}
