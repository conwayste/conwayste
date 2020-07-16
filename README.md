# conwayste

Multiplayer Conway's Game of Life!

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0) ![build status](https://api.travis-ci.com/conwayste/conwayste.svg?branch=master) [![Discord](https://img.shields.io/discord/463752820026376202.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/5Z4E3w)
![life in action](https://s7.gifyu.com/images/BlaringTidyDutchsmoushond-mobile.gif)

## How to Play

Click on the desired menu option after the game boots. `Start Game` is a good place to... start ;).

When in the game:

* Left click toggles a cell (by default).
* The number keys control what left click does (whether it toggles a cell or drops a pattern).
* If dropping a pattern, you can use `Shift-left` and `Shift-right` to rotate the pattern.
* `Enter` to toggle chatbox focus.
* `+` and `-` to zoom in and out
* Press `r` to toggle running/paused (*Will not work in multiplayer mode*).
* `Space` to single step (*Will not work in multiplayer mode*).
* `Esc` to go back to the menu.

## Installation
First, install the `cargo` command if you have not already done so. The recommended way is [Rustup](https://rustup.rs/).

Next, clone this repository, and build the game:

```
$ git clone https://github.com/conwayste/conwayste
$ cargo build --bin client
```

To run the game as a client:

```
$ cargo run --bin client
```

The GUI client (`cargo run --bin client`) depends on ALSA. We do plan on bundling these libraries with the binary at some point in the future, but for now you will need to manually install them.

### Windows
_Note: This has been validated as working on Windows 10; tread carefully elsewhere_ :smile:_._

TODO (will use Win10)

### macOS

TODO

### Linux

On Ubuntu, you can install with `apt`:

```
sudo apt install libudev-dev libasound2-dev
```

_Note: if installation doesn't work on Ubuntu we may not have kept this up to date. Check the Ubuntu section in `.travis.yml` for a guaranteed up-to-date list of packages_ :)

On Fedora you can use `dnf`; this will install pretty much everything you will need:

```
sudo dnf install alsa-lib-devel
```

## Running the server

```
cargo run --bin server
```

## FAQ

### Did you write your own game engine?

We are using the [`ggez`](https://github.com/ggez/ggez) engine and give many thanks to its developers and contributers. Head over to their [GitHub page](https://github.com/ggez/ggez) to learn more about it.

### When will this be ready?

The developers have busy lives and enjoy working on this in their spare time. If you are waiting for a release, then you I encourage you to contribute :smile:. If you feel like donating, we always accept donations in liquid form, such as cup of coffee.


## Contributors

* manghi

* aaronm04

_Your name could be here! Pull requests are welcome!_
