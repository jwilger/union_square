# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/jwilger/union_square/compare/v0.1.1...v0.1.2) - 2025-07-26

### Added

- implement AWS Bedrock provider handler (issue #28) ([#136](https://github.com/jwilger/union_square/pull/136))
- implement performance benchmarking framework ([#127](https://github.com/jwilger/union_square/pull/127))
- implement Tower middleware stack for proxy ([#125](https://github.com/jwilger/union_square/pull/125))
- improve GitHub Pages website UX and release display ([#121](https://github.com/jwilger/union_square/pull/121))
- adjust logo glow to have white background inside shape ([#119](https://github.com/jwilger/union_square/pull/119))
- add white glow effect to logo SVG ([#116](https://github.com/jwilger/union_square/pull/116))

### Fixed

- *(ci)* use RELEASE_PLZ_PAT for GITHUB_TOKEN in release-plz workflow ([#135](https://github.com/jwilger/union_square/pull/135))
- *(ci)* use correct secret name RELEASE_PLZ_PAT in workflow ([#133](https://github.com/jwilger/union_square/pull/133))
- *(ci)* use RELEASE_PLZ_TOKEN in checkout action to trigger workflows ([#132](https://github.com/jwilger/union_square/pull/132))
- use RELEASE_PLZ_TOKEN to trigger workflows on release PRs ([#131](https://github.com/jwilger/union_square/pull/131))
- status and date on ADR-0007 ([#129](https://github.com/jwilger/union_square/pull/129))
- correct ADR dates to match actual commit dates ([#124](https://github.com/jwilger/union_square/pull/124))
- correct logo white background to fill only inside the rounded square ([#120](https://github.com/jwilger/union_square/pull/120))

### Other

- standardize ADR naming convention with numbered titles ([#128](https://github.com/jwilger/union_square/pull/128))
- accept ADR-0017 and improve ADR documentation ([#123](https://github.com/jwilger/union_square/pull/123))
- update log4brains from 1.0.1 to 1.1.0 ([#122](https://github.com/jwilger/union_square/pull/122))
- remove logo backgrounds and increase logo sizes by 25% ([#118](https://github.com/jwilger/union_square/pull/118))

## [0.1.1](https://github.com/jwilger/union_square/compare/v0.1.0...v0.1.1) - 2025-07-24

### Other

- Trigger release pr build on main ([#115](https://github.com/jwilger/union_square/pull/115))
