FROM ubuntu:latest

ENV DEBIAN_FRONTEND noninteractive
RUN apt update -y && \
    apt install -y lld bison bc make python3 libncurses-dev libssl-dev libelf-dev flex curl git clang && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \    
    ln -s $(which python2) /usr/bin/python && \
   . $HOME/.cargo/env && mkdir /home/t && cd /home/t && git clone https://github.com/docfate111/libafl_cc-for-lkl.git && \
    cd libafl_cc-for-lkl && cargo build --bin libafl_cc --release && cd / && \
   git clone https://github.com/docfate111/lklfuzzergrimore && cd lklfuzzergrimore
