# ConWaysteTheEnemy

Multiplayer Conway's Game of Life!

## Building

```
sudo pkg_manager install gtk3-dev
cargo build
```

## Running the server

```
cargo run --bin server 0.0.0.0:9000
```

## Running the client

The client talks to the server using UDP, so ensure that your firewall allows it.

```
cargo run --bin client 127.0.0.1:9000
```

## Hacking

### Updating libconway to the latest version in github

```
cargo update -p libconway
```
