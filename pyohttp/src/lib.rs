// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use core::str;
use ohttp_client::OhttpClientBuilder;
use pyo3::prelude::*;
use reqwest::Response;
use std::{collections::HashMap, path::PathBuf, string::String, sync::Arc};
use tokio::sync::Mutex;

#[pyclass]
struct OhttpResponse {
    response: Arc<Mutex<Response>>,
}

#[pymethods]
impl OhttpResponse {
    fn status(&self) -> u16 {
        let response = Arc::clone(&self.response);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let f = async move { response.lock().await.status().as_u16() };
        rt.block_on(f)
    }

    fn headers(&self) -> HashMap<String, String> {
        let response = Arc::clone(&self.response);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let f = async move {
            response
                .lock()
                .await
                .headers()
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value_str| (key.as_str().to_string(), value_str.to_string()))
                })
                .collect::<HashMap<String, String>>()
        };
        rt.block_on(f)
    }

    fn chunk<'py>(&mut self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let response = Arc::clone(&self.response);
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let chunk = match response.lock().await.chunk().await {
                Ok(Some(chunk)) => Some(chunk.to_vec()),
                _ => None,
            };
            Ok(chunk)
        })
    }
}

#[pyclass]
struct OhttpClient {
    kms_url: String,
    kms_cert: PathBuf,
}

#[pymethods]
impl OhttpClient {
    #[new]
    fn new(kms_url: String, kms_cert: PathBuf) -> Self {
        OhttpClient { kms_url, kms_cert }
    }

    pub fn post<'py>(
        &self,
        url: String,
        headers: HashMap<String, String>,
        form_fields: HashMap<String, String>,
        outer_headers: HashMap<String, String>,
        py: Python<'py>,
    ) -> PyResult<&'py PyAny> {
        let kms_url = self.kms_url.clone();
        let kms_cert = self.kms_cert.clone();
        let headers = headers
            .iter()
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect();
        let form_fields = form_fields
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();
        let outer_headers = outer_headers
            .iter()
            .map(|(key, value)| format!("{}: {}", key, value))
            .collect();

        pyo3_asyncio::tokio::future_into_py(py, async move {
            let client = OhttpClientBuilder::new()
                .kms_url(&Some(kms_url.clone()))
                .kms_cert(&Some(kms_cert.clone()))
                .build()
                .await
                .map_err(|e: Box<dyn std::error::Error>| {
                    PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e))
                })?;

            let response = client
                .post(&url, "/", &headers, &form_fields, &outer_headers)
                .await
                .map_err(|e: Box<dyn std::error::Error>| {
                    PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e))
                })?;

            Ok(OhttpResponse {
                response: Arc::new(Mutex::new(response)),
            })
        })
    }
}

#[pymodule]
fn pyohttp(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<OhttpResponse>()?;
    m.add_class::<OhttpClient>()?;
    Ok(())
}
