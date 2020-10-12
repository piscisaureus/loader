#!/bin/sh
wasm-pack build --no-typescript --target no-modules
prettier --write pkg/*.js
