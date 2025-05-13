# Attested OHTTP Client

This repository contains a reference implementation of an attested OHTTP client for 
Azure AI confidential inferencing on Linux.

## Prerequisites 

1. Linux environment. 
    You can use [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/microsoft/attested-ohttp-client)
2. An AzureML endpoint with a confidential whisper model. 
3. Docker 

## Clients

We support the following client profiles. 

- [Python package](#python-package)
- [Docker CLI](#docker)
- [Rust CLI](#rust)

Rust and python packages can be built using this repo. We support development and build using GitHub Codespaces and devcontainers. The repository includes a devcontainer configuration that installs all dependencies. 

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/microsoft/attested-ohttp-client)

## Python package

Build the pyohttp package as follows. 

```shell
./scripts/build-pyohttp.sh
```
Install the package from ```target/wheels``` using pip. The [sample python script](samples/ohttp-client-cli.py) shows how use this package and make an attested OHTTP inference request to a confidential whisper endpoint. 

## Docker

### Using pre-built image

You can use pre-built attested OHTTP container images to send inferencing requests and print completions. 

Set the inferencing endpoint and access key as follows.
```
export TARGET_URI=<URL for your endpoint>
export API_KEY=<key for accessing the endpoint>
```

Run inferencing using a pre-packaged audio file. 
```
export KMS_URL=https://accconfinferenceprod.confidential-ledger.azure.com
docker run -e KMS_URL=${KMS_URL} mcr.microsoft.com/acc/samples/attested-ohttp-client:latest \
  ${TARGET_URI} -F "file=@/examples/audio.mp3" -O "api-key: ${API_KEY}" -F "response_format=json"
```

Run inferencing using a pre-packaged audio file and receive the attestation token.
The attestation token will be returned as a blob. It can be decoded at [jwt.io](https://jwt.io/).
```
export KMS_URL=https://accconfinferenceprod.confidential-ledger.azure.com
docker run -e KMS_URL=${KMS_URL} mcr.microsoft.com/acc/samples/attested-ohttp-client:latest \
  ${TARGET_URI} -F "file=@/examples/audio.mp3" -O "api-key: ${API_KEY}" -O "x-attestation-token:true" \
  -F "response_format=json"
```

Run inferencing using your own audio file by mounting the file into the container.
The maximum audio file size supported is 25MB.
```
export KMS_URL=https://accconfinferenceprod.confidential-ledger.azure.com
export INPUT_PATH=<path_to_your_input_audio_file_excluding_name>
export INPUT_FILE=<name_of_your_audio_file>
export MOUNTED_PATH=/test
docker run -e KMS_URL=${KMS_URL} --volume ${INPUT_PATH}:${MOUNTED_PATH} \
  mcr.microsoft.com/acc/samples/attested-ohttp-client:latest \
  ${TARGET_URI} -F "file=@${MOUNTED_PATH}/${INPUT_FILE}" -O "api-key: ${API_KEY}" -F "response_format=json"
```

### Build your own container image

You can build the client container using docker.
```
docker build -f docker/Dockerfile -t attested-ohttp-client .
```

## Rust 
Setup environment by installing dependencies.
```
sudo apt update
sudo apt install -y curl build-essential jq libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build the client using cargo. 
```
cargo build --bin ohttp-client-cli
```

Run the CLI as follows.
```
curl -s -k ${KMS_URL}/node/network | jq -r .service_certificate > /tmp/service_cert.pem
cargo run --bin=ohttp-client-cli -- ${TARGET_URI} -F "file=examples/audio.mp3" \
	-O "api-key: ${API_KEY}" --kms-url=${KMS_URL} --kms-cert=/tmp/service_cert.pem
```