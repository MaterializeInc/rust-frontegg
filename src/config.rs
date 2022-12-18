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

use crate::Client;

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
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        ClientBuilder {
            vendor_endpoint: DEFAULT_VENDOR_ENDPOINT.clone(),
        }
    }
}

impl ClientBuilder {
    /// Creates a [`Client`] that incorporates the optional parameters
    /// configured on the builder and the specified required parameters.
    pub fn build(self, config: ClientConfig) -> Client {
        let inner = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Client {
            inner,
            client_id: config.client_id,
            secret_key: config.secret_key,
            vendor_endpoint: self.vendor_endpoint,
            auth: Default::default(),
        }
    }
}
