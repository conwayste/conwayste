# conwayste

Multiplayer Conway's Game of Life!

![build status](https://api.travis-ci.com/conwayste/conwayste.svg?branch=master)

![life in action](https://giant.gfycat.com/BlaringTidyDutchsmoushond.gif)

## Installation
Clone this repository:

```
$ git clone https://github.com/conwayste/conwayste
```

The GUI client (`cargo run --bin client`) depends on `SDL2`, `SDL2_Mixer` and `SDL2_Image`. We do plan on bundling these libraries with the binary at some point in the future, but for now you will need to manually install them. The versions (at least) needed are:

* `SDL2 v2.0.5`
* `SDL2_Mixer v2.0.1`
* `SDL2_Image v2.0.1`

Please follow the instructions listed on the [rust-sdl2](https://github.com/Rust-SDL2/rust-sdl2) bindings page for your specific platform.

Make sure your Rust is up to date! The easiest way is through `rustup update`.

### Windows
_Note: This has been validated as working on Windows 10; tread carefully elsewhere_:smile:_._

Grab the development libraries for SDL2, SDL2 Mixer, and SDL2 Image and place each of them in your toolchain's library folder. An example of this may be `~\.multirust\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\lib`.
Also place the `SDL2.dll` within the crate root folder.

You will not need to do this step once there is proper binary release of Conwayste (TBD).

### Mac/Linux
_Note: I used homebrew on Mac to accomplish these steps._

```
brew install sdl2
brew install sdl2_image 
brew install sdl2_mixer --with-libvorbis
```

On Fedora you can use `dnf`; this will install pretty much everything you will need:

```
sudo dnf install SDL2*
```

**Note:** Debian stable only supports SDL2 v.2.0.4 so you will need to compile SDL2 from source.
If you are compiling from source make sure you specify vorbis support.

```
...
./configure --with-vorbis
```

**(If necessary)** Add the libraries to your path. This step is necessary if cargo fails to link against the SDL2 libraries.
 
Under Linux, I had to export `$LD_LIBRARY_PATH`, but in Mac it was `$LIBRARY_PATH`.
Homebrew will install the libraries to the Cellar. 
```
export LIBRARY_PATH="/usr/local/Cellar/sdl2_mixer/2.0.1/lib/:/usr/local/Cellar/sdl2_image/2.0.1_2/lib/:/usr/local/Cellar/sdl2/2.0.5/lib/"
```
I ended up adding these to my `~/.profile` .

## Building

```
cargo build
```

## Running the server

```
cargo run --bin server
```

## Running the GUI client

```
cargo run --bin client
```
Note: at the time of this writing, it does not have network support.

## Running the CLI client

```
cargo run --bin cli-client
```
Note: at the time of this writing, it only has partial network support.

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
