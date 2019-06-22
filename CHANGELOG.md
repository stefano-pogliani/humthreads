# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Fixed
- Warnings about trait objects without `dyn`.

## [0.1.4] - 2019-06-22
### Added
- Mock `ThreadScope` for writing tests.
- New `with_test_support` feature to enable test utils.

## [0.1.3] - 2019-05-07
### Added
- Wait for thread exit events using crossbeam-channel `Select` interface.

### Changed
- Convert handle's join methods to immutable references to the handle itself.

## [0.1.2] - 2019-05-05
### Added
- Mappable thread handles to transform `join` results.

## [0.1.1] - 2019-04-09
### Changed
- Fix `name`/`short_name` swap in thread status.

## 0.1.0 - 2019-04-01
### Added
- Join with timeout.
- Threads can report current activity.
- Threads introspection.
- Threads spawning and joining.


[Unreleased]: https://github.com/stefano-pogliani/humthreads/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/stefano-pogliani/humthreads/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/stefano-pogliani/humthreads/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/stefano-pogliani/humthreads/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/stefano-pogliani/humthreads/compare/v0.1.0...v0.1.1
