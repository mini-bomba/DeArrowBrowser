# Using alpine:edge instead of rust:alpine to get access to packages from testing
# also prebuilt trunk is available which reduces initial container build times
FROM docker.io/library/alpine:edge AS alpine-builder
RUN echo https://dl-cdn.alpinelinux.org/alpine/edge/testing >> /etc/apk/repositories
RUN apk --no-cache add git rust rust-wasm binaryen dart-sass trunk
ADD . /source
WORKDIR /source
# Bring back .dockerignored files to avoid triggering "uncommited changes" labels in info menus
RUN git restore config.toml.example Dockerfile LICENSE README.md .dockerignore .gitignore
RUN --mount=type=cache,target=/root/.cargo,id=alpine_cargo_dir --mount=type=cache,target=/source/target,id=dearrow_browser_target touch /source/add_metadata.rs && cargo build --release --bin dearrow-browser-server && cp /source/target/release/dearrow-browser-server /
WORKDIR /source/dearrow-browser-frontend
RUN --mount=type=cache,target=/root/.cargo,id=alpine_cargo_dir --mount=type=cache,target=/source/target,id=dearrow_browser_target touch /source/add_metadata.rs && trunk build --release


FROM docker.io/library/alpine:latest
RUN apk --no-cache add libgcc
COPY --from=alpine-builder /source/dearrow-browser-frontend/dist/ /static/
COPY --from=alpine-builder /dearrow-browser-server /usr/bin/dearrow-browser-server
WORKDIR /
CMD ["/usr/bin/dearrow-browser-server"]
