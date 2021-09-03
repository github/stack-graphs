// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

pub mod test_graphs;

mod arena;
mod c;
mod can_create_graph;
mod can_find_all_references_with_reverse_partial_path_stitching;
mod can_find_partial_paths_from_root_in_database;
mod can_find_partial_paths_from_start_node_in_database;
mod can_find_partial_paths_in_file;
mod can_find_partial_paths_to_end_node_in_database;
mod can_find_partial_paths_to_jump_to_in_database;
mod can_find_partial_paths_to_root_in_database;
mod can_jump_to_definition;
mod can_jump_to_definition_with_forward_partial_path_stitching;
mod can_jump_to_definition_with_forward_path_stitching;
mod graph;
mod partial;
mod paths;
