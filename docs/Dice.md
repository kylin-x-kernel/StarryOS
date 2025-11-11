# Dice Build Guide

## Prerequisites
Ensure you have access to the company intranet to retrieve necessary dependencies.

## Update Cargo.toml
Execute the dice-dep.sh script to modify the Cargo.toml file, adding the required dependencies for dice and mbedtls.
```bash
bash scripts/dice-dep.sh
```

## Compilation
Use the following command to compile the project with the dice feature enabled. 
```bash
make dice
```