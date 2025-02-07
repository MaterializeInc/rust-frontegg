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

use std::time::{Duration, SystemTime};

use reqwest::{Method, Url};
use reqwest_middleware::{ClientWithMiddleware, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::util::RequestBuilderExt;

use crate::error::ApiError;
use crate::{ClientBuilder, ClientConfig, Error};

pub mod roles;
pub mod tenants;
pub mod users;

const AUTH_VENDOR_PATH: [&str; 2] = ["auth", "vendor"];

/// An API client for Frontegg.
///
/// The API client is designed to be wrapped in an [`Arc`] and used from
/// multiple threads simultaneously. A successful authentication response is
/// shared by all threads.
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug)]
pub struct Client {
    pub(crate) client_retryable: ClientWithMiddleware,
    pub(crate) client_non_retryable: ClientWithMiddleware,
    pub(crate) client_id: String,
    pub(crate) secret_key: String,
    pub(crate) vendor_endpoint: Url,
    pub(crate) auth: Mutex<Option<Auth>>,
}

impl Client {
    /// Creates a new `Client` from its required configuration parameters.
    pub fn new(config: ClientConfig) -> Client {
        ClientBuilder::default().build(config)
    }

    /// Creates a builder for a `Client` that allows for customization of
    /// optional parameters.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    fn build_request<P>(&self, method: Method, path: P) -> RequestBuilder
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let mut url = self.vendor_endpoint.clone();
        url.path_segments_mut()
            .expect("builder validated URL can be a base")
            .clear()
            .extend(path);
        match method {
            // GET and HEAD requests are idempotent and we can safely retry
            // them without fear of duplicating data.
            Method::GET | Method::HEAD => self.client_retryable.request(method, url),
            // All other requests are assumed to be mutating and therefore
            // we leave it to the caller to retry them.
            _ => self.client_non_retryable.request(method, url),
        }
    }

    async fn send_request<T>(&self, req: RequestBuilder) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let token = self.ensure_authenticated().await?;
        let req = req.bearer_auth(token);
        self.send_unauthenticated_request(req).await
    }

    async fn send_unauthenticated_request<T>(&self, req: RequestBuilder) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ErrorResponse {
            #[serde(default)]
            message: Option<String>,
            #[serde(default)]
            errors: Vec<String>,
        }

        let res = req.send().await?;
        let status_code = res.status();
        if status_code.is_success() {
            Ok(res.json().await?)
        } else {
            match res.json::<ErrorResponse>().await {
                Ok(e) => {
                    let mut messages = e.errors;
                    messages.extend(e.message);
                    Err(Error::Api(ApiError {
                        status_code,
                        messages,
                    }))
                }
                Err(_) => Err(Error::Api(ApiError {
                    status_code,
                    messages: vec!["unable to decode error details".into()],
                })),
            }
        }
    }

    async fn ensure_authenticated(&self) -> Result<String, Error> {
        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct AuthenticationRequest<'a> {
            client_id: &'a str,
            secret: &'a str,
        }

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AuthenticationResponse {
            token: String,
            expires_in: u64,
        }

        let mut auth = self.auth.lock().await;
        match &*auth {
            Some(auth) if SystemTime::now() < auth.refresh_at => {
                return Ok(auth.token.clone());
            }
            _ => (),
        }
        let req = self.build_request(Method::POST, AUTH_VENDOR_PATH);
        let req = req.json(&AuthenticationRequest {
            client_id: &self.client_id,
            secret: &self.secret_key,
        });
        let res: AuthenticationResponse = self.send_unauthenticated_request(req).await?;
        *auth = Some(Auth {
            token: res.token.clone(),
            // Refresh twice as frequently as we need to, to be safe.
            refresh_at: SystemTime::now() + (Duration::from_secs(res.expires_in) / 2),
        });
        Ok(res.token)
    }
}

#[derive(Debug, Clone)]
pub struct Auth {
    token: String,
    refresh_at: SystemTime,
}
