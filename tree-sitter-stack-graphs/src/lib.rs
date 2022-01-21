// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! This crate lets you construct [stack graphs][] using tree-sitter's [graph construction DSL][].
//! The graph DSL lets you construct arbitrary graph structures from the parsed syntax tree of a
//! source file.  If you construct a graph using the vocabulary of attributes described below, then
//! the result of executing the graph DSL will be a valid stack graph, which we can then use for
//! name binding lookups.
//!
//! ## Prerequisites
//!
//! [stack graphs]: https://docs.rs/stack-graphs/*/
//! [graph construction DSL]: https://docs.rs/tree-sitter-graph/*/
//!
//! To process a particular source language, you will first need a tree-sitter grammar for that
//! language.  There are already tree-sitter grammars [available][] for many languages.  If you do
//! not have a tree-sitter grammar for your language, you will need to create that first.  (Check
//! out the tree-sitter [discussion forum][] if you have questions or need pointers on how to do
//! that.)
//!
//! [available]: https://tree-sitter.github.io/tree-sitter/#available-parsers
//! [discussion forum]: https://github.com/tree-sitter/tree-sitter/discussions
//!
//! You will then need to create _stack graph construction rules_ for your language.  These rules
//! are implemented using tree-sitter's [graph construction DSL][].  They define the particular
//! stack graph nodes and edges that should be created for each part of the parsed syntax tree of a
//! source file.
//!
//! ## Graph DSL vocabulary
//!
//! **Please note**: This documentation assumes you are already familiar with stack graphs, and how
//! to use different stack graph node types, and the connectivity between nodes, to implement the
//! name binding semantics of your language.  We assume that you know what kind of stack graph you
//! want to produce; this documentation focuses only on the mechanics of _how_ to create that stack
//! graph content.
//!
//! As mentioned above, your stack graph construction rules should create stack graph nodes and
//! edges from the parsed content of a source file.  You will use TSG [stanzas][] to match on
//! different parts of the parsed syntax tree, and create stack graph content for each match.
//!
//! ### Creating stack graph nodes
//!
//! To create a stack graph node for each identifier in a Python file, you could use the following
//! TSG stanza:
//!
//! ``` skip
//! (identifier) {
//!   node new_node
//! }
//! ```
//!
//! (Here, `node` is a TSG statement that creates a new node, and `new_node` is the name of a local
//! variable that the new node is assigned to, letting you refer to the new node in the rest of the
//! stanza.)
//!
//! [stanzas]: https://docs.rs/tree-sitter-graph/*/tree_sitter_graph/reference/index.html#high-level-structure
//!
//! By default, this new node will be an _internal scope node_.  If you need to create a different
//! kind of stack graph node, set the `type` attribute on the new node:
//!
//! ``` skip
//! (identifier) {
//!   node new_node
//!   attr (new_node) type = "drop"
//! }
//! ```
//!
//! The valid `type` values are:
//!
//! - `definition`: a _definition_ node
//! - `drop`: a _drop scopes_ node
//! - `exported` or `endpoint`: an _exported scope_ node
//! - `internal`: an _internal scope node_ (note that this is the default if you don't provide a
//!   `type` attribute)
//! - `pop`: a _pop symbol_ or _pop scoped symbol_ node
//! - `push`: a _push symbol_ or _push scoped symbol_ node
//! - `reference`: a _reference_ node
//!
//! Certain node types — `definition`, `pop`, `push`, and `reference` — also require you to provide
//! a `symbol` attribute.  Its value must be a string, but will typically come from the content of
//! a parsed syntax node using the [`source-text`][] function and a syntax capture:
//!
//! [`source-text`]: https://docs.rs/tree-sitter-graph/*/tree_sitter_graph/reference/functions/index.html#source-text
//!
//! ``` skip
//! (identifier) @id {
//!   node new_node
//!   attr (new_node) type = "reference", symbol = (source-text @id)
//! }
//! ```
//!
//! To make a _push scoped symbol_ node, you must provide a `scope` attribute.  Its value must be a
//! reference to an `exported` node that you've already created. (This is the exported scope node
//! that will be pushed onto the scope stack.)  For instance:
//!
//! ``` skip
//! (identifier) @id {
//!   node new_exported_scope_node
//!   attr (new_exported_scope_node) type = "exported"
//!   node new_push_scoped_symbol_node
//!   attr (new_push_scoped_symbol_node)
//!     type = "push",
//!     symbol = (source-text @id),
//!     scope = new_exported_scope_node
//! }
//! ```
//!
//! To make a _pop scoped symbol_ node, you must provide a `scoped` attribute, whose value must be
//! `#true`.  (You don't know in advance which particular scope stack will be popped off.) For
//! instance:
//!
//! ``` skip
//! (identifier) @id {
//!   node new_pop_scoped_symbol_node
//!   attr (new_pop_scoped_symbol_node)
//!     type = "pop",
//!     symbol = (source-text @id),
//!     scoped = #true
//! }
//! ```
//!
//! ### Annotating nodes with location information
//!
//! You can annotate any stack graph node that you create with location information, identifying
//! the portion of the source file that the node “belongs to”.  This is _required_ for `definition`
//! and `reference` nodes, since the location information determines which parts of the source file
//! the user can “click on”, and the “destination” of any code navigation queries the user makes.
//! To do this, add a `source_node` attribute, whose value is a syntax node capture:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "definition", symbol = (source-text @id), source_node = @func
//! }
//! ```
//!
//! Note how in this example, we use a different syntax node for the “target” of the definition
//! (the entirety of the function definition) and for the _name_ of the definition (the content of
//! the function's `name`).
//!
//! ### Connecting stack graph nodes with edges
//!
//! To connect two stack graph nodes, use the `edge` statement to add an edge between them:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "definition", symbol = (source-text @id), source_node = @func
//!   node body
//!   edge def -> body
//! }
//! ```
//!
//! To implement shadowing (where later definitions of the same name in the same scope “replace” or
//! “shadow” earlier ones), you can add a `precedence` attribute to each edge to indicate which
//! paths are prioritized:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "definition", symbol = (source-text @id), source_node = @func
//!   node body
//!   edge def -> body
//!   attr (def -> body) precedence = 1
//! }
//! ```
//!
//! (If you don't specify a `precedence`, the default is 0.)
//!
//! ### Referring to the singleton nodes
//!
//! The _root node_ and _jump to scope node_ are singleton nodes that always exist for all stack
//! graphs.  You can refer to them using the `ROOT_NODE` and `JUMP_TO_SCOPE_NODE` global variables:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "definition", symbol = (source-text @id), source_node = @func
//!   edge (ROOT_NODE -> def)
//! }
//! ```
//!
//! ## Using this crate from Rust
//!
//! If you need very fine-grained control over how to use the resulting stack graphs, you can
//! construct and operate on [`StackGraph`][stack_graphs::graph::StackGraph] instances directly
//! from Rust code.  You will need Rust bindings for the tree-sitter grammar for your source
//! language — for instance, [`tree-sitter-python`][].  Grammar Rust bindings provide two global
//! symbols that you will need: [`language`][] and [`STACK_GRAPH_RULES`][].
//!
//! [`tree-sitter-python`]: https://docs.rs/tree-sitter-python/*/
//! [`language`]: https://docs.rs/tree-sitter-python/*/tree_sitter_python/fn.language.html
//! [`STACK_GRAPH_RULES`]: https://docs.rs/tree-sitter-python/*/tree_sitter_python/constant.STACK_GRAPH_RULES.html
//!
//! Once you have those, and the contents of the source file you want to analyze, you can construct
//! a stack graph as follows:
//!
//! ```
//! # use stack_graphs::graph::StackGraph;
//! # use tree_sitter_graph::Variables;
//! # use tree_sitter_graph::functions::Functions;
//! # use tree_sitter_stack_graphs::StackGraphLanguage;
//! # use tree_sitter_stack_graphs::LoadError;
//! #
//! # // This module is a hack to override the STACK_GRAPH_RULES from the actual tree-sitter-python
//! # // crate.  This documentation test is not meant to test Python's actual stack graph
//! # // construction rules.  An empty TSG file is perfectly valid (it just won't produce any stack
//! # // graph content).  This minimizes the amount of work that we do when running `cargo test`.
//! # mod tree_sitter_python {
//! #   pub use ::tree_sitter_python::language;
//! #   pub static STACK_GRAPH_RULES: &str = "";
//! # }
//! #
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let python_source = r#"
//!   import sys
//!   print(sys.path)
//! "#;
//! let grammar = tree_sitter_python::language();
//! let tsg_source = tree_sitter_python::STACK_GRAPH_RULES;
//! let functions = Functions::stdlib();
//! let mut language = StackGraphLanguage::from_str(grammar, tsg_source, functions)?;
//! let mut stack_graph = StackGraph::new();
//! let file_handle = stack_graph.get_or_create_file("test.py");
//! let mut globals = Variables::new();
//! language.build_stack_graph_into(&mut stack_graph, file_handle, python_source, &mut globals)?;
//! # Ok(())
//! # }
//! ```

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
use tree_sitter_graph::graph::Value;
use tree_sitter_graph::Variables;

