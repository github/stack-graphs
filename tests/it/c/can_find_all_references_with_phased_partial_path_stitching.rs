// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

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
use stack_graphs::c::sg_reverse_partial_path_stitcher_free;
use stack_graphs::c::sg_reverse_partial_path_stitcher_new;
use stack_graphs::c::sg_reverse_partial_path_stitcher_process_next_phase;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::copious_debugging;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialScopeStackBindings;
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

fn check_find_all_references(graph: &TestGraph, file: &str, expected_partial_paths: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let file = rust_graph.get_file_unchecked(file);
    let partials = sg_partial_path_arena_new();
    let rust_partials = unsafe { &mut (*partials).inner };
    let db = sg_partial_path_database_new();

    // Create a new external storage layer holding _all_ of the partial paths in the stack graph.
    let mut storage_layer = StorageLayer::new(graph.graph, partials);

    // Find every definition in the requested file.  These will be the ending nodes for the
    // stitching algorithm.
    let definitions = rust_graph
        .iter_nodes()
        .filter(|handle| {
            let node = &rust_graph[*handle];
            node.is_in_file(file) && node.is_definition()
        })
        .collect::<Vec<_>>();

    // Seed the database with the partial paths that end at any of the ending nodes.
    copious_debugging!("==> Add initial partial paths");
    storage_layer.add_to_database(graph.graph, partials, db, |partial_path| {
        definitions.contains(&partial_path.end_node)
    });

    // Create the reverse partial path stitcher.
    let stitcher = sg_reverse_partial_path_stitcher_new(
        graph.graph,
        partials,
        db,
        definitions.len(),
        definitions.as_ptr() as *const _,
    );
    let rust_stitcher = unsafe { &mut *stitcher };

    // Keep processing phases until the stitching algorithm is done.
    let mut results = HashSet::new();
    while rust_stitcher.previous_phase_partial_paths_length > 0 {
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
            if partial_path.is_complete(rust_graph) {
                copious_debugging!(
                    "    COMPLETE PARTIAL PATH {}",
                    partial_path.display(rust_graph, rust_partials)
                );
                results.insert(partial_path.display(rust_graph, rust_partials).to_string());
            }

            // Find any extensions of this partial path in the storage layer, and add them to the
            // database.
            let start_node = &rust_graph[partial_path.start_node];
            if start_node.is_root() || start_node.is_jump_to() {
                // The candidate partial path starts at the root node or jump to scope node, so add
                // any partial path whose symbol stack postcondition satisfies the candidate
                // partial path's precondition.
                storage_layer.add_to_database(graph.graph, partials, db, |extension| {
                    if extension.end_node != partial_path.start_node {
                        return false;
                    }

                    let mut symbol_bindings = PartialSymbolStackBindings::new();
                    let mut scope_bindings = PartialScopeStackBindings::new();
                    extension
                        .symbol_stack_postcondition
                        .unify(
                            rust_partials,
                            partial_path.symbol_stack_precondition,
                            &mut symbol_bindings,
                            &mut scope_bindings,
                        )
                        .is_ok()
                });
            } else {
                // The candidate partial path start at a non-singleton node, so add any partial
                // path that ends at the candidate partial path's start node.
                storage_layer.add_to_database(graph.graph, partials, db, |extension| {
                    extension.end_node == partial_path.start_node
                });
            }
        }

        // And then kick off the next phase!
        sg_reverse_partial_path_stitcher_process_next_phase(graph.graph, partials, db, stitcher);
    }
    copious_debugging!("==> Path stitching done");

    // And finally verify that we found all of the partial paths that we expected to.
    let expected_partial_paths = expected_partial_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<HashSet<_>>();
    assert_eq!(results, expected_partial_paths);

    sg_reverse_partial_path_stitcher_free(stitcher);
    sg_partial_path_database_free(db);
    sg_partial_path_arena_free(partials);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_find_all_references(&graph, "main.py", &[]);
    check_find_all_references(
        &graph,
        "a.py",
        &[
            "<%2> () [main.py(17) reference a] -> [a.py(0) definition a] <%2> ()",
            "<%2> () [main.py(13) reference foo] -> [a.py(5) definition foo] <%2> ()",
            "<%1> () [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
        ],
    );
    check_find_all_references(
        &graph,
        "b.py",
        &[
            "<%2> () [main.py(15) reference b] -> [b.py(0) definition b] <%2> ()",
            "<%2> () [main.py(9) reference A] -> [b.py(5) definition A] <%2> ()",
            "<%2> () [main.py(10) reference bar] -> [b.py(8) definition bar] <%2> ()",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_find_all_references(&graph, "main.py", &[]);
    check_find_all_references(
        &graph,
        "a.py",
        &[
            "<%2> () [main.py(8) reference a] -> [a.py(0) definition a] <%2> ()",
            "<%2> () [b.py(8) reference a] -> [a.py(0) definition a] <%2> ()",
        ],
    );
    check_find_all_references(
        &graph,
        "b.py",
        &[
            "<%2> () [a.py(6) reference b] -> [b.py(0) definition b] <%2> ()",
            "<%2> () [main.py(6) reference foo] -> [b.py(6) definition foo] <%2> ()",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_find_all_references(
        &graph,
        "test.rs",
        &[
            "<%1> () [test.rs(103) reference a] -> [test.rs(201) definition a] <%1> ()",
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <%1> ()",
            "<%1> () [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <%1> ()",
            "<%1> () [test.rs(206) reference b] -> [test.rs(301) definition b] <%1> ()",
            "<%1> () [test.rs(307) reference a] -> [test.rs(201) definition a] <%1> ()",
            "<%1> () [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <%1> ()",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_find_all_references(&graph, "main.py", &[]);
    check_find_all_references(
        &graph,
        "a.py",
        &["<%2> () [main.py(8) reference a] -> [a.py(0) definition a] <%2> ()"],
    );
    check_find_all_references(
        &graph,
        "b.py",
        &[
            "<%2> () [a.py(6) reference b] -> [b.py(0) definition b] <%2> ()",
            "<%2> () [main.py(6) reference foo] -> [b.py(5) definition foo] <%2> ()",
        ],
    );
}
