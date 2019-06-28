FROM ubuntu:18.04
LABEL maintainer="huajnan@foxmail.com"
LABEL description="This is a docker for turing node"

WORKDIR /turing

COPY ./target/release/turing-node /turing

RUN apt-get update \
    && apt-get install -y libssl-dev \
    ca-certificates

EXPOSE 30333 9933 9944