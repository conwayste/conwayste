# conwayste

Multiplayer Conway's Game of Life!

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0) ![build status](https://api.travis-ci.com/conwayste/conwayste.svg?branch=master) [![Discord](https://img.shields.io/discord/463752820026376202.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/mjSsUMw)
![life in action](https://s7.gifyu.com/images/BlaringTidyDutchsmoushond-mobile.gif)
![Patterns!](https://s8.gifyu.com/images/conwayste.gif)

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

## Windows

The Conwayste client and server work right out of the box, just skip to [Playing the Game](#playing-the-game) follow the installation instructions below.

## macOS

TODO

## Linux

The GUI client depends on the ALSA audio framework. We do plan on bundling these libraries with the binary at some point in the future, but for now you will need to manually install them (see dependency instructions above).

On Ubuntu, you can install with `apt`:

```
sudo apt install libudev-dev libasound2-dev
```

_Note: if installation doesn't work on Ubuntu we may not have kept this up to date. Check the Ubuntu section in `.travis.yml` for a guaranteed up-to-date list of packages_ :)

On Fedora you can use `dnf`; this will install pretty much everything you will need:

```
sudo dnf install alsa-lib-devel
```

# Playing the Game
Now that your dependencies and rust compiler are setup, clone this repository, and build the client and server using `cargo`:
```
$ git clone https://github.com/conwayste/conwayste
$ cd conwayste/
$ cargo build --bin client
$ cargo build --bin server
```

## Running The Client
```
$ cargo run --bin client
```

_Note: This has been validated as working on Ubuntu Linux, Fedora Linux, Windows 10, OpenBSD, and MacOS; tread carefully elsewhere_ :smile:_._


## Running the Server
```
$ cargo run --bin server
```

# FAQ

### Did you write your own game engine?

Nope! We are using the [`ggez`](https://github.com/ggez/ggez) engine and give many thanks to its developers and contributers. Head over to their [GitHub page](https://github.com/ggez/ggez) to learn more about it.

### When will this be ready?

The developers have busy lives and enjoy working on this in their spare time. If you are waiting for a release, then you I encourage you to contribute :smile:.


## Contributors

* manghi

* aaronm04

_Your name could be here! Pull requests are welcome!_
