FROM ubuntu:latest

ENV DEBIAN_FRONTEND noninteractive
RUN apt update -y && \
    apt install -y bison bc make python3 libncurses-dev libssl-dev libelf-dev flex curl git clang && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \    
   . $HOME/.cargo/env && git clone https://github.com/docfate111/lklfuzzergrimore.git && \
    cd lklfuzzergrimore && cargo build --bin libafl_cc --release && \
    rm -rf linux && git clone -b clang14compatibility https://github.com/docfate111/linux-lkl.git linux && cd linux && make -C tools/lkl CC=/lklfuzzergrimore/target/release/libafl_cc HOSTCC=/lklfuzzergrimore/target/release/libafl_cc -j`nproc`   
