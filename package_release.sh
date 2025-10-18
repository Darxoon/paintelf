#!/usr/bin/env bash

cargo build --release
mkdir -p dist
tar -cJf dist/paintelf-linux.tar.xz -C target/release paintelf

cargo build --target x86_64-pc-windows-gnu --release
rm dist/paintelf-Windows.zip
zip dist/paintelf-Windows.zip -j target/x86_64-pc-windows-gnu/release/paintelf.exe
