# Build commands

build-container:
	docker build -f docker/Dockerfile -t attested-ohttp-client .

format-checks:
	cargo fmt --all -- --check --config imports_granularity=Crate
	cargo clippy --tests --no-default-features --features rust-hpke

# Containerized client deployment

run-client-container:
	docker run --net=host --volume ${INPUT_DIR}:${MOUNTED_INPUT_DIR} attested-ohttp-client \
	$(SCORING_ENDPOINT) -F "file=@${MOUNTED_INPUT_DIR}/${INPUT_FILE}" -O "api-key: ${API_KEY}"
