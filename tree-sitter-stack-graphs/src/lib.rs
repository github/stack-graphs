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
//!   attr (new_node) type = DROP_SCOPES_TYPE
//! }
//! ```
//!
//! The valid `type` values are:
//!
//! - `drop_scopes`: a _drop scopes_ node
//! - `pop_symbol`: a _pop symbol_ node
//! - `pop_scoped_symbol`: a _pop scoped symbol_ node
//! - `push_symbol`: a _push symbol_ node
//! - `push_scoped_symbol`: a _push scoped symbol_ node
//! - `scope`: a _scope_ node
//!
//! A node without an explicit `type` attribute is assumed to be of type `scope`.
//!
//! Certain node types — `pop_symbol`, `pop_scoped_symbol`, `push_symbol` and `push_scoped_symbol` —
//! also require you to provide a `symbol` attribute.  Its value must be a string, but will typically
//! come from the content of a parsed syntax node using the [`source-text`][] function and a syntax
//! capture:
//!
//! [`source-text`]: https://docs.rs/tree-sitter-graph/*/tree_sitter_graph/reference/functions/index.html#source-text
//!
//! ``` skip
//! (identifier) @id {
//!   node new_node
//!   attr (new_node) type = PUSH_SYMBOL_TYPE, symbol = (source-text @id)
//! }
//! ```
//!
//! Node types `pop_symbol` and `pop_scoped_symbol` allow an optional `is_definition` attribute, which
//! marks that node as a proper definition.  Node types `push_symbol` and `push_scoped_symbol` allow
//! an optiona `is_reference` attribute, which marks the node as a proper reference.  When `is_definition`
//! or `is_reference` are set, the `source_node` attribute is required.
//!
//! ``` skip
//! (identifier) @id {
//!   node new_node
//!   attr (new_node) type = PUSH_SYMBOL_TYPE, symbol = (source-text @id), is_reference, source_node = @id
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
//!   attr (new_exported_scope_node) is_exported
//!   node new_push_scoped_symbol_node
//!   attr (new_push_scoped_symbol_node)
//!     type = PUSH_SCOPED_SYMBOL_TYPE,
//!     symbol = (source-text @id),
//!     scope = new_exported_scope_node
//! }
//! ```
//!
//! Nodes of type `scope` allow an optional `is_exported` attribute, that is required to use the scope
//! in a `push_scoped_symbol` node.
//!
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
//!   attr (def) type = POP_SYMBOL_TYPE, symbol = (source-text @id), source_node = @func, is_definition
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
//!   attr (def) type = POP_SYMBOL_TYPE, symbol = (source-text @id), source_node = @func, is_definition
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
//!   attr (def) type = POP_SYMBOL_TYPE, symbol = (source-text @id), source_node = @func, is_definition
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
//!   attr (def) type = POP_SYMBOL_TYPE, symbol = (source-text @id), source_node = @func, is_definition
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

// Node type values
static DROP_SCOPES_TYPE: &'static str = "drop_scopes";
static POP_SCOPED_SYMBOL_TYPE: &'static str = "pop_scoped_symbol";
static POP_SYMBOL_TYPE: &'static str = "pop_symbol";
static PUSH_SCOPED_SYMBOL_TYPE: &'static str = "push_scoped_symbol";
static PUSH_SYMBOL_TYPE: &'static str = "push_symbol";
static SCOPE_TYPE: &'static str = "scope";

// Node attribute names
static IS_DEFINITION_ATTR: &'static str = "is_definition";
static IS_EXPORTED_ATTR: &'static str = "is_exported";
static IS_ENDPOINT_ATTR: &'static str = "is_endpoint";
static IS_REFERENCE_ATTR: &'static str = "is_reference";
static SCOPE_ATTR: &'static str = "scope";
static SOURCE_NODE_ATTR: &'static str = "source_node";
static SYMBOL_ATTR: &'static str = "symbol";
static TYPE_ATTR: &'static str = "type";

