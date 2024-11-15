// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::{collections::HashMap, path::PathBuf, string::String};

use ohttp_client::OhttpClientBuilder;
use pyo3::prelude::*;

#[pyclass]
struct PyOhttpClient {
    kms_url: String,
    kms_cert: PathBuf,
}

#[pymethods]
impl PyOhttpClient {
    #[new]
    fn new(kms_url: String, kms_cert: PathBuf) -> Self {
        PyOhttpClient { kms_url, kms_cert }
    }

    pub fn post(
        &self,
        url: String,
        binary: bool,
        target_path: String,
        headers: HashMap<String, String>,
        form_fields: HashMap<String, String>,
        outer_headers: HashMap<String, String>,
    ) -> PyResult<()> {
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

        let f = async {
            let client = OhttpClientBuilder::new()
                .kms_url(&Some(self.kms_url.clone()))
                .kms_cert(&Some(self.kms_cert.clone()))
                .build()
                .await
                .unwrap();

            client
                .post(
                    &url,
                    binary,
                    &target_path,
                    &headers,
                    &form_fields,
                    &outer_headers,
                )
                .await
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyException, _>(format!("{}", e)))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn pyohttp(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyOhttpClient>()?;
    Ok(())
}
