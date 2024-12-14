#!bash

echo "Running all commands"
cargo run --release --bin timesheet -- -V
cargo run --release --bin timesheet -- sync -i TIME-9 -i TIME-160 -i TIME-155 -i TIME-148 -i TIME-147 -s 2023-09-01 -v debug
cargo run --release --bin timesheet -- status -a 2024-08-01 -v debug
