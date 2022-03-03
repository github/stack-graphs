# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
