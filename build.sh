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
<<<<<<< HEAD

cargo +nightly build --bin libafl_cc
cargo +nightly build --bin libafl_cxx
=======
# i removed the +nigthly
cargo build --release

>>>>>>> 374941173992dcc21af17c789a6cf671197e0aa1
export CC=`pwd`/target/release/libafl_cc
export CXX=`pwd`/target/release/libafl_cxx
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'
cargo +nightly build
