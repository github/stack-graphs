# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## stack-graphs 0.5.0 - 2022-02-17

### Added

- Added the ability to bound the amount of work performed in each phase of the
  path stitching algorithm.  Exposed in the C API via the following functions:

  - `sg_forward_path_stitcher_set_max_work_per_phase`
  - `sg_forward_partial_path_stitcher_set_max_work_per_phase`

- Added a `grapheme_offset` field to the `sg_offset` type in the C API, to track
  grapheme positions along with UTF-8 and UTF-16 positions.
