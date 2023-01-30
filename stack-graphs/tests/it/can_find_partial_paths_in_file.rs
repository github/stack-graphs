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
use stack_graphs::NoCancellation;

use crate::test_graphs;

fn check_partial_paths_in_file(graph: &StackGraph, file: &str, expected_paths: &[&str]) {
    let file = graph.get_file_unchecked(file);
    let mut partials = PartialPaths::new();
    let mut results = BTreeSet::new();
    partials
        .find_all_partial_paths_in_file(graph, file, &NoCancellation, |graph, partials, path| {
            results.insert(path.display(graph, partials).to_string());
        })
        .expect("should never be cancelled");
    let expected_paths = expected_paths
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_paths, results);
}

#[test]
fn class_field_through_function_parameter() {
    let graph = test_graphs::class_field_through_function_parameter::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> ($1) [main.py(17) reference a] -> [root] <a,%1> ($1)",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `b` in import statement
            "<%1> ($1) [main.py(15) reference b] -> [root] <b,%1> ($1)",
            // `from b import *` means we can rewrite any lookup of `__main__.*` → `b.*`
            "<__main__.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
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
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // definition of `foo` function
            "<a.foo,%1> ($1) [root] -> [a.py(5) definition foo] <%1> ($1)",
            // reference to `x` in function body can resolve to formal parameter, which might get formal parameters...
            "<%1> ($1) [a.py(8) reference x] -> [a.py(14) definition x] <%1> ()",
            // ...which we can look up either the 0th actual positional parameter...
            "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <0,%1> ($1)",
            // ...or the actual named parameter `x`
            "<%1> ($1) [a.py(8) reference x] -> [jump to scope] <x,%1> ($1)",
            // result of function is `x`, which is passed in as a formal parameter...
            "<a.foo()/($2),%1> ($1) [root] -> [a.py(14) definition x] <%1> ()",
            // ...which we can look up either the 0th actual positional parameter...
            "<a.foo()/($2),%1> ($1) [root] -> [jump to scope] <0,%1> ($2)",
            // ...or the actual named parameter `x`
            "<a.foo()/($2),%1> ($1) [root] -> [jump to scope] <x,%1> ($2)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // definition of class `A`
            "<b.A,%1> ($1) [root] -> [b.py(5) definition A] <%1> ($1)",
            // definition of class member `A.bar`
            "<b.A.bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($1)",
            // `bar` can also be accessed as an instance member
            "<b.A()/($2).bar,%1> ($1) [root] -> [b.py(8) definition bar] <%1> ($2)",
        ],
    );
}

#[test]
fn cyclic_imports_python() {
    let graph = test_graphs::cyclic_imports_python::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> ($1) [main.py(8) reference a] -> [root] <a,%1> ($1)",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `foo` becomes `a.foo` because of import statement
            "<%1> ($1) [main.py(6) reference foo] -> [root] <a.foo,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // reference to `b` in import statement
            "<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)",
            // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> ($1) [b.py(8) reference a] -> [root] <a,%1> ($1)",
            // `from a import *` means we can rewrite any lookup of `b.*` → `a.*`
            "<b.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // definition of `foo`
            "<b.foo,%1> ($1) [root] -> [b.py(6) definition foo] <%1> ($1)",
        ],
    );
}

#[test]
fn cyclic_imports_rust() {
    let graph = test_graphs::cyclic_imports_rust::new();
    check_partial_paths_in_file(
        &graph,
        "test.rs",
        // NOTE: Because everything in this example is local to one file, there aren't any partial
        // paths involving the root node.
        &[
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
        ],
    );
}

#[test]
fn sequenced_import_star() {
    let graph = test_graphs::sequenced_import_star::new();
    check_partial_paths_in_file(
        &graph,
        "main.py",
        &[
            // definition of `__main__` module
            "<__main__,%1> ($1) [root] -> [main.py(0) definition __main__] <%1> ($1)",
            // reference to `a` in import statement
            "<%1> ($1) [main.py(8) reference a] -> [root] <a,%1> ($1)",
            // `from a import *` means we can rewrite any lookup of `__main__.*` → `a.*`
            "<__main__.,%1> ($1) [root] -> [root] <a.,%1> ($1)",
            // reference to `foo` becomes `a.foo` because of import statement
            "<%1> ($1) [main.py(6) reference foo] -> [root] <a.foo,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "a.py",
        &[
            // definition of `a` module
            "<a,%1> ($1) [root] -> [a.py(0) definition a] <%1> ($1)",
            // reference to `b` in import statement
            "<%1> ($1) [a.py(6) reference b] -> [root] <b,%1> ($1)",
            // `from b import *` means we can rewrite any lookup of `a.*` → `b.*`
            "<a.,%1> ($1) [root] -> [root] <b.,%1> ($1)",
        ],
    );
    check_partial_paths_in_file(
        &graph,
        "b.py",
        &[
            // definition of `b` module
            "<b,%1> ($1) [root] -> [b.py(0) definition b] <%1> ($1)",
            // definition of `foo` inside of `b` module
            "<b.foo,%1> ($1) [root] -> [b.py(5) definition foo] <%1> ($1)",
        ],
    );
}
