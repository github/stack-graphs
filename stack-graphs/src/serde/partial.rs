// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::partial::PartialPaths;

use super::Error;
use super::NodeID;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialPath {
    pub(crate) start_node: NodeID,
    pub(crate) end_node: NodeID,
    pub(crate) symbol_stack_precondition: PartialSymbolStack,
    pub(crate) symbol_stack_postcondition: PartialSymbolStack,
    pub(crate) scope_stack_precondition: PartialScopeStack,
    pub(crate) scope_stack_postcondition: PartialScopeStack,
    pub(crate) edges: PartialPathEdgeList,
}

impl PartialPath {
    pub fn from_partial_path(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialPath,
    ) -> Self {
        Self {
            start_node: NodeID::from_node(graph, value.start_node),
            end_node: NodeID::from_node(graph, value.end_node),
            symbol_stack_precondition: PartialSymbolStack::from_partial_symbol_stack(
                graph,
                partials,
                &value.symbol_stack_precondition,
            ),
            symbol_stack_postcondition: PartialSymbolStack::from_partial_symbol_stack(
                graph,
                partials,
                &value.symbol_stack_postcondition,
            ),
            scope_stack_precondition: PartialScopeStack::from_partial_scope_stack(
                graph,
                partials,
                &value.scope_stack_precondition,
            ),
            scope_stack_postcondition: PartialScopeStack::from_partial_scope_stack(
                graph,
                partials,
                &value.scope_stack_postcondition,
            ),
            edges: PartialPathEdgeList::from_partial_path_edge_list(graph, partials, &value.edges),
        }
    }

