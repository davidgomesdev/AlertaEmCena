FROM ubuntu:22.04

RUN apt-get update && apt-get install -y software-properties-common && \
    apt-add-repository ppa:fish-shell/release-3 && \
    apt-get install -y \
        curl \
        git \
        gnupg2 \
        jq \
        sudo \
        fish \
        vim \
        build-essential \
        openssl \
        libssl-dev \
        cmake \
        pkg-config \
        libssh-dev

RUN echo /usr/bin/fish >> /etc/shells && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    echo 'source /root/.cargo/env' >> /root/.bashrc && \
    echo 'set PATH $PATH /root/.cargo/bin' >> ~/.config/fish/config.fish

ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install cargo-expand && \
    cargo install cargo-edit

SHELL ["fish", "--command"]

ENV SHELL /usr/bin/fish
ENV LANG=C.UTF-8 LANGUAGE=C.UTF-8 LC_ALL=C.UTF-8

COPY . .

RUN chown root:root .
