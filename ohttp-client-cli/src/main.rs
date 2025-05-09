// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use clap::Parser;
use core::str;
use ohttp_client::{HexArg, OhttpClientBuilder};
use std::path::PathBuf;
use tracing::error;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Parser)]
#[command(version = "0.1", about = "Make an oblivious HTTP request.")]
struct Args {
    /// The URL of an oblivious proxy resource.
    /// If you use an oblivious request resource, this also works, though
    /// you don't get any of the privacy guarantees.
    url: String,

    /// Target path of the oblivious resource
    #[arg(long, short = 'p', default_value = "/")]
    target_path: String,

    /// key configuration
    #[arg(long, short = 'c')]
    config: Option<HexArg>,

    /// URL of the KMS to obtain HPKE keys from
    #[arg(long, short = 'f')]
    kms_url: Option<String>,

    /// Trusted KMS service certificate
    #[arg(long, short = 'k')]
    kms_cert: Option<PathBuf>,

    /// Where to write response content.
    /// If you omit this, output is written to `stdout`.
    #[arg(long, short = 'o')]
    output: Option<PathBuf>,

    /// When creating message/bhttp, use the indeterminate-length form.
    #[arg(long, short = 'n', alias = "indefinite")]
    indeterminate: bool,

    /// List of headers in the inner request
    #[arg(long, short = 'H')]
    headers: Option<Vec<String>>,

    #[arg(long, short = 'd', default_value = "")]
    data: Option<String>,

    /// List of fields in the inner request
    #[arg(long, short = 'F')]
    form_fields: Option<Vec<String>>,

    /// List of headers in the outer request
    #[arg(long, short = 'O')]
    outer_headers: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Res<()> {
    // Build a simple subscriber that outputs to stdout
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_file(true)
        .with_line_number(true)
        .finish();

    // Set the subscriber as global default
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let args = Args::parse();
    let ohttp_client = OhttpClientBuilder::new()
        .kms_url(&args.kms_url)
        .kms_cert(&args.kms_cert)
        .config(&args.config)
        .build()
        .await?;

    let mut response = ohttp_client
        .post(
            &args.url,
            &args.target_path,
            &args.headers,
            &args.data,
            &args.form_fields,
            &args.outer_headers,
        )
        .await?;

    let status = response.status();
    if status.is_success() {
        while let Some(chunk) = response.chunk().await? {
            let chunk = str::from_utf8(&chunk)?;
            println!("{chunk}");
        }
    } else {
        error!("Request failed with status {status} {}", response.text().await.unwrap_or_default());
    }
    Ok(())
}
