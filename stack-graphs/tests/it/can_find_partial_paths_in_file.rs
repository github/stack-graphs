// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use stack_graphs::NoCancellation;

use crate::test_graphs;

fn check_partial_paths_in_file(graph: &StackGraph, file: &str, expected_paths: &[&str]) {
    let file = graph.get_file_unchecked(file);
    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    partials
        .find_minimal_partial_paths_set_in_file(
            graph,
            file,
            &NoCancellation,
            |graph, partials, path| {
                db.add_partial_path(graph, partials, path);
            },
        )
        .expect("should never be cancelled");
    let mut results = BTreeSet::new();
    #[allow(deprecated)]
    ForwardPartialPathStitcher::find_locally_complete_partial_paths(
        graph,
        &mut partials,
        &mut db,
        &NoCancellation,
        |g, ps, p| {
            results.insert(p.display(g, ps).to_string());
        },
    )
    .expect("should never be cancelled");
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_paths, results, "failed in file {}", graph[file]);
}

pub(crate) static CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_MAIN_PATHS: &[&str] = &[
    // definition of `__main__` module
    "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
    // reference to `a` in import statement
    "<%1> ($1) [main.py(17) reference a] -> [root] <a,%1> ($1)",
    // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
    "<__main__.,%2> ($1) [root] -> [root] <a.,%2> ($1)",
    // reference to `b` in import statement
    "<%1> ($1) [main.py(15) reference b] -> [root] <b,%1> ($1)",
    // `from b import *` means we can rewrite any lookup of `__main__.*` → `b.*`
    "<__main__.,%2> ($1) [root] -> [root] <b.,%2> ($1)",
    // we can look for every reference in either `a` or `b`
    "<%1> ($1) [main.py(9) reference A] -> [root] <a.A,%1> ($1)",
    "<%1> ($1) [main.py(9) reference A] -> [root] <b.A,%1> ($1)",
    "<%1> ($1) [main.py(10) reference bar] -> [root] <a.foo()/([main.py(7)],$1).bar,%1> ($1)",
    "<%1> ($1) [main.py(10) reference bar] -> [root] <b.foo()/([main.py(7)],$1).bar,%1> ($1)",
    "<%1> ($1) [main.py(13) reference foo] -> [root] <a.foo,%1> ($1)",
    "<%1> ($1) [main.py(13) reference foo] -> [root] <b.foo,%1> ($1)",
    // parameter 0 of function call is `A`, which we can look up in either `a` or `b`
    "<0,%1> ($1) [main.py(7) exported scope] -> [root] <a.A,%1> ($1)",
    "<0,%1> ($1) [main.py(7) exported scope] -> [root] <b.A,%1> ($1)",
];

pub(crate) static CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_A_PATHS: &[&str] = &[
    // definition of `a` module
    "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
    // definition of `foo` function
    "<a.foo,%2> ($1) [root] -> [a.py(5) definition foo] <%2> ($1)",
    // reference to `x` in function body can resolve to formal parameter, which might get formal parameters...
    "<%1> ($1) [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
    // ...which we can look up either the 0th actual positional parameter...
    "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <0,%1> ($1)",
    // ...or the actual named parameter `x`
    "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <x,%1> ($1)",
    // result of function is `x`, which is passed in as a formal parameter...
    "<a.foo()/($3),%3> ($1) [root] -> [a.py(14) definition x] <%3> ()",
    // ...which we can look up either the 0th actual positional parameter...
    "<a.foo()/($3),%3> ($1) [root] -> [jump to scope] <0,%3> ($3)",
    // ...or the actual named parameter `x`
    "<a.foo()/($3),%3> ($1) [root] -> [jump to scope] <x,%3> ($3)",
];

pub(crate) static CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_B_PATHS: &[&str] = &[
    // definition of `b` module
    "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
    // definition of class `A`
    "<b.A,%2> ($1) [root] -> [b.py(5) definition A] <%2> ($1)",
    // definition of class member `A.bar`
    "<b.A.bar,%3> ($1) [root] -> [b.py(8) definition bar] <%3> ($1)",
    // `bar` can also be accessed as an instance member
    "<b.A()/($3).bar,%3> ($1) [root] -> [b.py(8) definition bar] <%3> ($3)",
];

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_MAIN_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_A_PATHS,
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        CLASS_FIELD_THROUGH_FUNCTION_PARAMETER_B_PATHS,
    );
}

