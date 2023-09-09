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

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A Frontegg role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen))]
pub struct Role {
    /// The ID of the role.
    pub id: Uuid,
    /// The machine-readable name for the role.
    pub key: String,
    /// The human-readable name for the role.
    pub name: String,
    /// A description of the role.
    pub description: Option<String>,
    /// The level of the role.
    pub level: i64,
    /// Whether the role is a default role assigned to new users.
    pub is_default: bool,
    /// The IDs of the permissions granted by the role.
    #[serde(rename = "permissions")]
    pub permission_ids: Vec<Uuid>,
    /// The time at which the role was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// A Frontegg permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen))]
pub struct Permission {
    /// The ID of the permission.
    pub id: Uuid,
    /// The ID of the category to which the permission belongs.
    pub category_id: String,
    /// The machine-readable name for the permission.
    pub key: String,
    /// The human-readable name for the permission.
    pub name: String,
    /// A description of the permission.
    pub description: Option<String>,
    /// The time at which the permission was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The time at which the permission was updated.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
