# Version TBD (2023-06-01)

- Removed submodules for ggez, rodio, cpal, and gilrs crates.For the changes in
  this commit to take effect, run `git submodule deinit --all`. To undo (for
example, if switching to another branch that doesn't have this commit), run `git
submodule update --init --recursive` (I haven't tested this yet :) ).

# Version 0.3.5 (2020-06-30)

- New widget event system (`UIContext`).
- Wireshark dissector plugin for Conwayste network protocol (`dissect-netwayste` crate).
- Breaking netwayste protocol changes.
- Server creates general room by default.
- Can query server status and both client and server can get the ping/latency.

# Version 0.3.4-alpha2 (2020-05-21)

...
