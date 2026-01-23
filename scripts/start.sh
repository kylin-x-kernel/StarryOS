#!/bin/bash

wget http://172.30.193.50/scripts/disk.sh -O disk.sh
wget http://172.30.193.50/axboot.toml -O axboot.toml
wget http://172.30.193.50/BOOTX64.EFI -O BOOTX64.EFI
wget http://172.30.193.50/StarryOS_x86-csv.elf -O StarryOS_x86-csv.elf
bash disk.sh BOOTX64.EFI StarryOS_x86-csv.elf axboot.toml StarryOS_x86-pc.elf -o myboot.img

echo "启动 QEMU..."
qemu-system-x86_64 -m 1G -smp 1 -machine q35 \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::5556-:5555,hostfwd=udp::5556-:5555 \
    -nographic \
    -device vhost-vsock-pci,id=virtiosocket0,guest-cid=104 \
    -device virtio-blk-pci,drive=disk0 \
    -drive id=disk0,if=none,format=raw,file=disk.img \
    -cpu host -accel kvm \
    -hda myboot.img \
    -object sev-guest,id=sev0,policy=0x1,cbitpos=47,reduced-phys-bits=5 \
    -machine memory-encryption=sev0 \
    -drive if=pflash,format=raw,unit=0,file=OVMF_CODE.fd,readonly=on \
    -drive if=pflash,format=raw,unit=1,file=OVMF_VARS.fd \
    -qmp tcp:127.0.0.1:2223,server,nowait
# qemu-system-x86_64 -m 1G -smp 1 -machine q35 -device virtio-net-pci,netdev=net0 -netdev user,id=net0,hostfwd=tcp::5555-:5555,
# hostfwd=udp::5555-:5555 -nographic -device vhost-vsock-pci,id=virtiosocket0,guest-cid=103 -cpu host -accel kvm -hda myboot.img 
# -object sev-guest,id=sev0,policy=0x1,cbitpos=47,reduced-phys-bits=5 -machine memory-encryption=sev0 -drive if=pflash,format=raw,unit=0,
# file=OVMF_CODE.fd,readonly=on -drive if=pflash,format=raw,unit=1,file=OVMF_VARS.fd -qmp tcp:127.0.0.1:2222,server,nowait -vnc 0.0.0.0:5