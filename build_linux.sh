#!/bin/bash
#!/bin/zsh
#

echo "Building ...."
cargo build --release
cargo install --path jira_worklog
cargo build --target x86_64-unknown-linux-gnu --release

if [ $? != 0 ]; then
  exit -1
fi

# Add cargo bin to path.

# Path to be added
CARGO_PATH="$HOME/.cargo/bin"

# Query user for confirmation
read -p "Do you want to add $CARGO_PATH to your PATH (Mandatory for first run)? (y/n): " confirm

if [[ "$confirm" == "y" || "$confirm" == "Y" ]]; then
    # Check if the path is already in the PATH
    if [[ ":$PATH:" != *":$CARGO_PATH:"* ]]; then
        # Determine which shell is being used
        case "$SHELL" in
            */bash)
                CONFIG_FILE="$HOME/.bashrc"
                ;;
            */zsh)
                CONFIG_FILE="$HOME/.zshrc"
                ;;
            *)
                echo "Unknown shell. Please manually add $CARGO_PATH to your PATH."
                exit 1
                ;;
        esac

        # Add the path to the config file
        echo "export PATH=\"$CARGO_PATH:\$PATH\"" >> "$CONFIG_FILE"
        echo "Added $CARGO_PATH to PATH in $CONFIG_FILE."
        echo "Please run 'source $CONFIG_FILE' or restart your terminal for changes to take effect."
    else
        echo "$CARGO_PATH is already in your PATH."
    fi
else
    echo "No changes made to your PATH."
fi

echo "Done"
