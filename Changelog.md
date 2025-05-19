# Rustest 0.2.0

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
