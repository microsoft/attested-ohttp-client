# Attested OHTTP Client

This repository contains a reference implementation of an attested OHTTP client for 
Azure AI confidential inferencing on Linux.

## Prerequisites 

1. Linux environment. 
    You can use [![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/microsoft/attested-ohttp-client)
2. An AzureML endpoint with a confidential whisper model. 
3. Docker 

## Using pre-built image

You can use pre-built attested OHTTP container images to send an inferencing request. 

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

Run inferencing using your own audio file by mounting the file into the container.
```
export KMS_URL=https://accconfinferenceprod.confidential-ledger.azure.com
export INPUT_PATH=<path_to_your_input_audio_file_exclusing_name>
export INPUT_FILE=<name_of_your_audio_file>
export MOUNTED_PATH=/test
docker run -e KMS_URL=${KMS_URL} --volume ${INPUT_PATH}:${MOUNTED_PATH} \
  mcr.microsoft.com/acc/samples/attested-ohttp-client:latest \
  ${TARGET_URI} -F "file=@${MOUNTED_PATH}/${INPUT_FILE}" -O "api-key: ${API_KEY}" -F "response_format=json"
```

## Building your own container image

### Development Environment

The repo supports development using GitHub Codespaces and devcontainers. The repository includes a devcontainer configuration that installs all dependencies. 

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/microsoft/attested-ohttp-client)

You can build the client container using docker.
```
docker build -f docker/Dockerfile -t attested-ohttp-client .
```

Alternatively, you can setup your own environment by installing dependencies.
```
sudo apt update
sudo apt install -y curl build-essential jq libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build the client using cargo. 
```
cargo build --bin ohttp-client
```
