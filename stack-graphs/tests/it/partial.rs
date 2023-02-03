// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use stack_graphs::arena::Handle;
use stack_graphs::graph::Edge;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPathEdgeList;
use stack_graphs::partial::PartialPaths;
use stack_graphs::partial::PartialScopeStack;
use stack_graphs::partial::PartialScopeStackBindings;
use stack_graphs::partial::PartialScopedSymbol;
use stack_graphs::partial::PartialSymbolStack;
use stack_graphs::partial::PartialSymbolStackBindings;
use stack_graphs::partial::ScopeStackVariable;
use stack_graphs::partial::SymbolStackVariable;
use stack_graphs::paths::PathResolutionError;
use stack_graphs::stitching::Database;

type NiceSymbolStack<'a> = (&'a [NiceScopedSymbol<'a>], Option<SymbolStackVariable>);
type NiceScopedSymbol<'a> = (&'a str, Option<NiceScopeStack<'a>>);
type NiceScopeStack<'a> = (&'a [u32], Option<ScopeStackVariable>);
type NicePartialPath<'a> = &'a [Handle<Node>];

fn create_symbol_stack(
    graph: &mut StackGraph,
    partials: &mut PartialPaths,
    contents: NiceSymbolStack,
) -> PartialSymbolStack {
    let mut stack = if let Some(var) = contents.1 {
        PartialSymbolStack::from_variable(var)
    } else {
        PartialSymbolStack::empty()
    };
    for scoped_symbol in contents.0 {
        let symbol = graph.add_symbol(scoped_symbol.0);
        let scopes = scoped_symbol
            .1
            .map(|scopes| create_scope_stack(graph, partials, scopes));
        let scoped_symbol = PartialScopedSymbol {
            symbol,
            scopes: ControlledOption::from_option(scopes),
        };
        stack.push_back(partials, scoped_symbol);
    }
    stack
}

#[test]
fn will_skip_divergent_partial_paths() {
    let mut graph = StackGraph::new();
    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    let start_node = StackGraph::root_node();
    let end_node = StackGraph::root_node();
    let symbol_stack_precondition = create_symbol_stack(&mut graph, &mut partials, (&[], None));
    let symbol_stack_postcondition =
        create_symbol_stack(&mut graph, &mut partials, (&[("a", None)], None));
    let variable = ScopeStackVariable::new(1).unwrap();
    let scope_stack_precondition = PartialScopeStack::from_variable(variable);
    let scope_stack_postcondition = PartialScopeStack::from_variable(variable);
    let edges = PartialPathEdgeList::empty();
    let partial_path = PartialPath {
        start_node,
        end_node,
        symbol_stack_precondition,
        symbol_stack_postcondition,
        scope_stack_precondition,
        scope_stack_postcondition,
        edges,
    };
    db.add_partial_path(&graph, &mut partials, partial_path);
}

