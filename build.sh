#!/bin/zsh
#

echo "Building ...."
cargo build --release
cargo install --path jira_worklog
cargo build --target x86_64-pc-windows-gnu --release

echo "Deploying to shared areas..."
cp target/release/jira_worklog /Users/steinar/Library/CloudStorage/OneDrive-SharedLibraries-AUTOSTOREAS/QubIt\ -\ Documents/042-worklog/MacOs
cp target/x86_64-pc-windows-gnu/release/jira_worklog.exe /Users/steinar/Library/CloudStorage/OneDrive-SharedLibraries-AUTOSTOREAS/QubIt\ -\ Documents/042-worklog/Windows/jira_worklog.exe
cp jira_worklog/README.pdf /Users/steinar/Library/CloudStorage/OneDrive-SharedLibraries-AUTOSTOREAS/QubIt\ -\ Documents/042-worklog
echo "Done"