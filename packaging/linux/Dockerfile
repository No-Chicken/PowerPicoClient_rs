FROM ubuntu:22.04 AS builder

ARG DEBIAN_FRONTEND=noninteractive
ARG NODE_VERSION=20.19.4
ARG RUST_VERSION=1.88.0
ARG PNPM_VERSION=10.18.1

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      curl \
      file \
      libayatana-appindicator3-dev \
      libssl-dev \
      libudev-dev \
      libwebkit2gtk-4.1-dev \
      libxdo-dev \
      librsvg2-dev \
      patchelf \
      pkg-config \
      wget \
      xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
    | tar -xJ --strip-components=1 -C /usr/local

RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain "${RUST_VERSION}"

ENV PATH="/root/.cargo/bin:${PATH}"

RUN corepack enable \
    && corepack prepare "pnpm@${PNPM_VERSION}" --activate

WORKDIR /workspace

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .

RUN pnpm tauri build --bundles deb,appimage

FROM scratch AS artifacts

COPY --from=builder /workspace/src-tauri/target/release/bundle/deb/*.deb /deb/
COPY --from=builder /workspace/src-tauri/target/release/bundle/appimage/*.AppImage /appimage/
