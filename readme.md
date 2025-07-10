# iroh ping

A very simple iroh protocol for pinging a remote node. It's a high level example & easy starting point for new projects.

Try it with

```
$ cargo run server
Connect to this server with:
cargo run client --ticket=node...
```

and from another terminal (even across computers!)

```
$ cargo run client --ticket=node...
```

## This is not the "real" ping

Iroh has all sorts of internal ping-type messages, this is a high level demo of a protocol, and in no way necessary for iroh's normal operation.
