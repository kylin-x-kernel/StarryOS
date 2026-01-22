# Developer Guide

## Introduction
Welcome to the Developer Guide! This document is designed to help developers 
understand the architecture, tools, and best practices for contributing to
our project.

## Mutil repo
Starry now uses a multi-repo architecture to separate different components
of the project. This allows for better modularity, easier maintenance, and
independent versioning of each component.

But it also brings some challenges, such as managing dependencies and ensuring
compatibility between different repositories. To address these challenges,
we have established some guidelines and tools to help developers navigate


### repo management
Multi-repository development: The entire repository is managed centrally by `repo`,the development branch is `kylin-starry-dev`.
```bash
repo init -u https://github.com/kylin-x-kernel/starryos-manifest -m base.xml
```
you will find the axcpu,arm-gic,etc.. in the `local_crates` folder and arceos .If you are developing code in repositories under the arces or local_crates folders, you first need to ensure the entire project compiles successfully. For arces, you can simply use bitbake after making changes. However, for repositories with fixed remote links in cargo.toml, you need to first push this repository to a specific remote branch and then manually modify the repository URL in cargo.toml.

### Bitbake related
[todo]
A predefined Docker image will be provided, which can be pulled to the local machine and run in the working directory. This will ensure consistency in the build environment for everyone, and the development paradigm will change to modifying code locally and then using Bitbake to compile Starry or QEMU and start it in a Docker container.More detailed syntax information can be found in your browser.
```bash
bitbake starry -c compile
bitbake starry -f
bitbake starry-minimal-image
bitbake starry-test-image
bitbake starry -e | grep 
```
### Change Code in local_crates(submodule)
When you need to change the code in the submodule, you can follow these steps:

1. Navigate to the submodule directory:
   ```bash
   cd local_crates/<submodule_name>
   ```
2. Create a new branch for your changes:
   ```bash
    git checkout -b <your_branch_name>
    ```
3. Make your changes and commit them:
    ```bash
    git add .
    git commit -m "Your commit message"
    ```

4. Push your changes to the remote repository:
    ```bash
    git push origin <your_branch_name>
    ```

5. Change `Cargo.toml` in the main repo to point to the git repository:
   ```toml
   [patch.crates-io]
   <submodule_name> = { git = "<repository_url>", branch = "<your_branch_name>" }
   ```

6. Test
   ```bash
   cargo update && bitbake starry
   ```

#### Merge
After test,you can on submodule repo to create PR,and No need to submit a PR on Starryos again.







