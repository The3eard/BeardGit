# E2E test runner image for BeardGit.
#
# Tauri-driver only runs on Linux (WebKitGTK) and Windows (Edge WebView2), so
# local/dev validation on macOS has to go through a Linux container. This
# mirrors the CI environment in ci.yml:e2e-tests as closely as possible —
# ubuntu-22.04, webkit2gtk-4.1, xvfb, tauri-driver, node 22, rust stable.
#
# Usage from repo root:
#   docker build -f e2e/Dockerfile -t beardgit-e2e .
#   docker run --rm -it \
#     -v "$PWD:/workspace" \
#     -v beardgit-e2e-cargo-registry:/root/.cargo/registry \
#     -v beardgit-e2e-cargo-git:/root/.cargo/git \
#     -v beardgit-e2e-target:/workspace/target \
#     -v beardgit-e2e-node_modules:/workspace/node_modules \
#     beardgit-e2e \
#     bash e2e/run-in-docker.sh
#
# Named volumes keep cargo + node_modules warm across runs so iteration after
# the first successful build is fast (<3 min per wdio cycle).

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    PATH=/root/.cargo/bin:/usr/local/bin:/usr/bin:/bin

# Tauri runtime deps + build tooling + xvfb for headless display
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        curl \
        ca-certificates \
        git \
        file \
        xz-utils \
        xvfb \
        libwebkit2gtk-4.1-dev \
        webkit2gtk-driver \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf \
        unzip \
        procps \
    && rm -rf /var/lib/apt/lists/*

# Node 22 (matches CI setup-node@v4)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Rust stable via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup --version \
    && cargo --version

# tauri-driver compiled once into the image (tauri-driver-v2 is tied to the
# tauri 2.x runtime we use). Output to /usr/local/bin so it's on PATH.
RUN cargo install tauri-driver --locked \
    && cp /root/.cargo/bin/tauri-driver /usr/local/bin/tauri-driver

WORKDIR /workspace

# Container entrypoint is explicit bash — the actual script lives in the repo
# so iterating on it doesn't require image rebuilds.
CMD ["bash"]
