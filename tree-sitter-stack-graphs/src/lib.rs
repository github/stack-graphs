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
//! By default, this new node will be a _scope node_.  If you need to create a different kind of stack
//! graph node, set the `type` attribute on the new node:
//!
//! ``` skip
//! (identifier) {
//!   node new_node
//!   attr (new_node) type = "push_symbol"
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
//!   attr (new_node) type = "push_symbol", symbol = (source-text @id)
//! }
//! ```
//!
//! Node types `pop_symbol` and `pop_scoped_symbol` allow an optional `is_definition` attribute, which
//! marks that node as a proper definition.  Node types `push_symbol` and `push_scoped_symbol` allow
//! an optional `is_reference` attribute, which marks the node as a proper reference.  When `is_definition`
//! or `is_reference` are set, the `source_node` attribute is required.
//!
//! ``` skip
//! (identifier) @id {
//!   node new_node
//!   attr (new_node) type = "push_symbol", symbol = (source-text @id), is_reference, source_node = @id
//! }
//! ```
//!
//! A _push scoped symbol_ node requires a `scope` attribute.  Its value must be a reference to an `exported`
//! node that you've already created. (This is the exported scope node that will be pushed onto the scope
//! stack.)  For instance:
//!
//! ``` skip
//! (identifier) @id {
//!   node new_exported_scope_node
//!   attr (new_exported_scope_node) is_exported
//!   node new_push_scoped_symbol_node
//!   attr (new_push_scoped_symbol_node)
//!     type = "push_scoped_symbol",
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
//! the portion of the source file that the node "belongs to".  This is _required_ for definition
//! and reference nodes, since the location information determines which parts of the source file
//! the user can _click on_, and the _destination_ of any code navigation queries the user makes.
//! To do this, add a `source_node` attribute, whose value is a syntax node capture:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "pop_symbol", symbol = (source-text @id), source_node = @func, is_definition
//! }
//! ```
//!
//! Note how in this example, we use a different syntax node for the _target_ of the definition
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
//!   attr (def) type = "pop_symbol", symbol = (source-text @id), source_node = @func, is_definition
//!   node body
//!   edge def -> body
//! }
//! ```
//!
//! To implement shadowing (which determines which definitions are selected when multiple are available),
//! you can add a `precedence` attribute to each edge to indicate which paths are prioritized:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "pop_symbol", symbol = (source-text @id), source_node = @func, is_definition
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
//! global ROOT_NODE
//!
//! (function_definition name: (identifier) @id) @func {
//!   node def
//!   attr (def) type = "pop_symbol", symbol = (source-text @id), source_node = @func, is_definition
//!   edge ROOT_NODE -> def
//! }
//! ```
//!
//! ### Attaching debug information to nodes
//!
//! It is possible to attach extra information to nodes for debugging purposes.  This is done by adding
//! `debug_*` attributes to nodes.  Each attribute defines a debug entry, with the key derived from the
//! attribute name, and the value the string representation of the attribute value.  For example, mark
//! a scope node with a kind as follows:
//!
//! ``` skip
//! (function_definition name: (identifier) @id) @func {
//!   ; ...
//!   node param_scope
//!   attr (param_scope) debug_kind = "param_scope"
//!   ; ...
//! }
//! ```
//!
//! ### Working with paths
//!
//! Built-in path functions are available to compute symbols that depend on path information, such as
//! module names or imports. The path of the file is provided in the global variable `FILE_PATH`.
//!
//! The following path functions are available:
//! - `path-dir`: get the path consisting of all but the last component of the argument path, or `#null` if it ends in root
//! - `path-fileext`: get the file extension, i.e. everything after the final `.` of the file name of the argument path, or `#null` if it has extension
//! - `path-filename`: get the last component of the argument path, or `#null` if it has no final component
//! - `path-filestem`: get the file stem of the argument path, i.e., everything before the extension, or `#null` if it has no file name
//! - `path-join`: join all argument paths together
//! - `path-normalize`: normalize the argument path by eliminating `.` and `..` components where possible
//! - `path-split`: split the argument path into a list of its components
//!
//! The following example computes a module name from a file path:
//!
//! ``` skip
//! global FILE_PATH
//!
//! (program)@prog {
//!   ; ...
//!   let dir = (path-dir FILE_PATH)
//!   let stem = (path-filestem FILE_PATH)
//!   let mod_name = (path-join dir stem)
//!   node mod_def
//!   attr mod_def type = "pop_symbol", symbol = mod_name, is_definition, source_node = @prog
//!   ; ...
//! }
//! ```
//!
//! The following example resolves an import relative to the current file:
//!
//! ``` skip
//! global FILE_PATH
//!
//! (import name:(_)@name)@import {
//!   ; ...
//!   let dir = (path-dir FILE_PATH)
//!   let mod_name = (path-normalize (path-join dir (Source-text @name)))
//!   node mod_def
//!   attr mod_def type = "pop_symbol", symbol = mod_name, is_definition, source_node = @prog
//!   ; ...
//! }
//! ```
//!
//! ## Using this crate from Rust
//!
//! If you need very fine-grained control over how to use the resulting stack graphs, you can
//! construct and operate on [`StackGraph`][stack_graphs::graph::StackGraph] instances directly
//! from Rust code.  You will need Rust bindings for the tree-sitter grammar for your source
//! language — for instance, [`tree-sitter-python`][].  Grammar Rust bindings provide a global
//! symbol [`language`][] that you will need.  For this example we assume the source of the stack
//! graph rules is defined in a constant `STACK_GRAPH_RULES`.
//!
//! [`tree-sitter-python`]: https://docs.rs/tree-sitter-python/*/
//! [`language`]: https://docs.rs/tree-sitter-python/*/tree_sitter_python/fn.language.html
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
//! # use tree_sitter_stack_graphs::NoCancellation;
//! #
//! # // This documentation test is not meant to test Python's actual stack graph
//! # // construction rules.  An empty TSG file is perfectly valid (it just won't produce any stack
//! # // graph content).  This minimizes the amount of work that we do when running `cargo test`.
//! # static STACK_GRAPH_RULES: &str = "";
//! #
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let python_source = r#"
//!   import sys
//!   print(sys.path)
//! "#;
//! let grammar = tree_sitter_python::language();
//! let tsg_source = STACK_GRAPH_RULES;
//! let mut language = StackGraphLanguage::from_str(grammar, tsg_source)?;
//! let mut stack_graph = StackGraph::new();
//! let file_handle = stack_graph.get_or_create_file("test.py");
//! let mut globals = Variables::new();
//! language.build_stack_graph_into(&mut stack_graph, file_handle, python_source, &mut globals, &NoCancellation)?;
//! # Ok(())
//! # }
//! ```

