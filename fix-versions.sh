#!/bin/bash
# 将 repo 管理的仓库固定到与子模块相同的版本
# 使用方法：在 StarryOS 目录下运行 ./fix-versions.sh

cd "$(dirname "$0")" || exit 1

echo "=== 固定所有仓库到子模块版本 ==="

# arceos
echo "→ arceos: 5569b4d"
cd arceos && git checkout 5569b4d 2>/dev/null && cd .. || echo "  跳过"

# arm-gic
echo "→ arm-gic: 35bfb52"
cd local_crates/arm-gic && git checkout 35bfb52 2>/dev/null && cd ../.. || echo "  跳过"

# axcpu
echo "→ axcpu: df21232"
cd local_crates/axcpu && git checkout df21232 2>/dev/null && cd ../.. || echo "  跳过"

# axdriver_crates
echo "→ axdriver_crates: cab57c7"
cd local_crates/axdriver_crates && git checkout cab57c7 2>/dev/null && cd ../.. || echo "  跳过"

# axplat-aarch64-crosvm-virt
echo "→ axplat-aarch64-crosvm-virt: 3b9ef26"
cd local_crates/axplat-aarch64-crosvm-virt && git checkout 3b9ef26 2>/dev/null && cd ../.. || echo "  跳过"

# axplat_crates
echo "→ axplat_crates: 28d9b73"
cd local_crates/axplat_crates && git checkout 28d9b73 2>/dev/null && cd ../.. || echo "  跳过"

# fdtree-rs
echo "→ fdtree-rs: d69bcb0"
cd local_crates/fdtree-rs && git checkout d69bcb0 2>/dev/null && cd ../.. || echo "  跳过"

# kernel_guard
echo "→ kernel_guard: 58b0f7b"
cd local_crates/kernel_guard && git checkout 58b0f7b 2>/dev/null && cd ../.. || echo "  跳过"

# page_table_multiarch
echo "→ page_table_multiarch: 01df818"
cd local_crates/page_table_multiarch && git checkout 01df818 2>/dev/null && cd ../.. || echo "  跳过"

echo ""
echo "✓ 版本固定完成！现在可以编译了："
echo "  make build"
