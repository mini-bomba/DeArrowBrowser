FROM docker.io/library/rust:latest AS builder
RUN rustup target add wasm32-unknown-unknown 
RUN cargo install trunk
COPY . /source
WORKDIR /source
RUN cargo build --release --bin dearrow-browser-server
WORKDIR /source/dearrow-browser-frontend
RUN trunk build --release

FROM docker.io/library/rust:slim
COPY --from=builder /source/dearrow-browser-frontend/dist /static
COPY --from=builder /source/target/release/dearrow-browser-server /usr/bin/dearrow-browser-server
WORKDIR /
CMD ["/usr/bin/dearrow-browser-server"]