use controlled_option::ControlledOption;
use lazy_static::lazy_static;
use lsp_positions::SpanCalculator;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::Node;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use thiserror::Error;
use tree_sitter::Parser;
use tree_sitter_graph::functions::Functions;
use tree_sitter_graph::graph::Graph;
use tree_sitter_graph::graph::GraphNode;
use tree_sitter_graph::graph::GraphNodeRef;
use tree_sitter_graph::graph::Value;
use tree_sitter_graph::parse_error::ParseError;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;
use tree_sitter_graph::ExecutionConfig;
use tree_sitter_graph::Variables;

pub mod functions;
pub mod loader;
pub mod test;

// Node type values
static DROP_SCOPES_TYPE: &'static str = "drop_scopes";
static POP_SCOPED_SYMBOL_TYPE: &'static str = "pop_scoped_symbol";
static POP_SYMBOL_TYPE: &'static str = "pop_symbol";
static PUSH_SCOPED_SYMBOL_TYPE: &'static str = "push_scoped_symbol";
static PUSH_SYMBOL_TYPE: &'static str = "push_symbol";
static SCOPE_TYPE: &'static str = "scope";

// Node attribute names
static DEBUG_ATTR_PREFIX: &'static str = "debug_";
static IS_DEFINITION_ATTR: &'static str = "is_definition";
static IS_EXPORTED_ATTR: &'static str = "is_exported";
static IS_ENDPOINT_ATTR: &'static str = "is_endpoint";
static IS_REFERENCE_ATTR: &'static str = "is_reference";
static SCOPE_ATTR: &'static str = "scope";
static SOURCE_NODE_ATTR: &'static str = "source_node";
static SYMBOL_ATTR: &'static str = "symbol";
static TYPE_ATTR: &'static str = "type";

