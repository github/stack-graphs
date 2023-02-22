// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;

use crate::util::*;

// ----------------------------------------------------------------------------
// productive paths

#[test]
fn renaming_path_is_productive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let foo = g.add_symbol("foo");
    let bar = g.add_symbol("bar");
    let foo_def = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), foo, true)
        .unwrap();
    let bar_ref = g
        .add_push_symbol_node(NodeID::new_in_file(f, 2), bar, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, foo_def, bar_ref, s]).unwrap();

    assert!(p.is_productive(&g, &mut ps));
}

#[test]
fn renaming_root_path_is_productive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = StackGraph::root_node();

    let foo = g.add_symbol("foo");
    let bar = g.add_symbol("bar");
    let foo_def = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), foo, true)
        .unwrap();
    let bar_ref = g
        .add_push_symbol_node(NodeID::new_in_file(f, 2), bar, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, foo_def, bar_ref, s]).unwrap();

    assert!(p.is_productive(&g, &mut ps));
}

#[test]
fn introducing_path_is_unproductive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let bar = g.add_symbol("bar");
    let bar_ref = g
        .add_push_symbol_node(NodeID::new_in_file(f, 1), bar, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, bar_ref, s]).unwrap();

    assert!(!p.is_productive(&g, &mut ps));
}

#[test]
fn eliminating_path_is_productive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let foo = g.add_symbol("foo");
    let foo_def = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), foo, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, foo_def, s]).unwrap();

    assert!(p.is_productive(&g, &mut ps));
}

#[test]
fn identity_path_is_unproductive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let bar = g.add_symbol("bar");
    let bar_def = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), bar, true)
        .unwrap();
    let bar_ref = g
        .add_push_symbol_node(NodeID::new_in_file(f, 2), bar, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, bar_def, bar_ref, s]).unwrap();

    assert!(!p.is_productive(&g, &mut ps));
}

#[test]
fn one_step_forward_two_steps_back_path_is_unproductive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let foo = g.add_symbol("foo");
    let foo_def = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), foo, true)
        .unwrap();
    let foo_ref1 = g
        .add_push_symbol_node(NodeID::new_in_file(f, 2), foo, true)
        .unwrap();
    let foo_ref2 = g
        .add_push_symbol_node(NodeID::new_in_file(f, 3), foo, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, foo_def, foo_ref1, foo_ref2, s])
        .unwrap();

    assert!(!p.is_productive(&g, &mut ps));
}

#[test]
fn two_steps_forward_one_step_back_path_is_productive() {
    let mut g = StackGraph::new();

    let f = g.add_file("test").unwrap();

    let s = g.add_scope_node(NodeID::new_in_file(f, 0), false).unwrap();

    let foo = g.add_symbol("foo");
    let foo_def1 = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 1), foo, true)
        .unwrap();
    let foo_def2 = g
        .add_pop_symbol_node(NodeID::new_in_file(f, 2), foo, true)
        .unwrap();
    let foo_ref = g
        .add_push_symbol_node(NodeID::new_in_file(f, 3), foo, true)
        .unwrap();

    let mut ps = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut g, &mut ps, &[s, foo_def1, foo_def2, foo_ref, s])
        .unwrap();

    assert!(p.is_productive(&g, &mut ps));
}