#[test]
fn can_apply_offset_to_partial_symbol_stacks() {
    fn verify(
        stack: NiceSymbolStack,
        symbol_variable_offset: u32,
        scope_variable_offset: u32,
        expected: &str,
    ) {
        let mut graph = StackGraph::new();
        let mut partials = PartialPaths::new();
        let stack = create_symbol_stack(&mut graph, &mut partials, stack);
        let with_offset =
            stack.with_offset(&mut partials, symbol_variable_offset, scope_variable_offset);
        let actual = with_offset.display(&graph, &mut partials).to_string();
        assert_eq!(actual, expected);
    }

    verify((&[], None), 0, 0, "");
    verify((&[], None), 0, 1, "");
    verify((&[], None), 1, 0, "");
    verify((&[], None), 1, 1, "");

    let a = ("a", None);
    verify((&[a], None), 0, 0, "a");
    verify((&[a], None), 0, 1, "a");
    verify((&[a], None), 1, 0, "a");
    verify((&[a], None), 1, 1, "a");

    let var1 = Some(SymbolStackVariable::new(1).unwrap());
    verify((&[a], var1), 0, 0, "a,%1");
    verify((&[a], var1), 0, 1, "a,%1");
    verify((&[a], var1), 1, 0, "a,%2");
    verify((&[a], var1), 1, 1, "a,%2");

    let empty_scopes: NiceScopeStack = (&[], None);
    let a_empty = ("a", Some(empty_scopes));
    verify((&[a_empty], None), 0, 0, "a/()");
    verify((&[a_empty], None), 0, 1, "a/()");
    verify((&[a_empty], None), 1, 0, "a/()");
    verify((&[a_empty], None), 1, 1, "a/()");
    verify((&[a_empty], var1), 0, 0, "a/(),%1");
    verify((&[a_empty], var1), 0, 1, "a/(),%1");
    verify((&[a_empty], var1), 1, 0, "a/(),%2");
    verify((&[a_empty], var1), 1, 1, "a/(),%2");

    let scope_var1 = Some(ScopeStackVariable::new(1).unwrap());
    let scopes_var1: NiceScopeStack = (&[], scope_var1);
    let a_var1 = ("a", Some(scopes_var1));
    verify((&[a_var1], None), 0, 0, "a/($1)");
    verify((&[a_var1], None), 0, 1, "a/($2)");
    verify((&[a_var1], None), 1, 0, "a/($1)");
    verify((&[a_var1], None), 1, 1, "a/($2)");
    verify((&[a_var1], var1), 0, 0, "a/($1),%1");
    verify((&[a_var1], var1), 0, 1, "a/($2),%1");
    verify((&[a_var1], var1), 1, 0, "a/($1),%2");
    verify((&[a_var1], var1), 1, 1, "a/($2),%2");
}