// Expected attributes per node type
lazy_static! {
    static ref DROP_SCOPES_ATTRS: HashSet<&'static str> = HashSet::from([TYPE_ATTR]);
    static ref POP_SCOPED_SYMBOL_ATTRS: HashSet<&'static str> =
        HashSet::from([TYPE_ATTR, SYMBOL_ATTR, IS_DEFINITION_ATTR]);
    static ref POP_SYMBOL_ATTRS: HashSet<&'static str> =
        HashSet::from([TYPE_ATTR, SYMBOL_ATTR, IS_DEFINITION_ATTR]);
    static ref PUSH_SCOPED_SYMBOL_ATTRS: HashSet<&'static str> =
        HashSet::from([TYPE_ATTR, SYMBOL_ATTR, SCOPE_ATTR, IS_REFERENCE_ATTR]);
    static ref PUSH_SYMBOL_ATTRS: HashSet<&'static str> =
        HashSet::from([TYPE_ATTR, SYMBOL_ATTR, IS_REFERENCE_ATTR]);
    static ref SCOPE_ATTRS: HashSet<&'static str> =
        HashSet::from([TYPE_ATTR, IS_EXPORTED_ATTR, IS_ENDPOINT_ATTR]);
}

// Edge attribute names
static PRECEDENCE_ATTR: &'static str = "precedence";

// Global variables
static ROOT_NODE_VAR: &'static str = "ROOT_NODE";
static JUMP_TO_SCOPE_NODE_VAR: &'static str = "JUMP_TO_SCOPE_NODE";
static FILE_PATH_VAR: &'static str = "FILE_PATH";

/// Holds information about how to construct stack graphs for a particular language
pub struct StackGraphLanguage {
    language: tree_sitter::Language,
    tsg: tree_sitter_graph::ast::File,
    functions: Functions,
    builtins: StackGraph,
}

impl StackGraphLanguage {
    /// Creates a new stack graph language for the given language and
    /// TSG stack graph construction rules.
    pub fn new(
        language: tree_sitter::Language,
        tsg: tree_sitter_graph::ast::File,
    ) -> Result<StackGraphLanguage, LanguageError> {
        debug_assert_eq!(language, tsg.language);
        Ok(StackGraphLanguage {
            language,
            tsg,
            functions: Self::default_functions(),
            builtins: StackGraph::new(),
        })
    }

    /// Creates a new stack graph language for the given language, loading the
    /// TSG stack graph construction rules from a string.
    pub fn from_str(
        language: tree_sitter::Language,
        tsg_source: &str,
    ) -> Result<StackGraphLanguage, LanguageError> {
        let tsg = tree_sitter_graph::ast::File::from_str(language, tsg_source)?;
        Ok(StackGraphLanguage {
            language,
            tsg,
            functions: Self::default_functions(),
            builtins: StackGraph::new(),
        })
    }

    fn default_functions() -> tree_sitter_graph::functions::Functions {
        let mut functions = tree_sitter_graph::functions::Functions::stdlib();
        crate::functions::add_path_functions(&mut functions);
        functions
    }

    pub fn functions_mut(&mut self) -> &mut tree_sitter_graph::functions::Functions {
        &mut self.functions
    }

    pub fn builtins(&self) -> &StackGraph {
        &self.builtins
    }

    pub fn builtins_mut(&mut self) -> &mut StackGraph {
        &mut self.builtins
    }
}

/// An error that can occur while loading in the TSG stack graph construction rules for a language
#[derive(Debug, Error)]
pub enum LanguageError {
    #[error(transparent)]
    ParseError(#[from] tree_sitter_graph::ParseError),
}

impl StackGraphLanguage {
    /// Executes the graph construction rules for this language against a source file, creating new
    /// nodes and edges in `stack_graph`.  Any new nodes that we create will belong to `file`.
    /// (The source file must be implemented in this language, otherwise you'll probably get a
    /// parse error.)
    pub fn build_stack_graph_into<'a>(
        &'a self,
        stack_graph: &'a mut StackGraph,
        file: Handle<File>,
        source: &'a str,
        globals: &'a mut Variables<'a>,
        cancellation_flag: &dyn CancellationFlag,
    ) -> Result<(), LoadError> {
        Builder {
            sgl: self,
            stack_graph,
            file,
            source,
            globals,
        }
        .build(cancellation_flag)
    }

    /// Create a builder that will the graph construction rules for this language against a source
    /// file, creating new nodes and edges in `stack_graph`.  Any new nodes that we create will
    /// belong to `file`.  (The source file must be implemented in this language, otherwise you'll
    /// probably get a parse error.)
    pub fn into_stack_graph_builder<'a>(
        &'a self,
        stack_graph: &'a mut StackGraph,
        file: Handle<File>,
        source: &'a str,
        globals: &'a mut Variables<'a>,
    ) -> Builder<'a> {
        Builder {
            sgl: self,
            stack_graph,
            file,
            source,
            globals,
        }
    }
}

