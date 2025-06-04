# Rustest's Changelog

## [0.3.0] - 2025-06-04

### Added

- New `TempDir` fixture
- New `MatrixUnique` scope (use "matrix" in macro fixture definition)
- Allow custom visibility for Param
- Allow user to give a explicit name for the params
- Various improvements of the CI and github tools (thanks to @rursprung)

### Changed
- `Unique`("unique") scope is now called `Once`("once").
- Introduce `ParamName` trait. User should implement `ParamName` instead of `TestName` when needed.
- Test functions do not need to be `UnwindSafe`. We assume they are with `AssertUnwindSafe`.

### Fixed

- Fix "unique" fixture to not share states.
- Fix `TempFile` fixture to not share states.
- Fix `Global` fixture, to cache the subfixture (as unique scope do not share scope).

## [0.2.0] - 2025-05-19

- Support unittest.
- Better testing of rustest
- Add support for googletest matchers
- Add documentation
- Introduce 2 steps setup
  . Tests are setup at first step, without fixture creation.
  . Fixtures are created just before tests run.
- TestName (formelly FixtureName) only have to be defined (and are automatically) for Param.
  User types don't need to impl TestName
- Fix definining fixtures in sub-modules
- Introduce new crate rustest-fixtures, a collection of standard fixtures.
- Add support for (conditional) ignore test.
- Lot of code improvements.


[0.3.0] https://github.com/mgautierfr/rustest/compare/0.2.0...0.3.0
[0.2.0] https://github.com/mgautierfr/rustest/compare/0.1.0...0.2.0
