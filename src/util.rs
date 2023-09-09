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

use reqwest_middleware::RequestBuilder;
use uuid::Uuid;

pub trait RequestBuilderExt {
    fn tenant(self, uuid: Uuid) -> RequestBuilder;
}

impl RequestBuilderExt for RequestBuilder {
    fn tenant(self, uuid: Uuid) -> RequestBuilder {
        self.header("Frontegg-Tenant-Id", uuid.to_string())
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

#[cfg(feature = "python")]
pub(crate) mod py {
    use super::Uuid;
    use pyo3::ToPyObject;
    use std::collections::HashMap;
    use std::convert::TryFrom;

    pub(crate) fn json_to_object(
        py: pyo3::Python,
        val: &serde_json::Value,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        match val {
            serde_json::Value::Null => Ok(py.None()),
            serde_json::Value::Bool(v) => {
                let b = pyo3::types::PyBool::new(py, *v);
                let o: pyo3::Py<pyo3::PyAny> = b.into();
                Ok(o)
            }
            serde_json::Value::Number(v) => {
                let n = if v.is_i64() {
                    v.as_i64().to_object(py)
                } else if v.is_f64() {
                    v.as_f64().to_object(py)
                } else {
                    panic!("JSON value {v:?} not convertible to Python")
                };
                Ok(n)
            }
            serde_json::Value::String(v) => {
                let s = pyo3::types::PyString::new(py, v);
                let o: pyo3::Py<pyo3::PyAny> = s.into();
                Ok(o)
            }
            serde_json::Value::Array(v) => {
                let a = v.iter().map(|e| {
                    json_to_object(py, e)
                        .unwrap_or_else(|_| panic!("JSON element {e:?} not convertible to Python"))
                });
                let l = pyo3::types::PyList::new(py, a);
                let o: pyo3::Py<pyo3::PyAny> = l.into();
                Ok(o)
            }
            // TODO: is there a neater way to do this?
            serde_json::Value::Object(o) => {
                let d = pyo3::types::PyDict::new(py);
                for (k, v) in o.iter() {
                    d.set_item(k, json_to_object(py, v)?)?;
                }
                let o: pyo3::Py<pyo3::PyAny> = d.into();
                Ok(o)
            }
        }
    }

    pub(crate) fn object_to_json(
        py: pyo3::Python,
        val: &pyo3::PyAny,
    ) -> pyo3::PyResult<serde_json::Value> {
        if val.eq(py.None())? {
            Ok(serde_json::Value::Null)
        } else if let Ok(s) = val.extract::<&str>() {
            Ok(serde_json::Value::String(s.to_string()))
        } else if let Ok(b) = val.extract::<bool>() {
            Ok(serde_json::Value::Bool(b))
        } else if let Ok(o) = val.extract::<HashMap<&str, &pyo3::PyAny>>() {
            let elements = o
                .iter()
                .filter_map(|(k, v)| object_to_json(py, v).map(|j| (k.to_string(), j)).ok());
            let mapping = serde_json::Map::from_iter(elements);
            Ok(serde_json::Value::Object(mapping))
        } else if let Ok(a) = val.extract::<Vec<&pyo3::PyAny>>() {
            let contents = a
                .iter()
                .filter_map(|e| object_to_json(py, e).ok())
                .collect();
            Ok(serde_json::Value::Array(contents))
        } else if let Ok(n) = val.extract::<f64>() {
            Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(n).expect("Could not numberize"),
            ))
        } else {
            Err(pyo3::exceptions::PyAssertionError::new_err("Can't JSONify"))
        }
    }

    pub(crate) struct PyUuid(pub Uuid);

    impl<'gil> TryFrom<&'gil pyo3::PyAny> for PyUuid {
        type Error = pyo3::PyErr;

        fn try_from(value: &'gil pyo3::PyAny) -> Result<Self, Self::Error> {
            let py_string = if value.is_instance_of::<pyo3::types::PyString>() {
                value
            } else {
                value.call_method0("__str__")?
            };
            let s: &str = py_string.extract()?;
            let i = Uuid::parse_str(s).map_err(|_| {
                pyo3::PyErr::from_value(
                    pyo3::exceptions::PyValueError::new_err(format!("{s} is not a valid UUID"))
                        .value(value.py()),
                )
            })?;
            Ok(Self(i))
        }
    }

    impl TryFrom<PyUuid> for pyo3::PyObject {
        type Error = pyo3::PyErr;

        fn try_from(value: PyUuid) -> Result<Self, Self::Error> {
            pyo3::Python::with_gil(|py| {
                let as_str = value
                    .0
                    .hyphenated()
                    .encode_lower(&mut Uuid::encode_buffer())
                    .to_string();
                let args = (as_str,);
                let uuid = pyo3::types::PyModule::import(py, "uuid")?;
                Ok(uuid.getattr("UUID")?.call(args, None)?.into())
            })
        }
    }
}
