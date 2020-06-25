# Why not X?

## Why not [Signal](https://signal.org/)?

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

## Why not Pond?

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

