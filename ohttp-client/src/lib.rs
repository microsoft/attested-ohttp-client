// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use bhttp::{Message, Mode};
use futures_util::stream::unfold;
use ohttp::ClientRequest;
use rand::distributions::{Alphanumeric, DistString};
use reqwest::{Client, Response};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    ops::Deref,
    path::PathBuf,
    str::FromStr,
};
use tracing::{error, info, trace};
use warp::hyper::body::Body;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
/// This allows a `HexArg` to be created from a string slice (`&str`) by decoding
/// the string as hexadecimal.
pub struct HexArg(Vec<u8>);
impl FromStr for HexArg {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex::decode(s).map(HexArg)
    }
}

/// This allows `HexArg` instances to be dereferenced to a slice of bytes (`[u8]`).
impl Deref for HexArg {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Writes the request line for an HTTP POST request to the provided buffer.
/// The request line follows the format:
/// `POST {target_path} HTTP/1.1\r\n`.
fn write_post_request_line(request: &mut Vec<u8>, target_path: &str) -> Res<()> {
    write!(request, "POST {target_path} HTTP/1.1\r\n")?;
    Ok(())
}

/// Appends HTTP headers to the provided request buffer.
fn append_headers(request: &mut Vec<u8>, headers: &Option<Vec<String>>) -> Res<()> {
    if let Some(headers) = headers {
        for header in headers {
            write!(request, "{header}\r\n")?;
            info!("{header}\r\n");
        }
    }
    Ok(())
}

/// Creates a multipart/form-data body for an HTTP request.
fn create_multipart_body(
    data: &Option<String>,
    fields: &Option<Vec<String>>,
    boundary: &str,
) -> Res<Vec<u8>> {
    let mut body = Vec::new();

    if let Some(data) = data {
        write!(&mut body, "{data}")?;
    }

    let fields = match fields {
        Some(fields) => fields,
        None => return Ok(body),
    };

    for field in fields {
        let (name, value) = field.split_once('=').unwrap();

        if value.starts_with('@') {
            // If the value starts with '@', it is treated as a file path.
            let filename = value.strip_prefix('@').unwrap();
            let mut file = File::open(filename)?;
            let mut file_contents = Vec::new();
            file.read_to_end(&mut file_contents)?;

            let kind = infer::get(&file_contents).expect("file type is unknown");
            let mime_type = kind.mime_type();

            // Add the file
            write!(
                &mut body,
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {mime_type}\r\n\r\n"
            )?;
            body.extend_from_slice(&file_contents);
        } else {
            write!(
                &mut body,
                "\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n"
            )?;
            write!(&mut body, "{value}")?;
        }
        write!(&mut body, "\r\n--{boundary}--\r\n")?;
    }

    Ok(body)
}

/// Append the headers for a multipart/form-data HTTP request to the provided buffer.
fn append_multipart_headers(request: &mut Vec<u8>, boundary: &str, body_len: usize) -> Res<()> {
    write!(
        request,
        "Content-Type: multipart/form-data; boundary={boundary}\r\n"
    )?;
    write!(request, "Content-Length: {}\r\n", body_len)?;
    write!(request, "\r\n")?;
    Ok(())
}

/// Creates an http multipart message.
fn create_multipart_request(
    target_path: &str,
    headers: &Option<Vec<String>>,
    data: &Option<String>,
    fields: &Option<Vec<String>>,
) -> Res<Vec<u8>> {
    // Define boundary for multipart
    let boundary_string = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
    let boundary = &format!("----{boundary_string}");

    // Create a POST request for target target_path
    let mut request = Vec::new();

    write_post_request_line(&mut request, target_path)?;
    append_headers(&mut request, headers)?;

    // Create multipart body
    let mut body = create_multipart_body(data, fields, boundary)?;

    // Append multipart headers
    append_multipart_headers(&mut request, boundary, body.len())?;

    // Append body to the request
    request.append(&mut body);

    Ok(request)
}

/// Prepares a http message based on the `is_bhttp` flag and other parameters.
fn create_request_buffer(
    target_path: &str,
    headers: &Option<Vec<String>>,
    data: &Option<String>,
    form_fields: &Option<Vec<String>>,
) -> Res<Vec<u8>> {
    let request = create_multipart_request(target_path, headers, data, form_fields)?;
    let mut cursor = Cursor::new(request);
    let request = Message::read_http(&mut cursor)?;
    let mut request_buf = Vec::new();
    request.write_bhttp(Mode::KnownLength, &mut request_buf)?;
    Ok(request_buf)
}

// Get key configuration from KMS
async fn get_kms_config(kms_url: String, cert: &str) -> Res<String> {
    // Create a client with the CA certificate
    let client = Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(cert.as_bytes())?)
        .build()?;

