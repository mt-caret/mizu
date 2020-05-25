# Mizu

Mizu is an experiment to try and provide secure and asynchronous messaging
similar to email combined with PGP, while being forward secure and trying to
mimimalize information leakage, similar to
[Pond](https://web.archive.org/web/20151101081526/https://pond.imperialviolet.org/).

## security, asynchronicity, forward security, and minimization of information leakage

- security: TODO
- asynchronicity: TODO
- forward security: TODO
- minimization of information leakage: TODO

TODO: does Mizu really minimize traffic leakage?
TODO: operations on smart contracts require authentication of address, this
      will most likely leak information (ex. Alice posting a Discovery message to
      Bob show Alice paid for it, linking Bob and Alice together)

## how does Mizu work?

Mizu initiates sessions between users with a protocol based on
[Signal's X3DH Key Agreement Protocol](https://signal.org/docs/specifications/x3dh/)
with the One-time Prekey omitted. Each message is encrypted with
[Double Ratchet](https://signal.org/docs/specifications/doubleratchet/) to
provide forward secrecy.

### postal boxes and discovery requests

Each user has associated with it a postal box (which is public) and a list of
contacts (which is private). When sending a message, the user will encrypt
the message with recipient's identity and add it their own postal box.
Periodically, Mizu will check the postal boxes of each identity in its
contact list and attempt to decrypt all new messages found.
Those it successfully decrypts will have been addressed towards the user.

TODO: define "periodically"

Say Alice wants to communicate with Bob. If Bob is aware of this,
both Alice and Bob can manually add each other's identities to their contact
lists. However, if Bob is not aware of Alice, communication can be initiated
from Alice by sending a *discovery request* message solely consisting of her identity,
encrypted with Bob's public key. Bob can then choose to add Alice to his
contact list and start communicating.

### transport

Identities are Tezos public keys / addresses, and both the postal box and
discovery requests are maintained by a Tezos smart contract, thus are
globally replicated across all Tezos nodes.

### spam

As messages sent from identities in the contact list are processed and shown,
it is impossible to "spam" a large number of users via messages. The most
that a malicious spammer can do is to send a large number of (possibly invalid)
discovery requests to users. However, since appending data to the storage of
a smart contract incurrs a per-byte cost with no discernable benefit to the
spammer, doing this at a large scale is impractical.

### attacks against Tezos

If an adversary mounts a successfull (albeit extremely costly) attack against
the Tezos blockchain, it will be possible to remove blocks which will
constitute a Denial of Service attack on Mizu. However, messages in Mizu
will still be impossible to forge or replay, and stay confidential.

## why not pond?

TODO

- pond is deprecated in favor of Signal

TODO

- identity and the spam issue

TODO

## why not [Signal](https://signal.org/)?

While I agree Signal should be your first choice when looking for a secure
messenger, it has different design goals from Mizu.

- Signal requires a smartphone and a phone number

Signal identifies users by their phone number, making it almost impossible to
communicate without revealing your identity and/or your location.

- [Signal is centralized and the server is closed source](https://signal.org/blog/the-ecosystem-is-moving/)

TODO

## issues to consider

- How should Mizu resolve the following problem outlined here:  
  > Alice and Bob might simultaneously initiate a new session with each other,
  > so that two new sessions are created. For the Double Ratchet to be
  > maximally effective Alice and Bob must send and receive messages using
  > matching sessions, so somehow they must agree on which matching sessions to
  > use.  
  [Signal >> Specifications >> The Sesame Algorithm: Session Management for Asynchronous Message Encryption](https://signal.org/docs/specifications/sesame/)
  - As messages should be totally ordered, if clients always use the newest
    ephemeral keys, keeping the two newest sets of keys should be enough?
- A blockchain combined with a smart contract offers significantly different
  security characteristics when compared to sending messages over an unreliable
  network. What are the security implication of this, and what tradeoffs are
  we making?
  - Messages cannot be lost (except for deletions by the owner or if the
    blockchain is discarded) or fetched out-of-order, so the message number
    in Double Ratchet is not needed.
- Discovery requests are currently very naive, should possibly be reworked to
  ease key agreement.

## TODOs

- [ ] add a way to remove pokes
