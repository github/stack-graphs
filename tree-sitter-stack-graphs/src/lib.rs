// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use lsp_positions::SpanCalculator;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use thiserror::Error;
use tree_sitter::Parser;
use tree_sitter_graph::functions::Functions;
use tree_sitter_graph::graph::Graph;
use tree_sitter_graph::graph::GraphNode;
use tree_sitter_graph::graph::GraphNodeRef;
use tree_sitter_graph::Variables;

/// Holds information about how to construct stack graphs for a particular language
pub struct StackGraphLanguage {
    parser: Parser,
    tsg: tree_sitter_graph::ast::File,
    functions: Functions,
}

impl StackGraphLanguage {
    /// Creates a new stack graph language, loading in the TSG graph construction rules from a
    /// string.
    pub fn new(
        language: tree_sitter::Language,
        tsg_source: &str,
    ) -> Result<StackGraphLanguage, LanguageError> {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        let tsg = tree_sitter_graph::ast::File::from_str(language, tsg_source)?;
        let functions = Functions::stdlib();
        Ok(StackGraphLanguage {
            parser,
            tsg,
            functions,
        })
    }
}

/// An error that can occur while loading in the TSG stack graph construction rules for a language
#[derive(Debug, Error)]
pub enum LanguageError {
    #[error(transparent)]
    LanguageError(#[from] tree_sitter::LanguageError),
    #[error(transparent)]
    ParseError(#[from] tree_sitter_graph::ParseError),
}

impl StackGraphLanguage {
    /// Executes the graph construction rules for this language against a source file, creating new
    /// nodes and edges in `stack_graph`.  Any new nodes that we create will belong to `file`.
    /// (The source file must be implemented in this language, otherwise you'll probably get a
    /// parse error.)
    pub fn load_stack_graph(
        &mut self,
        stack_graph: &mut StackGraph,
        file: Handle<File>,
        source: &str,
    ) -> Result<(), LoadError> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or(LoadError::ParseError)?;
        let mut graph = Graph::new();
        let mut globals = Variables::new();
        globals
            .add("ROOT_NODE".into(), graph.add_graph_node().into())
            .unwrap();
        globals
            .add("JUMP_TO_SCOPE_NODE".into(), graph.add_graph_node().into())
            .unwrap();
        self.tsg
            .execute_into(&mut graph, &tree, source, &mut self.functions, &mut globals)?;
        let mut loader = StackGraphLoader {
            stack_graph,
            file,
            graph: &graph,
            source,
            span_calculator: SpanCalculator::new(source),
        };
        loader.load()
    }
}

/// An error that can occur while loading a stack graph from a TSG file
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Missing ‘type’ attribute on graph node")]
    MissingNodeType(GraphNodeRef),
    #[error("Missing ‘symbol’ attribute on graph node")]
    MissingSymbol(GraphNodeRef),
    #[error("Unknown node type {0}")]
    UnknownNodeType(String),
    #[error(transparent)]
    ExecutionError(#[from] tree_sitter_graph::ExecutionError),
    #[error("Error parsing source")]
    ParseError,
}

struct StackGraphLoader<'a> {
    stack_graph: &'a mut StackGraph,
    file: Handle<File>,
    graph: &'a Graph<'a>,
    source: &'a str,
    span_calculator: SpanCalculator<'a>,
}

impl<'a> StackGraphLoader<'a> {
    fn load(&mut self) -> Result<(), LoadError> {
        // First create a stack graph node for each TSG node.  (The skip(2) is because the first
        // two DSL nodes that we create are the proxies for the stack graph's “root” and “jump to
        // scope” nodes.)
        for node_ref in self.graph.iter_nodes().skip(2) {
            let node = &self.graph[node_ref];
            let handle = match get_node_type(node)? {
                NodeType::Definition => self.load_definition(node, node_ref)?,
                NodeType::DropScopes => self.load_drop_scopes(node_ref),
                NodeType::ExportedScope => self.load_exported_scope(node_ref),
                NodeType::InternalScope => self.load_internal_scope(node_ref),
                NodeType::PopSymbol => self.load_pop_symbol(node, node_ref)?,
                NodeType::PushSymbol => self.load_push_symbol(node, node_ref)?,
                NodeType::Reference => self.load_reference(node, node_ref)?,
            };
            self.load_span(node, handle)?;
        }

        // Then add stack graph edges for each TSG edge.  Note that we _don't_ skip(2) here because
        // there might be outgoing nodes from the “root” node that we need to process.
        // (Technically the caller could add outgoing nodes from “jump to scope” as well, but those
        // are invalid according to the stack graph semantics and will never be followed.
        for source_ref in self.graph.iter_nodes() {
            let source = &self.graph[source_ref];
            let source_node_id = self.node_id_for_graph_node(source_ref);
            let source_handle = self.stack_graph.node_for_id(source_node_id).unwrap();
            for (sink_ref, edge) in source.iter_edges() {
                let precedence = match edge.attributes.get("precedence") {
                    Some(precedence) => precedence.as_integer()? as i32,
                    None => 0,
                };
                let sink_node_id = self.node_id_for_graph_node(sink_ref);
                let sink_handle = self.stack_graph.node_for_id(sink_node_id).unwrap();
                self.stack_graph
                    .add_edge(source_handle, sink_handle, precedence);
            }
        }

        Ok(())
    }
}

