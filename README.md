# bittorrent

A ground up implementation of a bittorrent client using rust. This is still unfinished but you are welcome to follow along and critique :)

## Current status
1. currently this is able to parse bencoded metadata (torrent files) and responses from HTTP(S) trackers.
2. It is able to ping trackers and receive peer addresses
3. It is able to connect via TCP to peers
4. It is able to handshake with peers
5. Decodes messages from streams (Read implementers)
6. Handles messages reactively
7. Concurrently reads and writes from TCP sockets
8. Sparse bitfield operations, implemented as a vector of ranges
9. State passing between actor style threads (threadsafe)
10. Packet request, downloading and order.
11. Timeout, request strategy

## Outstanding issues
1) Endgame needs to be completed
2) Persistence (in memory and on fs)
3) Uploading to peers
4) Peer discovery (after initial tracker calls)

These will probably be deferred until RC because I've gotten most of what I wanted to cover within 3 weeks and the rest might be better served after my batch.

With the exception of the combine parser, random, and url library this is done completely using stable rust (1.3.0)
Included as a local dependency is a standalone bencode crate which provides facilities for deserializing byte streams to objects and serializing back to bytes. This is built on top of the combine library and extends the combinators by adding a 'take' combinator as well as its 'SizedBuffer' companion perhaps someday I will submit a PR back to combine :). By itself it takes almost 10 seconds to compile, which is part of the reason why it's in its own crate.

Currently this is taking a multi-threaded approach, one thread per peer. This should probably move to a kqueue/epoll backed option but that's a rabbit hole to follow on a different day. Either that or a thread-pool at the very least. mio is another viable option but it seems broken right now.
Additionally DHT and PEX are not supported currently (neither are magnet links) but maybe will be in the future.
Only HTTP(S) trackers are supported currently (UDP is also on the laundry list)


More directions to come.
