# Circle Mover

A proof of concept for how to use Rust and Godot to implement a real time strategy game using a deterministic lock-step simulation.

The game is a simple distillation of RTS games: you can create circles and move them with typical RTS commands.

## Components:

- cm-sim is a standalone implementation of all game logic
- godot is the Godot project
  - imports godot-rust-client via gdextension
- godot-rust-client
  - imports cm-sim as a library
- cm-server is a rust application that synchronizes player input
  - imports cm-sim as a library

In this way the game client and the running the exact same logic.

## References

- [Actors With Tokio](https://ryhl.io/blog/actors-with-tokio/)
- [1500 Archers on a 28.8: Network Programming in Age of Empires and Beyond](https://www.gamedeveloper.com/programming/1500-archers-on-a-28-8-network-programming-in-age-of-empires-and-beyond)
- [James Anhalt & Tim Morten Interview: SnowPlay Technology & Stormgate](https://screenrant.com/james-anhalt-tim-morten-interview-snowplay-technology-stormgate/)
- [The Revolution of StarCraft Network Traffic](https://myslu.stlawu.edu/~clee/docs/starcraft2.netgames.2012.pdf)
- [Don’t use Lockstep in RTS games](https://medium.com/@treeform/dont-use-lockstep-in-rts-games-b40f3dd6fddb)
- [RTS Client-Server Networking](https://medium.com/@evan_73063/rts-client-server-networking-36e8154ff740)
- [Synchronous RTS Engines and a Tale of Desyncs](https://www.forrestthewoods.com/blog/synchronous_rts_engines_and_a_tale_of_desyncs/)