enum NodeType {
    Definition,
    DropScopes,
    ExportedScope,
    InternalScope,
    PopSymbol,
    PushSymbol,
    Reference,
}

fn get_node_type(node: &GraphNode) -> Result<NodeType, LoadError> {
    let node_type = match node.attributes.get("type") {
        Some(node_type) => node_type.as_str()?,
        None => return Ok(NodeType::InternalScope),
    };
    if node_type == "definition" {
        return Ok(NodeType::Definition);
    } else if node_type == "drop" {
        return Ok(NodeType::DropScopes);
    } else if node_type == "exported" || node_type == "endpoint" {
        return Ok(NodeType::ExportedScope);
    } else if node_type == "internal" {
        return Ok(NodeType::InternalScope);
    } else if node_type == "pop" {
        return Ok(NodeType::PopSymbol);
    } else if node_type == "push" {
        return Ok(NodeType::PushSymbol);
    } else if node_type == "reference" {
        return Ok(NodeType::Reference);
    } else {
        return Err(LoadError::UnknownNodeType(format!("{:?}", node_type)));
    }
}

impl<'a> StackGraphLoader<'a> {
    fn node_id_for_graph_node(&self, node_ref: GraphNodeRef) -> NodeID {
        let index = node_ref.index();
        if index == 0 {
            NodeID::root()
        } else if index == 1 {
            NodeID::jump_to()
        } else {
            NodeID::new_in_file(self.file, (node_ref.index() as u32) - 2)
        }
    }

    fn load_definition(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get("symbol") {
            Some(symbol) => symbol.as_str()?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(symbol);
        let id = self.node_id_for_graph_node(node_ref);
        Ok(self
            .stack_graph
            .add_pop_symbol_node(id, symbol, true)
            .unwrap())
    }

    fn load_drop_scopes(&mut self, node_ref: GraphNodeRef) -> Handle<Node> {
        let id = self.node_id_for_graph_node(node_ref);
        self.stack_graph.add_drop_scopes_node(id).unwrap()
    }

    fn load_exported_scope(&mut self, node_ref: GraphNodeRef) -> Handle<Node> {
        let id = self.node_id_for_graph_node(node_ref);
        self.stack_graph.add_scope_node(id, true).unwrap()
    }

    fn load_internal_scope(&mut self, node_ref: GraphNodeRef) -> Handle<Node> {
        let id = self.node_id_for_graph_node(node_ref);
        self.stack_graph.add_scope_node(id, false).unwrap()
    }

    fn load_pop_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get("symbol") {
            Some(symbol) => symbol.as_str()?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(symbol);
        let id = self.node_id_for_graph_node(node_ref);
        if let Some(scoped) = node.attributes.get("scoped") {
            if scoped.as_boolean()? {
                return Ok(self
                    .stack_graph
                    .add_pop_scoped_symbol_node(id, symbol, false)
                    .unwrap());
            }
        }
        Ok(self
            .stack_graph
            .add_pop_symbol_node(id, symbol, false)
            .unwrap())
    }

    fn load_push_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get("symbol") {
            Some(symbol) => symbol.as_str()?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(symbol);
        let id = self.node_id_for_graph_node(node_ref);
        if let Some(scope) = node.attributes.get("scope") {
            let scope = scope.as_graph_node_ref()?;
            let scope = self.node_id_for_graph_node(scope);
            return Ok(self
                .stack_graph
                .add_push_scoped_symbol_node(id, symbol, scope, false)
                .unwrap());
        }
        Ok(self
            .stack_graph
            .add_push_symbol_node(id, symbol, false)
            .unwrap())
    }

    fn load_reference(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get("symbol") {
            Some(symbol) => symbol.as_str()?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(symbol);
        let id = self.node_id_for_graph_node(node_ref);
        Ok(self
            .stack_graph
            .add_push_symbol_node(id, symbol, true)
            .unwrap())
    }

    fn load_span(&mut self, node: &GraphNode, node_handle: Handle<Node>) -> Result<(), LoadError> {
        let source_node = match node.attributes.get("source_node") {
            Some(source_node) => &self.graph[source_node.as_syntax_node_ref()?],
            None => return Ok(()),
        };
        let span = self.span_calculator.for_node(source_node);
        let containing_line = &self.source[span.start.containing_line.clone()];
        let containing_line = self.stack_graph.add_string(containing_line);
        let source_info = self.stack_graph.source_info_mut(node_handle);
        source_info.span = span;
        source_info.containing_line = ControlledOption::some(containing_line);
        Ok(())
    }
}
