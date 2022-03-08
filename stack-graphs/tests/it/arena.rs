// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::arena::Arena;
use stack_graphs::arena::Deque;
use stack_graphs::arena::DequeArena;
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
    assert_eq!(collect(&list, &arena), vec![] as Vec<u32>);
    list.push_front(&mut arena, 1);
    assert_eq!(collect(&list, &arena), vec![1]);
    list.push_front(&mut arena, 2);
    list.push_front(&mut arena, 3);
    assert_eq!(collect(&list, &arena), vec![3, 2, 1]);
}

#[test]
fn can_compare_lists() {
    use std::cmp::Ordering;
    let mut arena = List::new_arena();
    let mut from_slice = |slice: &[u32]| {
        let mut list = List::empty();
        for element in slice.iter().rev() {
            list.push_front(&mut arena, *element);
        }
        list
    };
    let list0 = from_slice(&[]);
    let list1 = from_slice(&[1]);
    let list2 = from_slice(&[2]);
    let list12 = from_slice(&[1, 2]);
    assert!(list0.equals(&arena, list0));
    assert_eq!(list0.cmp(&arena, list0), Ordering::Equal);
    assert!(!list0.equals(&arena, list1));
    assert_eq!(list0.cmp(&arena, list1), Ordering::Less);
    assert!(list1.equals(&arena, list1));
    assert_eq!(list1.cmp(&arena, list1), Ordering::Equal);
    assert!(!list1.equals(&arena, list2));
    assert_eq!(list1.cmp(&arena, list2), Ordering::Less);
    assert!(list2.equals(&arena, list2));
    assert_eq!(list2.cmp(&arena, list12), Ordering::Greater);
    assert_eq!(list1.cmp(&arena, list12), Ordering::Less);
}

