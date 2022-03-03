// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use controlled_option::ControlledOption;
use pretty_assertions::assert_eq;
use stack_graphs::c::sg_forward_partial_path_stitcher_free;
use stack_graphs::c::sg_forward_partial_path_stitcher_from_partial_paths;
use stack_graphs::c::sg_forward_partial_path_stitcher_process_next_phase;
use stack_graphs::c::sg_forward_partial_path_stitcher_set_max_work_per_phase;
use stack_graphs::c::sg_partial_path;
use stack_graphs::c::sg_partial_path_arena;
use stack_graphs::c::sg_partial_path_arena_find_partial_paths_in_file;
use stack_graphs::c::sg_partial_path_arena_free;
use stack_graphs::c::sg_partial_path_arena_new;
use stack_graphs::c::sg_partial_path_database;
use stack_graphs::c::sg_partial_path_database_add_partial_paths;
use stack_graphs::c::sg_partial_path_database_free;
use stack_graphs::c::sg_partial_path_database_new;
use stack_graphs::c::sg_partial_path_handle;
use stack_graphs::c::sg_partial_path_list_count;
use stack_graphs::c::sg_partial_path_list_free;
use stack_graphs::c::sg_partial_path_list_new;
use stack_graphs::c::sg_partial_path_list_paths;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::copious_debugging;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPathEdgeList;
use stack_graphs::partial::PartialScopeStack;
use stack_graphs::partial::PartialScopeStackBindings;
use stack_graphs::partial::PartialScopedSymbol;
use stack_graphs::partial::PartialSymbolStack;
use stack_graphs::partial::PartialSymbolStackBindings;

use crate::c::test_graph::TestGraph;
use crate::test_graphs;

/// This type mimics an external data store that is the system of record for partial paths.  An
/// important part of the test case will be verifying that we can lazily load content from this
/// system of record into the sg_partial_path_database instance.
struct StorageLayer {
    partial_paths: Vec<PartialPath>,
}

impl StorageLayer {
    // Creates a new StorageLayer containing all of the partial paths that we can find in a test
    // stack graph.
    fn new(graph: *const sg_stack_graph, partials: *mut sg_partial_path_arena) -> StorageLayer {
        let rust_graph = unsafe { &(*graph).inner };
        let path_list = sg_partial_path_list_new();
        for file in rust_graph.iter_files() {
            sg_partial_path_arena_find_partial_paths_in_file(
                graph,
                partials,
                file.as_u32(),
                path_list,
            );
        }
        let path_slice = unsafe {
            std::slice::from_raw_parts(
                sg_partial_path_list_paths(path_list) as *const PartialPath,
                sg_partial_path_list_count(path_list),
            )
        };
        let partial_paths = path_slice.to_vec();
        sg_partial_path_list_free(path_list);
        StorageLayer { partial_paths }
    }

    // Copies partial paths that match a predicate into an sg_partial_path_database.  We ensure
    // that any particular partial path is only copied into the database at most once.
    fn add_to_database<F>(
        &mut self,
        graph: *const sg_stack_graph,
        partials: *mut sg_partial_path_arena,
        db: *mut sg_partial_path_database,
        mut f: F,
    ) where
        F: FnMut(&PartialPath) -> bool,
    {
        self.partial_paths.retain(|path| {
            // If the path _doesn't_ satisfy the predicate, then we keep it in the storage layer
            // and move on.
            if !f(path) {
                return true;
            }

            // If it _does_ satsify the predicate, we add it to the database.  We also _remove_ it
            // from the storage layer so that we never add it again.
            let mut out = sg_partial_path_handle::default();
            sg_partial_path_database_add_partial_paths(
                graph,
                partials,
                db,
                1,
                path as *const _ as *const sg_partial_path,
                &mut out,
            );
            false
        });
    }
}

