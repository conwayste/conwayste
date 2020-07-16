# conwayste

Multiplayer Conway's Game of Life!

![build status](https://api.travis-ci.com/conwayste/conwayste.svg?branch=master)

![life in action](https://s7.gifyu.com/images/BlaringTidyDutchsmoushond-mobile.gif)

## How to Play

Use the arrow keys to navigate the menu (*Buttons are coming soon!*).

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

Now, you can run the game like this:

```
$ cargo run --bin client
```

The GUI client (`cargo run --bin client`) depends on ALSA. We do plan on bundling these libraries with the binary at some point in the future, but for now you will need to manually install them.

Please follow the instructions listed on the [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2) bindings page for your specific platform.

### Windows
_Note: This has been validated as working on Windows 10; tread carefully elsewhere_:smile:_._

Grab the development libraries for SDL2, SDL2 Mixer, and SDL2 Image and place each of them in your toolchain's library folder. An example of this may be `~\.multirust\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\lib`.
Also place the `SDL2.dll` within the crate root folder.

You will not need to do this step once there is proper binary release of Conwayste (TBD).

### macOS

**TODO: revise this section for ggez 0.5**
_Note: I used [homebrew](https://brew.sh/) to accomplish these steps._
```
brew install sdl2
brew install sdl2_image 
brew install sdl2_mixer --with-libvorbis

```

### Linux

On Ubuntu, you can install with `apt`:

```
sudo apt install libudev-dev libasound2-dev
```

_Note: if installation doesn't work on Ubuntu we may not have kept this up to date. Check the Ubuntu section in `.travis.yml` for a guaranteed up-to-date list of packages :)_

On Fedora you can use `dnf`; this will install pretty much everything you will need:

```
sudo dnf install alsa-lib-devel
```

## Running the server

```
cargo run --bin server
```

## FAQ

### When will this be ready?

The developers have busy lives and enjoy working on this in their spare time. If you are waiting for a release, then you should find something else to do. We always accept donations in liquid form, such as cup of coffee.

### ResourceNotFound Error
```
λ cargo run --bin client                                                                                                
    Finished dev [unoptimized + debuginfo] target(s) in 0.2 secs                                                        
     Running `target\debug\client.exe`                                                                                  
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ResourceNotFound("conwayste.ico")', src\libcore\
result.rs:906:4                                                                                                         
note: Run with `RUST_BACKTRACE=1` for a backtrace.                                                                      
error: process didn't exit successfully: `target\debug\client.exe` (exit code: 101)                                     
```
You need to link your resources folder. Please see the Build section above.

## Contributors

* manghi

* aaronm04

_Your name could be here! Pull requests are welcome!_
