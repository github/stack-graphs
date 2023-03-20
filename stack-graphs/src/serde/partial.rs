// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde::Deserialize;
use serde::Serialize;

use crate::arena::Handle;
use crate::partial::PartialPaths;

use super::Error;
use super::NodeID;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PartialPath {
    start_node: NodeID,
    end_node: NodeID,
    symbol_stack_precondition: PartialSymbolStack,
    symbol_stack_postcondition: PartialSymbolStack,
    scope_stack_precondition: PartialScopeStack,
    scope_stack_postcondition: PartialScopeStack,
    edges: PartialPathEdgeList,
}

impl PartialPath {
    pub fn from(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialPath,
    ) -> Self {
        Self {
            start_node: NodeID::from_node(graph, value.start_node),
            end_node: NodeID::from_node(graph, value.end_node),
            symbol_stack_precondition: PartialSymbolStack::from(
                graph,
                partials,
                &value.symbol_stack_precondition,
            ),
            symbol_stack_postcondition: PartialSymbolStack::from(
                graph,
                partials,
                &value.symbol_stack_postcondition,
            ),
            scope_stack_precondition: PartialScopeStack::from(
                graph,
                partials,
                &value.scope_stack_precondition,
            ),
            scope_stack_postcondition: PartialScopeStack::from(
                graph,
                partials,
                &value.scope_stack_postcondition,
            ),
            edges: PartialPathEdgeList::from(graph, partials, &value.edges),
        }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPath, Error> {
        Ok(crate::partial::PartialPath {
            start_node: self.start_node.to_node(graph)?,
            end_node: self.end_node.to_node(graph)?,
            symbol_stack_precondition: self.symbol_stack_precondition.to(graph, partials)?,
            symbol_stack_postcondition: self.symbol_stack_postcondition.to(graph, partials)?,
            scope_stack_precondition: self.scope_stack_precondition.to(graph, partials)?,
            scope_stack_postcondition: self.scope_stack_postcondition.to(graph, partials)?,
            edges: self.edges.to(graph, partials)?,
        })
    }
}

impl NodeID {
    pub fn from_node(graph: &crate::graph::StackGraph, handle: Handle<crate::graph::Node>) -> Self {
        Self::from(graph, &graph[handle].id())
    }

    pub fn to_node(
        &self,
        graph: &mut crate::graph::StackGraph,
    ) -> Result<Handle<crate::graph::Node>, Error> {
        let value = self.to(graph)?;
        Ok(graph
            .node_for_id(value)
            .ok_or_else(|| Error::NodeNotFound(self.clone()))?)
    }

    pub fn from(graph: &crate::graph::StackGraph, value: &crate::graph::NodeID) -> Self {
        Self {
            file: value.file().map(|f| graph[f].to_string()),
            local_id: value.local_id(),
        }
    }

