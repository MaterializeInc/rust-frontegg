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

//! Integration tests.
//!
//! To run these tests, you must create a Frontegg workspace and provide the
//! vendor credentials via `FRONTEGG_CLIENT_ID` and `FRONTEGG_SECRET_KEY`.
//!
//! These tests must be run serially, as via
//!
//!     $ cargo test -- --test-threads=1
//!
//! because each test competes for access to the same test Frontegg workspace.

use std::collections::HashSet;
use std::env;
use std::time::Duration;

use futures::stream::TryStreamExt;
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use reqwest_retry::policies::ExponentialBackoff;
use serde_json::json;
use test_log::test;
use tracing::info;
use uuid::Uuid;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

use frontegg::{ApiError, Client, ClientConfig, Error, TenantRequest, UserListConfig, UserRequest};

pub static CLIENT_ID: Lazy<String> =
    Lazy::new(|| env::var("FRONTEGG_CLIENT_ID").expect("missing FRONTEGG_CLIENT_ID"));
pub static SECRET_KEY: Lazy<String> =
    Lazy::new(|| env::var("FRONTEGG_SECRET_KEY").expect("missing FRONTEGG_SECRET_KEY"));

const TENANT_NAME_PREFIX: &str = "test tenant";

fn new_client() -> Client {
    Client::new(ClientConfig {
        client_id: CLIENT_ID.clone(),
        secret_key: SECRET_KEY.clone(),
    })
}

async fn delete_existing_tenants(client: &Client) {
    for tenant in client.list_tenants().await.unwrap() {
        if tenant.name.starts_with(TENANT_NAME_PREFIX) {
            info!(%tenant.id, "deleting existing tenant");
            client.delete_tenant(tenant.id).await.unwrap();
        }
    }
}

