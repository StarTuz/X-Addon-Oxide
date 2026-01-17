FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    libasound2-dev \
    libfontconfig1-dev \
    libwayland-dev \
    libx11-dev \
    libxkbcommon-dev \
    libdbus-1-dev \
    wget \
    file \
    fuse \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install linuxdeploy
RUN wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage -O /usr/local/bin/linuxdeploy && \
    chmod +x /usr/local/bin/linuxdeploy

# Install linuxdeploy-plugin-appimage
RUN wget https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage -O /usr/local/bin/linuxdeploy-plugin-appimage && \
    chmod +x /usr/local/bin/linuxdeploy-plugin-appimage

WORKDIR /app

CMD ["/bin/bash"]
