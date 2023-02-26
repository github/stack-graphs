// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::cycles::Appendables;
use stack_graphs::cycles::AppendingCycleDetector;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::Cyclicity;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::OwnedOrDatabasePath;
use stack_graphs::CancelAfterDuration;
use std::time::Duration;

use crate::util::*;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);

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

    assert!(matches!(p.is_cyclic(&graph, &mut partials), None));
}

#[test]
fn renaming_root_path_is_not_cyclic() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = StackGraph::root_node();
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, bar_ref, s])
        .unwrap();

    assert_eq!(None, p.is_cyclic(&graph, &mut partials));
}

#[test]
fn introducing_path_is_cyclic() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, bar_ref, s]).unwrap();

    assert_eq!(
        Some(Cyclicity::StrengthensPostcondition),
        p.is_cyclic(&graph, &mut partials)
    );
}

#[test]
fn eliminating_path_is_cyclic() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, s]).unwrap();

    assert_eq!(
        Some(Cyclicity::StrengthensPrecondition),
        p.is_cyclic(&graph, &mut partials)
    );
}

#[test]
fn identity_path_is_cyclic() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let s = create_scope_node(&mut graph, file, false);
    let bar_def = create_pop_symbol_node(&mut graph, file, "bar", true);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", true);

    let mut partials = PartialPaths::new();
    let p = create_partial_path_and_edges(&mut graph, &mut partials, &[s, bar_def, bar_ref, s])
        .unwrap();

    assert_eq!(
        Some(Cyclicity::StrengthensPostcondition),
        p.is_cyclic(&graph, &mut partials)
    );
}

#[test]
fn one_step_forward_two_steps_back_path_is_cyclic() {
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

    assert_eq!(
        Some(Cyclicity::StrengthensPostcondition),
        p.is_cyclic(&graph, &mut partials)
    );
}

#[test]
fn two_steps_forward_one_step_back_path_is_cyclic() {
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

    assert_eq!(
        Some(Cyclicity::StrengthensPrecondition),
        p.is_cyclic(&graph, &mut partials)
    );
}

// ----------------------------------------------------------------------------
// cycle detection

#[test]
fn finding_simple_identity_cycle_is_detected() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let r = StackGraph::root_node();
    let s = create_scope_node(&mut graph, file, false);
    let foo_ref = create_push_symbol_node(&mut graph, file, "foo", false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", false);

    let mut partials = PartialPaths::new();
    create_partial_path_and_edges(&mut graph, &mut partials, &[r, foo_ref, s, foo_def, r]).unwrap();

    // test edge cycle detector
    {
        let mut edges = Appendables::new();
        let mut cd = AppendingCycleDetector::new();
        let ctx = &mut ();

        for edge in &[
            edge(r, foo_ref, 0),
            edge(foo_ref, s, 0),
            edge(s, foo_def, 0),
        ] {
            cd.append(&mut edges, *edge);
            assert!(cd
                .is_cyclic(&graph, &mut partials, ctx, &mut edges)
                .unwrap()
                .is_empty());
        }
        cd.append(&mut edges, edge(foo_def, r, 0));
        assert_eq!(
            vec![Cyclicity::StrengthensPostcondition],
            cd.is_cyclic(&graph, &mut partials, ctx, &mut edges)
                .unwrap()
        );
    }

    // test termination of path finding
    {
        let mut path_count = 0usize;
        let cancellation_flag = CancelAfterDuration::new(TEST_TIMEOUT);
        let result = partials.find_minimal_partial_path_set_in_file(
            &graph,
            file,
            &cancellation_flag,
            |_, _, _| path_count += 1,
        );
        assert!(result.is_ok());
        assert_eq!(1, path_count);
    }
}

#[test]
fn stitching_simple_identity_cycle_is_detected() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let r = StackGraph::root_node();
    let s = create_scope_node(&mut graph, file, true);
    let foo_ref = create_push_symbol_node(&mut graph, file, "foo", false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", false);

    let mut partials = PartialPaths::new();
    let p0 = create_partial_path_and_edges(&mut graph, &mut partials, &[r, foo_ref, s]).unwrap();
    let p1 = create_partial_path_and_edges(&mut graph, &mut partials, &[s, foo_def, r]).unwrap();

    let mut db = Database::new();
    let p0 = db.add_partial_path(&graph, &mut partials, p0);
    let p1 = db.add_partial_path(&graph, &mut partials, p1);

    // test partial path cycle detector
    {
        let mut paths = Appendables::new();
        let mut cd: AppendingCycleDetector<OwnedOrDatabasePath> =
            AppendingCycleDetector::from(&mut paths, p0.into());
        cd.append(&mut paths, p1.into());
        assert_eq!(
            vec![Cyclicity::StrengthensPostcondition],
            cd.is_cyclic(&graph, &mut partials, &mut db, &mut paths)
                .unwrap()
        );
    }
}

