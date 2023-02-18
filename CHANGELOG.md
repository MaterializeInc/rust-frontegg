# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic
Versioning].

<!-- #release:next-header -->

## [Unreleased] <!-- #release:date -->

## [0.2.0] - 2023-02-18

* Add the `WebhookUser` and `WebhookTenantBinding` to represent the user object
  delivered by the `frontegg.user.*` webhook events types.

## [0.1.1] - 2022-12-23

* Imbue `Error` and `ApiError` with an `std::error::Error` implementation.
* Handle API responses that do not include the `metadata` field in `Tenant`,
  `User`, and `CreatedUser` responses.

## 0.1.0 - 2022-12-18

Initial release.

<!-- #release:next-url -->
[Unreleased]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.0...v0.1.0

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
