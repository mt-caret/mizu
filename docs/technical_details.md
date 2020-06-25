# technical details

Mizu initiates sessions between users with a protocol based on
[Signal's X3DH Key Agreement Protocol](https://signal.org/docs/specifications/x3dh/)
with the One-time Prekey omitted. Each message is encrypted with
[Double Ratchet](https://signal.org/docs/specifications/doubleratchet/) to
provide forward secrecy.

## postal boxes and discovery requests

Each user has associated with it a **postal box** (which is public) and a list of
contacts (which is private). When sending a message, the user will encrypt
the message with recipient's identity and add it their own postal box.
Periodically, Mizu will check the postal boxes of each identity in its
contact list and attempt to decrypt all new messages found.
Those it successfully decrypts will have been addressed towards the user.

Say Alice wants to communicate with Bob. If Bob is aware of this,
both Alice and Bob can manually add each other's identities to their contact
lists. However, if Bob is not aware of Alice, communication can be initiated
from Alice by sending a **discovery request** message solely consisting of her
identity encrypted with Bob's public key. Bob can then choose to add Alice to his
contact list and start communicating.

## transport

Identities are Tezos public keys / addresses, and both the postal box and
discovery requests are maintained by a Tezos smart contract, thus are
globally replicated across all Tezos nodes.

## interfacing with Tezos

Mizu interfaces with the Tezos blockchain by connecting to a Tezos node over
the network. This, however, leaks a substantial amount of information about
the user as the node will be able to correlate the IP address with
identities as well as find out the identities in the contact list of the user.
See the [threat model](./threat_model.md) for details

This can probably be significantly alleviated by having a pool of Tezos nodes
which are randomly selected and contacted over Tor. However, care must be
taken so each read and write are sufficiently distributed accross space
([originate from differing Tor exit nodes](https://tails.boum.org/contribute/design/stream_isolation/))
and time to make correlation difficult.

## potential attacks

### spam

As messages sent from identities in the contact list are processed and shown,
it is impossible to "spam" a large number of users via messages. The most
that a malicious spammer can do is to send a large number of (possibly invalid)
discovery requests to users. However, since appending data to the storage of
a smart contract incurrs a per-byte cost with no discernable benefit to the
spammer, this sort of attack occurring on a large scale seems implausible.

### attacks against Tezos

If an adversary mounts a successfull (albeit extremely costly) attack against
the Tezos blockchain, it will be possible to remove blocks which will
constitute a Denial of Service attack on Mizu. However, messages in Mizu
will still be impossible to forge or replay, and stay confidential.

