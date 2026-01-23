#!/bin/bash

# 检查参数
if [ $# -eq 0 ]; then
    echo "用法: $0 [files...] -o <image_name>"
    echo "示例: $0 axplat-bootloader.efi"
    echo "示例: $0 axplat-bootloader.efi file1 file2 -o myboot.img"
    exit 1
fi

# 解析参数
IMG_NAME="fat32_boot.img"
IMG_SIZE="50M"
FILES=()

while [ $# -gt 0 ]; do
    case "$1" in
        -o|--output)
            shift
            if [ -z "$1" ]; then
                echo "错误: -o 需要镜像文件名"
                exit 1
            fi
            IMG_NAME="$1"
            ;;
        -h|--help)
            echo "用法: $0 [files...] -o <image_name>"
            echo "示例: $0 axplat-bootloader.efi"
            echo "示例: $0 axplat-bootloader.efi file1 file2 -o myboot.img"
            exit 0
            ;;
        -* )
            echo "错误: 未知选项 $1"
            exit 1
            ;;
        * )
            FILES+=("$1")
            ;;
    esac
    shift
done

if [ ${#FILES[@]} -eq 0 ]; then
    echo "错误: 未提供要写入镜像的文件"
    exit 1
fi

# 检查文件是否存在
for f in "${FILES[@]}"; do
    if [ ! -f "$f" ]; then
        echo "错误: 找不到 $f 文件"
        exit 1
    fi
done

# 检查是否已存在镜像文件
if [ -f "$IMG_NAME" ]; then
    rm -f "$IMG_NAME"
fi

# 创建镜像文件
echo "创建 $IMG_SIZE 的镜像文件 '$IMG_NAME'..."
if ! dd if=/dev/zero of="$IMG_NAME" bs=1 count=0 seek="$IMG_SIZE" status=progress; then
    echo "创建镜像文件失败"
    exit 1
fi

# 格式化为 FAT32
echo "格式化分区为 FAT32..."
if ! mkfs.vfat -F 32 "$IMG_NAME" > /dev/null 2>&1; then
    echo "格式化失败，请检查是否安装了 mkfs.vfat (dosfstools 包)"
    echo "安装命令: sudo apt install dosfstools"
    exit 1
fi

# 创建临时挂载点
TEMP_DIR=$(mktemp -d)
if [ $? -ne 0 ]; then
    echo "创建临时目录失败"
    exit 1
fi

# 挂载并复制文件
echo "挂载镜像并复制文件..."
if ! mount "$IMG_NAME" "$TEMP_DIR" 2>/dev/null; then
    # 如果普通挂载失败，尝试使用 sudo
    echo "需要 root 权限挂载镜像..."
    if ! sudo mount "$IMG_NAME" "$TEMP_DIR"; then
        echo "挂载失败"
        rmdir "$TEMP_DIR"
        exit 1
    fi
    
    # 复制文件
    for f in "${FILES[@]}"; do
        sudo cp "$f" "$TEMP_DIR/"
    done
    
    # 列出文件
    echo "镜像中的文件:"
    sudo ls -la "$TEMP_DIR/"
    
    # 卸载
    sync
    sudo umount "$TEMP_DIR"
else
    # 复制文件
    for f in "${FILES[@]}"; do
        cp "$f" "$TEMP_DIR/"
    done
    
    # 列出文件
    echo "镜像中的文件:"
    ls -la "$TEMP_DIR/"
    
    # 卸载
    sync
    umount "$TEMP_DIR"
fi

# 清理临时目录
rmdir "$TEMP_DIR"

echo "========================================="
echo "镜像文件 '$IMG_NAME' 创建成功！"
echo ""
echo "文件系统信息:"
file "$IMG_NAME"
echo ""
echo "使用示例："
echo "  1. 挂载查看: sudo mount $IMG_NAME /mnt && ls /mnt"
echo "  2. UEFI 测试: 使用 QEMU 启动"
echo "        qemu-system-x86_64 -bios /usr/share/ovmf/OVMF.fd -drive format=raw,file=$IMG_NAME"
echo "========================================="