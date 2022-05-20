# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## stack-graphs 0.8.0 - 2022-05-19

### Added

- Source info field `definiens_span` for the source span that corresponds to the definiens of a function declaration (i.e. the body of a function declaration, not an assignment).

## stack-graphs 0.7.3 -- 2022-05-17

### Fixed

- We no longer add divergent partial paths to a `Database`.  A divergent partial
  path starts at the root node and has an empty symbol stack precondition.  That
  empty precondition means that it can be concatenated to _any_ path that
  currently ends at the root node — including the result of that concatenation!
  That gives us a divergence, since we can continually prepend the path's
  postcondition to the current symbol stack, forever.  To avoid this divergence,
  we skip these partial paths when constructing a database.

## stack-graphs 0.7.2 -- 2022-05-09

### Added

- Method `StackGraph::add_from_graph` to copy the one graph into another.

## stack-graphs 0.7.1 - 2022-04-19

### Fixes

- Resolves build problems of version 0.7.0

## stack-graphs 0.7.0 - 2022-04-19

### Added

- The module `stack_graphs::assert` defines assertions that can be run against a
  stack graph to test resolution behavior.

- The module `stack_graphs::json` defines JSON rendering of stack graphs and paths.
  This module requires the `json` feature.

- The module `stack_graphs::visualization` defines rendering an interactive HTML
  visualization of stack graphs.  This module requires the `json` feature.

- Stack graph nodes can have associated `DebugInfo`, consisting of key-value pairs
  of strings.

### Changed

- Internal and exported scopes as separate node types are removed and replaced by
  a single scope node type. Whether a scope is exported is indicated by a boolean
  attribute.

  The `Node::{Exported,Internal}Scope` are replaced by a single `Node::Scope`, and
  their implementation types `{Exported,Internal}ScopeNode` are replaced by the type
  `ScopeNode`. The `ScopeNode::is_exported` field indicates whether a scope is
  exported or internal. The `StackGraph::add_{internal,exported}_scope_node` methods
  are replaced by `StackGraph::add_scope_node`.

  In the C API, the enum values `sg_node_kind::SG_NODE_KIND_EXPORTED_SCOPE` and
  `sg_node_kind:SG_NODE_KIND_INTERNAL_SCOPE` are replaced by a single value
  `sg_node_kind::SG_NODE_KIND_SCOPE`.  The field `sg_node::is_clickable` is renamed
  to `sg_node::is_endpoint`.

### Removed

### Deprecated

## stack-graphs 0.6.0 - 2022-03-03

### Added

- The new `ForwardPartialPathStitcher::from_partial_paths` constructor lets you
  seed a path stitching search with a specific list of partial paths (which need
  not be in the `Database`).  This can be used, for instance, to implement “find
  qualified definitions”, where we look for any definition of a fully qualified
  name (expressed as a symbol stack).  There is a new test case that shows an
  example implementation.

### Changed

- The `ForwardPartialPathStitcher::new` constructor has been renamed to
  `from_nodes`, to be more consistent with the new `from_partial_paths`
  constructor.

## stack-graphs 0.5.0 - 2022-02-17

### Added

- Added the ability to bound the amount of work performed in each phase of the
  path stitching algorithm.  Exposed in the C API via the following functions:

  - `sg_forward_path_stitcher_set_max_work_per_phase`
  - `sg_forward_partial_path_stitcher_set_max_work_per_phase`

- Added a `grapheme_offset` field to the `sg_offset` type in the C API, to track
  grapheme positions along with UTF-8 and UTF-16 positions.
