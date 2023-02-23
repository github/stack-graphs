// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;

use crate::util::*;

// ----------------------------------------------------------------------------
// productive paths

#[test]
fn renaming_path_is_productive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, bar_ref, s])
        .unwrap();

    assert!(p.is_productive(&graph, &mut partials));
}

#[test]
fn renaming_root_path_is_productive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = StackGraph::root_node();
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, bar_ref, s])
        .unwrap();

    assert!(p.is_productive(&graph, &mut partials));
}

#[test]
fn introducing_path_is_unproductive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, bar_ref, s]).unwrap();

    assert!(!p.is_productive(&graph, &mut partials));
}

#[test]
fn eliminating_path_is_productive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, s]).unwrap();

    assert!(p.is_productive(&graph, &mut partials));
}

#[test]
fn identity_path_is_unproductive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let bar_def = create_pop_symbol_node(&mut graph, file, "bar", true);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, bar_def, bar_ref, s])
        .unwrap();

    assert!(!p.is_productive(&graph, &mut partials));
}

#[test]
fn one_step_forward_two_steps_back_path_is_unproductive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);
    let foo_ref1 = create_push_symbol_node(&mut graph, file, "foo", true);
    let foo_ref2 = create_push_symbol_node(&mut graph, file, "foo", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(
        &mut graph,
        &mut partials,
        &[s, foo_def, foo_ref1, foo_ref2, s],
    )
    .unwrap();

    assert!(!p.is_productive(&graph, &mut partials));
}

#[test]
fn two_steps_forward_one_step_back_path_is_productive() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def1 = create_pop_symbol_node(&mut graph, file, "foo", true);
    let foo_def2 = create_pop_symbol_node(&mut graph, file, "foo", true);
    let foo_ref = create_push_symbol_node(&mut graph, file, "foo", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(
        &mut graph,
        &mut partials,
        &[s, foo_def1, foo_def2, foo_ref, s],
    )
    .unwrap();

    assert!(p.is_productive(&graph, &mut partials));
}