#[test]
fn can_unify_partial_symbol_stacks() -> Result<(), PathResolutionError> {
    fn verify(
        lhs: NiceSymbolStack,
        rhs: NiceSymbolStack,
        expected_unification: &str,
        expected_symbol_bindings: &str,
        expected_scope_bindings: &str,
    ) -> Result<(), PathResolutionError> {
        let mut graph = StackGraph::new();
        let mut partials = PartialPaths::new();
        let lhs = create_symbol_stack(&mut graph, &mut partials, lhs);
        let rhs = create_symbol_stack(&mut graph, &mut partials, rhs);
        let mut symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        let unified = lhs.unify(
            &mut partials,
            rhs,
            &mut symbol_bindings,
            &mut scope_bindings,
        )?;
        let unified = unified.display(&graph, &mut partials).to_string();
        assert_eq!(unified, expected_unification);
        let symbol_bindings = symbol_bindings.display(&graph, &mut partials).to_string();
        assert_eq!(symbol_bindings, expected_symbol_bindings);
        let scope_bindings = scope_bindings.display(&graph, &mut partials).to_string();
        assert_eq!(scope_bindings, expected_scope_bindings);
        Ok(())
    }

    fn verify_not(lhs: NiceSymbolStack, rhs: NiceSymbolStack) -> Result<(), PathResolutionError> {
        let mut graph = StackGraph::new();
        let mut partials = PartialPaths::new();
        let lhs = create_symbol_stack(&mut graph, &mut partials, lhs);
        let rhs = create_symbol_stack(&mut graph, &mut partials, rhs);
        let mut symbol_bindings = PartialSymbolStackBindings::new();
        let mut scope_bindings = PartialScopeStackBindings::new();
        assert!(lhs
            .unify(
                &mut partials,
                rhs,
                &mut symbol_bindings,
                &mut scope_bindings
            )
            .is_err());
        Ok(())
    }

    let var1 = Some(SymbolStackVariable::new(1).unwrap());
    let var2 = Some(SymbolStackVariable::new(2).unwrap());
    let a = ("a", None);

    verify((&[], None), (&[], None), "", "{}", "{}")?;
    verify((&[], var1), (&[], None), "%1", "{%1 => <>}", "{}")?;
    verify((&[], None), (&[], var2), "%2", "{%2 => <>}", "{}")?;
    verify((&[], var1), (&[], var2), "%1", "{%2 => <%1>}", "{}")?;

    verify_not(
        (&[], None), //
        (&[a], None),
    )?;
    verify(
        (&[], var1), //
        (&[a], None),
        "a",
        "{%1 => <a>}",
        "{}",
    )?;
    verify_not(
        (&[], None), //
        (&[a], var2),
    )?;
    verify(
        (&[], var1), //
        (&[a], var2),
        "a,%2",
        "{%1 => <a,%2>}",
        "{}",
    )?;

    verify_not(
        (&[a], None), //
        (&[], None),
    )?;
    verify_not(
        (&[a], var1), //
        (&[], None),
    )?;
    verify(
        (&[a], None), //
        (&[], var2),
        "a",
        "{%2 => <a>}",
        "{}",
    )?;
    verify(
        (&[a], var1), //
        (&[], var2),
        "a,%1",
        "{%2 => <a,%1>}",
        "{}",
    )?;

    verify(
        (&[a], None), //
        (&[a], None),
        "a",
        "{}",
        "{}",
    )?;
    verify(
        (&[a], var1), //
        (&[a], None),
        "a,%1",
        "{%1 => <>}",
        "{}",
    )?;
    verify(
        (&[a], None), //
        (&[a], var2),
        "a,%2",
        "{%2 => <>}",
        "{}",
    )?;
    verify(
        (&[a], var1), //
        (&[a], var2),
        "a,%1",
        "{%2 => <%1>}",
        "{}",
    )?;

    let dot = (".", None);
    let b = ("b", None);

    verify(
        (&[a, dot, b], None),
        (&[a, dot, b], None),
        "a.b",
        "{}",
        "{}",
    )?;
    verify(
        (&[a, dot, b], var1),
        (&[a, dot, b], None),
        "a.b,%1",
        "{%1 => <>}",
        "{}",
    )?;
    verify(
        (&[a, dot, b], None),
        (&[a, dot, b], var2),
        "a.b,%2",
        "{%2 => <>}",
        "{}",
    )?;
    verify(
        (&[a, dot, b], var1),
        (&[a, dot, b], var2),
        "a.b,%1",
        "{%2 => <%1>}",
        "{}",
    )?;

    verify_not(
        (&[a], None), //
        (&[a, dot, b], None),
    )?;
    verify_not(
        (&[a], None), //
        (&[a, dot, b], var2),
    )?;
    verify(
        (&[a], var1), //
        (&[a, dot, b], None),
        "a.b",
        "{%1 => <.b>}",
        "{}",
    )?;
    verify(
        (&[a], var1), //
        (&[a, dot, b], var2),
        "a.b,%2",
        "{%1 => <.b,%2>}",
        "{}",
    )?;

    let empty_scopes: NiceScopeStack = (&[], None);
    let a_empty = ("a", Some(empty_scopes));

    let scope_var1 = Some(ScopeStackVariable::new(1).unwrap());
    let scopes_var1: NiceScopeStack = (&[], scope_var1);
    let a_var1 = ("a", Some(scopes_var1));

    verify_not((&[a], None), (&[a_empty], None))?;
    verify_not((&[a_empty], None), (&[a], None))?;

    verify_not((&[a], None), (&[a_var1], None))?;
    verify_not((&[a_var1], None), (&[a], None))?;

    verify(
        (&[a_empty], None),
        (&[a_var1], None),
        "a/($1)",
        "{}",
        "{$1 => ()}",
    )?;
    verify(
        (&[a_var1], None),
        (&[a_empty], None),
        "a/($1)",
        "{}",
        "{$1 => ()}",
    )?;

    Ok(())
}

fn create_scope_stack(
    graph: &mut StackGraph,
    partials: &mut PartialPaths,
    contents: NiceScopeStack,
) -> PartialScopeStack {
    let file = graph.get_or_create_file("file");
    let mut stack = if let Some(var) = contents.1 {
        PartialScopeStack::from_variable(var)
    } else {
        PartialScopeStack::empty()
    };
    for scope in contents.0 {
        let node_id = NodeID::new_in_file(file, *scope);
        let node = match graph.node_for_id(node_id) {
            Some(node) => node,
            None => graph.add_scope_node(node_id, true).unwrap(),
        };
        stack.push_back(partials, node);
    }
    stack
}

