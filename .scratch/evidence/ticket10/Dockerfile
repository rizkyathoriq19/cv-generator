# Build the Linux packages against the accepted Ubuntu 24.04 LTS x64 baseline.
#
# A binary built on Arch/Omarchy links against a newer glibc than Ubuntu 24.04
# ships, so the package must be produced inside the baseline itself. The image
# only provisions build tooling; the deb/AppImage still run without network.
#
#   docker build -t cvgen-ubuntu2404-build .scratch/evidence/ticket10
#   docker run --rm --user 1000:1000 -e HOME=/home/ubuntu \
#     -v "$PWD:/work" -w /work \
#     -e APPIMAGE_EXTRACT_AND_RUN=1 \
#     -e CARGO_TARGET_DIR=/work/src-tauri/target/ubuntu-24.04 \
#     cvgen-ubuntu2404-build \
#     bash -lc 'npm ci && npx tauri build --bundles deb,appimage'
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/home/ubuntu/.cargo \
    RUSTUP_HOME=/home/ubuntu/.rustup \
    PATH=/home/ubuntu/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      curl \
      file \
      git \
      libayatana-appindicator3-dev \
      libgtk-3-dev \
      librsvg2-dev \
      libssl-dev \
      libwebkit2gtk-4.1-dev \
      libxdo-dev \
      patchelf \
      xz-utils \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN chown -R ubuntu:ubuntu /home/ubuntu
USER ubuntu
WORKDIR /home/ubuntu

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain stable \
    && cargo --version && node --version
