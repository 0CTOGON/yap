# Protocol

**Not Yet Another Protocol**

This document describes the YAP wire protocol.

YAP is designed as a peer-to-peer messaging protocol.

## Status

The protocol is currently experimental.

The format described here is not yet stable.

Implementations should expect breaking changes.

---

## Transport

YAP currently uses **QUIC** as its transport protocol.

QUIC provides:

- Encrypted connections
- Reliable streams
- Multiplexing
- Connection migration
- UDP-based transport

YAP does not currently require a central server for communication between directly reachable peers.

---

## Connection

A YAP peer listens for incoming QUIC connections.

After establishing a connection, peers exchange identity information.

The first protocol message is currently a `HELLO` message.

Conceptually:

    HELLO
    username
    peer identity
    protocol version

The exact binary representation is not yet stable.

---

## Messages

The protocol currently supports the following conceptual message types.

### HELLO

Used to identify a peer after connecting.

    HELLO
    username

### CHAT

Broadcast a message to connected peers.

    CHAT
    sender
    message

### DIRECT

Send a message to a specific peer.

    DIRECT
    sender
    recipient
    message

---

## Identity

YAP currently identifies peers using usernames.

This is temporary.

Future versions are expected to use cryptographic peer identities.

A future identity system may allow peers to prove that they control a particular identity without relying on a central authority.

---

## Protocol Versioning

The protocol is expected to include an explicit protocol version.

Breaking changes should increment the protocol version.

Implementations should reject or gracefully handle versions they do not understand.

The exact versioning scheme has not yet been finalised.

---

## Wire Format

The current prototype uses a simple serialization format.

YAP is intended to use a binary wire format rather than JSON for the stable protocol.

The binary format has not yet been finalised.

---

## Security

YAP uses QUIC, which provides encrypted transport.

Transport encryption alone does not necessarily provide all properties required for secure peer-to-peer messaging.

Future versions may add:

- Cryptographic peer identities
- Peer authentication
- End-to-end message encryption
- Message signatures
- Replay protection
- Message IDs

---

## Compatibility

YAP is currently experimental.

Protocol compatibility between releases is not guaranteed.

Once the protocol becomes stable, compatibility requirements will be documented here.