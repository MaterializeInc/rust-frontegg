# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic
Versioning].

<!-- #release:next-header -->

## [Unreleased] <!-- #release:date -->

* Automatically retry read-only HTTP requests.

## [0.3.0] - 2023-02-26

* Add the `Client::get_tenant` method to get a tenant by ID.

* Uniformly derive `Serialize` and `Deserialize` on all API types, even if the
  type is not serialized or deserialized by `Client`. The idea is to allow
  downstream users to serialize and deserialize these types for their own
  purposes (e.g., to store them on disk).

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
[Unreleased]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MaterializeInc/rust-frontegg/compare/v0.1.0...v0.1.0

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
