# Mizu

> 上善若水

![CI](https://github.com/mt-caret/mizu/workflows/CI/badge.svg)

Mizu is an experiment to explore what secure, private, and asynchronous
messaging backed by a decentralized store.
Mizu is similar to email combined with PGP, while being forward secret and
tries to reduce information leakage, similar to
[Pond](https://web.archive.org/web/20151101081526/https://pond.imperialviolet.org/).

**DISCLAIMER**: The design and implementation of Mizu is done by
non-cryptographers who have not had substantial experience designing or
implementing security-critical communication protocols. Please do not use
this for anything real.

- [threat model](./threat_model.md)
- [technical details](./technical_details.md)
- [build instructions](./build_instructions.md)

## why not [Signal](https://signal.org/)?

Signal is a messaging application for smartphones which provides
various desireable properties (end-to-end encryption, forward and future
secrecy, deniability) with respect to security.

While we agree Signal should be your first choice when looking for a secure
messenger, it has different design goals from Mizu.

- Signal requires a smartphone and a phone number

Signal identifies users by their phone number, making it almost impossible to
communicate without revealing your identity and/or your location.
This is partly by design, as minimizing information leakage is probably not
one of the goals of Signal.

- [Signal is centralized and the server is closed source](https://signal.org/blog/the-ecosystem-is-moving/)

While we think the reasons behind this decision are valid, we believe there are
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

## Acknowledgements

Thanks to Shiho Midorikawa from Cybozu Labs for helpful suggestions and discussion.
