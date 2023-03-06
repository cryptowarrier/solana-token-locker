# 🔐 token-locker on solana


<p align="center">
    spl-token locker programs on Solana.
</p>

This project demonstrates how to write a program that allows you to lock arbitrary SPL tokens and release the locked tokens with a determined unlock schedule.

The project comprises of:

* An on-chain lockup tokens program
* A set of test cases for interacting with the on-chain program

## Prerequisites

* [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
* [NodeJS & NPM](https://nodejs.org/en/) version 14+
* [Rust](https://rustup.rs/) - install [from here](https://www.rust-lang.org/tools/install#), it's pretty straightforward.
* [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) - follow this to install.
* [Anchor](https://project-serum.github.io/anchor/) - follow the [easy installation steps](https://project-serum.github.io/anchor/getting-started/installation.html).

## Install Rust && Solana Cli && Anchor

* See [this doc](https://github.com/solana-labs/solana#) for more details

### Install rustup

```sh
$ curl https://sh.rustup.rs -sSf | sh
...

$ rustup component add rustfmt
...

$ rustup update
...

$ rustup install 1.61.0
...
```

### Install solana-cli

* See [this doc](https://docs.solana.com/cli/install-solana-cli-tools) for more details

```sh
$ sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
...

$ solana -V
solana-cli 1.8.17 (src:f63505df; feat:3263758455)

$ solana-keygen new
...
```



### Install `rust-analyzer` (Optional)

* See [this repo](https://github.com/rust-analyzer/rust-analyzer) for more details

`rust-analyzer` can be very handy if you are using Visual Studio Code. For example, the analyzer can help download the missing dependencies for you automatically.

### Install avm & anchor

```sh
$ cargo install --git https://github.com/project-serum/anchor avm --locked --force
...
```

Use latest `anchor` version:

```sh
avm use latest
```

### Extra Dependencies on Linux(Optional)

You may have to install some extra dependencies on Linux(eg. Ubuntu):

```sh
$ sudo apt-get update && sudo apt-get upgrade && sudo apt-get install -y pkg-config build-essential openssl libssl-dev libudev-dev
...

```

### Verify the Installation

Check if Anchor is successfully installed.

```sh
$ anchor --version
anchor-cli 0.26.0
```

## Build and Deployment

```sh

$ anchor build

Check program Id (address)

$solana address -k target/deploy/token_locker-keypair.json

AEH6XDoe6hK6XNVJpuVos19xUaE1yXNdjcVK8RRifpUB

Replace programId

open programs/token-locker/src/lib.rs and replace with new key

declare_id!("AEH6XDoe6hK6XNVJpuVos19xUaE1yXNdjcVK8RRifpUB");

open Anchor.toml and replace

[features]
seeds = false
skip-lint = false
[programs.localnet]
token_locker = "AEH6XDoe6hK6XNVJpuVos19xUaE1yXNdjcVK8RRifpUB"
+ [programs.mainnet]
+ token_locker = "AEH6XDoe6hK6XNVJpuVos19xUaE1yXNdjcVK8RRifpUB"

...

- cluster = "Localnet" 
+ cluster = "mainnet"

wallet = "/home/peace/.config/solana/id.json"


Check you fund

$solana balance

0 SOL

Check your paper wallet address

$solana address

5Dtys26BJhQ28z4WiWnW7cXbfkqcCcNc1o2dgJ5MJcX9

Fund your address

$anchor deploy


```