fn check_find_qualified_definitions(
    graph: &TestGraph,
    stack_symbols: &[&str],
    expected_partial_paths: &[&str],
) {
    let rust_graph = unsafe { &mut (*graph.graph).inner };
    let partials = sg_partial_path_arena_new();
    let rust_partials = unsafe { &mut (*partials).inner };
    let db = sg_partial_path_database_new();

    // Create a new external storage layer holding _all_ of the partial paths in the stack graph.
    let mut storage_layer = StorageLayer::new(graph.graph, partials);

    // Create the requested partial symbol stack.
    let mut partial_symbol_stack = PartialSymbolStack::empty();
    for stack_symbol in stack_symbols {
        let symbol = rust_graph.add_symbol(stack_symbol);
        partial_symbol_stack.push_back(
            rust_partials,
            PartialScopedSymbol {
                symbol,
                scopes: ControlledOption::none(),
            },
        );
    }

    // Seed the database with the partial paths that start at any of the starting nodes.
    copious_debugging!(
        "==> Add initial partial paths for <{}>",
        partial_symbol_stack.display(rust_graph, rust_partials),
    );
    storage_layer.add_to_database(graph.graph, partials, db, |partial_path| {
        if partial_path.start_node != StackGraph::root_node() {
            return false;
        }

        let mut symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        partial_symbol_stack
            .unify(
                rust_partials,
                partial_path.symbol_stack_precondition,
                &mut symbol_bindings,
                &mut scope_bindings,
            )
            .is_ok()
    });

    // Create the forward partial path stitcher.
    let initial_partial_path = PartialPath {
        start_node: StackGraph::root_node(),
        end_node: StackGraph::root_node(),
        symbol_stack_precondition: partial_symbol_stack,
        symbol_stack_postcondition: partial_symbol_stack,
        scope_stack_precondition: PartialScopeStack::empty(),
        scope_stack_postcondition: PartialScopeStack::empty(),
        edges: PartialPathEdgeList::empty(),
    };
    let stitcher = sg_forward_partial_path_stitcher_from_partial_paths(
        graph.graph,
        partials,
        db,
        1,
        &initial_partial_path as *const PartialPath as *const _,
    );
    sg_forward_partial_path_stitcher_set_max_work_per_phase(stitcher, 1);
    let rust_stitcher = unsafe { &mut *stitcher };

    // Keep processing phases until the stitching algorithm is done.
    let mut results = BTreeSet::new();
    while !rust_stitcher.is_complete {
        let partial_paths_slice = unsafe {
            std::slice::from_raw_parts(
                rust_stitcher.previous_phase_partial_paths as *const PartialPath,
                rust_stitcher.previous_phase_partial_paths_length,
            )
        };
        for partial_path in partial_paths_slice {
            // Verify that path's stacks and edge list are available in both directions.
            assert!(partial_path
                .symbol_stack_precondition
                .have_reversal(rust_partials));
            assert!(partial_path
                .scope_stack_precondition
                .have_reversal(rust_partials));
            assert!(partial_path
                .symbol_stack_postcondition
                .have_reversal(rust_partials));
            assert!(partial_path
                .scope_stack_postcondition
                .have_reversal(rust_partials));
            assert!(partial_path.edges.have_reversal(rust_partials));

            // Ditto for any attached scopes in the symbol stack pre- and postcondition.
            assert!(partial_path
                .symbol_stack_precondition
                .iter_unordered(rust_partials)
                .filter_map(|symbol| symbol.scopes.into_option())
                .all(|stack| stack.have_reversal(rust_partials)));
            assert!(partial_path
                .symbol_stack_postcondition
                .iter_unordered(rust_partials)
                .filter_map(|symbol| symbol.scopes.into_option())
                .all(|stack| stack.have_reversal(rust_partials)));

            // If we found a complete partial path, add it to the result set.
            if partial_path.ends_at_definition(rust_graph) {
                copious_debugging!(
                    "    FOUND DEFINITION {}",
                    partial_path.display(rust_graph, rust_partials)
                );
                results.insert(partial_path.display(rust_graph, rust_partials).to_string());
            }

            // Find any extensions of this partial path in the storage layer, and add them to the
            // database.
            if rust_graph[partial_path.end_node].is_root() {
                // The candidate partial path ends at the root node, so add any partial path whose
                // symbol stack precondition is satisfied by the candidate partial path's
                // postcondition.
                storage_layer.add_to_database(graph.graph, partials, db, |extension| {
                    if extension.start_node != partial_path.end_node {
                        return false;
                    }

                    let mut symbol_bindings = PartialSymbolStackBindings::new();
                    let mut scope_bindings = PartialScopeStackBindings::new();
                    partial_path
                        .symbol_stack_postcondition
                        .unify(
                            rust_partials,
                            extension.symbol_stack_precondition,
                            &mut symbol_bindings,
                            &mut scope_bindings,
                        )
                        .is_ok()
                });
            } else {
                // The candidate partial path ends at a non-root node, so add any partial path that
                // starts at the candidate partial path's end node.
                storage_layer.add_to_database(graph.graph, partials, db, |extension| {
                    extension.start_node == partial_path.end_node
                });
            }
        }

        // And then kick off the next phase!
        sg_forward_partial_path_stitcher_process_next_phase(graph.graph, partials, db, stitcher);
    }
    copious_debugging!("==> Path stitching done");

    // And finally verify that we found all of the partial paths that we expected to.
    let expected_partial_paths = expected_partial_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_partial_paths, results);

    sg_forward_partial_path_stitcher_free(stitcher);
    sg_partial_path_database_free(db);
    sg_partial_path_arena_free(partials);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_find_qualified_definitions(
        &graph,
        &["a"],
        &["<a> () [root] -> [a.py(0) definition a] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["a", ".", "foo"],
        &["<a.foo> () [root] -> [a.py(5) definition foo] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b"],
        &["<b> () [root] -> [b.py(0) definition b] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b", ".", "A"],
        &["<b.A> () [root] -> [b.py(5) definition A] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b", ".", "A", ".", "bar"],
        &["<b.A.bar> () [root] -> [b.py(8) definition bar] <> ()"],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_find_qualified_definitions(
        &graph,
        &["a"],
        &["<a> () [root] -> [a.py(0) definition a] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["a", ".", "foo"],
        &["<a.foo> () [root] -> [b.py(6) definition foo] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b"],
        &["<b> () [root] -> [b.py(0) definition b] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b", ".", "foo"],
        &["<b.foo> () [root] -> [b.py(6) definition foo] <> ()"],
    );
}

#[test]
fn cyclic_imports_rust() {
    // NOTE: Because everything in this example is local to one file, there aren't any partial
    // paths involving the root node.
    let graph = test_graphs::cyclic_imports_rust::new();
    check_find_qualified_definitions(&graph, &["a"], &[]);
    check_find_qualified_definitions(&graph, &["a", "::", "BAR"], &[]);
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_find_qualified_definitions(
        &graph,
        &["a"],
        &["<a> () [root] -> [a.py(0) definition a] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["a", ".", "foo"],
        &["<a.foo> () [root] -> [b.py(5) definition foo] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b"],
        &["<b> () [root] -> [b.py(0) definition b] <> ()"],
    );
    check_find_qualified_definitions(
        &graph,
        &["b", ".", "foo"],
        &["<b.foo> () [root] -> [b.py(5) definition foo] <> ()"],
    );
}
