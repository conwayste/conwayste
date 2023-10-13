# conwayste

Multiplayer Conway's Game of Life!

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0) ![build status](https://api.travis-ci.com/conwayste/conwayste.svg?branch=master) [![Discord](https://img.shields.io/discord/463752820026376202.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/mjSsUMw)

Life In Action | Patterns!
:-: | :-:
<video src='https://user-images.githubusercontent.com/1715672/142133919-a080f383-8403-4162-9ea6-a8e6d9360148.mp4' width=180/> | <video src='https://user-images.githubusercontent.com/1715672/142133963-320767e9-32f7-4cc8-96b0-e3f8cf164f41.mp4' width=180/>

## How to Play

Click on the desired menu option after the game boots. `Start Game` is a good place to... start ;).

Once in game:

* Left click toggles a cell (by default).
* The number keys control what left click does (whether it toggles a cell or drops a pattern).
* If dropping a pattern, you can use `Shift-left` and `Shift-right` to rotate the pattern.
* `Enter` to toggle chatbox focus.
* `+` and `-` to zoom in and out
* Press `r` to toggle running/paused (*Will not work in multiplayer mode*).
* `Space` to single step (*Will not work in multiplayer mode*).
* `Esc` to go back to the menu.

# Setup
Conwayste has been developed with cross-platform support in mind since day one using the Rust programming language! Your dependencies will likely vary based on your choice of operating system.

The easiest way to get the Rust compiler and toolchain is using [Rustup](https://rustup.rs/).

This has been validated as runnable on:
  * Ubuntu Linux (18.04 and 20.04)
  * Fedora Linux 32
  * Windows 10
  * OpenBSD
  * macOS Catalina (10.15.7)

There be dragons for any other operating system not listed above. Please tread carefully :smile:.

## Windows / MacOS

The Conwayste client and server compile and run right out of the box. Skip directly to [Installation](#installation).

## Linux

### Debian/Ubuntu

```
sudo apt install build-essential pkg-config libasound2-dev libudev-dev
```

### Fedora (possibly incomplete/outdated)

```
sudo dnf install alsa-lib-devel
```

### Other

Please refer to your package manager for dependencies.

## OpenBSD

```
doas pkg_add llvm
```

You will also need this environment variable. Add to your profile if desired:
```
export LIBCLANG_PATH=/usr/local/lib
```

## Installation

Please clone this repository, and build the client and server using `cargo`. The build may take several minutes to complete, depending on your system specs.

```
$ git clone https://github.com/conwayste/conwayste
$ cd conwayste/
```

If you cloned this previously and want to update, note that you may need to run `git submodule init` then `git submodule update` after pulling.

# Playing the Game

## Running The Client
```
$ cargo run --bin client
```

## Running the Server
```
$ cargo run --bin server --name "Example Server" --public-address yourserver.example.com:2016
```

If `--public-address` is specified, the server automatically registers itself with the [Official Conwayste Registrar](https://github.com/conwayste/registrar). Leave this off if you are running a private server.

An alternate registrar can be specified with the `--registrar-url` option:

```
$ cargo run --bin server --name "Example Server" --public-address yourserver.example.com:2016 --registrar-url https://yourregistrar.example.com/addServer
```

Use this if we didn't pay our server bills and someone else has their own registrar running. :)

# FAQ

### Did you write your own game engine?

Nope! We are using the [`ggez`](https://github.com/ggez/ggez) engine and give many thanks to its developers and contributers. Head over to their [GitHub page](https://github.com/ggez/ggez) to learn more about it.

### When will this be ready?

The developers have busy lives and enjoy working on this in their spare time. If you are waiting for a release, then I encourage you to contribute :smile:. This could take the form of bug reports or design feedback as well as lines of code.

### My installation fails in Linux. What should I do?

It's likely that we have not kept the installation steps up-to-date. Please Check the Ubuntu section in `.travis.yml` for a guaranteed up-to-date list of packages if your installation fails. :)

### I found a bug! What should I do?

It would help the developers a lot if you could submit an issue in GitHub describing the bug.

## Contributors

* aaronm04
* manghi

_Your name could be here! Pull requests are welcome!_
