# Tee build guide

## Current Status

### Syscalls

 - [x] tee_scn_log
 - [ ] tee_scn_get_time


### SDK
 - [x] Already support kylin teaclave-trustzone-sdk and build  hello-world TA with it.
 - [ ] TA sessions management

## Build Starry

### Qemu
We have already default enabled tee feature in our qemu build configuration.
```bash
make run LOG=info
```


## Build TA

### kylin teaclave-trustzone-sdk repo
we need to clone kylin teaclave-trustzone-sdk repo first.

```
git clone -b kylin https://github.com/kylin-x-kernel/teaclave-trustzone-sdk.git && \
  cd teaclave-trustzone-sdk
```


### Build Docker image

We need to build a docker image with teaclave-trustzone-sdk.
TODO: Future we can push the image to dockerhub for easy use.

```bash
./scripts/release/build_dev_docker.sh
```

### Build TA inside Docker

We can build TA inside the docker image we just built.

```bash
docker run -it --rm   --name teaclave_dev_env   -v $(pwd):/root/teaclave_sdk_src   -w /root/teaclave_sdk_src   kylin-starry/teaclave-trustzone-emulator-nostd-expand-memory:latest
```

Then we can build TA with the following command inside the docker container.

```bash
make -C examples/hello_world-rs/
```


## TA

### Install TA to Starry
now we can install the built TA to starry.here we assume you have mounted starry's rootfs to /mnt.

```bash
sudo cp ~/third_code/teaclave-trustzone-sdk/examples/hello_world-rs/ta/target/aarch64-unknown-linux-musl/release/ta /mnt/
```

### Run TA in Starry

now we can run starry with the installed TA.

```bash
make run LOG=info

starry:/# ./ta
[  3.095773 3:15 starry_api::syscall::task::wait:64] sys_waitpid <= pid: -1, options: WaitOptions(WUNTRACED)
[  3.100905 2:16 starry_api::task:42] Enter user space: ip=0x4048300, sp=0x7ffefffff6e0
[  3.109598 2:16 starry_api::tee::log:14] TEE log syscall invoked with len: 14
[  3.109976 2:16 starry_api::tee::log:16] TEE Log: [+] TA create
TA
[  3.110846 2:16 starry_api::task:166] Task(16, "ta") exit with code: 0
[  3.111155 2:16 starry_core::task:500] Send signal SIGCHLD to process 15
[  3.121573 3:15 starry_api::syscall::task::wait:64] sys_waitpid <= pid: -1, options: WaitOptions(WUNTRACED)
[  3.122846 3:15 starry_api::syscall::task::wait:64] sys_waitpid <= pid: -1, options: WaitOptions(WNOHANG | WUNTRACED)
```