pub(crate) const CYCLIC_IMPORTS_PYTHON_MAIN_PATHS: &[&str] = &[
    // definition of `__main__` module
    "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
    // reference to `a` in import statement
    "<%1> ($1) [main.py(8) reference a] -> [root] <a,%1> ($1)",
    // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
    "<__main__.,%2> ($1) [root] -> [root] <a.,%2> ($1)",
    // reference to `foo` becomes `a.foo` because of import statement
    "<%1> ($1) [main.py(6) reference foo] -> [root] <a.foo,%1> ($1)",
];

pub(crate) const CYCLIC_IMPORTS_PYTHON_A_PATHS: &[&str] = &[
    // definition of `a` module
    "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
    // reference to `b` in import statement
    "<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)",
    // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
    "<a.,%2> ($1) [root] -> [root] <b.,%2> ($1)",
];

pub(crate) const CYCLIC_IMPORTS_PYTHON_B_PATHS: &[&str] = &[
    // definition of `b` module
    "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
    // reference to `a` in import statement
    "<%1> ($1) [b.py(8) reference a] -> [root] <a,%1> ($1)",
    // `from a import *` means we can rewrite any lookup of `b.*` → `a.*`
    "<b.,%2> ($1) [root] -> [root] <a.,%2> ($1)",
    // definition of `foo`
    "<b.foo,%2> ($1) [root] -> [b.py(6) definition foo] <%2> ($1)",
];

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_partial_paths_in_file(&graph, "main.py", CYCLIC_IMPORTS_PYTHON_MAIN_PATHS);
    check_partial_paths_in_file(&graph, "a.py", CYCLIC_IMPORTS_PYTHON_A_PATHS);
    check_partial_paths_in_file(&graph, "b.py", CYCLIC_IMPORTS_PYTHON_B_PATHS);
}

// NOTE: Because everything in this example is local to one file, there aren't any partial
// paths involving the root node.
pub(crate) const CYCLIC_IMPORTS_RUST_PATHS: &[&str] = &[
    // reference to `a` in `main` function
    "<%1> ($1) [test.rs(103) reference a] -> [test.rs(201) definition a] <%1> ($1)",
    // reference to `a` in `b` function
    "<%1> ($1) [test.rs(307) reference a] -> [test.rs(201) definition a] <%1> ($1)",
    // reference to `b` in `a` function
    "<%1> ($1) [test.rs(206) reference b] -> [test.rs(301) definition b] <%1> ($1)",
    // reference to `FOO` in `main` can resolve either to `a::BAR` or `b::FOO`
    "<%1> ($1) [test.rs(101) reference FOO] -> [test.rs(204) definition BAR] <%1> ($1)",
    "<%1> ($1) [test.rs(101) reference FOO] -> [test.rs(304) definition FOO] <%1> ($1)",
    // reference to `BAR` in `b` resolves _only_ to `a::BAR`
    "<%1> ($1) [test.rs(305) reference BAR] -> [test.rs(204) definition BAR] <%1> ($1)",
];

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_partial_paths_in_file(&graph, "test.rs", CYCLIC_IMPORTS_RUST_PATHS);
}

pub(crate) const SEQUENCED_IMPORT_STAR_MAIN_PATHS: &[&str] = &[
    // definition of `__main__` module
    "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
    // reference to `a` in import statement
    "<%1> ($1) [main.py(8) reference a] -> [root] <a,%1> ($1)",
    // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
    "<__main__.,%2> ($1) [root] -> [root] <a.,%2> ($1)",
    // reference to `foo` becomes `a.foo` because of import statement
    "<%1> ($1) [main.py(6) reference foo] -> [root] <a.foo,%1> ($1)",
];

pub(crate) const SEQUENCED_IMPORT_STAR_A_PATHS: &[&str] = &[
    // definition of `a` module
    "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
    // reference to `b` in import statement
    "<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)",
    // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
    "<a.,%2> ($1) [root] -> [root] <b.,%2> ($1)",
];

pub(crate) const SEQUENCED_IMPORT_STAR_B_PATHS: &[&str] = &[
    // definition of `b` module
    "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
    // definition of `foo` inside of `b` module
    "<b.foo,%2> ($1) [root] -> [b.py(5) definition foo] <%2> ($1)",
];

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_partial_paths_in_file(&graph, "main.py", SEQUENCED_IMPORT_STAR_MAIN_PATHS);
    check_partial_paths_in_file(&graph, "a.py", SEQUENCED_IMPORT_STAR_A_PATHS);
    check_partial_paths_in_file(&graph, "b.py", SEQUENCED_IMPORT_STAR_B_PATHS);
}