    pub fn to(&self, graph: &mut crate::graph::StackGraph) -> Result<crate::graph::NodeID, Error> {
        if let Some(file) = &self.file {
            let file = graph
                .get_file(file)
                .ok_or_else(|| Error::FileNotFound(file.clone()))?;
            Ok(crate::graph::NodeID::new_in_file(file, self.local_id))
        } else if self.local_id == crate::graph::JUMP_TO_NODE_ID {
            Ok(crate::graph::NodeID::jump_to())
        } else if self.local_id == crate::graph::ROOT_NODE_ID {
            Ok(crate::graph::NodeID::root())
        } else {
            Err(Error::InvalidGlobalNodeID(self.local_id))
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PartialScopeStack {
    scopes: Vec<NodeID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variable: Option<ScopeStackVariable>,
}

impl PartialScopeStack {
    pub fn from(
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
            variable: value.variable().map(|v| ScopeStackVariable::from(v)),
        }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialScopeStack, Error> {
        let mut value = crate::partial::PartialScopeStack::empty();
        for scope in &self.scopes {
            let scope = scope.to_node(graph)?;
            value.push_back(partials, scope);
        }
        Ok(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(transparent)]
pub struct ScopeStackVariable(u32);

impl ScopeStackVariable {
    pub fn from(value: crate::partial::ScopeStackVariable) -> Self {
        Self(value.as_u32())
    }

    pub fn to(&self) -> Result<crate::partial::ScopeStackVariable, Error> {
        crate::partial::ScopeStackVariable::new(self.0)
            .ok_or_else(|| Error::InvalidStackVariable(self.0))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PartialSymbolStack {
    symbols: Vec<PartialScopedSymbol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variable: Option<SymbolStackVariable>,
}

impl PartialSymbolStack {
    pub fn from(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialSymbolStack,
    ) -> Self {
        let mut value = *value;
        let mut symbols = Vec::new();
        while let Some(symbol) = value.pop_front(partials) {
            symbols.push(PartialScopedSymbol::from(graph, partials, &symbol));
        }
        Self {
            symbols,
            variable: value.variable().map(|v| SymbolStackVariable::from(v)),
        }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialSymbolStack, Error> {
        let mut value = crate::partial::PartialSymbolStack::empty();
        for symbol in &self.symbols {
            let symbol = symbol.to(graph, partials)?;
            value.push_back(partials, symbol);
        }
        Ok(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(transparent)]
pub struct SymbolStackVariable(u32);

impl SymbolStackVariable {
    pub fn from(value: crate::partial::SymbolStackVariable) -> Self {
        Self(value.as_u32())
    }

    pub fn to(&self) -> Result<crate::partial::SymbolStackVariable, Error> {
        crate::partial::SymbolStackVariable::new(self.0)
            .ok_or_else(|| Error::InvalidStackVariable(self.0))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PartialScopedSymbol {
    symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<PartialScopeStack>,
}

impl PartialScopedSymbol {
    pub fn from(
        graph: &crate::graph::StackGraph,
        partials: &mut crate::partial::PartialPaths,
        value: &crate::partial::PartialScopedSymbol,
    ) -> Self {
        Self {
            symbol: graph[value.symbol].to_string(),
            scopes: value
                .scopes
                .into_option()
                .map(|scopes| PartialScopeStack::from(graph, partials, &scopes)),
        }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut crate::partial::PartialPaths,
    ) -> Result<crate::partial::PartialScopedSymbol, Error> {
        Ok(crate::partial::PartialScopedSymbol {
            symbol: graph.add_symbol(&self.symbol),
            scopes: self
                .scopes
                .as_ref()
                .map(|scopes| scopes.to(graph, partials))
                .transpose()?
                .into(),
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(transparent)]
pub struct PartialPathEdgeList {
    edges: Vec<PartialPathEdge>,
}

impl PartialPathEdgeList {
    pub fn from(
        graph: &crate::graph::StackGraph,
        partials: &mut PartialPaths,
        value: &crate::partial::PartialPathEdgeList,
    ) -> Self {
        let mut value = *value;
        let mut edges = Vec::new();
        while let Some(edge) = value.pop_front(partials) {
            edges.push(PartialPathEdge::from(graph, partials, &edge));
        }
        Self { edges }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPathEdgeList, Error> {
        let mut value = crate::partial::PartialPathEdgeList::empty();
        for edge in &self.edges {
            let edge = edge.to(graph, partials)?;
            value.push_back(partials, edge);
        }
        Ok(value)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct PartialPathEdge {
    source: NodeID,
    precedence: i32,
}

impl PartialPathEdge {
    pub fn from(
        graph: &crate::graph::StackGraph,
        _partials: &mut PartialPaths,
        value: &crate::partial::PartialPathEdge,
    ) -> Self {
        Self {
            source: NodeID::from(graph, &value.source_node_id),
            precedence: value.precedence,
        }
    }

    pub fn to(
        &self,
        graph: &mut crate::graph::StackGraph,
        _partials: &mut PartialPaths,
    ) -> Result<crate::partial::PartialPathEdge, Error> {
        Ok(crate::partial::PartialPathEdge {
            source_node_id: self.source.to(graph)?,
            precedence: self.precedence,
        })
    }
}
