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

FROM docker.io/library/alpine:edge AS builder-base-base
RUN echo https://dl-cdn.alpinelinux.org/alpine/edge/testing >> /etc/apk/repositories
RUN apk --no-cache add git rust cargo pkgconfig openssl-dev

FROM builder-base-base AS dep-builder
# https://github.com/trunk-rs/trunk/pull/868, required for enable-threads=true (fails to build when disabled)
RUN git clone https://github.com/trunk-rs/trunk /trunk
WORKDIR /trunk
RUN git fetch origin fe4fc9d2509843f787dfc65f89111adc1987e059 && git checkout fe4fc9d2509843f787dfc65f89111adc1987e059
RUN --mount=type=cache,target=/root/.cargo,id=alpine_cargo_dir --mount=type=cache,target=/trunk/target,id=trunk_target cargo build --release --locked && mv target/release/trunk /trunk.bin

FROM builder-base-base AS builder-base
RUN apk --no-cache add rust-wasm binaryen dart-sass wasm-bindgen
COPY --from=dep-builder /trunk.bin /usr/local/bin/trunk
