# bittorrent

A bottom up implementation of a bittorrent client using rust. There are still outstanding known issues but you are welcome to test it out and critique. For the most part the components are stable and the client can transfer robustly according to standard bittorrent protocol.

```
$ cargo run --bin main cargo run --bin main Ubuntu\ 15.04\ Desktop\ %2864-bit%29.torrent
```

## Component design
Currently this is taking a multi-threaded approach, one spawned thread per peer to read (and only read) from a TCP socket. (use of mio is out of scope as my personal feeling is that mio is not at the level of maturity that I require yet)

A single 'sink' thread is also spawned that handles assembled messages built from incoming packets read from client sockets. This is inspired by an actor model (not unlike akka) and it updates a shared state object.

Another separate 'spinner' thread spins (or ticks) in a regular time interval sending outgoing messages out to the clients to cancel or request chunked pieces according to their respective strategies.

The main thread does initial parsing of bencoded torrent files and the pinging of the tracker.

Global state is protected by mutexes and read-write locks with relatively low overhead (less than 1ms to acquire). It is possible to add more sinks and spinners trivially but I personally don't see a use case yet.

## Current status
1. currently this is able to parse bencoded metadata (torrent files) and responses from HTTP(S) trackers.
2. It is able to ping trackers and receive peer addresses
3. It is able to connect via TCP to peers
4. It is able to handshake with peers
5. Decodes messages from raw byte streams (Read implementers)
6. Handles messages reactively
7. Concurrently reads and writes from TCP sockets
8. Sparse bitfield operations, implemented as a vector of ranges
9. State passing between actor style threads (threadsafe)
10. Packet request, downloading and order.
11. Timeout, request strategy

## Outstanding issues
1. Endgame needs to be completed
2. Persistence (in memory and on fs)
3. Uploading to peers
4. Peer discovery (after initial tracker calls)

These will probably be deferred until RC because I've gotten most of what I wanted to cover within 3 weeks and the rest might be better served after my batch.

##Aside from that
Additionally DHT and PEX are not supported currently (neither are magnet links) but maybe will be in the future.
Only HTTP(S) trackers are supported currently (UDP is also on the laundry list)

With the exception of the combine parser, random, and url library this is done completely using stable rust (1.3.0)
Included as a local dependency is a standalone bencode crate which provides facilities for deserializing byte streams to objects and serializing back to bytes. This is built on top of the combine library and extends the combinators by adding a 'take' combinator as well as its 'SizedBuffer' companion perhaps someday I will submit a PR back to combine :). By itself it takes almost 10 seconds to compile, which is part of the reason why it's in its own crate.

## RC presentation slides tbd
