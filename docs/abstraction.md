# HTTP1 Server and Client Abstraction

Using the same connection logic for server and client.

```
        /------>------\
       /               \
|---------|         |----------|
| Request |         | Response |
|---------|         |----------|
       \               /
        \------<------/
```

## HTTP1 Connection

The `Connection` will be generic over `Request` and `Response` service.

```rust
struct Connection {
    io: IO,
    phase: Phase,
    request_service: RequestService,
    response_service: ResponseWrite,
}

impl Connection {
    fn poll(&mut self) {
        match self.phase {
            RequestRead => {
                self.request_service.poll_read(self.io);
            },
            RequestService(service) => {
                service.poll()
            },
            RequestWrite(request) => {
                self.request_service.poll_write(response, self.io);
            },

            ResponseRead => {
                self.response_service.poll_read(self.io);
            },
            ResponseService(service) => {
                service.poll()
            },
            ResponseWrite(response) => {
                self.response_service.poll_write(response, self.io);
            },
        }
    }
}
```

## Body

A body can be:

- `ExactSize`, io streaming with known length
- `Buffered`, fully buffered in memory
- `Streaming`, unknown length io streaming

