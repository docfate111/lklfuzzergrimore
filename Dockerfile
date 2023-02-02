FROM ubuntu:latest

ENV DEBIAN_FRONTEND noninteractive
RUN apt update -y && \
    apt install -y lld bison bc make python3 libncurses-dev libssl-dev libelf-dev flex curl git clang && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \    
    ln -s $(which python2) /usr/bin/python && \
   . $HOME/.cargo/env && git clone https://github.com/docfate111/lklfuzzergrimore.git && \
    cd lklfuzzergrimore && git clone https://github.com/lkl/linux.git &&  cargo build --bin libafl_cc --release && cd linux && CC=/lklfuzzergrimore/target/release/libafl_cc HOSTCC=/lklfuzzergrimore/target/release/libafl_cc LLVM=/usr/bin make -C tools/lkl -j`nproc` CROSS_COMPILE=x86_64-linux-gnu  
