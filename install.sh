#!/bin/bash

# Function to check if a command exists
command_exists () {
    command -v "$1" >/dev/null 2>&1 ;
}

# Function to install Xcode Command Line Tools (Clang) if not installed
install_xcode_clang () {
    if ! command_exists clang ; then
        echo "Xcode Command Line Tools (Clang) not found. Installing..."
        # Check if xcode-select tools are already installed
        if ! xcode-select -p &>/dev/null; then
            xcode-select --install
            echo "Please follow the instructions to install Xcode Command Line Tools."
            echo "Once the installation is complete, press [ENTER] to continue..."
            read
        else
            echo "Xcode Command Line Tools are already installed."
        fi
    else
        echo "Xcode Command Line Tools (Clang) are already installed."
    fi
}

# Function to install Rust
install_rust () {
    if ! command_exists rustc ; then
        echo "Rust not found. Installing Rust..."
        # Check if Homebrew is installed and use it for Rust installation
        if command_exists brew ; then
            brew install rustup-init
            rustup-init -y
        else
            # Fall back to rustup installation from the official website
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        fi
        source "$HOME/.cargo/env"
    else
        echo "Rust is already installed."
    fi
}

# Function to ensure Rust path is added to system-wide shell profile
ensure_rust_in_path () {
    if ! grep -q 'cargo/env' "/etc/bashrc" "/etc/zshrc" 2>/dev/null; then
        echo "Adding Rust to system-wide PATH."
        sudo bash -c 'echo "source \"$HOME/.cargo/env\"" >> /etc/bashrc'
        sudo bash -c 'echo "source \"$HOME/.cargo/env\"" >> /etc/zshrc'
        source "$HOME/.cargo/env"
    fi
}

# Function to clone and install your terminal application system-wide
install_application () {
    REPO_URL="https://github.com/baintonaj/playdsp.git"
    INSTALL_DIR="/usr/local/share/playdsp"

    echo "Cloning your GitHub repository into $INSTALL_DIR..."
    if [ ! -d "$INSTALL_DIR" ]; then
        sudo git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    else
        echo "Directory already exists. Pulling the latest changes..."
        cd "$INSTALL_DIR"
        sudo git pull origin main
    fi

    # Build the Rust application
    echo "Building the application in release mode..."
    sudo cargo build --release

    # Install the binary to /usr/local/bin
    echo "Installing the binary to /usr/local/bin..."
    sudo cp target/release/playdsp /usr/local/bin/

    # Set executable permissions
    sudo chmod +x /usr/local/bin/playdsp

    echo "Installation complete! You can now run 'playdsp' from any terminal."
}

# Main script execution
echo "Starting system-wide installation process..."

# Step 1: Install Xcode Command Line Tools (Clang)
install_xcode_clang

# Step 2: Install Rust
install_rust

# Step 3: Ensure Rust's cargo is in the system-wide path
ensure_rust_in_path

# Step 4: Clone the application repository and build it
install_application

echo "All done!"