// Edge attribute names
static PRECEDENCE_ATTR: &'static str = "precedence";

// Node attribute shorthands
static NODE_DEFINITION_SHORTHAND: &'static str = "node_definition";
static SCOPED_NODE_DEFINITION_SHORTHAND: &'static str = "scoped_node_definition";
static NODE_REFERENCE_SHORTHAND: &'static str = "node_reference";
static SCOPED_NODE_REFERENCE_SHORTHAND: &'static str = "scoped_node_reference";
static POP_NODE_SHORTHAND: &'static str = "pop_node";
static POP_SCOPED_NODE_SHORTHAND: &'static str = "pop_scoped_node";
static POP_SYMBOL_SHORTHAND: &'static str = "pop_symbol";
static POP_SCOPED_SYMBOL_SHORTHAND: &'static str = "pop_scoped_symbol";
static PUSH_NODE_SHORTHAND: &'static str = "push_node";
static PUSH_SCOPED_NODE_SHORTHAND: &'static str = "push_scoped_node";
static PUSH_SYMBOL_SHORTHAND: &'static str = "push_symbol";
static PUSH_SCOPED_SYMBOL_SHORTHAND: &'static str = "push_scoped_symbol";

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

        GraphConverter::convert_shorthands(&mut graph, source)?;

        let mut loader = StackGraphLoader::new(stack_graph, file, &graph, source);
        loader.load()
    }
}

struct GraphConverter;

impl GraphConverter {
    fn convert_shorthands<'a>(graph: &mut Graph<'a>, source: &str) -> Result<(), LoadError> {
        for node_ref in graph.iter_nodes().skip(2) {
            Self::convert_node_shorthands(graph, source, node_ref).map_err(|(name, value)| {
                LoadError::ConversionError(
                    name.to_string(),
                    format!("{}", node_ref),
                    format!("{}", value),
                )
            })?;
        }
        Ok(())
    }

    fn convert_node_shorthands<'a>(
        graph: &mut Graph<'a>,
        source: &str,
        node_ref: GraphNodeRef,
    ) -> Result<(), (&'static str, Value)> {
        let node = &graph[node_ref];
        let mut new_attributes: Vec<(&'static str, Value)> = Vec::new();

        if let Some(value) = node.attributes.get(NODE_DEFINITION_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (NODE_DEFINITION_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
            new_attributes.push((IS_DEFINITION_ATTR, true.into()));
        } else if let Some(value) = node.attributes.get(SCOPED_NODE_DEFINITION_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SCOPED_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (SCOPED_NODE_DEFINITION_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
            new_attributes.push((IS_DEFINITION_ATTR, true.into()));
        } else if let Some(value) = node.attributes.get(POP_NODE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (POP_NODE_SHORTHAND, value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
        } else if let Some(value) = node.attributes.get(POP_SCOPED_NODE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SCOPED_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (POP_SCOPED_NODE_SHORTHAND, value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
        } else if let Some(value) = node.attributes.get(POP_SYMBOL_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SYMBOL_TYPE.into()));
            let symbol = Self::convert_symbol(value)
                .map_err(|_| (POP_SYMBOL_SHORTHAND.into(), value.clone()))?;
            new_attributes.push((SYMBOL_ATTR, symbol.clone().into()));
        } else if let Some(value) = node.attributes.get(POP_SCOPED_SYMBOL_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, POP_SCOPED_SYMBOL_TYPE.into()));
            let symbol = Self::convert_symbol(value)
                .map_err(|_| (POP_SCOPED_SYMBOL_SHORTHAND.into(), value.clone()))?;
            new_attributes.push((SYMBOL_ATTR, symbol.clone().into()));
        } else if let Some(value) = node.attributes.get(NODE_REFERENCE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (NODE_REFERENCE_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
            new_attributes.push((IS_REFERENCE_ATTR, true.into()));
        } else if let Some(value) = node.attributes.get(SCOPED_NODE_REFERENCE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SCOPED_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (SCOPED_NODE_REFERENCE_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
            new_attributes.push((IS_REFERENCE_ATTR, true.into()));
        } else if let Some(value) = node.attributes.get(PUSH_NODE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (PUSH_NODE_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
        } else if let Some(value) = node.attributes.get(PUSH_SCOPED_NODE_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SCOPED_SYMBOL_TYPE.into()));
            let node = value
                .as_syntax_node_ref()
                .map_err(|_| (PUSH_SCOPED_NODE_SHORTHAND.into(), value.clone()))?;
            let symbol = source[graph[node].byte_range()].to_string();
            new_attributes.push((SYMBOL_ATTR, symbol.into()));
            new_attributes.push((SOURCE_NODE_ATTR, node.clone().into()));
        } else if let Some(value) = node.attributes.get(PUSH_SYMBOL_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SYMBOL_TYPE.into()));
            let symbol = Self::convert_symbol(value)
                .map_err(|_| (PUSH_SYMBOL_TYPE.into(), value.clone()))?;
            new_attributes.push((SYMBOL_ATTR, symbol.clone().into()));
        } else if let Some(value) = node.attributes.get(PUSH_SCOPED_SYMBOL_SHORTHAND) {
            new_attributes.push((TYPE_ATTR, PUSH_SCOPED_SYMBOL_TYPE.into()));
            let symbol = Self::convert_symbol(value)
                .map_err(|_| (PUSH_SCOPED_SYMBOL_TYPE.into(), value.clone()))?;
            new_attributes.push((SYMBOL_ATTR, symbol.clone().into()));
        }

        if !new_attributes.is_empty() {
            for (name, value) in new_attributes {
                graph[node_ref]
                    .attributes
                    .add(name.into(), value.clone())
                    .map_err(|_| (name, value))?;
            }
        }
        Ok(())
    }

    fn convert_symbol<'a>(value: &Value) -> Result<Value, ()> {
        match value {
            Value::String(_) | Value::Integer(_) => Ok(value.clone()),
            _ => Err(()),
        }
    }
}

