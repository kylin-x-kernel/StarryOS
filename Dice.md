# Dice Build Guide

## 前提条件
确保能够访问公司内网

## 更新 Cargo.toml
执行 dice.sh 脚本更新 Cargo.toml,增加 dice 和 mebedtls 的依赖
```bash
bash dice-dep.sh
```

## 编译
使用如下命令编译项目并开启 dice 特性
```bash
make dice
```