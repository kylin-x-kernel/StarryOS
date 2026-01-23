# Starry OS

*An experimental monolithic OS based on ArceOS*

[![GitHub Stars](https://img.shields.io/github/stars/Starry-OS/StarryOS?style=for-the-badge)](https://github.com/Starry-OS/StarryOS/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/Starry-OS/StarryOS?style=for-the-badge)](https://github.com/Starry-OS/StarryOS/network)
[![GitHub License](https://img.shields.io/github/license/Starry-OS/StarryOS?style=for-the-badge)](https://github.com/Starry-OS/StarryOS/blob/main/LICENSE)
[![Build status](https://img.shields.io/github/check-runs/Starry-OS/StarryOS/main?style=for-the-badge)](https://github.com/Starry-OS/StarryOS/actions)

## Supported Architectures

- [x] RISC-V 64
- [x] LoongArch64
- [x] AArch64
- [ ] x86_64 (work in progress)

## Features

TODO

## Quick Start

### Build with Yocto (Recommended)

StarryOS is designed to be built with the Yocto Project for full embedded Linux system integration.

#### 1. Sync repositories

```bash
mkdir -p starryos-workspace && cd starryos-workspace

repo init -u https://github.com/kylin-x-kernel/starryos-manifest -m base.xml

repo sync -j$(nproc)
```

#### 2. Build with Yocto

```bash
cd starryos-workspace
source poky/oe-init-build-env build

# 在build/conf/local.conf中，模板是
#DL_DIR = "/workspace/downloads"
#SSTATE_DIR = "/workspace/sstate"
#DL_DIR = "/home/yean/yocto-cache/downloads"
#SSTATE_DIR = "/home/yean/yocto-cache/sstate"
#需要进行设置以实现产物复用，本地能跑的可以先在本地跑出产物
#这里的两套路径上面两行是docker启动的，下面两行是本地直接跑的，这里的路径换成你自己的，最简单就是直接把yean换成你的用户名，写的不对会报错
#直接改好了取消注释。关于docker启动见.docker文件夹的readme（还没提，今天提上来），如果你自己直接下ubuntu的docker镜像也可以，改好了进行下面。
# Build StarryOS kernel
bitbake starry

# Or build complete system image
bitbake starry-minimal-image

# Or runqemu for starryos
runqemu starry-minimal-image nographic
```

**Note:** If the `local.conf.sample` template in `meta-starry/conf/` has been updated, you may need to manually merge the changes into your `build/conf/local.conf`

#### 3. Daily development 


```bash
# Sync all repositories to latest commits on their branches (recommended)
repo sync -c

# Sync all repositories (fetches all branches, slower)
repo sync

# Sync specific repository only
repo sync StarryOS
repo sync arceos

# View status of all repositories
repo status

# View current branches
repo branches
```
more details in Developer.md 

### Standalone Build 

If you need to quickly test without Yocto, you can use the standalone build:

#### 1. Install dependencies

```bash
# Rust toolchain
rustup target add aarch64-unknown-none-softfloat

# QEMU (Debian/Ubuntu)
sudo apt install qemu-system

# Musl toolchain (optional, for userspace programs)
# Download from https://github.com/arceos-org/setup-musl/releases
```

#### 2. Prepare rootfs

```bash
# Default target: riscv64
make rootfs
# Explicit target
make ARCH=riscv64 rootfs
make ARCH=loongarch64 rootfs
```

This will download rootfs image from [Starry-OS/rootfs](https://github.com/Starry-OS/rootfs/releases) and set up the disk file for running on QEMU.

#### 3. Build and run on QEMU

```bash
# Default target: riscv64
make build
# Explicit target
make ARCH=riscv64 build
make ARCH=loongarch64 build

# Run on QEMU (also rebuilds if necessary)
make ARCH=riscv64 run
make ARCH=loongarch64 run
```

**Note:**

1. Binary dependencies will be automatically built during `make build`.
2. You don't have to rerun `build` every time. `run` automatically rebuilds if necessary.
3. The disk file will **not** be reset between each run. As a result, if you want to switch to another architecture, you must run `make rootfs` with the new architecture before `make run`.

## What next?

You can check out the [GUI guide](./docs/x11.md) to set up a graphical environment, or explore other documentation in this folder.

If you're interested in contributing to the project, please see our [Contributing Guide](./CONTRIBUTING.md).

See more build options in the [Makefile](./Makefile).

## License

This project is now released under the Apache License 2.0. All modifications and new contributions in our project are distributed under the same license. See the [LICENSE](./LICENSE) and [NOTICE](./NOTICE) files for details.