pub struct Builder<'a> {
    sgl: &'a StackGraphLanguage,
    stack_graph: &'a mut StackGraph,
    file: Handle<File>,
    source: &'a str,
    globals: &'a mut Variables<'a>,
}

impl Builder<'_> {
    /// Executes this builder.
    pub fn build(self, cancellation_flag: &dyn CancellationFlag) -> Result<(), LoadError> {
        let mut parser = Parser::new();
        parser.set_language(self.sgl.language)?;
        unsafe { parser.set_cancellation_flag(cancellation_flag.flag()) };
        let tree = parser
            .parse(self.source, None)
            .ok_or(LoadError::ParseError)?;

        let parse_errors = ParseError::into_all(tree);
        if parse_errors.errors().len() > 0 {
            return Err(LoadError::ParseErrors(parse_errors));
        }
        let tree = parse_errors.into_tree();

        let mut graph = Graph::new();
        self.globals
            .add(ROOT_NODE_VAR.into(), graph.add_graph_node().into())
            .expect("Failed to set ROOT_NODE");
        self.globals
            .add(JUMP_TO_SCOPE_NODE_VAR.into(), graph.add_graph_node().into())
            .expect("Failed to set JUMP_TO_SCOPE_NODE");
        self.globals
            .add(
                FILE_PATH_VAR.into(),
                format!("{}", &self.stack_graph[self.file]).into(),
            )
            .expect("Failed to set FILE_PATH");
        let mut config = ExecutionConfig::new(&self.sgl.functions, &self.globals)
            .lazy(true)
            .debug_attributes(
                [DEBUG_ATTR_PREFIX, "tsg_location"].concat().as_str().into(),
                [DEBUG_ATTR_PREFIX, "tsg_variable"].concat().as_str().into(),
            );
        self.sgl.tsg.execute_into(
            &mut graph,
            &tree,
            self.source,
            &mut config,
            &cancellation_flag,
        )?;

        let mut loader = StackGraphLoader::new(self.stack_graph, self.file, &graph, self.source);
        loader.load(cancellation_flag)
    }
}

/// Trait to signal that the execution is cancelled
pub trait CancellationFlag {
    fn flag(&self) -> Option<&AtomicUsize>;
}
impl stack_graphs::CancellationFlag for &dyn CancellationFlag {
    fn check(&self, at: &'static str) -> Result<(), stack_graphs::CancellationError> {
        if self.flag().map_or(0, |f| f.load(Ordering::Relaxed)) != 0 {
            return Err(stack_graphs::CancellationError(at));
        }
        Ok(())
    }
}
impl tree_sitter_graph::CancellationFlag for &dyn CancellationFlag {
    fn check(&self, at: &'static str) -> Result<(), tree_sitter_graph::CancellationError> {
        if self.flag().map_or(0, |f| f.load(Ordering::Relaxed)) != 0 {
            return Err(tree_sitter_graph::CancellationError(at));
        }
        Ok(())
    }
}

pub struct NoCancellation;
impl CancellationFlag for NoCancellation {
    fn flag(&self) -> Option<&AtomicUsize> {
        None
    }
}

/// An error that can occur while loading a stack graph from a TSG file
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("{0}")]
    Cancelled(&'static str),
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
    ExecutionError(tree_sitter_graph::ExecutionError),
    #[error("Error parsing source")]
    ParseError,
    #[error("Error parsing source")]
    ParseErrors(TreeWithParseErrorVec),
    #[error("Error converting shorthand ‘{0}’ on {1} with value {2}")]
    ConversionError(String, String, String),
    #[error(transparent)]
    LanguageError(#[from] tree_sitter::LanguageError),
}

impl From<stack_graphs::CancellationError> for LoadError {
    fn from(value: stack_graphs::CancellationError) -> Self {
        Self::Cancelled(value.0)
    }
}

impl From<tree_sitter_graph::ExecutionError> for LoadError {
    fn from(value: tree_sitter_graph::ExecutionError) -> Self {
        match value {
            tree_sitter_graph::ExecutionError::Cancelled(err) => Self::Cancelled(err.0),
            err => Self::ExecutionError(err),
        }
    }
}

struct StackGraphLoader<'a> {
    stack_graph: &'a mut StackGraph,
    file: Handle<File>,
    graph: &'a Graph<'a>,
    source: &'a str,
    span_calculator: SpanCalculator<'a>,
    remapped_local_ids: HashMap<usize, NodeID>,
}

