FROM rust:slim-bookworm as builder


RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config

WORKDIR /cripta

COPY . .
RUN cargo build --release --features release


FROM debian:bookworm-slim

ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local/bin

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq-dev curl procps

EXPOSE 5000

ARG BINARY=cripta

RUN mkdir -p ${BIN_DIR}

ENV CONFIG_DIR=${CONFIG_DIR} \
    BINARY=${BINARY}

COPY --from=builder /cripta/target/release/${BINARY} ${BIN_DIR}/${BINARY}

WORKDIR ${BIN_DIR}

CMD ./${BINARY}
