// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::partial::PartialScopeStack;
use stack_graphs::partial::PartialScopeStackBindings;
use stack_graphs::partial::ScopeStackVariable;
use stack_graphs::paths::PathResolutionError;

fn create_scope_stack(
    graph: &mut StackGraph,
    partials: &mut PartialPaths,
    contents: (&[u32], Option<ScopeStackVariable>),
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
            None => graph.add_exported_scope_node(node_id).unwrap(),
        };
        stack.push_back(partials, node);
    }
    stack
}

#[test]
fn can_unify_partial_scope_stacks() -> Result<(), PathResolutionError> {
    fn verify(
        lhs: (&[u32], Option<ScopeStackVariable>),
        rhs: (&[u32], Option<ScopeStackVariable>),
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

    fn verify_not(
        lhs: (&[u32], Option<ScopeStackVariable>),
        rhs: (&[u32], Option<ScopeStackVariable>),
    ) -> Result<(), PathResolutionError> {
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
