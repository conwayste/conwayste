# ConWaysteTheEnemy

Multiplayer Conway's Game of Life!

## Installing Depenencies
Conwayste depends on `SDL2`, `SDL2_Mixer` and `SDL2_Image`. We do plan on bundling these libraries with the binary at some point in the future, but for now you'll need to manually install them. Current verions needed are:

* SDL2 v2.0.5
* SDL2\_Mixer v2.0.1
* SDL2\_Image v2.0.1

Make sure your Rust is up to date!

`rustup update`

### Windows
TBD...

### Mac/Linux
I used homebrew on Mac to accomplish these steps. 

```
brew install sdl2
brew install sdl2_image 
brew install sdl2_mixer --with-libvorbis
```

On Fedora you can use yum:

```
TBD
```

Debian stable only supports SDL2 v.2.0.4 so you'll need to compile SDL2 from source.
If you're compiling from source it will be very similar.

```
TBD
./configure --with-vorbis
```

And then add the libraries to your path. This step is necessary if cargo fails to link against theSDL2 libraries.
 
Under Linux, I had to export `LD_LIBRARY_PATH`, but in Mac it was simply `LD_LIBRARY_PATH`.
Homebrew will install the libraries to the Cellar. 
```
export LIBRARY_PATH="/usr/local/Cellar/sdl2_mixer/2.0.1/lib/:/usr/local/Cellar/sdl2_image/2.0.1_2/lib/:/usr/local/Cellar/sdl2/2.0.5/lib/"
```

## Building

**TODO: OpenGL development packages, etc.**

```
cargo build
ln -s ../resources target    # needed only for client
```

## Running the server

```
cargo run --bin server 0.0.0.0:9000
```

## Running the client

```
cargo run --bin client 127.0.0.1:9000
```

## Hacking

### Updating the life engine crate to the latest version in github

```
cargo update -p conway
```

## Contributors

* mang

* aaronm04

_Your name could be here! Pull requests are welcome!_