#[test]
fn can_unify_partial_scope_stacks() -> Result<(), PathResolutionError> {
    fn verify(
        lhs: NiceScopeStack,
        rhs: NiceScopeStack,
        expected_unification: &str,
        expected_bindings: &str,
    ) -> Result<(), PathResolutionError> {
        let mut graph = StackGraph::new();
        let mut partials = PartialPaths::new();
        let lhs = create_scope_stack(&mut graph, &mut partials, lhs);
        let rhs = create_scope_stack(&mut graph, &mut partials, rhs);
        let mut bindings = PartialScopeStackBindings::new();
        let unified = lhs.unify(&mut partials, rhs, &mut bindings)?;
        let unified = unified.display(&graph, &mut partials).to_string();
        assert_eq!(unified, expected_unification);
        let bindings = bindings.display(&graph, &mut partials).to_string();
        assert_eq!(bindings, expected_bindings);
        Ok(())
    }

    fn verify_not(lhs: NiceScopeStack, rhs: NiceScopeStack) -> Result<(), PathResolutionError> {
        let mut graph = StackGraph::new();
        let mut partials = PartialPaths::new();
        let lhs = create_scope_stack(&mut graph, &mut partials, lhs);
        let rhs = create_scope_stack(&mut graph, &mut partials, rhs);
        let mut bindings = PartialScopeStackBindings::new();
        assert!(lhs.unify(&mut partials, rhs, &mut bindings).is_err());
        Ok(())
    }

    let var1 = Some(ScopeStackVariable::new(1).unwrap());
    let var2 = Some(ScopeStackVariable::new(2).unwrap());

    verify((&[], None), (&[], None), "", "{}")?;
    verify((&[], var1), (&[], None), "$1", "{$1 => ()}")?;
    verify((&[], None), (&[], var2), "$2", "{$2 => ()}")?;
    verify((&[], var1), (&[], var2), "$1", "{$2 => ($1)}")?;

    verify_not(
        (&[], None), //
        (&[10], None),
    )?;
    verify(
        (&[], var1), //
        (&[10], None),
        "[file(10)]",
        "{$1 => ([file(10)])}",
    )?;
    verify_not(
        (&[], None), //
        (&[10], var2),
    )?;
    verify(
        (&[], var1), //
        (&[10], var2),
        "[file(10)],$2",
        "{$1 => ([file(10)],$2)}",
    )?;

    verify_not(
        (&[10], None), //
        (&[], None),
    )?;
    verify_not(
        (&[10], var1), //
        (&[], None),
    )?;
    verify(
        (&[10], None), //
        (&[], var2),
        "[file(10)]",
        "{$2 => ([file(10)])}",
    )?;
    verify(
        (&[10], var1), //
        (&[], var2),
        "[file(10)],$1",
        "{$2 => ([file(10)],$1)}",
    )?;

    verify(
        (&[10], None), //
        (&[10], None),
        "[file(10)]",
        "{}",
    )?;
    verify(
        (&[10], var1), //
        (&[10], None),
        "[file(10)],$1",
        "{$1 => ()}",
    )?;
    verify(
        (&[10], None), //
        (&[10], var2),
        "[file(10)],$2",
        "{$2 => ()}",
    )?;
    verify(
        (&[10], var1), //
        (&[10], var2),
        "[file(10)],$1",
        "{$2 => ($1)}",
    )?;

    Ok(())
}

