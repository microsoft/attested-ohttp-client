use anyhow::Result;
use clap::Parser;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Method, Request, Response, StatusCode};
use hyper::server::Server;
use log::info;
use regex::Regex;
use std::convert::Infallible;
use std::io::Write as IoWrite;
use std::net::SocketAddr;
use std::path::PathBuf;
use ohttp_client::OhttpClientBuilder;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// URL of the KMS to obtain HPKE keys from
    #[arg(long, short = 'k', default_value = "https://accconfinferenceproduction.confidential-ledger.azure.com")]
    kms_url: Option<String>,

    /// Service identity certificate of the KMS
    #[arg(long, short = 'c')]
    kms_cert: Option<String>,
}

async fn make_ohttp_request(
    https_url: String,
    raw_request: Vec<u8>,
    outer_headers: Vec<String>,
    kms_url: String,
    kms_cert: PathBuf,
) -> Result<(reqwest::StatusCode, reqwest::header::HeaderMap, Vec<u8>), String> {
    let kms_url_opt = Some(kms_url);
    let kms_cert_opt = Some(kms_cert);
    
    let ohttp_client = match OhttpClientBuilder::new()
        .kms_url(&kms_url_opt)
        .kms_cert(&kms_cert_opt)
        .build()
        .await
    {
        Ok(client) => client,
        Err(e) => return Err(format!("Failed to create OHTTP client: {}", e)),
    };

    // Make OHTTP request
    let mut response = match ohttp_client.post_raw(&https_url, &Some(outer_headers), &raw_request).await {
        Ok(response) => response,
        Err(e) => return Err(format!("OHTTP request failed: {}", e)),
    };

    let status = response.status();
    let headers = response.headers().clone();
    let mut body_chunks = Vec::new();
    
    // Read response body chunks
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => body_chunks.extend_from_slice(&chunk),
            Ok(None) => break,
            Err(e) => return Err(format!("Error reading response chunk: {}", e)),
        }
    }
    
    Ok((status, headers, body_chunks))
}

