#!/bin/sh
git clone https://github.com/lkl/linux.git
pushd linux
export CFLAGS='-fsanitize-coverage=trace-pc-guard'
export CXXFLAGS='-fsanitize-coverage=trace-pc-guard'
sed -i -e 's/CFLAGS=/CFLAGS+=/' Makefile
CONFIG_CLANG=y make -j`nproc`
popd

cp linux/tools/lkl/liblkl.a .

