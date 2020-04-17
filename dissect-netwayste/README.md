# Prerequisites
In order to compile the dissector to be used with Wireshark, you'll need to install glib-2.0 and bindgen.

## glib-2.0
`glib-2.0` is required so please refer to your distribution's package manager for installation instructions.

## Bindgen / Clang
Bindgen has its own installation requirements, so see the [rust-bindgen documentation](https://rust-lang.github.io/rust-bindgen/requirements.html) for more info.

## Wireshark Headers

This dissector has been verified to work with Wireshark 3.1 and 3.2.

It will not work with Wireshark 2 or older.

Bindgen requires Wireshark development headers in order to generate calls used by the `dissect-netwayste` library. You can either:
1) Use your package manager to install Wireshark, as well as the development libraries (such as `libwireshark-dev`)
2) Build wireshark from source

*Note: Ubuntu 18.04 only has Wireshark 2 in the repo so I had to follow the instructions here to generate and install the debian package from source.*
Instructions: https://www.consentfactory.com/install-wireshark-3-0-2-on-ubuntu-desktop-18-04-redux/

# Install Wireshark

1. Install Wireshark 3.1 or 3.2.

If you've installed wireshark from your package manager (or built the package from source), you should see `config.h` within `/usr/include/wireshark/`. If you don't see this, then something went wrong. You won't see this file if you've only compiled Wireshark from source as that won't install the development headers to the necessary paths in `/usr/`.

2. Make sure you enable `dumpcap` to be runnable as non-root.

For Ubuntu 18.04, I had to execute the following after installing wireshark.

```
$ whereis dumpcap
$ sudo setcap cap_net_raw,cap_net_admin+eip /path/to/dumpcap
```

2. Create the wireshark personal plugins directory if it is not already present

`mkdir -p $HOME/.local/lib/wireshark/plugins/3.2/epan/`

# Building dissect-netwayste
You must ensure the version of wireshark specified towards the top of the file in `lib.rs` matches the version you installed. If not, update it. This has been tested to work with wireshark version 3.1 and 3.2.
```Rust
/// Wireshark major & minor version
#[no_mangle]
pub static plugin_want_major: c_int = 3;
#[no_mangle]
pub static plugin_want_minor: c_int = 2;
```

## Compiled Wireshark from Source
If you compiled Wireshark from source and are not opting to go through your package manager, then the undesirably hack-ish way is to update the clang argument within `build.rs` with the correct include path(s) if you're building from source on a non-debian-based system. For example:
```
.clang_arg("-I/local/path/to/compiled/wireshark/wireshark-3.2.1")
```

Now simply call `cargo build`. If you get any issues related to missing header files, then wireshark was not installed or pathed-to via `build.rs` correctly.
```
# Build the library
cargo build
```

# Installing dissect-netwayste
Finally, copy the built library to the local wireshark plugins directory. The following command assumes  wireshark 3.2 was installed.
```
cp ../target/debug/libdissect_netwayste.so $HOME/.local/lib/wireshark/plugins/3.2/epan/conwayste.so
```

If everything was successful, you will see a `conwayste.so` entry in Wireshark's `Help > About Wireshark > Plugins`.
