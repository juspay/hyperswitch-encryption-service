FROM rust:bookworm as builder

WORKDIR /cripta

COPY . .
RUN cargo build --release --features release


FROM debian:bookworm

ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local/bin

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq-dev curl procps

EXPOSE 5000

ARG BINARY=cripta

RUN mkdir -p ${BIN_DIR}

ENV CONFIG_DIR=${CONFIG_DIR} \
    BINARY=${BINARY}

COPY --from=builder /cripta/config/development.toml ${CONFIG_DIR}/development.toml
COPY --from=builder /cripta/target/release/${BINARY} ${BIN_DIR}/${BINARY}

WORKDIR ${BIN_DIR}

CMD ./${BINARY}
