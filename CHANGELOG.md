# Changelog

All notable changes to this project are documented in this file, based on [Keep a Changelog][keepachangelog].


## [Unreleased]


## [0.2.2] - 2024-09-18


## [0.2.1] - 2024-03-16
### Fixed
- Prevent infinite recursion by checking that trait objects implement `erased_serde::Serialize`, using the same trick 
  from the `erased_serde::serialize_trait_object` macro.


## [0.2.0] - 2024-02-03
### Changed
- `Registry` is now a trait, with a single built-in implementation: `MapRegistry`.
- The type of identifiers is now generic, defaulting to `&'static str`.
- `deserialize_trait_object` is now a method on `Registry`.

### Added
- Added convenience `register_type` and `register_id_type` methods to `Registry`.


# References

[Unreleased]: https://github.com/Gohla/serde_flexitos/compare/release/0.2.2...HEAD
[0.2.2]: https://github.com/Gohla/serde_flexitos/compare/release/0.2.1...release/0.2.2
[0.2.1]: https://github.com/Gohla/serde_flexitos/compare/release/0.2.0...release/0.2.1
[0.2.0]: https://github.com/Gohla/serde_flexitos/compare/release/0.1.0...release/0.2.0
[0.1.0]: https://github.com/Gohla/serde_flexitos/compare/...release/0.1.0

[keepachangelog]: https://keepachangelog.com/en/1.0.0/
