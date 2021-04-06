// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Arena;
use stack_graphs::arena::List;
use stack_graphs::arena::ListArena;
use stack_graphs::arena::ReversibleList;
use stack_graphs::arena::ReversibleListArena;
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

#[test]
fn can_create_lists() {
    fn collect(list: &List<u32>, arena: &ListArena<u32>) -> Vec<u32> {
        list.iter(arena).copied().collect()
    }

    let mut arena = List::new_arena();
    let mut list = List::empty();
    assert_eq!(collect(&list, &arena), vec![]);
    list.push_front(&mut arena, 1);
    assert_eq!(collect(&list, &arena), vec![1]);
    list.push_front(&mut arena, 2);
    list.push_front(&mut arena, 3);
    assert_eq!(collect(&list, &arena), vec![3, 2, 1]);
}

#[test]
fn can_create_reversible_lists() {
    fn collect(list: &ReversibleList<u32>, arena: &ReversibleListArena<u32>) -> Vec<u32> {
        list.iter(arena).copied().collect()
    }

    let mut arena = ReversibleList::new_arena();
    let mut list = ReversibleList::empty();
    assert_eq!(collect(&list, &arena), vec![]);
    list.push_front(&mut arena, 1);
    assert_eq!(collect(&list, &arena), vec![1]);
    list.push_front(&mut arena, 2);
    list.push_front(&mut arena, 3);
    assert_eq!(collect(&list, &arena), vec![3, 2, 1]);
    list.reverse(&mut arena);
    assert_eq!(collect(&list, &arena), vec![1, 2, 3]);
    list.push_front(&mut arena, 4);
    list.push_front(&mut arena, 5);
    assert_eq!(collect(&list, &arena), vec![5, 4, 1, 2, 3]);
    list.reverse(&mut arena);
    assert_eq!(collect(&list, &arena), vec![3, 2, 1, 4, 5]);
    // Verify that we stash away the re-reversal so that we don't have to recompute it.
    assert!(list.have_reversal(&arena));
}
