// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use stack_graphs::arena::Handle;
use stack_graphs::graph::Edge;
use stack_graphs::graph::File;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPath;
use stack_graphs::partial::PartialPaths;
use stack_graphs::partial::PartialScopeStack;
use stack_graphs::partial::PartialScopedSymbol;
use stack_graphs::partial::PartialSymbolStack;
use stack_graphs::partial::ScopeStackVariable;
use stack_graphs::partial::SymbolStackVariable;
use stack_graphs::paths::PathResolutionError;

pub(crate) type NiceSymbolStack<'a> = (&'a [NiceScopedSymbol<'a>], Option<SymbolStackVariable>);
pub(crate) type NiceScopedSymbol<'a> = (&'a str, Option<NiceScopeStack<'a>>);
pub(crate) type NiceScopeStack<'a> = (&'a [u32], Option<ScopeStackVariable>);
pub(crate) type NicePartialPath<'a> = &'a [Handle<Node>];

pub(crate) fn create_scope_node(
    graph: &mut StackGraph,
    file: Handle<File>,
    is_exported: bool,
) -> Handle<Node> {
    let id = graph.new_node_id(file);
    graph.add_scope_node(id, is_exported).unwrap()
}

pub(crate) fn create_push_symbol_node(
    graph: &mut StackGraph,
    file: Handle<File>,
    symbol: &str,
    is_reference: bool,
) -> Handle<Node> {
    let id = graph.new_node_id(file);
    let symbol = graph.add_symbol(symbol);
    graph
        .add_push_symbol_node(id, symbol, is_reference)
        .unwrap()
}

pub(crate) fn create_pop_symbol_node(
    graph: &mut StackGraph,
    file: Handle<File>,
    symbol: &str,
    is_definition: bool,
) -> Handle<Node> {
    let id = graph.new_node_id(file);
    let symbol = graph.add_symbol(symbol);
    graph
        .add_pop_symbol_node(id, symbol, is_definition)
        .unwrap()
}

pub(crate) fn create_symbol_stack(
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

pub(crate) fn create_scope_stack(
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

pub(crate) fn create_partial_path_and_edges(
    graph: &mut StackGraph,
    partials: &mut PartialPaths,
    contents: NicePartialPath,
) -> Result<PartialPath, PathResolutionError> {
    let mut nodes = contents.iter();
    let mut prev = nodes.next().unwrap();
    let mut path = PartialPath::from_node(graph, partials, *prev);
    for next in nodes {
        graph.add_edge(*prev, *next, 0);
        path.append(
            graph,
            partials,
            Edge {
                source: *prev,
                sink: *next,
                precedence: 0,
            },
        )?;
        prev = next;
    }

    Ok(path)
}
