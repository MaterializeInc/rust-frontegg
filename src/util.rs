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

use std::fmt;
use std::iter;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest_middleware::RequestBuilder;
use serde::Serialize;
use uuid::Uuid;

pub trait RequestBuilderExt {
    fn tenant(self, uuid: Uuid) -> RequestBuilder;
    fn json<T: Serialize + ?Sized>(self, json: &T) -> RequestBuilder;
}

impl RequestBuilderExt for RequestBuilder {
    fn tenant(self, uuid: Uuid) -> RequestBuilder {
        self.header(
            "Frontegg-Tenant-Id",
            HeaderValue::from_str(&uuid.to_string())
                .expect("UUID should always be valid header value"),
        )
    }

    fn json<T: Serialize + ?Sized>(self, json: &T) -> RequestBuilder {
        // Serialize the JSON payload
        let body = serde_json::to_vec(json).expect("Failed to serialize JSON payload");

        // Create headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add the body and headers
        self.headers(headers).body(body)
    }
}

pub trait StrIteratorExt {
    fn chain_one<S>(self, s: S) -> Vec<String>
    where
        S: fmt::Display;
}

impl<T> StrIteratorExt for T
where
    T: IntoIterator,
    T::Item: AsRef<str>,
{
    fn chain_one<S>(self, s: S) -> Vec<String>
    where
        S: fmt::Display,
    {
        self.into_iter()
            .map(|s| s.as_ref().into())
            .chain(iter::once(s.to_string()))
            .collect()
    }
}
