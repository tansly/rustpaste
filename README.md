# rustpaste

[![Build Status](https://travis-ci.com/tansly/rustpaste.svg?token=37nt8ydfT1ey69USyytm&branch=master)](https://travis-ci.com/tansly/rustpaste)
[![codecov](https://codecov.io/gh/tansly/rustpaste/branch/master/graph/badge.svg)](https://codecov.io/gh/tansly/rustpaste)

A private pastebin server written in Rust, using actix-web framework. Aiming to be very simple.

This work is just starting and nothing is stable yet.
API will be documented once things are relatively stable.

## Planned features
After implementing a very basic API with `GET` and `POST` requests, I'm planning to add the following features.
- [x] HTTP basic authentication for posting/deleting pastes.
- [x] Syntax highlighting with [syntect](https://github.com/trishume/syntect).
- [ ] HTTP basic authentication for posting/deleting pastes.
- [ ] Configuration via a config file.
- [ ] `DELETE` method to delete pastes.
- [ ] Maybe, just maybe a frontend for this.