#[test]
fn can_create_partial_path_from_node() {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").expect("");
    let foo = graph.add_symbol("foo");
    let drop_scopes_node = graph
        .add_drop_scopes_node(NodeID::new_in_file(file, 0))
        .unwrap();
    let jump_to_scope_node = StackGraph::jump_to_node();
    let pop_symbol_node = graph
        .add_pop_symbol_node(NodeID::new_in_file(file, 1), foo, false)
        .unwrap();
    let pop_scoped_symbol_node = graph
        .add_pop_scoped_symbol_node(NodeID::new_in_file(file, 2), foo, false)
        .unwrap();
    let push_symbol_node = graph
        .add_push_symbol_node(NodeID::new_in_file(file, 3), foo, false)
        .unwrap();
    let exported_scope_id = NodeID::new_in_file(file, 99);
    graph.add_scope_node(exported_scope_id, true);
    let push_scoped_symbol_node = graph
        .add_push_scoped_symbol_node(NodeID::new_in_file(file, 4), foo, exported_scope_id, false)
        .unwrap();
    let root_node = StackGraph::root_node();
    let scope_node = graph
        .add_scope_node(NodeID::new_in_file(file, 5), false)
        .unwrap();

    fn verify(graph: &StackGraph, node: Handle<Node>, expected: &str) {
        let mut partials = PartialPaths::new();
        let path = PartialPath::from_node(graph, &mut partials, node);
        let actual = path.display(&graph, &mut partials).to_string();
        assert_eq!(actual, expected);
    }

    verify(
        &graph,
        drop_scopes_node,
        "<%1> ($1) [test(0) drop scopes] -> [test(0) drop scopes] <%1> ()",
    );

    verify(
        &graph,
        jump_to_scope_node,
        "<%1> ($1) [jump to scope] -> [jump to scope] <%1> ($1)",
    );

    verify(
        &graph,
        pop_symbol_node,
        "<foo,%1> ($1) [test(1) pop foo] -> [test(1) pop foo] <%1> ($1)",
    );

    verify(
        &graph,
        pop_scoped_symbol_node,
        "<foo/($2),%1> ($1) [test(2) pop scoped foo] -> [test(2) pop scoped foo] <%1> ($2)",
    );

    verify(
        &graph,
        push_symbol_node,
        "<%1> ($1) [test(3) push foo] -> [test(3) push foo] <foo,%1> ($1)",
    );

    verify(
        &graph,
        push_scoped_symbol_node,
        "<%1> ($1) [test(4) push scoped foo test(99)] -> [test(4) push scoped foo test(99)] <foo/([test(99)],$1),%1> ($1)",
    );

    verify(&graph, root_node, "<%1> ($1) [root] -> [root] <%1> ($1)");

    verify(
        &graph,
        scope_node,
        "<%1> ($1) [test(5) scope] -> [test(5) scope] <%1> ($1)",
    );
}