#[test]
fn can_create_reversible_lists() {
    fn collect(list: &ReversibleList<u32>, arena: &ReversibleListArena<u32>) -> Vec<u32> {
        list.iter(arena).copied().collect()
    }

    let mut arena = ReversibleList::new_arena();
    let mut list = ReversibleList::empty();
    assert_eq!(collect(&list, &arena), vec![] as Vec<u32>);
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

#[test]
fn can_compare_reversible_lists() {
    use std::cmp::Ordering;
    let mut arena = ReversibleList::new_arena();
    let mut from_slice = |slice: &[u32]| {
        let mut list = ReversibleList::empty();
        for element in slice.iter().rev() {
            list.push_front(&mut arena, *element);
        }
        list
    };
    let list0 = from_slice(&[]);
    let list1 = from_slice(&[1]);
    let list2 = from_slice(&[2]);
    let list12 = from_slice(&[1, 2]);
    assert!(list0.equals(&arena, list0));
    assert_eq!(list0.cmp(&arena, list0), Ordering::Equal);
    assert!(!list0.equals(&arena, list1));
    assert_eq!(list0.cmp(&arena, list1), Ordering::Less);
    assert!(list1.equals(&arena, list1));
    assert_eq!(list1.cmp(&arena, list1), Ordering::Equal);
    assert!(!list1.equals(&arena, list2));
    assert_eq!(list1.cmp(&arena, list2), Ordering::Less);
    assert!(list2.equals(&arena, list2));
    assert_eq!(list2.cmp(&arena, list12), Ordering::Greater);
    assert_eq!(list1.cmp(&arena, list12), Ordering::Less);
    let mut list21 = list12;
    list21.reverse(&mut arena);
    assert_eq!(list2.cmp(&arena, list21), Ordering::Less);
    assert_eq!(list1.cmp(&arena, list21), Ordering::Less);
}

#[test]
fn can_create_deques() {
    fn collect(deque: &Deque<u32>, arena: &mut DequeArena<u32>) -> Vec<u32> {
        deque.iter(arena).copied().collect()
    }
    fn collect_reused(deque: &Deque<u32>, arena: &DequeArena<u32>) -> Vec<u32> {
        deque.iter_reused(arena).copied().collect()
    }
    fn collect_rev(deque: &Deque<u32>, arena: &mut DequeArena<u32>) -> Vec<u32> {
        deque.iter_reversed(arena).copied().collect()
    }

    let mut arena = Deque::new_arena();
    let mut deque = Deque::empty();
    assert_eq!(collect(&deque, &mut arena), vec![] as Vec<u32>);
    assert_eq!(collect_rev(&deque, &mut arena), vec![] as Vec<u32>);
    deque.push_front(&mut arena, 1);
    assert_eq!(collect(&deque, &mut arena), vec![1]);
    assert_eq!(collect_rev(&deque, &mut arena), vec![1]);
    deque.push_front(&mut arena, 2);
    deque.push_front(&mut arena, 3);
    assert_eq!(collect(&deque, &mut arena), vec![3, 2, 1]);
    assert_eq!(collect_rev(&deque, &mut arena), vec![1, 2, 3]);
    deque.push_back(&mut arena, 4);
    assert_eq!(collect(&deque, &mut arena), vec![3, 2, 1, 4]);
    assert_eq!(collect_rev(&deque, &mut arena), vec![4, 1, 2, 3]);
    deque.push_back(&mut arena, 5);
    deque.push_back(&mut arena, 6);
    assert_eq!(collect(&deque, &mut arena), vec![3, 2, 1, 4, 5, 6]);
    assert_eq!(collect_rev(&deque, &mut arena), vec![6, 5, 4, 1, 2, 3]);
    deque.push_front(&mut arena, 7);
    deque.push_front(&mut arena, 8);
    assert_eq!(collect(&deque, &mut arena), vec![8, 7, 3, 2, 1, 4, 5, 6]);
    assert_eq!(collect_reused(&deque, &arena), vec![8, 7, 3, 2, 1, 4, 5, 6]);
    assert_eq!(
        collect_rev(&deque, &mut arena),
        vec![6, 5, 4, 1, 2, 3, 7, 8]
    );
}

#[test]
fn can_compare_deques() {
    use std::cmp::Ordering;
    let mut arena = Deque::new_arena();
    // Build up deques in both directions so that our comparisons have to test the "reverse if
    // needed" logic.
    let from_slice_forwards = |slice: &[u32], arena: &mut DequeArena<u32>| {
        let mut deque = Deque::empty();
        for element in slice.iter() {
            deque.push_back(arena, *element);
        }
        deque
    };
    let from_slice_backwards = |slice: &[u32], arena: &mut DequeArena<u32>| {
        let mut deque = Deque::empty();
        for element in slice.iter().rev() {
            deque.push_front(arena, *element);
        }
        deque
    };
    let deque0 = from_slice_forwards(&[], &mut arena);
    let mut deque1 = from_slice_backwards(&[1], &mut arena);
    let deque2 = from_slice_forwards(&[2], &mut arena);
    let mut deque10 = from_slice_backwards(&[1, 0], &mut arena);
    let deque12 = from_slice_backwards(&[1, 2], &mut arena);
    assert!(deque0.equals(&mut arena, deque0));
    assert_eq!(deque0.cmp(&mut arena, deque0), Ordering::Equal);
    assert!(!deque0.equals(&mut arena, deque1));
    assert_eq!(deque0.cmp(&mut arena, deque1), Ordering::Less);
    assert!(deque1.equals(&mut arena, deque1));
    assert_eq!(deque1.cmp(&mut arena, deque1), Ordering::Equal);
    assert!(!deque1.equals(&mut arena, deque2));
    assert_eq!(deque1.cmp(&mut arena, deque2), Ordering::Less);
    assert!(deque2.equals(&mut arena, deque2));
    assert_eq!(deque2.cmp(&mut arena, deque12), Ordering::Greater);
    assert_eq!(deque1.cmp(&mut arena, deque12), Ordering::Less);

    // We should get the same result regardless of which direction the deques are pointing.
    deque1.ensure_forwards(&mut arena);
    deque10.ensure_forwards(&mut arena);
    assert_eq!(deque1.cmp(&mut arena, deque10), Ordering::Less);
    deque1.ensure_backwards(&mut arena);
    deque10.ensure_backwards(&mut arena);
    assert_eq!(deque1.cmp(&mut arena, deque10), Ordering::Less);
}
