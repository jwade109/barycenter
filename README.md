# Working Title: Barycenter

A game about building factories in space and exploring the cosmos
to discover the hidden jelly-filled center of the galaxy.

## Setup

1. [Install LLVM](https://rust-lang.github.io/rust-bindgen/requirements.html#windows)

    **Windows**

    ```bash
    winget install LLVM.LLVM
    ```

    **Linux**

    ```bash
    apt install libclang-dev
    ```


## TODO Tags

- `TODO(optimization)` - can be implemented faster, just haven't gotten around to yet
- `TODO(slow)` - it's slower than I think it can/should be, but not sure how
- `TODO(deprecated)` - this needs to be removed
- `TODO(testing)` - this thing needs to be verified with test cases
- `TODO(cleanup)` - this can be less lines of code
- `TODO(gross)` - this is terrible
- `TODO(bug)` - this is a known bug
- `TODO(feature)` - a thing can be added here to improve things

## Client Server Handshake

1. Server is initialized in empty state
2. Server is told to load a [MessageKind::LoadSave]
3. Server loads the save and notifies clients
   of success with [MessageKind::HasNewSave], or failure with [MessageKind::ServerError]
4. Probably nobody will be connected at this point
5. On connection, server sends the client [MessageKind::WhoGoesThere].
5. Client responds with a [MessageKind::Introduction] containing their username
6. If the server accepts their connection, responds with [MessageKind::Welcome] or
   with [MessageKind::ServerError] on error
7. When client gets [MessageKind::Welcome], will send server [MessageKind::BeginWorldDownload]