#[test]
fn can_concatenate_partial_paths() -> Result<(), PathResolutionError> {
    let mut graph = StackGraph::new();
    let file = graph.add_file("test").expect("");

    let jump_to_scope_node = StackGraph::jump_to_node();

    let scope0_id = graph.new_node_id(file);
    let scope0 = graph.add_scope_node(scope0_id, false).unwrap();

    let scope1_id = graph.new_node_id(file);
    let scope1 = graph.add_scope_node(scope1_id, false).unwrap();

    let foo = graph.add_symbol("foo");
    let foo_ref_id = graph.new_node_id(file);
    let foo_ref = graph.add_push_symbol_node(foo_ref_id, foo, false).unwrap();
    let foo_def_id = graph.new_node_id(file);
    let foo_def = graph.add_pop_symbol_node(foo_def_id, foo, false).unwrap();

    let bar = graph.add_symbol("bar");
    let bar_ref_id = graph.new_node_id(file);
    let bar_ref = graph.add_push_symbol_node(bar_ref_id, bar, false).unwrap();
    let bar_def_id = graph.new_node_id(file);
    let bar_def = graph.add_pop_symbol_node(bar_def_id, bar, false).unwrap();

    let exported_scope_id = graph.new_node_id(file);
    graph.add_scope_node(exported_scope_id, true);
    let baz = graph.add_symbol("baz");
    let baz_ref_id = graph.new_node_id(file);
    let baz_ref = graph
        .add_push_scoped_symbol_node(baz_ref_id, baz, exported_scope_id, false)
        .unwrap();
    let baz_def_id = graph.new_node_id(file);
    let baz_def = graph
        .add_pop_scoped_symbol_node(baz_def_id, baz, false)
        .unwrap();

    let drop_scopes_id = graph.new_node_id(file);
    let drop_scopes = graph.add_drop_scopes_node(drop_scopes_id).unwrap();

    fn run(
        graph: &StackGraph,
        left: NicePartialPath,
        right: NicePartialPath,
        expected: &str,
    ) -> Result<(), PathResolutionError> {
        let mut g = StackGraph::new();
        g.add_from_graph(graph).expect("");

        let mut ps = PartialPaths::new();

        let mut lns = left.iter();
        let mut prev = lns.next().unwrap();
        let mut l = PartialPath::from_node(&g, &mut ps, *prev);
        for next in lns {
            g.add_edge(*prev, *next, 0);
            l.append(
                &g,
                &mut ps,
                Edge {
                    source: *prev,
                    sink: *next,
                    precedence: 0,
                },
            )
            .expect("");
            prev = next;
        }

        let mut rns = right.iter();
        let mut prev = rns.next().unwrap();
        let mut r = PartialPath::from_node(&g, &mut ps, *prev);
        for next in rns {
            g.add_edge(*prev, *next, 0);
            r.append(
                &g,
                &mut ps,
                Edge {
                    source: *prev,
                    sink: *next,
                    precedence: 0,
                },
            )
            .expect("");
            prev = next;
        }

        r.ensure_no_overlapping_variables(&mut ps, &l);
        l.concatenate(&g, &mut ps, &r)?;
        let actual = l.display(&g, &mut ps).to_string();
        assert_eq!(actual, expected);

        Ok(())
    }

    fn verify(graph: &StackGraph, left: NicePartialPath, right: NicePartialPath, expected: &str) {
        run(graph, left, right, expected).expect("");
    }

    fn verify_not(graph: &StackGraph, left: NicePartialPath, right: NicePartialPath) {
        run(graph, left, right, "").expect_err("");
    }

    verify(
        &graph,
        &[scope0],
        &[scope0],
        "<%1> ($1) [test(0) scope] -> [test(0) scope] <%1> ($1)",
    );

    verify_not(&graph, &[scope0], &[scope1]);

    verify(
        &graph,
        &[foo_ref, scope0],
        &[scope0, foo_def],
        "<%1> ($1) [test(2) push foo] -> [test(3) pop foo] <%1> ($1)",
    );

    verify(
        &graph,
        &[foo_ref, scope0],
        &[scope0, scope1],
        "<%1> ($1) [test(2) push foo] -> [test(1) scope] <foo,%1> ($1)",
    );

    verify(
        &graph,
        &[scope0, scope1],
        &[scope1, foo_ref],
        "<%1> ($1) [test(0) scope] -> [test(2) push foo] <foo,%1> ($1)",
    );

    verify(
        &graph,
        &[foo_def, scope0],
        &[scope0, bar_ref],
        "<foo,%1> ($1) [test(3) pop foo] -> [test(4) push bar] <bar,%1> ($1)",
    );

    verify(
        &graph,
        &[foo_ref, scope0, foo_def],
        &[foo_def, scope1, bar_ref],
        "<%1> ($1) [test(2) push foo] -> [test(4) push bar] <bar,%1> ($1)",
    );

    verify(
        &graph,
        &[foo_def, scope0, bar_ref],
        &[bar_ref, scope1, bar_def],
        "<foo,%1> ($1) [test(3) pop foo] -> [test(5) pop bar] <%1> ($1)",
    );

    verify(
        &graph,
        &[baz_ref, scope0, baz_def],
        &[baz_def, scope1, bar_ref],
        "<%1> ($1) [test(7) push scoped baz test(6)] -> [test(4) push bar] <bar,%1> ([test(6)],$1)",
    );

    verify(
        &graph,
        &[foo_def, scope0, baz_ref],
        &[baz_ref, scope1, baz_def],
        "<foo,%1> ($1) [test(3) pop foo] -> [test(8) pop scoped baz] <%1> ([test(6)],$1)",
    );

    verify(
        &graph,
        &[scope0, drop_scopes],
        &[drop_scopes, scope1],
        "<%1> ($1) [test(0) scope] -> [test(1) scope] <%1> ()",
    );

    verify_not(
        &graph,
        &[scope0, drop_scopes, scope1],
        &[scope1, jump_to_scope_node],
    );

    verify(
        &graph,
        &[baz_def, scope0],
        &[scope0, jump_to_scope_node],
        "<baz/($2),%1> ($1) [test(8) pop scoped baz] -> [jump to scope] <%1> ($2)",
    );

    verify(
        &graph,
        &[baz_ref, scope0],
        &[scope0, jump_to_scope_node],
        "<%1> ($1) [test(7) push scoped baz test(6)] -> [jump to scope] <baz/([test(6)],$1),%1> ($1)",
    );

    Ok(())
}
