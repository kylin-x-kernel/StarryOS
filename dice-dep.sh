#!/bin/bash

# modify_cargo_advanced.sh
CARGO_FILE="Cargo.toml"
API_CARGO_FILE="api/Cargo.toml"

# 函数：检查并添加配置项
add_patch_config() {
    local section="$1"
    local key="$2"
    local value="$3"
    
    # 检查段落是否存在
    if ! grep -q "\[$section\]" "$CARGO_FILE"; then
        echo -e "\n[$section]" >> "$CARGO_FILE"
        echo "已添加段落: [$section]"
    fi
    
    # 检查配置项是否存在
    if grep -A 10 "\[$section\]" "$CARGO_FILE" | grep -q "$key ="; then
        echo "配置项已存在: $key，跳过"
    else
        # 在段落后添加配置项
        sed -i.tmp "/\[$section\]/a$key = $value" "$CARGO_FILE"
        rm -f "$CARGO_FILE.tmp"
        echo "已添加: $key = $value"
    fi
}

# 函数：修改api/Cargo.toml的dependencies
modify_api_dependencies() {
    local api_file="$1"
    
    if [ ! -f "$api_file" ]; then
        return
    fi
    
    echo "开始修改 $api_file 的 dependencies..."
    
    # 检查[dependencies]段落是否存在
    if ! grep -q "^\[dependencies\]" "$api_file"; then
        echo -e "\n[dependencies]" >> "$api_file"
        echo "已添加段落: [dependencies]"
    fi
    
    # 添加dice依赖
    if grep -A 20 "^\[dependencies\]" "$api_file" | grep -q "dice ="; then
        echo "dice依赖已存在，跳过添加"
    else
        sed -i.tmp '/^\[dependencies\]/a\dice = { workspace = true, optional = true }' "$api_file"
        echo "已添加dice依赖"
    fi
    
    # 添加mbedtls依赖
    if grep -A 20 "^\[dependencies\]" "$api_file" | grep -q "mbedtls ="; then
        echo "mbedtls依赖已存在，跳过添加"
    else
        sed -i.tmp '/^\[dependencies\]/a\mbedtls = { workspace = true, optional = true }' "$api_file"
        echo "已添加mbedtls依赖"
    fi
    
    # 修改features中的dice特性
    if grep -q "^dice = \\[" "$api_file"; then
        # 备份当前dice特性行
        local old_dice_line=$(grep "^dice = \\[" "$api_file")
        local new_dice_line='dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]'
        
        # 使用sed替换整行
        sed -i.tmp "s|^dice = \\[.*|$new_dice_line|" "$api_file"
        echo "已修改dice特性"
        echo "原配置: $old_dice_line"
        echo "新配置: $new_dice_line"
    else
        # 如果features段落存在但dice特性不存在，则添加
        if grep -q "^\[features\]" "$api_file"; then
            sed -i.tmp '/^\[features\]/a\dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]' "$api_file"
            echo "已添加dice特性到features"
        else
            # 如果features段落不存在，先创建再添加
            echo -e "\n[features]" >> "$api_file"
            echo 'dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]' >> "$api_file"
            echo "已创建[features]段落并添加dice特性"
        fi
    fi
    
    # 清理临时文件
    rm -f "$api_file.tmp"
}

# 添加补丁配置到根Cargo.toml
add_patch_config "patch.crates-io" "dice" '{ git = "ssh://dev.futlab.me:29418/security/rust-dice", branch = "starry-os" }'
add_patch_config "patch.crates-io" "mbedtls" '{ git = "ssh://dev.futlab.me:29418/security/rust-mbedtls", branch = "smx-mbedtls-sys-auto_v2.28.12" }'

# 添加依赖配置到根Cargo.toml
add_patch_config "workspace.dependencies" "dice" '"0.1.0"'
add_patch_config "workspace.dependencies" "mbedtls" '{ version = "0.13.3", package = "mbedtls", default-features = false, features = ["no_std_deps"] }'

# 修改api/Cargo.toml
modify_api_dependencies "$API_CARGO_FILE"

echo "所有修改完成！"