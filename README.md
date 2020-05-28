# Mizu

> 上善若水

Mizu is an experiment to explore what secure, private, and asynchronous
messaging might look on the blockchain.
Mizu is similar to email combined with PGP, while being forward secure and
mimimizing information leakage, similar to
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
from Alice by sending a *discovery request* message solely consisting of her
identity encrypted with Bob's public key. Bob can then choose to add Alice to his
contact list and start communicating.

### transport

Identities are Tezos public keys / addresses, and both the postal box and
discovery requests are maintained by a Tezos smart contract, thus are
globally replicated across all Tezos nodes.

## potential attacks

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

## modes of operation

Mizu has two modes of operation with respect to where the Tezos node is located:

- self-contained mode: In this mode, the Tezos node is operated on the same
  machine as Mizu.
- delegated mode: In this node, Mizu connects to (possibly multiple) Tezos
  nodes over the network to interface with the Tezos blockchain.

The advantage of the self-contained mode is minimal metadata leakage.
If Mizu reads the user data of a some address, this will tell the
node the following information: either the Mizu instance is the address
(if Mizu is trying to check the list of discovery messages) or the address is
in the list of contacts (in which case Mizu is checking for new messages).
Note, however posting to the user's postal box will result in the
corresponding block being propagated among Tezos nodes originating with the
Tezos node used by Mizu.

TODO: This could possibly be alleviated to a significant extent, by having a
pool of nodes to post to over Tor. But then it's probably much easier to just
use delegated mode.

The drawback of this mode is the substantial storage and uptime requirements of
running a Tezos node. Users generally will not expect and should not have to
devote tens or possibly hundereds of gigabytes of storage and be constantly
online (or bear the prohibitive synchronization costs the first time around,
and subsequently for not being online) in order to run a messaging application.

Delegated mode does not have this issue, since reads and writes are done by
nodes hosted by others. This, however, leaks a substantial amount of metadata
leakage as outlined above. This can probably be improved to a significant
extent by having a pool of Tezos nodes which are randomly selected and
contacted over Tor. However, care must be taken so each read and write
are sufficiently distributed accross space (originate from differing Tor exit
nodes) and time to make correlation difficult.

## why not Pond?

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

- [ ] *really* understand X3DH and Double Ratchet
  - [ ] [A Formal Security Analysis of the Signal Messaging Protocol](https://eprint.iacr.org/2016/1013.pdf)
  - [ ] [Support conversion between X25519 and Ed25519 keypairs](https://github.com/briansmith/ring/issues/760)
  - [ ] [Security Analysis of the Signal Protocol](https://dspace.cvut.cz/bitstream/handle/10467/76230/F8-DP-2018-Rubin-Jan-thesis.pdf)
- [ ] document the differences between X3DH + Double Ratchet
- [ ] add a way to remove pokes