/// Tests that errors are retried automatically by the client for read API calls
/// but not for write API calls.
#[test(tokio::test)]
async fn test_retries_with_mock_server() {
    // Start a mock Frontegg API server and a client configured to target that
    // server. The retry policy disables backoff to speed up the tests.
    const MAX_RETRIES: u32 = 3;
    let server = MockServer::start().await;
    let client = Client::builder()
        .with_vendor_endpoint(server.uri().parse().unwrap())
        .with_retry_policy(
            ExponentialBackoff::builder()
                .retry_bounds(Duration::from_millis(1), Duration::from_millis(1))
                .build_with_max_retries(MAX_RETRIES),
        )
        .build(ClientConfig {
            client_id: "".into(),
            secret_key: "".into(),
        });

    // Register authentication handler.
    let mock = Mock::given(matchers::path("/auth/vendor"))
        .and(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{\"token\":\"test\", \"expiresIn\":2687784526}"),
        )
        .expect(1)
        .named("auth");
    server.register(mock).await;

    // Register a mock for the `get_tenant` call that returns a 429 response
    // code and ensure the client repeatedly retries the API call until giving
    // up after `MAX_RETRIES` retries and returning the error.
    let mock = Mock::given(matchers::method("GET"))
        .and(matchers::path_regex("/tenants/.*"))
        .respond_with(ResponseTemplate::new(429))
        .expect(u64::from(MAX_RETRIES) + 1)
        .named("get tenants");
    server.register(mock).await;
    let res = client.get_tenant(Uuid::new_v4()).await;
    assert!(res.is_err());

    // Register a mock for the `create_tenant` call that returns a 429 response
    // code and ensure the client only tries the API call once.
    let mock = Mock::given(matchers::method("POST"))
        .and(matchers::path_regex("/tenants/.*"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .named("post tenants");
    server.register(mock).await;
    let _ = client
        .create_tenant(&TenantRequest {
            id: Uuid::new_v4(),
            name: &format!("{TENANT_NAME_PREFIX} 1"),
            metadata: json!({
                "tenant_number": 1,
            }),
        })
        .await;
}

/// Tests basic functionality of creating and retrieving tenants and users.
#[test(tokio::test)]
async fn test_tenants_and_users() {
    // Set up.
    let client = new_client();
    delete_existing_tenants(&client).await;

    // Create two tenants.
    let tenant_id_1 = Uuid::new_v4();
    let tenant_id_2 = Uuid::new_v4();
    client
        .create_tenant(&TenantRequest {
            id: tenant_id_1,
            name: &format!("{TENANT_NAME_PREFIX} 1"),
            metadata: json!({
                "tenant_number": 1,
            }),
        })
        .await
        .unwrap();
    client
        .create_tenant(&TenantRequest {
            id: tenant_id_2,
            name: &format!("{TENANT_NAME_PREFIX} 2"),
            metadata: json!(42),
        })
        .await
        .unwrap();

    // Verify tenant properties.
    let mut tenants: Vec<_> = client
        .list_tenants()
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.name.starts_with(TENANT_NAME_PREFIX))
        .collect();
    // Sort tenants by name to match order. Default ordering is by tenant ID.
    tenants.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(tenants.len(), 2);
    assert_eq!(tenants[0].id, tenant_id_1);
    assert_eq!(tenants[1].id, tenant_id_2);
    assert_eq!(tenants[0].name, format!("{TENANT_NAME_PREFIX} 1"));
    assert_eq!(tenants[1].name, format!("{TENANT_NAME_PREFIX} 2"));
    assert_eq!(tenants[0].metadata, json!({"tenant_number": 1}));
    assert_eq!(tenants[1].metadata, json!(42));
    assert_eq!(tenants[0].deleted_at, None);
    assert_eq!(tenants[1].deleted_at, None);

    // Verify a single tenant can be fetched by ID
    let tenant = client.get_tenant(tenants[0].id).await.unwrap();
    assert_eq!(tenant.id, tenants[0].id);

    // Verify an unknown tenant raises a suitable error
    let tenant_result = client
        .get_tenant(uuid::uuid!("00000000-0000-0000-0000-000000000000"))
        .await;
    match tenant_result {
        Err(Error::Api(ApiError { status_code, .. })) if status_code == StatusCode::NOT_FOUND => (),
        _ => panic!("unexpected response: {tenant_result:?}"),
    };

    // Create three users in each tenant.
    let mut users = vec![];
    for (tenant_idx, tenant) in tenants.iter().enumerate() {
        for user_idx in 0..3 {
            let name = format!("user-{tenant_idx}-{user_idx}");
            let email = format!("frontegg-test-{tenant_idx}-{user_idx}@example.com");
            let created_user = client
                .create_user(&UserRequest {
                    tenant_id: tenant.id,
                    name: &name,
                    email: &email,
                    skip_invite_email: true,
                    ..Default::default()
                })
                .await
                .unwrap();

            // Verify that the API has roundtripped the key properties.
            assert_eq!(created_user.name, name);
            assert_eq!(created_user.email, email);

            // Verify that fetching the same user by ID from the API returns
            // the same properties.
            let user = client.get_user(created_user.id).await.unwrap();
            assert_eq!(created_user.id, user.id);
            assert_eq!(user.name, name);
            assert_eq!(user.email, email);
            assert_eq!(user.tenants.len(), 1);
            assert_eq!(user.tenants[0].tenant_id, tenant.id);

            users.push(user);
        }
    }

    // Ensure that listing users works for a variety of page sizes.
    for page_size in [1, 2, 10] {
        let expected: HashSet<_> = users.iter().map(|u| u.id).collect();
        let actual: HashSet<_> = client
            .list_users(UserListConfig::default().page_size(page_size))
            .map_ok(|u| u.id)
            .try_collect()
            .await
            .unwrap();
        assert!(expected.difference(&actual).collect::<Vec<_>>().is_empty());
    }

    // Ensure that the user list can be filtered to a single tenant.
    {
        let expected: HashSet<_> = users.iter().take(3).map(|u| u.id).collect();
        let actual: HashSet<_> = client
            .list_users(UserListConfig::default().tenant_id(tenant_id_1))
            .map_ok(|u| u.id)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(expected, actual);
    }

    // Delete all users;
    for user in &users {
        client.delete_user(user.id).await.unwrap();
    }

    // Verify that users are really gone.
    {
        let users: Vec<_> = client
            .list_users(Default::default())
            .try_collect::<Vec<_>>()
            .await
            .unwrap()
            .into_iter()
            .filter(|u| u.email.starts_with("frontegg-test-"))
            .collect();
        assert_eq!(users.len(), 0);
    }
}
