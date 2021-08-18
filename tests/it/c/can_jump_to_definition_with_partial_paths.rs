// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use stack_graphs::c::sg_forward_path_stitcher_free;
use stack_graphs::c::sg_forward_path_stitcher_new;
use stack_graphs::c::sg_forward_path_stitcher_process_next_phase;
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
use stack_graphs::c::sg_path_arena_free;
use stack_graphs::c::sg_path_arena_new;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::ScopeStackBindings;
use stack_graphs::partial::SymbolStackBindings;
use stack_graphs::paths::Path;

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
            eprintln!(
                "    add {}",
                path.display(unsafe { &(*graph).inner }, unsafe {
                    &mut (*partials).inner
                })
            );
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

fn check_jump_to_definition(graph: &TestGraph, file: &str, expected_paths: &[&str]) {
    let rust_graph = unsafe { &(*graph.graph).inner };
    let file = rust_graph.get_file_unchecked(file);
    let paths = sg_path_arena_new();
    let rust_paths = unsafe { &mut (*paths).inner };
    let partials = sg_partial_path_arena_new();
    let rust_partials = unsafe { &mut (*partials).inner };
    let db = sg_partial_path_database_new();

    // Create a new external storage layer holding _all_ of the partial paths in the stack graph.
    let mut storage_layer = StorageLayer::new(graph.graph, partials);

    // Find every reference in the requested file.  These will be the starting nodes for the
    // stitching algorithm.
    let references = rust_graph
        .iter_nodes()
        .filter(|handle| {
            let node = &rust_graph[*handle];
            node.is_in_file(file) && node.is_reference()
        })
        .collect::<Vec<_>>();

    // Seed the database with the partial paths that start at any of the starting nodes.
    eprintln!("==> Add initial partial paths");
    storage_layer.add_to_database(graph.graph, partials, db, |partial_path| {
        references.contains(&partial_path.start_node)
    });

    // Create the forward path stitcher.
    eprintln!("==> Starting first phase");
    let stitcher = sg_forward_path_stitcher_new(
        graph.graph,
        paths,
        partials,
        db,
        references.len(),
        references.as_ptr() as *const _,
    );
    let rust_stitcher = unsafe { &mut *stitcher };

    // Keep processing phases until the stitching algorithm is done.
    let mut results = HashSet::new();
    while rust_stitcher.previous_phase_paths_length > 0 {
        eprintln!(
            "    new path count {}",
            rust_stitcher.previous_phase_paths_length
        );
        let paths_slice = unsafe {
            std::slice::from_raw_parts(
                rust_stitcher.previous_phase_paths as *const Path,
                rust_stitcher.previous_phase_paths_length,
            )
        };
        for path in paths_slice {
            eprintln!("--> {}", path.display(rust_graph, rust_paths));

            // If we found a complete path, add it to the result set.
            if path.is_complete(rust_graph) {
                eprintln!("    COMPLETE");
                results.insert(path.display(rust_graph, rust_paths).to_string());
            }

            // Find any extensions of this path in the storage layer, and add them to the database.
            if rust_graph[path.end_node].is_root() {
                // The path ends at the root node, so add any partial path whose symbol stack
                // precondition is satisfied by the path.
                storage_layer.add_to_database(graph.graph, partials, db, |partial_path| {
                    if partial_path.start_node != path.end_node {
                        return false;
                    }

                    let mut symbol_bindings = SymbolStackBindings::new();
                    let mut scope_bindings = ScopeStackBindings::new();
                    partial_path
                        .symbol_stack_precondition
                        .match_stack(
                            rust_graph,
                            rust_paths,
                            rust_partials,
                            path.symbol_stack,
                            &mut symbol_bindings,
                            &mut scope_bindings,
                        )
                        .is_ok()
                });
            } else {
                // The path ends at a non-root node, so add any partial path that starts at the
                // path's end node.
                storage_layer.add_to_database(graph.graph, partials, db, |partial_path| {
                    partial_path.start_node == path.end_node
                });
            }
        }

        // And then kick off the next phase!
        eprintln!("==> Starting next phase");
        sg_forward_path_stitcher_process_next_phase(graph.graph, paths, partials, db, stitcher);
    }
    eprintln!("==> Path stitching done");

    // And finally verify that we found all of the paths that we expected to.
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<HashSet<_>>();
    assert_eq!(results, expected_paths);

    sg_forward_path_stitcher_free(stitcher);
    sg_partial_path_database_free(db);
    sg_partial_path_arena_free(partials);
    sg_path_arena_free(paths);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_jump_to_definition(
        &graph,
        "main.py",
        &[
            // reference to `a` in import statement
            "[main.py(17) reference a] -> [a.py(0) definition a]",
            // reference to `b` in import statement
            "[main.py(15) reference b] -> [b.py(0) definition b]",
            // reference to `foo` in function call resolves to function definition
            "[main.py(13) reference foo] -> [a.py(5) definition foo]",
            // reference to `A` as function parameter resolves to class definition
            "[main.py(9) reference A] -> [b.py(5) definition A]",
            // reference to `bar` on result flows through body of `foo` to find `A.bar`
            "[main.py(10) reference bar] -> [b.py(8) definition bar]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "a.py",
        &[
            // reference to `x` in function body resolves to formal parameter
            "[a.py(8) reference x] -> [a.py(14) definition x]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "b.py",
        &[
            // no references in b.py, so no paths
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_jump_to_definition(
        &graph,
        "main.py",
        &[
            // reference to `a` in import statement
            "[main.py(8) reference a] -> [a.py(0) definition a]",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "[main.py(6) reference foo] -> [b.py(6) definition foo]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "a.py",
        &[
            // reference to `b` in import statement
            "[a.py(6) reference b] -> [b.py(0) definition b]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "b.py",
        &[
            // reference to `a` in import statement
            "[b.py(8) reference a] -> [a.py(0) definition a]",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_jump_to_definition(
        &graph,
        "test.rs",
        &[
            // reference to `a` in `a::FOO` resolves to module definition
            "[test.rs(103) reference a] -> [test.rs(201) definition a]",
            // reference to `a::FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
            "[test.rs(101) reference FOO] -> [test.rs(304) definition FOO]",
            "[test.rs(101) reference FOO] -> [test.rs(204) definition BAR]",
            // reference to `b` in use statement resolves to module definition
            "[test.rs(206) reference b] -> [test.rs(301) definition b]",
            // reference to `a` in use statement resolves to module definition
            "[test.rs(307) reference a] -> [test.rs(201) definition a]",
            // reference to `BAR` in module `b` can _only_ resolve to `a::BAR`
            "[test.rs(305) reference BAR] -> [test.rs(204) definition BAR]",
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_jump_to_definition(
        &graph,
        "main.py",
        &[
            // reference to `a` in import statement
            "[main.py(8) reference a] -> [a.py(0) definition a]",
            // reference to `foo` resolves through intermediate file to find `b.foo`
            "[main.py(6) reference foo] -> [b.py(5) definition foo]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "a.py",
        &[
            // reference to `b` in import statement
            "[a.py(6) reference b] -> [b.py(0) definition b]",
        ],
    );
    check_jump_to_definition(
        &graph,
        "b.py",
        &[
            // no references in b.py, so no paths
        ],
    );
}
