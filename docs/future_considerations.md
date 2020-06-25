# Future considerations

## Considerations

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
  - This turned out to be a much more nuanced issue than originally imagined,
    and needs further investigation w/ possible protocol changes.
- Discovery requests are currently very naive, should possibly be reworked to
  ease key agreement.
- Does Mizu really minimize traffic leakage, and if so, to what extent?
  - Operations on smart contracts require authentication of address, this
    will most likely leak information (ex. Alice posting a Discovery message to
    Bob show Alice paid for it, linking Bob and Alice together)

## TODOs

- [ ] Go through formal security analyses of X3DH and Double Ratchet
  - [ ] [A Formal Security Analysis of the Signal Messaging Protocol](https://eprint.iacr.org/2016/1013.pdf)
  - [ ] [Support conversion between X25519 and Ed25519 keypairs](https://github.com/briansmith/ring/issues/760)
  - [ ] [Security Analysis of the Signal Protocol](https://dspace.cvut.cz/bitstream/handle/10467/76230/F8-DP-2018-Rubin-Jan-thesis.pdf)
  - [ ] [Olm: A Cryptographic Ratchet](https://gitlab.matrix.org/matrix-org/olm/-/blob/master/docs/olm.md)
- [ ] document the differences between X3DH + Double Ratchet
- [ ] add a way to remove pokes
- [ ] possibly use constant-time primitives available here: https://github.com/dalek-cryptography/subtle
- [ ] various UI/UX improvements
- [ ] reduce operation latency by performing some node operations locally
- [ ] build a full-fledged Tezos RPC client based on mizu-tezos-rpc
- [ ] improve API exposed by mizu-crypto
- [ ] handle large number of TODOs across the codebase
- [ ] work on anonymity and reducing information leakage

