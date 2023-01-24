#!/bin/sh
apt install -y bc make flex bison libncurses-dev libelf-dev libssl-dev
git clone https://github.com/lkl/linux.git
cd linux
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'
sed -i -e 's/CFLAGS=/CFLAGS+=/' Makefile
CONFIG_CLANG=y ARCH=lkl make -j`nproc`
cd ..

cp linux/tools/lkl/liblkl.a .
unset CC
unset CXX
unset CFLAGS
unset CXXFLAGS
# i removed the +nigthly
cargo build --release

export CC=`pwd`/target/release/libafl_cc
export CXX=`pwd`/target/release/libafl_cxx
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'

