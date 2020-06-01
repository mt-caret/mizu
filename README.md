# Mizu

> 上善若水

![Continuous integration](https://github.com/mt-caret/mizu/workflows/Continuous%20integration/badge.svg)

Mizu is an experiment to explore what secure, private, and asynchronous
messaging might look on the blockchain.
Mizu is similar to email combined with PGP, while being forward secure and
tries to minimize information leakage, similar to
[Pond](https://web.archive.org/web/20151101081526/https://pond.imperialviolet.org/).

## how does Mizu work?

Mizu initiates sessions between users with a protocol based on
[Signal's X3DH Key Agreement Protocol](https://signal.org/docs/specifications/x3dh/)
with the One-time Prekey omitted. Each message is encrypted with
[Double Ratchet](https://signal.org/docs/specifications/doubleratchet/) to
provide forward secrecy.

### postal boxes and discovery requests

Each user has associated with it a **postal box** (which is public) and a list of
contacts (which is private). When sending a message, the user will encrypt
the message with recipient's identity and add it their own postal box.
Periodically, Mizu will check the postal boxes of each identity in its
contact list and attempt to decrypt all new messages found.
Those it successfully decrypts will have been addressed towards the user.

TODO: define "periodically"

Say Alice wants to communicate with Bob. If Bob is aware of this,
both Alice and Bob can manually add each other's identities to their contact
lists. However, if Bob is not aware of Alice, communication can be initiated
from Alice by sending a **discovery request** message solely consisting of her
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

## why not [Signal](https://signal.org/)?

Signal is a messaging application for smartphones which provides
various desireable properties (end-to-end encryption, forward and future
secrecy, deniability) with respect to security.

While I agree Signal should be your first choice when looking for a secure
messenger, it has different design goals from Mizu.

- Signal requires a smartphone and a phone number

Signal identifies users by their phone number, making it almost impossible to
communicate without revealing your identity and/or your location.
This is partly by design, as minimizing information leakage is probably not
one of the goals of Signal.

- [Signal is centralized and the server is closed source](https://signal.org/blog/the-ecosystem-is-moving/)

While I think the reasons behind this decision are valid, I believe there are
very legitimate use cases in which this makes using Signal unviable.

## why not Pond?

Pond is an asynchronous messaging system that aims to prevent information
(content, message size, metadata, etc.) leaking to observers and attackers
while providing similar security properties to Signal.

- pond is deprecated in favor of Signal

The last commit in the [official repository](https://github.com/agl/pond) dates
back to 2016, and the author recommends using Signal instead.

- identity and the spam issue

Pond takes a fairly radical position on identity management.
Quoting from the [website](https://web.archive.org/web/20150917091955/https://pond.imperialviolet.org/tech.html):

> ... only authorised users can cause a message to be queued for delivery. This
> very clearly sets Pond apart from email. There are no public addresses to
> which a Pond message can be sent. Likewise, it's no longer true that the
> network is fully connected; if you send a message to two people, they may not
> be able to reply to each other.

The document explains that this design decision was motivated in part to
prevent spam. Mizu, on the other hand, sidesteps identity management by using
Tezos addresses as identities, and the Proof-of-Work protocol involved in
creating new Tezos addresses combined with the monetary cost of sending
messages heavily discourages spam.

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
- Does Mizu really minimize traffic leakage, and if so, to what extent?
  - Operations on smart contracts require authentication of address, this
    will most likely leak information (ex. Alice posting a Discovery message to
    Bob show Alice paid for it, linking Bob and Alice together)

## TODOs

- [ ] *really* understand X3DH and Double Ratchet
  - [ ] [A Formal Security Analysis of the Signal Messaging Protocol](https://eprint.iacr.org/2016/1013.pdf)
  - [ ] [Support conversion between X25519 and Ed25519 keypairs](https://github.com/briansmith/ring/issues/760)
  - [ ] [Security Analysis of the Signal Protocol](https://dspace.cvut.cz/bitstream/handle/10467/76230/F8-DP-2018-Rubin-Jan-thesis.pdf)
  - [ ] [Olm: A Cryptographic Ratchet](https://gitlab.matrix.org/matrix-org/olm/-/blob/master/docs/olm.md)
- [ ] document the differences between X3DH + Double Ratchet
- [ ] add a way to remove pokes
- [ ] possibly use constant-time primitives available here: https://github.com/dalek-cryptography/subtle
