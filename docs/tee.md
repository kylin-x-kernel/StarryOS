# Tee build guide

## Qemu Compilation
Use the following command to compile the project with the tee feature enabled.
```bash
make APP_FEATURES="tee, qemu" run
```

To run tee unit tests:
```bash
make APP_FEATURES="tee_test, qemu" run
```