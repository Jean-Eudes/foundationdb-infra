ARG FDB_VERSION=7.3.43
ARG RUST_VERSION=1.83.0
# Build Stage
FROM rust:${RUST_VERSION}-bullseye as builder

ARG FDB_VERSION

RUN wget "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-clients_${FDB_VERSION}-1_amd64.deb"
RUN dpkg -i foundationdb-clients_${FDB_VERSION}-1_amd64.deb

RUN apt-get update; apt-get install -y --no-install-recommends libclang-dev

WORKDIR /app

# Copy only the necessary files for dependency resolution
RUN mkdir s3
COPY Cargo.toml Cargo.lock ./
COPY s3/Cargo.toml s3/
RUN mkdir s3/src && echo "fn main() {}" > s3/src/main.rs
RUN cargo build --release

# Copy the rest of the source code
# COPY src ./src

COPY s3 ./s3
# Build the Rust project
RUN cargo build --release

RUN ls -l target/release
# Final Stage
FROM debian:bullseye
ARG FDB_VERSION

RUN apt update && apt install -y wget curl dnsutils

WORKDIR /tmp

RUN wget "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-clients_${FDB_VERSION}-1_amd64.deb"
RUN dpkg -i foundationdb-clients_${FDB_VERSION}-1_amd64.deb

WORKDIR /app

RUN echo "docker:docker@10.88.0.2:4500" > /etc/foundationdb/fdb.cluster

# Copy the built artifact from the build stage
COPY --from=builder /app/target/release/foundationdb-s3 .
# ADD .github/docker/run.sh /app/docker_entrypoint.sh

EXPOSE 3000 
EXPOSE 4000

# Set the command to run on container start
ENTRYPOINT ["./foundationdb-s3"]
