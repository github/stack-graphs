// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Please see the COPYING file in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Arena;

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