/// An error that can occur while loading a stack graph from a TSG file
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Missing ‘type’ attribute on graph node")]
    MissingNodeType(GraphNodeRef),
    #[error("Missing ‘symbol’ attribute on graph node")]
    MissingSymbol(GraphNodeRef),
    #[error("Missing ‘scope’ attribute on graph node")]
    MissingScope(GraphNodeRef),
    #[error("Unknown ‘{0}’ flag type {1}")]
    UnknownFlagType(String, String),
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
                NodeType::DropScopes => self.load_drop_scopes(node_ref),
                NodeType::PopScopedSymbol => self.load_pop_scoped_symbol(node, node_ref)?,
                NodeType::PopSymbol => self.load_pop_symbol(node, node_ref)?,
                NodeType::PushScopedSymbol => self.load_push_scoped_symbol(node, node_ref)?,
                NodeType::PushSymbol => self.load_push_symbol(node, node_ref)?,
                NodeType::Scope => self.load_scope(node, node_ref)?,
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
                let precedence = match edge.attributes.get(PRECEDENCE_ATTR) {
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
    DropScopes,
    PopSymbol,
    PopScopedSymbol,
    PushSymbol,
    PushScopedSymbol,
    Scope,
}

fn get_node_type(node: &GraphNode) -> Result<NodeType, LoadError> {
    let node_type = match node.attributes.get(TYPE_ATTR) {
        Some(node_type) => node_type.as_str()?,
        None => return Ok(NodeType::Scope),
    };
    if node_type == DROP_SCOPES_TYPE {
        return Ok(NodeType::DropScopes);
    } else if node_type == POP_SCOPED_SYMBOL_TYPE {
        return Ok(NodeType::PopScopedSymbol);
    } else if node_type == POP_SYMBOL_TYPE {
        return Ok(NodeType::PopSymbol);
    } else if node_type == PUSH_SCOPED_SYMBOL_TYPE {
        return Ok(NodeType::PushScopedSymbol);
    } else if node_type == PUSH_SYMBOL_TYPE {
        return Ok(NodeType::PushSymbol);
    } else if node_type == SCOPE_TYPE {
        return Ok(NodeType::Scope);
    } else {
        return Err(LoadError::UnknownNodeType(format!("{}", node_type)));
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

    fn load_drop_scopes(&mut self, node_ref: GraphNodeRef) -> Handle<Node> {
        let id = self.node_id_for_graph_node(node_ref);
        self.stack_graph.add_drop_scopes_node(id).unwrap()
    }

    fn load_pop_scoped_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get(SYMBOL_ATTR) {
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
        let id = self.node_id_for_graph_node(node_ref);
        let is_definition = self.load_flag(node, IS_DEFINITION_ATTR)?;
        Ok(self
            .stack_graph
            .add_pop_scoped_symbol_node(id, symbol, is_definition)
            .unwrap())
    }

    fn load_pop_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get(SYMBOL_ATTR) {
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
        let id = self.node_id_for_graph_node(node_ref);
        let is_definition = self.load_flag(node, IS_DEFINITION_ATTR)?;
        Ok(self
            .stack_graph
            .add_pop_symbol_node(id, symbol, is_definition)
            .unwrap())
    }

    fn load_push_scoped_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get(SYMBOL_ATTR) {
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
        let id = self.node_id_for_graph_node(node_ref);
        let scope = match node.attributes.get(SCOPE_ATTR) {
            Some(scope) => self.node_id_for_graph_node(scope.as_graph_node_ref()?),
            None => return Err(LoadError::MissingScope(node_ref)),
        };
        let is_reference = self.load_flag(node, IS_REFERENCE_ATTR)?;
        Ok(self
            .stack_graph
            .add_push_scoped_symbol_node(id, symbol, scope, is_reference)
            .unwrap())
    }

    fn load_push_symbol(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let symbol = match node.attributes.get(SYMBOL_ATTR) {
            Some(symbol) => self.load_symbol(symbol)?,
            None => return Err(LoadError::MissingSymbol(node_ref)),
        };
        let symbol = self.stack_graph.add_symbol(&symbol);
        let id = self.node_id_for_graph_node(node_ref);
        let is_reference = self.load_flag(node, IS_REFERENCE_ATTR)?;
        Ok(self
            .stack_graph
            .add_push_symbol_node(id, symbol, is_reference)
            .unwrap())
    }

    fn load_scope(
        &mut self,
        node: &GraphNode,
        node_ref: GraphNodeRef,
    ) -> Result<Handle<Node>, LoadError> {
        let id = self.node_id_for_graph_node(node_ref);
        let is_exported =
            self.load_flag(node, IS_EXPORTED_ATTR)? || self.load_flag(node, IS_ENDPOINT_ATTR)?;
        Ok(self.stack_graph.add_scope_node(id, is_exported).unwrap())
    }

    fn load_symbol(&self, value: &Value) -> Result<String, LoadError> {
        match value {
            Value::Integer(i) => Ok(i.to_string()),
            Value::String(s) => Ok(s.clone()),
            _ => Err(LoadError::UnknownSymbolType(format!("{}", value))),
        }
    }

    fn load_flag(&mut self, node: &GraphNode, attribute: &str) -> Result<bool, LoadError> {
        match node.attributes.get(attribute) {
            Some(value) => value.as_boolean().map_err(|_| {
                LoadError::UnknownFlagType(format!("{}", attribute), format!("{}", value))
            }),
            None => Ok(false),
        }
    }

    fn load_span(&mut self, node: &GraphNode, node_handle: Handle<Node>) -> Result<(), LoadError> {
        let source_node = match node.attributes.get(SOURCE_NODE_ATTR) {
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