async fn handle_request(
    req: Request<Body>,
    client: Client<hyper::client::HttpConnector>,
    kms_url: String,
    kms_cert: PathBuf,
) -> Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let body = req.into_body();

    println!("Proxying {} {}", method, uri);

    // Check if this is an OpenAI Azure audio transcription request
    let uri_str = uri.to_string();
    let openai_regex = Regex::new(r"https?://([^.]+)\.openai\.azure\.com/openai/deployments/([^/]+)/audio/transcriptions").unwrap();
    
    if let Some(captures) = openai_regex.captures(&uri_str) {
        let endpoint_name = captures.get(1).unwrap().as_str();
        let deployment_name = captures.get(2).unwrap().as_str();
        
        println!("Detected OpenAI Azure audio transcription request for endpoint: {}, deployment: {}", endpoint_name, deployment_name);
        
        // Convert HTTP URL to HTTPS for the actual destination
        let https_url = uri_str.replace("http://", "https://");
        
        // Convert body to bytes for form data
        let body_bytes = match hyper::body::to_bytes(body).await {
            Ok(bytes) => bytes.to_vec(),
            Err(e) => {
                eprintln!("Failed to read request body: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read request body"))
                    .unwrap());
            }
        };

        // Serialize the full HTTP request
        let mut raw_request = Vec::new();
        
        // Write request line
        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
        write!(raw_request, "{} {} HTTP/1.1\r\n", method, path_and_query).unwrap();
        
        // Write headers
        for (name, value) in headers.iter() {
            write!(raw_request, "{}: {}\r\n", name, value.to_str().unwrap_or("")).unwrap();
        }
        
        // Add empty line to separate headers from body
        write!(raw_request, "\r\n").unwrap();
        
        // Add body
        raw_request.extend_from_slice(&body_bytes);

        // Extract outer headers (API key for authorization)
        let outer_headers: Vec<String> = if let Some(api_key) = headers.get("api-key") {
            if let Ok(api_key_str) = api_key.to_str() {
                vec![format!("api-key: {}", api_key_str)]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // Make OHTTP request using the separate function
        match make_ohttp_request(https_url, raw_request, outer_headers, kms_url, kms_cert).await {
            Ok((status, response_headers, body_chunks)) => {
                let mut response_builder = Response::builder().status(status.as_u16());
                
                // Copy response headers
                // FIXME copy inner headers
                for (key, value) in response_headers.iter() {
                    //println!("Response header: {}: {}", key, value.to_str().unwrap_or(""));
                    if key != "content-length" {
                        response_builder = response_builder.header(key, value);
                    }
                }
                
                return Ok(response_builder.body(Body::from(body_chunks)).unwrap());
            }
            Err(e) => {
                eprintln!("OHTTP request failed: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from("OHTTP request failed"))
                    .unwrap());
            }
        }
    }

    // Handle CONNECT method for HTTPS tunneling
    if method == Method::CONNECT {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::from("CONNECT method not supported in this simple proxy"))
            .unwrap());
    }

    // For other HTTP methods, forward the request
    let mut proxy_req = Request::builder()
        .method(method)
        .uri(uri);

    // Copy headers
    for (key, value) in headers.iter() {
        proxy_req = proxy_req.header(key, value);
    }

    let proxy_req = proxy_req.body(body).unwrap();

    match client.request(proxy_req).await {
        Ok(response) => Ok(response),
        Err(err) => {
            eprintln!("Error forwarding request: {}", err);
            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("Proxy error"))
                .unwrap())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let cert = match args.kms_cert {
        Some(cert_path) => {
            // Read the certificate file content
            match std::fs::read_to_string(&cert_path) {
              Ok(_content) => PathBuf::from(cert_path),
              Err(e) => {
                eprintln!("Failed to read certificate file: {}", e);
                return Err(anyhow::anyhow!("Failed to read certificate file"));
            }
            }
        }
        None => {
            info!("KMS certificate not provided, querying ACL Identity service...\n");
            let kms_url = args.kms_url.as_deref().unwrap_or_default();
            let re = regex::Regex::new(r"https://([^.]+)\.confidential-ledger\.azure\.com").unwrap();
            let ledger = match re.captures(kms_url) {
              Some(caps) => caps.get(1).map(|m| m.as_str()).unwrap_or(""),
              None => {
                eprintln!("Could not extract ledger name from kms_url");
                return Err(anyhow::anyhow!("Could not extract ledger name from kms_url"));
              }
            };

            let identity_url = format!("https://identity.confidential-ledger.core.azure.com/ledgerIdentity/{}", ledger);

            // Send GET request to identity service
            let resp = reqwest::get(&identity_url).await?;

            if !resp.status().is_success() {
            eprintln!("Failed to fetch ledger identity: {}", resp.status());
            return Err(anyhow::anyhow!("Failed to fetch ledger identity"));
            }
            let json: serde_json::Value = resp.json().await?;
            match json.get("ledgerTlsCertificate") {
            Some(cert_val) => {
                let crt = cert_val.as_str().unwrap_or("").to_string();
                let temp_cert_path = PathBuf::from("kms_cert.pem");
                if let Err(_e) = std::fs::write(&temp_cert_path, &crt) {
                    return Err(anyhow::anyhow!("Failed to write KMS certificate to kms_cert.pem"));
                };
                temp_cert_path
            },
            None => {
                eprintln!("ledgerTlsCertificate not found in response");
                return Err(anyhow::anyhow!("ledgerTlsCertificate not found in response"));
            }
            }
        }
    };

    // Create socket address
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    println!("Starting HTTP proxy on {}", addr);

    // Get KMS URL for the service
    let kms_url = args.kms_url.unwrap_or_default();

    // Create make service function
    let make_service = make_service_fn(move |_conn| {        
        let kms_url = kms_url.clone();
        let cert = cert.clone();

        async move {
            // Create HTTP client for forwarding requests
            let client = Client::new();
            
            Ok::<_, Infallible>(service_fn(move |req| {
                let client = client.clone();
                let kms_url = kms_url.clone();
                let cert = cert.clone();
                async move {
                    handle_request(req, client, kms_url, cert).await
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}
