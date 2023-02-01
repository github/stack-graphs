use serde::Deserialize;
use serde::Serialize;

pub struct FilteredStackGraph {
    files: FilteredFiles,
    nodes: FilteredNodes,
    // edges: FilteredEdges,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilteredFiles {
    data: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilteredNodes {
    data: Vec<FilteredNode>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FilteredNode {
    #[serde(rename = "drop_scopes")]
    DropScopes(FilteredDropScopesNode),
    // #[serde(rename = "jump_to")]
    // JumpTo(FilteredJumpToNode),

    // #[serde(rename = "pop_scoped_symbol")]
    // PopScopedSymbol(FilteredPopScopedSymbolNode),

    // #[serde(rename = "pop_symbol")]
    // PopSymbol(FilteredPopSymbolNode),

    // #[serde(rename = "push_scoped_symbol")]
    // PushScopedSymbol(FilteredPushScopedSymbolNode),

    // #[serde(rename = "push_symbol")]
    // PushSymbol(FilteredPushSymbolNode),

    // #[serde(rename = "root")]
    // Root(FilteredRootNode),

    // #[serde(rename = "scope")]
    // Scope(FilteredScopeNode),
}

#[derive(Serialize, Deserialize)]
pub struct FilteredNodeID {
    file: String,
    local_id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct FilteredDropScopesNode {
    id: FilteredNodeID,
    // source_info: FilteredSourceInfo
    // debug_info: FilteredDebugInfo
}