/// Holds information about how to construct stack graphs for a particular language
pub struct StackGraphLanguage {
    parser: Parser,
    tsg: tree_sitter_graph::ast::File,
    functions: Functions,
}

impl StackGraphLanguage {
    /// Creates a new stack graph language for the given language and
    /// TSG stack graph construction rules.
    pub fn new(
        language: tree_sitter::Language,
        tsg: tree_sitter_graph::ast::File,
        functions: tree_sitter_graph::functions::Functions,
    ) -> Result<StackGraphLanguage, LanguageError> {
        debug_assert_eq!(language, tsg.language);
        let mut parser = Parser::new();
        parser.set_language(language)?;
        Ok(StackGraphLanguage {
            parser,
            tsg,
            functions,
        })
    }

    /// Creates a new stack graph language for the given language, loading the
    /// TSG stack graph construction rules from a string.
    pub fn from_str(
        language: tree_sitter::Language,
        tsg_source: &str,
        functions: tree_sitter_graph::functions::Functions,
    ) -> Result<StackGraphLanguage, LanguageError> {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        let tsg = tree_sitter_graph::ast::File::from_str(language, tsg_source)?;
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
    pub fn build_stack_graph_into(
        &mut self,
        stack_graph: &mut StackGraph,
        file: Handle<File>,
        source: &str,
        globals: &mut Variables,
    ) -> Result<(), LoadError> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or(LoadError::ParseError)?;

        let mut graph = Graph::new();
        globals
            .add("ROOT_NODE".into(), graph.add_graph_node().into())
            .unwrap();
        globals
            .add("JUMP_TO_SCOPE_NODE".into(), graph.add_graph_node().into())
            .unwrap();
        self.tsg
            .execute_lazy_into(&mut graph, &tree, source, &mut self.functions, globals)?;

        let mut loader = StackGraphLoader::new(stack_graph, file, &graph, source);
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
    #[error("Unknown symbol type {0}")]
    UnknownSymbolType(String),
    #[error(transparent)]
    ExecutionError(#[from] tree_sitter_graph::ExecutionError),
    #[error("Error parsing source")]
    ParseError,
    #[error("Error converting shorthand ‘{0}’ on {1} with value {2}")]
    ConversionError(String, String, String),
}

struct StackGraphLoader<'a> {
    stack_graph: &'a mut StackGraph,
    file: Handle<File>,
    graph: &'a Graph<'a>,
    source: &'a str,
    span_calculator: SpanCalculator<'a>,
}

impl<'a> StackGraphLoader<'a> {
    fn new(
        stack_graph: &'a mut StackGraph,
        file: Handle<File>,
        graph: &'a Graph<'a>,
        source: &'a str,
    ) -> Self {
        let span_calculator = SpanCalculator::new(source);
        StackGraphLoader {
            stack_graph,
            file,
            graph,
            source,
            span_calculator,
        }
    }
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
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
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
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
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
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
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
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
        let id = self.node_id_for_graph_node(node_ref);
        Ok(self
            .stack_graph
            .add_push_symbol_node(id, symbol, true)
            .unwrap())
    }

    fn load_symbol(&self, value: &Value) -> Result<String, LoadError> {
        match value {
            Value::Integer(i) => Ok(i.to_string()),
            Value::String(s) => Ok(s.clone()),
            _ => Err(LoadError::UnknownSymbolType(format!("{}", value))),
        }
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
