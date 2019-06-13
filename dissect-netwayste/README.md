```
cargo build
#ln -s /path/to/conwayste/target/debug/libdissect_netwayste.so ~/.local/lib/wireshark/plugins/3.0/conwayste.so
ln -s /path/to/conwayste/target/debug/libdissect_netwayste.so /path/to/wireshark/run/plugins/3.1/epan/conwayste.so
/path/to/wireshark/run/wireshark
```
