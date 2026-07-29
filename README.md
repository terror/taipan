## taipan

`taipan` is a native jupyter notebook frontend.

The channel transport uses `zeromq`, a Tokio-based pure Rust ZeroMQ
implementation. This keeps desktop and CI builds independent of a system
`libzmq` installation while supporting Jupyter's TCP and IPC socket topology.
