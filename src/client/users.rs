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

use async_stream::try_stream;
use futures_core::stream::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::client::roles::{Permission, Role};
use crate::client::Client;
use crate::error::Error;
use crate::serde::{Empty, Paginated};
use crate::util::{RequestBuilderExt, StrIteratorExt};

const USER_PATH: [&str; 4] = ["identity", "resources", "users", "v1"];
const VENDOR_USER_PATH: [&str; 5] = ["identity", "resources", "vendor-only", "users", "v1"];

/// Configuration for the [`Client::list_users`] operation.
#[derive(Debug, Clone)]
pub struct UserListConfig {
    tenant_id: Option<Uuid>,
    page_size: u64,
}

impl Default for UserListConfig {
    fn default() -> UserListConfig {
        UserListConfig {
            tenant_id: None,
            page_size: 50,
        }
    }
}

impl UserListConfig {
    /// Sets the tenant ID to filter users to.
    ///
    /// If this method is not called, users for all tenants are returned.
    pub fn tenant_id(mut self, tenant_id: Uuid) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Sets the page size.
    pub fn page_size(mut self, page_size: u64) -> Self {
        self.page_size = page_size;
        self
    }
}

/// The subset of [`User`] used in create requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRequest<'a> {
    /// The ID of the tenant to which the user will belong.
    #[serde(skip)]
    pub tenant_id: Uuid,
    /// The name of the user.
    pub name: &'a str,
    /// The email for the user.
    pub email: &'a str,
    /// Arbitrary metadata to attach to the user.
    pub metadata: serde_json::Value,
    /// Whether to skip sending an invitation email to the user.
    pub skip_invite_email: bool,
}

/// The subset of a [`User`] returned by [`Client::create_user`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedUser {
    /// The ID of the user.
    pub id: Uuid,
    /// The name of the user.
    pub name: String,
    /// The email for the user.
    pub email: String,
    /// Arbitrary metadata that is attached to the user.
    #[serde(default = "crate::serde::empty_json_object")]
    #[serde(deserialize_with = "crate::serde::nested_json::deserialize")]
    pub metadata: serde_json::Value,
    /// The roles to which this user belongs.
    pub roles: Vec<Role>,
    /// The permissions which this user holds.
    pub permissions: Vec<Permission>,
    /// The time at which the user was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// The subset of a [`User`] returned by a `frontegg.user.*` webhook event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookUser {
    /// The ID of the user.
    pub id: Uuid,
    /// The name of the user.
    pub name: Option<String>,
    /// The email for the user.
    pub email: String,
    /// Arbitrary metadata that is attached to the user.
    #[serde(default = "crate::serde::empty_json_object")]
    #[serde(deserialize_with = "crate::serde::nested_json::deserialize")]
    pub metadata: serde_json::Value,
    /// The roles to which this user belongs.
    pub roles: Vec<Role>,
    /// The permissions which this user holds.
    pub permissions: Vec<Permission>,
    /// The time at which the user was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The activation status of the user for the tenant.
    pub activated_for_tenant: Option<bool>,
    /// The locked status of the user.
    pub is_locked: Option<bool>,
    /// The entity managing the user.
    pub managed_by: String,
    /// The mfa enrollment status of the user.
    pub mfa_enrolled: bool,
    /// The mfa bypass status of the user.
    pub mfa_bypass: Option<bool>,
    /// The phone_number of the user.
    pub phone_number: Option<String>,
    /// The profile picture url of the user.
    pub profile_picture_url: Option<String>,
    /// The provider of the user.
    pub provider: String,
    /// The sub of the user.
    pub sub: Uuid,
    /// The ID of the tenant of the user.
    pub tenant_id: Uuid,
    /// The IDs of all tenants for the user. Missing on frontegg.user.disabledMFA events.
    pub tenant_ids: Option<Vec<Uuid>>,
    /// The tenants to which this user belongs. Missing on frontegg.user.disabledMFA events.
    pub tenants: Option<Vec<WebhookTenantBinding>>,
    /// The verified status of the user.
    pub verified: Option<bool>,
}

/// A Frontegg user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// The ID of the user.
    pub id: Uuid,
    /// The name of the user.
    pub name: String,
    /// The email for the user.
    pub email: String,
    /// Arbitrary metadata that is attached to the user.
    #[serde(default = "crate::serde::empty_json_object")]
    #[serde(deserialize_with = "crate::serde::nested_json::deserialize")]
    pub metadata: serde_json::Value,
    /// The tenants to which this user belongs.
    pub tenants: Vec<TenantBinding>,
    /// The time at which the user was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Binds a [`User`] to a [`Tenant`] for a `frontegg.user.*` webhook event
///
/// [`Tenant`]: crate::client::tenant::Tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookTenantBinding {
    /// The ID of the tenant.
    pub tenant_id: Uuid,
    /// The roles to which the user belongs in this tenant. Missing on frontegg.user.enrolledMFA events.
    pub roles: Option<Vec<Role>>,
}

/// Binds a [`User`] to a [`Tenant`].
///
/// [`Tenant`]: crate::client::tenant::Tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantBinding {
    /// The ID of the tenant.
    pub tenant_id: Uuid,
    /// The roles to which the user belongs in this tenant.
    pub roles: Vec<Role>,
}

impl Client {
    /// Lists users, either for all tenants or for a single tenant.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_users(
        &self,
        config: UserListConfig,
    ) -> impl Stream<Item = Result<User, Error>> + '_ {
        try_stream! {
            let mut page = 0;
            loop {
                let mut req = self.build_request(Method::GET, USER_PATH);
                if let Some(tenant_id) = config.tenant_id {
                    req = req.tenant(tenant_id);
                }
                let req = req.query(&[
                    ("_limit", &*config.page_size.to_string()),
                    ("_offset", &*page.to_string())
                ]);
                let res: Paginated<User> = self.send_request(req).await?;
                for user in res.items {
                    yield user;
                }
                page += 1;
                if page >= res.metadata.total_pages {
                    break;
                }
            }
        }
    }

    /// Creates a new user.
    ///
    /// Only partial information about the created user is returned. To fetch
    /// the full information about the user, call [`Client::get_user`].
    pub async fn create_user(&self, user: &UserRequest<'_>) -> Result<CreatedUser, Error> {
        let req = self.build_request(Method::POST, USER_PATH);
        let req = req.tenant(user.tenant_id);
        let req = req.json(user);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a user by ID.
    pub async fn get_user(&self, id: Uuid) -> Result<User, Error> {
        let req = self.build_request(Method::GET, VENDOR_USER_PATH.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Deletes a user by ID.
    pub async fn delete_user(&self, id: Uuid) -> Result<(), Error> {
        let req = self.build_request(Method::DELETE, USER_PATH.chain_one(id));
        let _: Empty = self.send_request(req).await?;
        Ok(())
    }
}
