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

use std::time::Duration;

use once_cell::sync::Lazy;
use reqwest::Url;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;

use crate::client::Client;

pub static DEFAULT_VENDOR_ENDPOINT: Lazy<Url> = Lazy::new(|| {
    "https://api.frontegg.com"
        .parse()
        .expect("url known to be valid")
});

/// Configures the required parameters of a [`Client`].
pub struct ClientConfig {
    /// The client ID for the vendor to authenticate as.
    pub client_id: String,
    /// The secret key for the vendor to authenticate as.
    pub secret_key: String,
}

/// A builder for a [`Client`].
pub struct ClientBuilder {
    vendor_endpoint: Url,
    retry_policy: Option<ExponentialBackoff>,
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        ClientBuilder {
            vendor_endpoint: DEFAULT_VENDOR_ENDPOINT.clone(),
            retry_policy: Some(
                ExponentialBackoff::builder()
                    .retry_bounds(Duration::from_millis(100), Duration::from_secs(3))
                    .build_with_max_retries(5),
            ),
        }
    }
}

impl ClientBuilder {
    /// Sets the policy for retrying failed read-only API calls.
    ///
    /// Note that the created [`Client`] will retry only read-only API calls,
    /// like [`get_tenant`](Client::get_tenant), but not mutating API calls,
    /// like [`create_user`](Client::create_user).
    pub fn with_retry_policy(mut self, policy: ExponentialBackoff) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Sets the vendor endpoint.
    pub fn with_vendor_endpoint(mut self, endpoint: Url) -> Self {
        self.vendor_endpoint = endpoint;
        self
    }

    /// Creates a [`Client`] that incorporates the optional parameters
    /// configured on the builder and the specified required parameters.
    pub fn build(self, config: ClientConfig) -> Client {
        let client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Client {
            client_retryable: match self.retry_policy {
                Some(policy) => reqwest_middleware::ClientBuilder::new(client.clone())
                    .with(RetryTransientMiddleware::new_with_policy(policy))
                    .build(),
                None => reqwest_middleware::ClientBuilder::new(client.clone()).build(),
            },
            client_non_retryable: reqwest_middleware::ClientBuilder::new(client).build(),
            client_id: config.client_id,
            secret_key: config.secret_key,
            vendor_endpoint: self.vendor_endpoint,
            auth: Default::default(),
        }
    }
}
