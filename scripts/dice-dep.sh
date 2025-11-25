#!/bin/bash

# modify_cargo_advanced.sh
CARGO_FILE="Cargo.toml"
API_CARGO_FILE="api/Cargo.toml"

# Function: Check and add configuration items
add_patch_config() {
    local section="$1"
    local key="$2"
    local value="$3"
    
    # Check if section exists
    if ! grep -q "\[$section\]" "$CARGO_FILE"; then
        echo -e "\n[$section]" >> "$CARGO_FILE"
        echo "Section added: [$section]"
    fi
    
    # Check if configuration item exists
    if grep -A 10 "\[$section\]" "$CARGO_FILE" | grep -q "$key ="; then
        echo "Configuration item already exists: $key, skipping"
    else
        # Add configuration item after section
        sed -i.tmp "/\[$section\]/a$key = $value" "$CARGO_FILE"
        rm -f "$CARGO_FILE.tmp"
        echo "Added: $key = $value"
    fi
}

# Function: Modify dependencies in api/Cargo.toml
modify_api_dependencies() {
    local api_file="$1"
    
    if [ ! -f "$api_file" ]; then
        return
    fi
    
    echo "Starting to modify $api_file dependencies..."
    
    # Check if [dependencies] section exists
    if ! grep -q "^\[dependencies\]" "$api_file"; then
        echo -e "\n[dependencies]" >> "$api_file"
        echo "Section added: [dependencies]"
    fi
    
    # Add dice dependency
    if grep -A 20 "^\[dependencies\]" "$api_file" | grep -q "dice ="; then
        echo "dice dependency already exists, skipping"
    else
        sed -i.tmp '/^\[dependencies\]/a\dice = { workspace = true, optional = true }' "$api_file"
        echo "dice dependency added"
    fi
    
    # Add mbedtls dependency
    if grep -A 20 "^\[dependencies\]" "$api_file" | grep -q "mbedtls ="; then
        echo "mbedtls dependency already exists, skipping"
    else
        sed -i.tmp '/^\[dependencies\]/a\mbedtls = { workspace = true, optional = true }' "$api_file"
        echo "mbedtls dependency added"
    fi
    
    # Modify dice feature in features
    if grep -q "^dice = \\[" "$api_file"; then
        # Backup current dice feature line
        local old_dice_line=$(grep "^dice = \\[" "$api_file")
        local new_dice_line='dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]'
        
        # Use sed to replace the entire line
        sed -i.tmp "s|^dice = \\[.*|$new_dice_line|" "$api_file"
        echo "dice feature modified"
        echo "Original configuration: $old_dice_line"
        echo "New configuration: $new_dice_line"
    else
        # If features section exists but dice feature doesn't, add it
        if grep -q "^\[features\]" "$api_file"; then
            sed -i.tmp '/^\[features\]/a\dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]' "$api_file"
            echo "dice feature added to features"
        else
            # If features section doesn't exist, create it first then add
            echo -e "\n[features]" >> "$api_file"
            echo 'dice = ["dep:dice","dep:mbedtls","dep:axplat-aarch64-crosvm-virt", "axalloc/dice","dep:rand_chacha"]' >> "$api_file"
            echo "[features] section created and dice feature added"
        fi
    fi
    
    # Clean up temporary files
    rm -f "$api_file.tmp"
}

# Add patch configurations to root Cargo.toml
add_patch_config "patch.crates-io" "dice" '{ git = "ssh://dev.futlab.me:29418/security/rust-dice", branch = "starry-os" }'
add_patch_config "patch.crates-io" "mbedtls" '{ git = "ssh://dev.futlab.me:29418/security/rust-mbedtls", branch = "smx-mbedtls-sys-auto_v2.28.12" }'

# Add dependency configurations to root Cargo.toml
add_patch_config "workspace.dependencies" "dice" '"0.1.0"'
add_patch_config "workspace.dependencies" "mbedtls" '{ version = "0.13.3", package = "mbedtls", default-features = false, features = ["no_std_deps"] }'

# Modify api/Cargo.toml
modify_api_dependencies "$API_CARGO_FILE"

echo "All modifications completed!"