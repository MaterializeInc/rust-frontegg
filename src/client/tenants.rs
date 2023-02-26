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

use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::serde::Empty;
use crate::util::StrIteratorExt;
use crate::{error, Client, Error};

const TENANT_PATH: [&str; 4] = ["tenants", "resources", "tenants", "v1"];

/// The subset of [`Tenant`] used in create requests.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantRequest<'a> {
    /// The ID of the tenant.
    #[serde(rename = "tenantId")]
    pub id: Uuid,
    /// The name of the tenant.
    pub name: &'a str,
    /// Arbitrary metadata to attach to the tenant.
    pub metadata: serde_json::Value,
}

/// A Frontegg tenant.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    /// The ID of the tenant.
    #[serde(rename = "tenantId")]
    pub id: Uuid,
    /// The name of the tenant.
    pub name: String,
    /// Arbitrary metadata that is attached to the tenant.
    #[serde(default = "crate::serde::empty_json_object")]
    #[serde(deserialize_with = "crate::serde::nested_json::deserialize")]
    pub metadata: serde_json::Value,
    /// The time at which the tenant was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The time at which the tenant was updated.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// The time at which the tenant was deleted.
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Client {
    /// Lists all tenants in the workspace.
    ///
    /// The returned vector is sorted by tenant ID.
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, Error> {
        let req = self.build_request(Method::GET, TENANT_PATH);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Creates a new tenant.
    pub async fn create_tenant(&self, tenant: &TenantRequest<'_>) -> Result<Tenant, Error> {
        let req = self.build_request(Method::POST, TENANT_PATH);
        let req = req.json(tenant);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Get a tenant by ID.
    pub async fn get_tenant(&self, id: Uuid) -> Result<Tenant, Error> {
        let req = self.build_request(Method::GET, TENANT_PATH.chain_one(id));
        let mut res: Vec<Tenant> = self.send_request(req).await?;
        res.pop().ok_or(Error::Api(error::ApiError {
            status_code: StatusCode::NOT_FOUND,
            messages: vec!["Tenant not found".to_string()],
        }))
    }

    /// Deletes a tenant by ID.
    pub async fn delete_tenant(&self, id: Uuid) -> Result<(), Error> {
        let req = self.build_request(Method::DELETE, TENANT_PATH.chain_one(id));
        let _: Empty = self.send_request(req).await?;
        Ok(())
    }
}
