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

use futures::stream::TryStreamExt;
use once_cell::sync::Lazy;
use serde_json::json;
use test_log::test;
use tracing::info;
use uuid::Uuid;

use frontegg::{Client, ClientConfig, TenantRequest, UserListConfig, UserRequest};

pub static CLIENT_ID: Lazy<String> =
    Lazy::new(|| env::var("FRONTEGG_CLIENT_ID").expect("missing FRONTEGG_CLIENT_ID"));
pub static SECRET_KEY: Lazy<String> =
    Lazy::new(|| env::var("FRONTEGG_SECRET_KEY").expect("missing FRONTEGG_SECRET_KEY"));

fn new_client() -> Client {
    Client::new(ClientConfig {
        client_id: CLIENT_ID.clone(),
        secret_key: SECRET_KEY.clone(),
    })
}

async fn delete_existing_tenants(client: &Client) {
    for tenant in client.list_tenants().await.unwrap() {
        info!(%tenant.id, "deleting existing tenant");
        client.delete_tenant(tenant.id).await.unwrap();
    }
}

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
            name: "test tenant 1",
            metadata: json!({
                "tenant_number": 1,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .create_tenant(&TenantRequest {
            id: tenant_id_2,
            name: "test tenant 2",
            metadata: json!(42),
            ..Default::default()
        })
        .await
        .unwrap();

    // Verify tenant properties.
    let tenants = client.list_tenants().await.unwrap();
    assert_eq!(tenants.len(), 2);
    assert_eq!(tenants[0].id, tenant_id_1);
    assert_eq!(tenants[1].id, tenant_id_2);
    assert_eq!(tenants[0].name, "test tenant 1");
    assert_eq!(tenants[1].name, "test tenant 2");
    assert_eq!(tenants[0].metadata, json!({"tenant_number": 1}));
    assert_eq!(tenants[1].metadata, json!(42));
    assert_eq!(tenants[0].deleted_at, None);
    assert_eq!(tenants[1].deleted_at, None);

    // Create three users in each tenant.
    let mut users = vec![];
    for (tenant_idx, tenant) in tenants.iter().enumerate() {
        for user_idx in 0..3 {
            let name = format!("user-{tenant_idx}-{user_idx}");
            let email = format!("frontegg-test-{tenant_idx}-{user_idx}@example.com");
            let created_user = client
                .create_user(&UserRequest {
                    tenant_id: tenant.id,
                    name: &*name,
                    email: &*email,
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
            assert_eq!(created_user.name, user.name);
            assert_eq!(created_user.email, user.email);
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
        assert_eq!(expected, actual);
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
            .try_collect()
            .await
            .unwrap();
        assert_eq!(users.len(), 0);
    }
}
