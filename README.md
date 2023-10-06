
## How to build on MacOS

The script `build.sh` will compile all the binaries and upload them
to the OneDrive directory.

## Cross compiling to Windows on MacOS
How to cross compile from Mac to Windows:
```shell
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu
```