# rustpaste

[![Build Status](https://travis-ci.com/tansly/rustpaste.svg?token=37nt8ydfT1ey69USyytm&branch=master)](https://travis-ci.com/tansly/rustpaste)
[![codecov](https://codecov.io/gh/tansly/rustpaste/branch/master/graph/badge.svg)](https://codecov.io/gh/tansly/rustpaste)

A private pastebin server written in Rust, using actix-web framework. Aiming to be very simple.

This is a work in progress.

## REST API
* To upload a paste, send a `POST` request to `/` with `application/x-www-form-urlencoded` data.
**This requires authentication (HTTP basic authentication).** If the request is successful, response contains the URL of the paste.

    Note that urlencoded data is not the best approach for this because of some limitations
    (for example the max file size imposed by actix-web, extra overhead for non-ASCII data).
    **I'm planning to change this to use `multipart/form-data`.** Dealing with urlencoded
    data is so much easier in actix-web; that's why I chose to do it this way initially.

    You can use `curl` as follows to upload the contents of a file named `filename`:
    ```
    curl <domain> --basic -u "user:pass" --data-urlencode "data@filename"
    ```
    You should definitely use HTTPS or you'll get owned.
    Since `rustpaste` does not have SSL support,
    put it behind some reverse proxy (such as nginx) with SSL support.

* To get a paste, send a `GET` request to the URL returned by the `POST`.
To get the paste with syntax highlighting, add the file extension for the file
as a separate path parameter in the `GET` request. For example, to highlight a
paste with name `QDRASC3w` as Rust source code, send a request to `/QDRASC3w/rs`.

* **TODO:** To delete a paste, send a `DELETE` request to the paste URL.
This requires authentication.

## Configuration
**TODO:** Things like where to store the pastes, username and password to be used by HTTP authentication,
base URL to prepend to the `POST` responses will be configured using a config file.

## Planned features
After implementing a very basic API with `GET` and `POST` requests, I'm planning to add the following features.
- [x] HTTP basic authentication for posting/deleting pastes.
- [x] Syntax highlighting with [syntect](https://github.com/trishume/syntect).
- [ ] `DELETE` method to delete pastes.
- [ ] Configuration via a config file.
