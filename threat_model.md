# Mizu threat model

The threat model is heavily modeled on Pond's:
[Pond - Threat Model](https://web.archive.org/web/20150913065100/https://pond.imperialviolet.org/threat.html)

## A. assumption about the user and the user's computer running Mizu:

1. The user acts in good faith.
2. The user has an authentic copy of Mizu.
3. The computer is not compromised (i.e. private key material, plaintext
  message content, etc. is not readable by others)

## B. assumptions regarding cryptographic primitives used:

1. The security assumptions regarding X25519, AES-256 GCM, HMAC-SHA256 hold.

## C. what the Tezos node Mizu connects to can achieve:

1. The node can learn which identity the user has by observing writes.
2. The node can learn identities on the contact list of the user by observing
   reads.
3. The node can learn when a user is online by observing reads and writes.
4. The node can drop messages by refusing to propagate messages to the Tezos
   network.

## D. what other Tezos nodes can achieve:

1. The node can learn some information based on which peer is first to
   propagate the user's writes.

## E. what anybody can achieve:

1. Anyone can link identities together by observing discovery requests to users.
2. Anyone can learn when a user is online by observing when writes have occured.
3. Anyone can learn the number of messages, message sizes,
   and when key agreement is initiated by observing writes.
4. Anyone can learn which Tezos addresses are using Mizu.
5. Anyone can create an identity and spam discovery requests to a user.
6. Anyone can attempt to perform an attack on Tezos itself and compromise
   availability of Mizu.

## F. what a global, passive adversary can achieve:

1. A GPA can learn which IP addresses are using Mizu and link identities to
   IP addresses.

## G. what a local network attacker can achieve:

1. A local network attacker can observe when user is using Mizu.
2. A local network attacker can compromise availability of Mizu by blocking
   access to Tezos nodes.

## H. what a physical seizure of the user's computer can achieve:

1. An attacker can attempt to guess the user's passphrase and obtain retained
   messages.

## I. what a persistent compromise of the user's computer can achieve:

1. An attacker can gain access to all retained messages and future messages.

## J. what a temporary compromise of the user's computer can achieve:

1. An attacker can gain access to all retained messages, messages within the
   period of compromise, and future messages encrypted with the current
   Double Ratchet session.

## K. what a contact can achieve:

1. A contact can spam a user with messages.
2. A contact can retain messages indefinitely.
3. A contact can prove to a third party that a message came from a user.
<!--
TODO: notably, deniability does not exist since messages are signed with
private keys and are stored for eternity on the Tezos blockchain, thus are
trivially reproducible.
-->