impl<'a> StackGraphLoader<'a> {
    fn new(
        stack_graph: &'a mut StackGraph,
        file: Handle<File>,
        graph: &'a Graph<'a>,
        source: &'a str,
    ) -> Self {
        let span_calculator = SpanCalculator::new(source);

        // By default graph ids are used for stack graph local_ids. A remapping is computed
        // for local_ids that already exist in the graph---all other graph ids are mapped to
        // the same local_id.
        let mut remapped_local_ids = HashMap::new();
        remapped_local_ids.insert(0usize, NodeID::root());
        remapped_local_ids.insert(1usize, NodeID::jump_to());
        // remap beyond the range of graph nodes
        let mut next_local_id = (graph.node_count() as u32) - 2;
        for node in stack_graph.nodes_for_file(file) {
            let local_id = stack_graph[node].id().local_id();
            let index = (local_id as usize) + 2;
            // find next available id for which no stack graph node exists yet
            while stack_graph
                .node_for_id(NodeID::new_in_file(file, next_local_id))
                .is_some()
            {
                next_local_id += 1;
            }
            // remap occupied local_id to next available id
            remapped_local_ids.insert(index, NodeID::new_in_file(file, next_local_id));
        }

        StackGraphLoader {
            stack_graph,
            file,
            graph,
            source,
            span_calculator,
            remapped_local_ids,
        }
    }
}

impl<'a> StackGraphLoader<'a> {
    fn load(&mut self, cancellation_flag: &dyn CancellationFlag) -> Result<(), LoadError> {
        let cancellation_flag: &dyn stack_graphs::CancellationFlag = &cancellation_flag;

        // First create a stack graph node for each TSG node.  (The skip(2) is because the first
        // two DSL nodes that we create are the proxies for the stack graph's “root” and “jump to
        // scope” nodes.)
        for node_ref in self.graph.iter_nodes().skip(2) {
            cancellation_flag.check("loading graph nodes")?;
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
            self.load_debug_info(node, handle)?;
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
                cancellation_flag.check("loading graph edges")?;
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
    fn node_id_for_graph_node(&mut self, node_ref: GraphNodeRef) -> NodeID {
        let index = node_ref.index();
        self.remapped_local_ids.get(&index).map_or_else(
            || NodeID::new_in_file(self.file, (index as u32) - 2),
            |id| *id,
        )
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
        self.verify_attributes(node, POP_SCOPED_SYMBOL_TYPE, &POP_SCOPED_SYMBOL_ATTRS);
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
        self.verify_attributes(node, POP_SYMBOL_TYPE, &POP_SYMBOL_ATTRS);
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
        self.verify_attributes(node, PUSH_SCOPED_SYMBOL_TYPE, &PUSH_SCOPED_SYMBOL_ATTRS);
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
        self.verify_attributes(node, PUSH_SYMBOL_TYPE, &PUSH_SYMBOL_ATTRS);
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
        self.verify_attributes(node, SCOPE_TYPE, &SCOPE_ATTRS);
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

    fn load_debug_info(
        &mut self,
        node: &GraphNode,
        node_handle: Handle<Node>,
    ) -> Result<(), LoadError> {
        for (name, value) in node.attributes.iter() {
            let name = name.to_string();
            if name.starts_with(DEBUG_ATTR_PREFIX) {
                let value = match value {
                    Value::String(value) => value.clone(),
                    value => value.to_string(),
                };
                let key = self
                    .stack_graph
                    .add_string(&name[DEBUG_ATTR_PREFIX.len()..]);
                let value = self.stack_graph.add_string(&value);
                self.stack_graph.debug_info_mut(node_handle).add(key, value);
            }
        }
        Ok(())
    }

    fn verify_attributes(
        &self,
        node: &GraphNode,
        node_type: &str,
        allowed_attributes: &HashSet<&'static str>,
    ) {
        for (id, _) in node.attributes.iter() {
            let id = id.as_str();
            if !allowed_attributes.contains(id)
                && id != SOURCE_NODE_ATTR
                && !id.starts_with(DEBUG_ATTR_PREFIX)
            {
                eprintln!("Unexpected attribute {} on node of type {}", id, node_type);
            }
        }
    }
}
