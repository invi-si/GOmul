# LGT offline socket-write handling

Tera: Eternal Chaos called unknown WIPI import `0x25c` while leaving its connection attempt. At guest PC `0x1287d6`, the caller passes its socket descriptor, a pointer to a zero byte, and length 1, then conditionally closes the socket and network. Adjacent import `0x25b` receives the socket-connect call shape. This identifies `0x25c` as `MC_netSocketWrite(fd, buffer, length)`.

The API signature and negative-error/positive-byte-count convention are described in the original author's [WIPI network tutorial](https://sway.tistory.com/entry/Clet%EC%A3%BC%EB%85%B8s-Clet%EA%B0%95%EC%A2%8C-8-API-%EB%A7%9B%EB%B3%B4%EA%B8%B0-Network-1-%ED%8E%B8). The numeric mapping is derived from the guest call sites rather than that tutorial.

GOmul already rejects network connections through its offline backend. The newly registered socket-write handler returns `M_E_ERROR` (-1) synchronously, without reading the buffer, claiming bytes sent, or generating a callback. It does not implement live sockets or emulate a game server. No game files, saves, guest timing, or application-specific branches are changed.

Regression coverage exercises the actual LGT import with null and invalid buffer pointers, multiple descriptors, and zero/nonzero lengths. Validation: 27 LGT and 46 WIPI-C tests passed; formatting and workspace lint completed. The normal ARM64 Android candidate was built and installed without clearing app data. Live re-entry to the connection screen is pending.
