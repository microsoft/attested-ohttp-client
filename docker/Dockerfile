FROM rust:1.75 AS builder

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -yq openssl libssl-dev

WORKDIR /usr/src/ohttp
COPY . .
RUN cargo install --path ohttp-client --debug

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y ca-certificates openssl curl jq && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/ohttp-client /usr/local/bin/ohttp-client
COPY ./examples/audio.mp3 ./examples/audio.mp3
COPY ./examples/15m_gpt-has-entered-the-chat.mp3 ./examples/15m_gpt-has-entered-the-chat.mp3
COPY --chmod=755 ./docker/run.sh .
ENV RUST_LOG=info
ENTRYPOINT [ "./run.sh" ]