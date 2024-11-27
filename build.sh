#!/bin/zsh
#

echo "Building ...."
cargo build --release
cargo install --path jira_worklog
cargo build --target x86_64-pc-windows-gnu --release

# Ugly path to OneDrive on MacOS
DEPLOY_DIR="$HOME/Library/CloudStorage/OneDrive-AUTOSTOREAS/Shared Documents - AutoStore - SW Engineering/tools/jira_worklog/"
MACOS_DIR="$DEPLOY_DIR/MacOs"
WINDOWS_DIR="$DEPLOY_DIR/Windows"

echo "Deploying to shared areas in OneDrive ..."
echo "$DEPLOY_DIR"

if [[ ! -d "$DEPLOY_DIR" ]]; then
  echo "Seems the OneDrive Directory is not available:"
  echo "$DEPLOY_DIR"
  exit 4
fi

if [[ ! -d "$MACOS_DIR" ]]; then
  mkdir "$MACOS_DIR"
fi
if [[ ! -d "$WINDOWS_DIR" ]]; then
  mkdir "$WINDOWS_DIR"
fi

cp target/release/jira_worklog $DEPLOY_DIR/MacOs
cp target/x86_64-pc-windows-gnu/release/jira_worklog.exe $DEPLOY_DIR/Windows/jira_worklog.exe
cp jira_worklog/README.pdf $DEPLOY_DIR
echo "Done"
