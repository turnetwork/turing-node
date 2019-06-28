FROM ubuntu:18.04
LABEL maintainer="hjn@foxmail.com"
LABEL description="This is a docker for turing node"

WORKDIR /turing

# Update rust dependencies
ENV RUSTUP_HOME "/turing/.rustup"
ENV CARGO_HOME "/turing/.cargo"
RUN apt-get update \
    && apt install curl \
    && curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH "$PATH:/turing/.cargo/bin"
RUN rustup update nightly

COPY ./target/release/turing-node /turing

EXPOSE 30333 9933 9944
VOLUME ["/data"]
