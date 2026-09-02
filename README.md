# YAP

**Yet Another Protocol**

A small peer-to-peer messaging project written in Rust.

YAP is an experiment in building a simple messaging protocol and network stack from scratch, using QUIC for transport.

> **YAPPPP.**
> I shouldn't do that.
> ANYWAY.

## What is YAP?

YAP lets peers connect directly to each other and exchange messages.

The basic idea is:

```text
Alice ───────── Internet ───────── Bob
          direct connection
```

Rather than:

```text
Alice ──> Central server ──> Bob
```

The project is still under active development, so expect things to break. Spectacularly.

## Workspace

YAP is split into several crates:

```text
yap/
├── yap-protocol/    Protocol and packet definitions
├── yap-core/        Networking and peer management
└── yap-cli/         Command-line client
```

### `yap-protocol`

Defines the data that YAP peers exchange.

This is kept separate from the networking code so that other YAP clients can implement the protocol without needing to use the CLI.

### `yap-core`

Handles the actual networking.

Currently this is built around:

* Rust
* QUIC
* Quinn
* Tokio
* Serde

### `yap-cli`

The terminal client.

Example:

```text
yap> connect 127.0.0.1:7332
Connecting to 127.0.0.1:7332...
Connected.

yap> yap hello!
```

## Current status

YAP is **early-stage**.

Currently working on:

* [x] Rust workspace
* [x] Separate protocol crate
* [x] Core networking crate
* [x] CLI client
* [x] QUIC connections
* [x] Peer usernames
* [x] Direct messaging
* [x] Chat messages
* [x] JSON packet serialization
* [x] Multiple peers
* [x] Git repository
* [ ] Internet-wide peer connections
* [ ] NAT traversal
* [ ] Peer discovery
* [ ] Protocol versioning
* [ ] Encryption/identity design
* [ ] GUI clients

The long-term goal is for two YAP clients on different networks to communicate directly over the Internet.

## Building

You need a recent Rust toolchain.

Clone the repository and run:

```powershell
cargo check
```

To run the CLI:

```powershell
cargo run -p yap-cli
```

## Development

YAP is primarily a learning project.

The goal isn't to reinvent every piece of networking infrastructure on Earth. It's to understand how the pieces fit together by actually building them.

If something is broken:

**that's probably part of the development process.**

## Contributing

Contributions, experiments, alternative clients, protocol ideas, and particularly GUI clients are welcome.

A GUI client does **not** need to use the YAP CLI. As long as it speaks the YAP protocol, it can be its own thing.

## License

See LICENSE for details.

---

**YAP.**

Yet Another Protocol.

Yet another packet.

Yet another compiler error.

Yet another `cargo check`.

🦀
