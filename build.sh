#!/bin/sh
git clone https://github.com/lkl/linux.git
pushd linux
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'
sed -i -e 's/CFLAGS=/CFLAGS+=/' Makefile
CONFIG_CLANG=y make -j`nproc`
popd

cp linux/tools/lkl/liblkl.a .
unset CC
unset CXX
unset CFLAGS
unset CXXFLAGS

cargo +nightly build --release

export CC=`pwd`/target/release/libafl_cc
export CXX=`pwd`/target/release/libafl_cxx
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'