#[test]
fn finding_composite_identity_cycle_is_detected() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let r = StackGraph::root_node();
    let s = create_scope_node(&mut graph, file, false);
    let foo_ref = create_push_symbol_node(&mut graph, file, "foo", false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", false);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", false);
    let bar_def = create_pop_symbol_node(&mut graph, file, "bar", false);

    let mut partials = PartialPaths::new();
    create_partial_path_and_edges(
        &mut graph,
        &mut partials,
        &[r, s, bar_def, foo_ref, s, foo_def, bar_ref, s, r],
    )
    .unwrap();

    // test edge cycle detector
    {
        let mut edges = Appendables::new();
        let mut cd = AppendingCycleDetector::new();
        let ctx = &mut ();
        for edge in &[
            edge(r, s, 0),
            edge(r, s, 0),
            edge(s, bar_def, 0),
            edge(bar_def, foo_ref, 0),
            edge(foo_ref, s, 0),
            edge(s, foo_def, 0),
            edge(foo_def, bar_ref, 0),
        ] {
            cd.append(&mut edges, *edge);
            assert!(cd
                .is_cyclic(&graph, &mut partials, ctx, &mut edges)
                .unwrap()
                .is_empty());
        }
        cd.append(&mut edges, edge(bar_ref, s, 0));
        assert_eq!(
            vec![Cyclicity::StrengthensPostcondition],
            cd.is_cyclic(&graph, &mut partials, ctx, &mut edges)
                .unwrap()
        );
    }

    // test termination of path finding
    {
        let mut path_count = 0usize;
        let cancellation_flag = CancelAfterDuration::new(TEST_TIMEOUT);
        let result = partials.find_minimal_partial_path_set_in_file(
            &graph,
            file,
            &cancellation_flag,
            |_, _, _| path_count += 1,
        );
        assert!(result.is_ok());
        assert_eq!(3, path_count);
    }
}

#[test]
fn stitching_composite_identity_cycle_is_detected() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let r = StackGraph::root_node();
    let foo_ref = create_push_symbol_node(&mut graph, file, "foo", false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", false);
    let bar_ref = create_push_symbol_node(&mut graph, file, "bar", false);
    let bar_def = create_pop_symbol_node(&mut graph, file, "bar", false);

    let mut partials = PartialPaths::new();
    let p0 = create_partial_path_and_edges(&mut graph, &mut partials, &[r, bar_def, foo_ref, r])
        .unwrap();
    let p1 = create_partial_path_and_edges(&mut graph, &mut partials, &[r, foo_def, bar_ref, r])
        .unwrap();

    let mut db = Database::new();
    let p0 = db.add_partial_path(&graph, &mut partials, p0);
    let p1 = db.add_partial_path(&graph, &mut partials, p1);

    // test joining cycle detector
    {
        let mut paths = Appendables::new();
        let mut cd: AppendingCycleDetector<OwnedOrDatabasePath> =
            AppendingCycleDetector::from(&mut paths, p0.into());
        cd.append(&mut paths, p1.into());
        assert_eq!(
            vec![Cyclicity::StrengthensPostcondition],
            cd.is_cyclic(&graph, &mut partials, &mut db, &mut paths)
                .unwrap()
        );
    }
}

#[test]
fn appending_eliminating_cycle_terminates() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").unwrap();
    let r = StackGraph::root_node();
    let s = create_scope_node(&mut graph, file, false);
    let foo_def = create_pop_symbol_node(&mut graph, file, "foo", false);

    let mut partials = PartialPaths::new();
    create_partial_path_and_edges(&mut graph, &mut partials, &[r, s, foo_def, s, r]).unwrap();

    // test termination of path finding
    {
        let mut path_count = 0usize;
        let cancellation_flag = CancelAfterDuration::new(TEST_TIMEOUT);
        let result = partials.find_minimal_partial_path_set_in_file(
            &graph,
            file,
            &cancellation_flag,
            |_, _, _| path_count += 1,
        );
        assert!(result.is_ok());
        assert_eq!(1, path_count);
    }
}
