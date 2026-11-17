# Net Socket API Design for Still

## Overview
Real TCP/UDP socket support via platform-specific C code.
Uses Winsock on Windows, POSIX sockets on Linux/macOS.

## API Design

### TCP Client
```
use net

sock = net.connect("example.com", 80)
net.send(sock, "GET / HTTP/1.0\r\n\r\n")
response = net.recv(sock, 4096)
net.close(sock)
```

### TCP Server
```
use net

listener = net.listen(8080)
while true:
    client = net.accept(listener)
    data = net.recv(client, 4096)
    net.send(client, "Hello from Still!")
    net.close(client)
```

### UDP
```
use net

sock = net.udp_bind(9000)
net.udp_send(sock, "hello", "127.0.0.1", 9001)
data, addr, port = net.udp_recv(sock, 1024)
net.close(sock)
```

## Implementation
- C runtime functions: still_net_connect, still_net_listen, still_net_accept, still_net_send, still_net_recv, still_net_close, still_net_udp_bind, still_net_udp_send, still_net_udp_recv
- extern "C" declarations in net.strc
- Platform-specific: #ifdef _WIN32 for Winsock, else POSIX

## Status: Design complete, implementation pending
Real socket C code needs to be added to still_rt.c and still_rt.h.