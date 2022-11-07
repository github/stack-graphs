# languages

This directory contains stack graphs definitions for specific languages.

The project name of each definition reflects the tool that is targeted and the language name. For example, `tree-sitter-stack-graphs-typescript` is the definition for TypeScript that is suitable for `tree-sitter-stack-graphs`. This name is used for the directory containing the definition, as well as any packages it defines.

## Versioning

Stack graphs definitions are versioned indepdently of the grammar version they depend on. The following adapted semantic versioning scheme is used. Given a version number MAJOR.MINOR.PATCH, increment the:

1. MAJOR version if the resulting stack graphs are incompatible with graphs produced by the previous version.

   This is the case if, given the set of resolutions in a combination of graphs produced by the previous version, some of these resolutions are missing when some of the graphs are replaced with the graph produced by the new version.

2. MINOR version if the stack graphs produced by the new version are compatible with graphs produced by the old version.

   This is the case if, given the set of resolutions in a combination of graphs produced by the previous version, all pre-existing resolutions remain when some of the graphs are replaced with the graph produced by the new version. The new graphs may result in more resolutions.

3. PATCH for bug fixes.

Definitions on major version zero (0.x.y) should increment the minor version when changes are incompatible with the previous version.

Versioned releases should only depend on versioned releases of the grammars they use.
