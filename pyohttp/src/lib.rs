// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::{collections::HashMap, path::PathBuf, string::String};

use ohttp_client::OhttpClientBuilder;
use pyo3::prelude::*;

#[pyclass]
struct OhttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[pymethods]
impl OhttpResponse {
    fn status(&self) -> u16 {
        self.status
    }

    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    fn body(&self) -> Vec<u8> {
        self.body.clone()
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

    pub fn post(
        &self,
        url: String,
        binary: bool,
        target_path: String,
        headers: HashMap<String, String>,
        form_fields: HashMap<String, String>,
        outer_headers: HashMap<String, String>,
    ) -> PyResult<OhttpResponse> {
        let f = async {
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

            let client = OhttpClientBuilder::new()
                .kms_url(&Some(self.kms_url.clone()))
                .kms_cert(&Some(self.kms_cert.clone()))
                .build()
                .await?;

            let mut response = client
                .post(
                    &url,
                    binary,
                    &target_path,
                    &headers,
                    &form_fields,
                    &outer_headers,
                )
                .await?;

            let headers = response
                .headers()
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value_str| (key.as_str().to_string(), value_str.to_string()))
                })
                .collect();

            let body = response.body_mut();
            let bytes = warp::hyper::body::to_bytes(body).await?;

            Ok(OhttpResponse {
                status: response.status().as_u16(),
                headers,
                body: bytes.to_vec(),
            })
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f).map_err(|e: Box<dyn std::error::Error>| {
            PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e))
        })
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyohttp(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<OhttpResponse>()?;
    m.add_class::<OhttpClient>()?;
    Ok(())
}
