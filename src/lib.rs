// Copyright Materialize, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! [<img src="https://user-images.githubusercontent.com/23521087/168297221-5d346edc-3a55-4055-b355-281b4bd76963.png" width=180 align=right>](https://materialize.com)
//! An async API client for the Frontegg user management service.
//!
//! # Maintainership
//!
//! This is not an official Frontegg product. This crate is developed by
//! [Materialize], the streaming data warehouse. Contributions are encouraged:
//!
//! * [View source code](https://github.com/MaterializeInc/rust-frontegg)
//! * [Report an issue](https://github.com/MaterializeInc/rust-frontegg/issues/new)
//! * [Submit a pull request](https://github.com/MaterializeInc/rust-frontegg/compare)
//!
//! [Materialize]: https://materialize.com
//!
//! # See also
//!
//! Additional information is available in the [official Frontegg API
//! documentation][official-api-docs].
//!
//! [official-api-docs]: https://docs.frontegg.com/reference/getting-started-with-your-api

#[warn(missing_debug_implementations, missing_docs)]
mod client;
mod config;
mod error;
mod serde;
mod util;

pub use client::roles::{Permission, Role};
pub use client::tenants::{Tenant, TenantRequest};
pub use client::users::{CreatedUser, User, UserListConfig, UserRequest, WebhookUser};
pub use client::Client;
pub use config::{ClientBuilder, ClientConfig};
pub use error::Error;
