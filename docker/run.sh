#!/bin/bash

is_valid_url() {
    local url="$1"

    # Regular expression to validate the URL
    local regex='^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$'

    if [[ $url =~ $regex ]]; then
        return 0
    else
        return 1
    fi
}

if [[ -n ${KMS_URL} ]]; then 
  if is_valid_url $KMS_URL; then 
    # Obtain KMS service certificate
    curl -s -k ${KMS_URL}/node/network | jq -r .service_certificate > /tmp/service_cert.pem
    ARGS="$ARGS --kms-url ${KMS_URL} --kms-cert /tmp/service_cert.pem"
  else 
    echo "Invalid KMS URL"
    exit 1
  fi
else 
  ARGS="$ARGS --config `curl -s http://localhost:9443/discover`"
fi

echo "Running /usr/local/bin/ohttp-client" "$@" ${ARGS}
RUST_LOG=info /usr/local/bin/ohttp-client "$@" ${ARGS}