    info!("Contacting key management service at {kms_url}...");
    let max_retries = 3;
    let mut retries = 0;
    let url = kms_url + "/listpubkeys";

    loop {
        // Make the GET request
        let response = client.get(url.clone()).send().await?.error_for_status()?;

        // We may have to wait for receipt to be ready
        match response.status().as_u16() {
            202 => {
                if retries < max_retries {
                    retries += 1;
                    trace!(
                        "Received 202 status code, retrying... (attempt {}/{})",
                        retries,
                        max_retries
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                } else {
                    Err("Max retries reached, giving up. Cannot reach key management service")?;
                }
            }
            200 => {
                let body = response.text().await?;
                assert!(!body.is_empty());
                return Ok(body);
            }
            e => {
                Err(format!("KMS returned unexpected {} status code.", e))?;
            }
        }
    }
}

#[derive(Deserialize)]
struct KmsKeyConfiguration {
    #[serde(rename = "publicKey")]
    key_config: String,
    receipt: String,
}

/// Reads a json containing key configurations with receipts and constructs
/// a single use client sender from the first supported configuration.
trait ClientRequestBuilder {
    fn from_kms_config(config: &str, cert: &str) -> Res<ClientRequest>;
}

impl ClientRequestBuilder for ClientRequest {
    /// Reads a json containing key configurations with receipts and constructs
    /// a single use client sender from the first supported configuration.
    fn from_kms_config(config: &str, cert: &str) -> Res<ClientRequest> {
        let mut kms_configs: Vec<KmsKeyConfiguration> = serde_json::from_str(config)?;
        let kms_config = match kms_configs.pop() {
            Some(config) => config,
            None => return Err("No KMS configuration found".into()),
        };
        info!("{}", "Establishing trust in key management service...");
        let _ = verifier::verify(&kms_config.receipt, cert)?;
        info!(
            "{}",
            "The receipt for the generation of the OHTTP key is valid."
        );
        let encoded_config = hex::decode(&kms_config.key_config)?;
        Ok(ClientRequest::from_encoded_config(&encoded_config)?)
    }
}

/// Creates an OHTTP client from the static config provided in Args.
///
fn create_request_from_encoded_config_list(config: &Option<HexArg>) -> Res<ohttp::ClientRequest> {
    let config = match config {
        Some(config) => config,
        None => return Err("config expected".into()),
    };
    Ok(ohttp::ClientRequest::from_encoded_config_list(config)?)
}

/// Creates an OHTTP client from KMS.
///
async fn create_request_from_kms_config(
    kms_url: &String,
    kms_cert: &PathBuf,
) -> Res<ohttp::ClientRequest> {
    let cert = fs::read_to_string(kms_cert)?;
    let config = get_kms_config(kms_url.to_owned(), &cert).await?;
    ClientRequest::from_kms_config(&config, &cert)
}

fn print_response_headers(response: &Response) {
    info!("Response headers:");
    for (key, value) in response.headers() {
        info!(
            "{}: {}",
            key,
            std::str::from_utf8(value.as_bytes()).unwrap()
        );
    }
}

async fn post_request(
    url: &String,
    outer_headers: &Option<Vec<String>>,
    enc_request: Vec<u8>,
) -> Res<reqwest::Response> {
    let client = reqwest::ClientBuilder::new().build()?;

    let mut builder = client
        .post(url)
        .header("content-type", "message/ohttp-chunked-req");

    // Add outer headers
    if let Some(outer_headers) = outer_headers {
        trace!("Outer request headers:");
        for header in outer_headers {
            let (key, value) = header.split_once(':').unwrap();
            trace!("Adding {key}: {value}");
            builder = builder.header(key, value);
        }
    }

    match builder.body(enc_request).send().await {
        Ok(response) => {
            print_response_headers(&response);
            let status = response.status();
            if !status.is_success() {
                let error_msg = format!("HTTP request failed with status {status}");
                error!("{}", error_msg);
            }
            Ok(response)
        }
        Err(e) => {
            error!("Request failed: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Decapsulate the http response
async fn decapsulate_response(
    response: reqwest::Response,
    client_response: ohttp::ClientResponse,
) -> Res<Response> {
    info!("checking token in response");
    if let Some(token) = response.headers().get("x-attestation-token") {
        info!("token: {}", std::str::from_utf8(token.as_bytes()).unwrap())
    }

    let status = response.status();
    let mut builder = warp::http::Response::builder().status(status);

    let headers = response.headers().clone();
    for (key, value) in headers {
        if let Some(key) = key {
            builder = builder.header(key, value.clone());
        }
    }

    let stream = Box::pin(unfold(response, |mut response| async move {
        match response.chunk().await {
            Ok(Some(chunk)) => Some((Ok(chunk.to_vec()), response)),
            _ => None,
        }
    }));

    let stream = client_response.decapsulate_stream(stream).await;
    let response = builder.body(Body::wrap_stream(stream))?;
    Ok(Response::from(response))
}

pub struct OhttpClient {
    ohttp_request: ClientRequest,
}

impl OhttpClient {
    #[allow(clippy::too_many_arguments)]
    async fn encapsulate_and_send(
        self,
        url: &String,
        headers: &Option<Vec<String>>,
        bhttp_request: &[u8],
    ) -> Res<Response> {
        // Encapsulate the http buffer using the OHTTP request
        let (enc_request, ohttp_response) = match self.ohttp_request.encapsulate(bhttp_request) {
            Ok(result) => result,
            Err(e) => {
                error!("{e}");
                return Err(Box::new(e));
            }
        };
        trace!(
            "Encapsulated the OHTTP request {}",
            hex::encode(&enc_request[0..60])
        );

        // Post the encapsulated ohttp request buffer to args.url
        let response = match post_request(url, headers, enc_request).await {
            Ok(response) => response,
            Err(e) => {
                error!("{e}");
                return Err(e);
            }
        };
        trace!("Posted the OHTTP request to {}", url);

        // decapsulate and output the http response
        match decapsulate_response(response, ohttp_response).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!("{e}");
                Err(e)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post_raw(
        self,
        url: &String,
        outer_headers: &Option<Vec<String>>,
        http_request: &Vec<u8>,
    ) -> Res<Response> {
        // transform the http request into bhttp
        let mut cursor = Cursor::new(http_request);
        let request = Message::read_http(&mut cursor)?;
        let mut request_buf = Vec::new();
        request.write_bhttp(Mode::KnownLength, &mut request_buf)?;
        trace!("Created the ohttp request buffer");

        self.encapsulate_and_send(url, outer_headers, &request_buf)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post(
        self,
        url: &String,
        target_path: &str,
        headers: &Option<Vec<String>>,
        data: &Option<String>,
        form_fields: &Option<Vec<String>>,
        outer_headers: &Option<Vec<String>>,
    ) -> Res<Response> {
        //  Create ohttp request buffer
        let request_buf = match create_request_buffer(target_path, headers, data, form_fields) {
            Ok(result) => result,
            Err(e) => {
                error!("{e}");
                return Err(e);
            }
        };
        trace!("Created the ohttp request buffer");

        self.encapsulate_and_send(url, outer_headers, &request_buf)
            .await
    }
}

#[derive(Default)]
pub struct OhttpClientBuilder {
    kms_url: Option<String>,
    kms_cert: Option<PathBuf>,
    config: Option<HexArg>,
}

impl OhttpClientBuilder {
    pub fn new() -> OhttpClientBuilder {
        OhttpClientBuilder {
            kms_url: None,
            kms_cert: None,
            config: None,
        }
    }

    pub fn kms_url(mut self, kms_url: &Option<String>) -> OhttpClientBuilder {
        self.kms_url.clone_from(kms_url);
        self
    }

    pub fn kms_cert(mut self, kms_cert: &Option<PathBuf>) -> OhttpClientBuilder {
        self.kms_cert.clone_from(kms_cert);
        self
    }

    pub fn config(mut self, config: &Option<HexArg>) -> OhttpClientBuilder {
        self.config.clone_from(config);
        self
    }

    pub async fn build(self) -> Res<OhttpClient> {
        //  create the OHTTP request using the KMS or the static config file
        let result = if let (Some(kms_url), Some(kms_cert)) = (self.kms_url, self.kms_cert) {
            create_request_from_kms_config(&kms_url, &kms_cert).await
        } else {
            create_request_from_encoded_config_list(&self.config)
        };

        let ohttp_request = match result {
            Ok(request) => request,
            Err(e) => {
                error!("{e}");
                return Err(e);
            }
        };

        trace!("Created ohttp client request");

        Ok(OhttpClient { ohttp_request })
    }
}
