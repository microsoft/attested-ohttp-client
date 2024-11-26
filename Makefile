# Build commands

build-cli:
	cargo build --bin ohttp-client-cli

build-container:
	docker build -f docker/Dockerfile -t attested-ohttp-client .

build-pyohttp:
	./scripts/build-pyohttp.sh

format-checks:
	cargo fmt --all -- --check --config imports_granularity=Crate
	cargo clippy --tests --no-default-features --features rust-hpke

run-client:
	curl -s -k ${KMS_URL}/node/network | jq -r .service_certificate > /tmp/service_cert.pem
	cargo run -- ${TARGET_URI} -F "file=@${INPUT_DIR}/${INPUT_FILE}" \
		-O "api-key: ${API_KEY}" --kms-url=${KMS_URL} --kms-cert=/tmp/service_cert.pem

# Containerized client deployment

run-client-container:
	docker run --net=host --volume ${INPUT_DIR}:${MOUNTED_INPUT_DIR} attested-ohttp-client \
	$(TARGET_URI) -F "file=@${MOUNTED_INPUT_DIR}/${INPUT_FILE}" -O "api-key: ${API_KEY}"