    pub fn to_partial_path(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPath, Error> {
        Ok(crate::partial::PartialPath {
            start_node: self.start_node.to_node(graph)?,
            end_node: self.end_node.to_node(graph)?,
            symbol_stack_precondition: self
                .symbol_stack_precondition
                .to_partial_symbol_stack(graph, partials)?,
            symbol_stack_postcondition: self
                .symbol_stack_postcondition
                .to_partial_symbol_stack(graph, partials)?,
            scope_stack_precondition: self
                .scope_stack_precondition
                .to_partial_scope_stack(graph, partials)?,
            scope_stack_postcondition: self
                .scope_stack_postcondition
                .to_partial_scope_stack(graph, partials)?,
            edges: self.edges.to_partial_path_edge_list(graph, partials)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialScopeStack {
    pub(crate) scopes: Vec<NodeID>,
    variable: Option<ScopeStackVariable>,
}

impl PartialScopeStack {
    pub fn from_partial_scope_stack(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialScopeStack,
    ) -> Self {
        let mut value = *value;
        let mut scopes = Vec::new();
        while let Some(scope) = value.pop_front(partials) {
            scopes.push(NodeID::from_node(graph, scope));
        }
        Self {
            scopes,
            variable: value
                .variable()
                .map(|v| ScopeStackVariable::from_scope_stack_variable(v)),
        }
    }

    pub fn to_partial_scope_stack(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialScopeStack, Error> {
        let mut value = match &self.variable {
            Some(variable) => crate::partial::PartialScopeStack::from_variable(
                variable.to_scope_stack_variable()?,
            ),
            None => crate::partial::PartialScopeStack::empty(),
        };
        for scope in &self.scopes {
            let scope = scope.to_node(graph)?;
            value.push_back(partials, scope);
        }
        Ok(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct ScopeStackVariable(u32);

impl ScopeStackVariable {
    pub fn from_scope_stack_variable(value: crate::partial::ScopeStackVariable) -> Self {
        Self(value.as_u32())
    }

    pub fn to_scope_stack_variable(&self) -> Result<crate::partial::ScopeStackVariable, Error> {
        crate::partial::ScopeStackVariable::new(self.0)
            .ok_or_else(|| Error::InvalidStackVariable(self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialSymbolStack {
    pub(crate) symbols: Vec<PartialScopedSymbol>,
    variable: Option<SymbolStackVariable>,
}

impl PartialSymbolStack {
    pub fn from_partial_symbol_stack(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialSymbolStack,
    ) -> Self {
        let mut value = *value;
        let mut symbols = Vec::new();
        while let Some(symbol) = value.pop_front(partials) {
            symbols.push(PartialScopedSymbol::from_partial_scoped_symbol(
                graph, partials, &symbol,
            ));
        }
        Self {
            symbols,
            variable: value
                .variable()
                .map(|v| SymbolStackVariable::from_symbol_stack_variable(v)),
        }
    }

    pub fn to_partial_symbol_stack(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialSymbolStack, Error> {
        let mut value = match &self.variable {
            Some(variable) => crate::partial::PartialSymbolStack::from_variable(
                variable.to_symbol_stack_variable()?,
            ),
            None => crate::partial::PartialSymbolStack::empty(),
        };
        for symbol in &self.symbols {
            let symbol = symbol.to_partial_scoped_symbol(graph, partials)?;
            value.push_back(partials, symbol);
        }
        Ok(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct SymbolStackVariable(u32);

impl SymbolStackVariable {
    pub fn from_symbol_stack_variable(value: crate::partial::SymbolStackVariable) -> Self {
        Self(value.as_u32())
    }

    pub fn to_symbol_stack_variable(&self) -> Result<crate::partial::SymbolStackVariable, Error> {
        crate::partial::SymbolStackVariable::new(self.0)
            .ok_or_else(|| Error::InvalidStackVariable(self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialScopedSymbol {
    symbol: String,
    pub(crate) scopes: Option<PartialScopeStack>,
}

impl PartialScopedSymbol {
    pub fn from_partial_scoped_symbol(
        graph: &crate::graph::StackGraph,
        partials: &mut crate::partial::PartialPaths,
        value: &crate::partial::PartialScopedSymbol,
    ) -> Self {
        Self {
            symbol: graph[value.symbol].to_string(),
            scopes: value.scopes.into_option().map(|scopes| {
                PartialScopeStack::from_partial_scope_stack(graph, partials, &scopes)
            }),
        }
    }

    pub fn to_partial_scoped_symbol(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut crate::partial::PartialPaths,
    ) -> Result<crate::partial::PartialScopedSymbol, Error> {
        Ok(crate::partial::PartialScopedSymbol {
            symbol: graph.add_symbol(&self.symbol),
            scopes: self
                .scopes
                .as_ref()
                .map(|scopes| scopes.to_partial_scope_stack(graph, partials))
                .transpose()?
                .into(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialPathEdgeList {
    pub(crate) edges: Vec<PartialPathEdge>,
}

impl PartialPathEdgeList {
    pub fn from_partial_path_edge_list(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialPathEdgeList,
    ) -> Self {
        let mut value = *value;
        let mut edges = Vec::new();
        while let Some(edge) = value.pop_front(partials) {
            edges.push(PartialPathEdge::from_partial_path_edge(
                graph, partials, &edge,
            ));
        }
        Self { edges }
    }

    pub fn to_partial_path_edge_list(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPathEdgeList, Error> {
        let mut value = crate::partial::PartialPathEdgeList::empty();
        for edge in &self.edges {
            let edge = edge.to_partial_path_edge(graph, partials)?;
            value.push_back(partials, edge);
        }
        Ok(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialPathEdge {
    pub(crate) source: NodeID,
    precedence: i32,
}

impl PartialPathEdge {
    pub fn from_partial_path_edge(
        graph: &crate::graph::StackGraph,
        _partials: &mut PartialPaths,
        value: &crate::partial::PartialPathEdge,
    ) -> Self {
        Self {
            source: NodeID::from_node_id(graph, value.source_node_id),
            precedence: value.precedence,
        }
    }

    pub fn to_partial_path_edge(
        &self,
        graph: &mut crate::graph::StackGraph,
        _partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPathEdge, Error> {
        Ok(crate::partial::PartialPathEdge {
            source_node_id: self.source.to_node_id(graph)?,
            precedence: self.precedence,
        })
    }
}
