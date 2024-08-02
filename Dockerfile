#  This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
#
#  Copyright (C) 2024 mini_bomba
# 
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU Affero General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU Affero General Public License for more details.
#
#  You should have received a copy of the GNU Affero General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.

# Using alpine:edge instead of rust:alpine to get access to packages from testing
# also prebuilt trunk is available which reduces initial container build times
FROM docker.io/library/alpine:edge AS builder-base
RUN echo https://dl-cdn.alpinelinux.org/alpine/edge/testing >> /etc/apk/repositories
RUN apk --no-cache add git rust rust-wasm binaryen dart-sass trunk pkgconfig openssl-dev

FROM builder-base AS builder
ADD . /source
WORKDIR /source
# Bring back .dockerignored files to avoid triggering "uncommited changes" labels in info menus
RUN git restore config.toml.example Dockerfile LICENSE README.md .dockerignore .gitignore
RUN --mount=type=cache,target=/root/.cargo,id=alpine_cargo_dir --mount=type=cache,target=/source/target,id=dearrow_browser_target touch /source/add_metadata.rs && cargo build --release --locked --bin dearrow-browser-server && cp /source/target/release/dearrow-browser-server /
WORKDIR /source/dearrow-browser-frontend
RUN --mount=type=cache,target=/root/.cargo,id=alpine_cargo_dir --mount=type=cache,target=/source/target,id=dearrow_browser_target touch /source/add_metadata.rs && trunk build --release --locked --offline --minify


FROM docker.io/library/alpine:latest AS full-server
RUN apk --no-cache add libgcc
COPY --from=builder /source/dearrow-browser-frontend/dist/ /static/
COPY --from=builder /dearrow-browser-server /usr/bin/dearrow-browser-server
WORKDIR /
CMD ["/usr/bin/dearrow-browser-server"]
