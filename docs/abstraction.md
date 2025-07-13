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

## Connection

```rust
struct Connection {
    phase: Phase,
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

