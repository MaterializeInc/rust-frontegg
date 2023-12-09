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
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::serde::Empty;
#[cfg(feature = "python")]
use crate::util::py;
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
    /// The name of the person who created the tenant.
    pub creator_name: Option<&'a str>,
    /// The email of the person who created the tenant.
    pub creator_email: Option<&'a str>,
}

/// A Frontegg tenant.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "python", pyo3::pyclass(frozen))]
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
    /// The name of the person who created the tenant.
    pub creator_name: Option<String>,
    /// The email of the person who created the tenant.
    pub creator_email: Option<String>,
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

#[cfg(feature = "python")]
#[pyo3::pymethods]
impl Tenant {
    fn __repr__(&self) -> String {
        format!("Tenant(id='{}', name={})", self.id, self.name)
    }

    #[getter]
    fn id(&self, _py: pyo3::Python) -> pyo3::PyResult<pyo3::PyObject> {
        py::PyUuid(self.id).try_into()
    }

    #[getter]
    fn name<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<&'a pyo3::types::PyString> {
        Ok(pyo3::types::PyString::new(py, &self.name))
    }

    #[getter]
    fn metadata<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<pyo3::PyObject> {
        let res = py::json_to_object(py, &self.metadata)?;
        Ok(res)
    }

    #[getter]
    fn creator_name<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<pyo3::PyObject> {
        match &self.creator_name {
            None => Ok(py.None()),
            Some(cn) => {
                let s = pyo3::types::PyString::new(py, &cn);
                let o: pyo3::Py<pyo3::PyAny> = s.into();
                Ok(o)
            }
        }
    }

    #[getter]
    fn creator_email<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<pyo3::PyObject> {
        match &self.creator_email {
            None => Ok(py.None()),
            Some(ce) => {
                let s = pyo3::types::PyString::new(py, &ce);
                let o: pyo3::Py<pyo3::PyAny> = s.into();
                Ok(o)
            }
        }
    }

    #[getter]
    fn created_at<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<&'a pyo3::types::PyDateTime> {
        pyo3::types::PyDateTime::from_timestamp(py, self.created_at.unix_timestamp() as f64, None)
    }

    #[getter]
    fn updated_at<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<&'a pyo3::types::PyDateTime> {
        pyo3::types::PyDateTime::from_timestamp(py, self.created_at.unix_timestamp() as f64, None)
    }

    #[getter]
    fn deleted_at<'a>(&self, py: pyo3::Python<'a>) -> pyo3::PyResult<pyo3::PyObject> {
        match self.deleted_at {
            None => Ok(py.None()),
            Some(dt) => {
                let py_dt =
                    pyo3::types::PyDateTime::from_timestamp(py, dt.unix_timestamp() as f64, None);
                match py_dt {
                    Ok(t) => {
                        let o: pyo3::Py<pyo3::PyAny> = t.into();
                        Ok(o)
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
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

    /// Set tenant metadata with an optional key
    ///
    /// This does not remove existing keys from the object if omitted.
    pub async fn set_tenant_metadata(
        &self,
        id: Uuid,
        metadata: &serde_json::Value,
    ) -> Result<Tenant, Error> {
        let req = self
            .build_request(
                Method::POST,
                TENANT_PATH.chain_one(id).chain_one("metadata"),
            )
            .json(&json!({ "metadata": metadata }));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Remove a key/value from a tenant's metadata
    pub async fn delete_tenant_metadata(&self, id: Uuid, key: &str) -> Result<Tenant, Error> {
        let req = self.build_request(
            Method::DELETE,
            TENANT_PATH
                .chain_one(id)
                .chain_one("metadata")
                .chain_one(key),
        );
        let res = self.send_request(req).await?;
        Ok(res)
    }
}

#[cfg(feature = "python")]
#[pyo3::pymethods]
impl Client {
    #[pyo3(name = "list_tenants")]
    fn py_list_tenants<'a>(&self, _py: pyo3::Python<'a>) -> pyo3::PyResult<Vec<Tenant>> {
        let tenants_result = self.block_on_runtime(async { self.list_tenants().await });
        tenants_result.map_err(|e| pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)))
    }

    #[pyo3(name = "create_tenant")]
    fn py_create_tenant<'a>(
        &self,
        py: pyo3::Python<'a>,
        id: pyo3::PyObject,
        name: &'a str,
        metadata: Option<pyo3::PyObject>,
        creator_name: Option<&'a str>,
        creator_email: Option<&'a str>,
    ) -> pyo3::PyResult<Tenant> {
        let id: py::PyUuid = id.as_ref(py).try_into()?;
        let req = TenantRequest {
            id: id.0,
            name,
            metadata: match metadata {
                Some(m) => py::object_to_json(py, m.as_ref(py))?,
                None => serde_json::Value::Null,
            },
            creator_name,
            creator_email,
        };
        let res = self.block_on_runtime(async { self.create_tenant(&req).await });
        res.map_err(|e| pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)))
    }

    #[pyo3(name = "get_tenant")]
    fn py_get_tenant(&self, py: pyo3::Python, id: pyo3::PyObject) -> pyo3::PyResult<Tenant> {
        let i: py::PyUuid = id.as_ref(py).try_into()?;
        let res = self.block_on_runtime(async { self.get_tenant(i.0).await });
        res.map_err(|e| match e {
            Error::Api(a) if a.status_code == StatusCode::NOT_FOUND => {
                crate::NotFoundError::new_err("Tenant not found")
            }
            _ => pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)),
        })
    }

    #[pyo3(name = "delete_tenant")]
    fn py_delete_tenant(&self, py: pyo3::Python, id: pyo3::PyObject) -> pyo3::PyResult<()> {
        let i: py::PyUuid = id.as_ref(py).try_into()?;
        let res = self.block_on_runtime(async { self.delete_tenant(i.0).await });
        res.map_err(|e| match e {
            Error::Api(a) if a.status_code == StatusCode::NOT_FOUND => {
                crate::NotFoundError::new_err("Tenant not found")
            }
            _ => pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)),
        })
    }

    #[pyo3(name = "set_tenant_metadata")]
    fn py_set_tenant_metadata(
        &self,
        py: pyo3::Python,
        id: pyo3::PyObject,
        metadata: pyo3::PyObject,
    ) -> pyo3::PyResult<Tenant> {
        let i: py::PyUuid = id.as_ref(py).try_into()?;
        let m: serde_json::Value = py::object_to_json(py, metadata.as_ref(py))?;
        let res = self.block_on_runtime(async { self.set_tenant_metadata(i.0, &m).await });
        res.map_err(|e| match e {
            Error::Api(a) if a.status_code == StatusCode::NOT_FOUND => {
                crate::NotFoundError::new_err("Tenant not found")
            }
            _ => pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)),
        })
    }

    #[pyo3(name = "delete_tenant_metadata")]
    fn py_delete_tenant_metadata(
        &self,
        py: pyo3::Python,
        id: pyo3::PyObject,
        key: &str,
    ) -> pyo3::PyResult<Tenant> {
        let i: py::PyUuid = id.as_ref(py).try_into()?;
        let res = self.block_on_runtime(async { self.delete_tenant_metadata(i.0, key).await });
        res.map_err(|e| match e {
            Error::Api(a) if a.status_code == StatusCode::NOT_FOUND => {
                crate::NotFoundError::new_err("Tenant not found")
            }
            _ => pyo3::exceptions::PyAssertionError::new_err(format!("{:?}", e)),
        })
    }
}
