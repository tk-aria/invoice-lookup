#!/bin/sh

set -e

if [ -n "${DEBUG}" ]; then
  set -x
fi

# デフォルト設定
DEFAULT_INSTALL_PATH="/usr/local/bin"
REPO="tk-aria/invoice-lookup"
BINARY_NAME="invoice-cli"

# ---------- ユーティリティ ----------

_latest_version() {
  curl -sSLf "https://api.github.com/repos/${REPO}/releases/latest" | \
    grep '"tag_name":' | \
    sed -E 's/.*"([^"]+)".*/\1/'
}

_detect_os() {
  os="$(uname -s)"
  case "$os" in
    Linux) echo "linux" ;;
    Darwin) echo "darwin" ;;
    CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
    *) echo "Unsupported operating system: $os" 1>&2; return 1 ;;
  esac
  unset os
}

_detect_arch() {
  arch="$(uname -m)"
  case "$arch" in
    amd64|x86_64) echo "x86_64" ;;
    arm64|aarch64) echo "aarch64" ;;
    *) echo "Unsupported processor architecture: $arch" 1>&2; return 1 ;;
  esac
  unset arch
}

_get_target() {
  _os="$1"
  _arch="$2"
  case "$_os" in
    linux)   echo "${_arch}-unknown-linux-gnu" ;;
    darwin)  echo "aarch64-apple-darwin" ;;
    windows) echo "${_arch}-pc-windows-msvc" ;;
  esac
}

_get_ext() {
  _os="$1"
  case "$_os" in
    windows) echo "zip" ;;
    *) echo "tar.gz" ;;
  esac
}

_get_binary_file() {
  _os="$1"
  case "$_os" in
    windows) echo "${BINARY_NAME}.exe" ;;
    *) echo "${BINARY_NAME}" ;;
  esac
}

_download_url() {
  _version="$1"; _target="$2"; _ext="$3"
  echo "https://github.com/${REPO}/releases/download/${_version}/${BINARY_NAME}-${_version}-${_target}.${_ext}"
}

_resolve_install_path() {
  echo "${INVOICE_CLI_INSTALL_PATH:-$DEFAULT_INSTALL_PATH}"
}

# ---------- install サブコマンド ----------

cmd_install() {
  # バージョン決定
  if [ -z "${INVOICE_CLI_VERSION}" ]; then
    echo "Getting latest version..."
    INVOICE_CLI_VERSION=$(_latest_version)
    if [ -z "${INVOICE_CLI_VERSION}" ]; then
      echo "Failed to get latest version" 1>&2
      return 1
    fi
  fi

  install_path="$(_resolve_install_path)"
  detected_os="$(_detect_os)"
  detected_arch="$(_detect_arch)"
  target="$(_get_target "$detected_os" "$detected_arch")"
  ext="$(_get_ext "$detected_os")"
  binary="$(_get_binary_file "$detected_os")"
  download_url="$(_download_url "$INVOICE_CLI_VERSION" "$target" "$ext")"

  echo "Installing ${BINARY_NAME} ${INVOICE_CLI_VERSION} for ${detected_os}/${detected_arch} (${target})..."
  echo "Download URL: $download_url"

  # インストールディレクトリ作成
  if [ ! -d "$install_path" ]; then
    echo "Creating install directory: $install_path"
    mkdir -p "$install_path"
  fi

  # 一時ディレクトリ
  tmp_dir=$(mktemp -d)
  trap 'rm -rf "$tmp_dir"' EXIT

  # ダウンロード
  echo "Downloading..."
  if ! curl -sSLf "$download_url" -o "$tmp_dir/archive.${ext}"; then
    echo "Failed to download from: $download_url" 1>&2
    echo "Check if version ${INVOICE_CLI_VERSION} exists for ${target}" 1>&2
    return 1
  fi

  # 展開
  echo "Extracting..."
  case "$ext" in
    tar.gz) tar -xzf "$tmp_dir/archive.tar.gz" -C "$tmp_dir" ;;
    zip)    unzip -q "$tmp_dir/archive.zip" -d "$tmp_dir" ;;
  esac

  # バイナリ検索
  archive_dir="${BINARY_NAME}-${INVOICE_CLI_VERSION}-${target}"
  if [ -f "$tmp_dir/${archive_dir}/${binary}" ]; then
    binary_path="$tmp_dir/${archive_dir}/${binary}"
  elif [ -f "$tmp_dir/${binary}" ]; then
    binary_path="$tmp_dir/${binary}"
  else
    echo "Binary not found in archive. Expected: ${archive_dir}/${binary}" 1>&2
    return 1
  fi

  # 配置 (権限がない場合は ~/.local/bin にフォールバック)
  _installed=0
  if cp "$binary_path" "$install_path/$binary" 2>/dev/null; then
    chmod 755 "$install_path/$binary"
    _installed=1
  else
    # フォールバック: ~/.local/bin
    fallback_path="$HOME/.local/bin"
    if [ "$install_path" != "$fallback_path" ]; then
      echo "Permission denied for $install_path — falling back to $fallback_path"
      mkdir -p "$fallback_path"
      if cp "$binary_path" "$fallback_path/$binary" 2>/dev/null; then
        chmod 755 "$fallback_path/$binary"
        install_path="$fallback_path"
        _installed=1
      fi
    fi
  fi

  if [ "$_installed" -eq 0 ]; then
    echo "Failed to copy binary to $install_path" 1>&2
    echo "Try: sudo sh setup.sh install" 1>&2
    return 1
  fi

  echo ""
  echo "${BINARY_NAME} ${INVOICE_CLI_VERSION} installed successfully!"
  echo "  Binary: $install_path/$binary"

  # PATH に含まれていない場合のヒント
  case ":$PATH:" in
    *":$install_path:"*) ;;
    *)
      echo ""
      echo "NOTE: $install_path is not in your PATH."
      echo "Add it with:"
      echo "  export PATH=\"$install_path:\$PATH\""
      echo ""
      echo "To make it permanent, add the line above to your ~/.zshrc or ~/.bashrc"
      ;;
  esac

  echo ""
  echo "Run '${BINARY_NAME} --help' to get started."
}

# ---------- uninstall サブコマンド ----------

cmd_uninstall() {
  install_path="$(_resolve_install_path)"
  detected_os="$(_detect_os)"
  binary="$(_get_binary_file "$detected_os")"
  binary_full="$install_path/$binary"

  # フォールバックパスも探す
  if [ ! -f "$binary_full" ]; then
    fallback_path="$HOME/.local/bin/$binary"
    if [ -f "$fallback_path" ]; then
      binary_full="$fallback_path"
      install_path="$HOME/.local/bin"
    else
      echo "${BINARY_NAME} is not installed at $binary_full" 1>&2
      echo "If installed elsewhere, set INVOICE_CLI_INSTALL_PATH" 1>&2
      return 1
    fi
  fi

  echo "Removing ${BINARY_NAME} from $binary_full ..."
  if ! rm -f "$binary_full"; then
    echo "Failed to remove $binary_full. Try: sudo sh setup.sh uninstall" 1>&2
    return 1
  fi

  echo ""
  echo "${BINARY_NAME} has been uninstalled."
}

# ---------- build-install サブコマンド (ソースからビルド＆インストール) ----------

cmd_build_install() {
  echo "Building ${BINARY_NAME} from source..."

  # Rust ツールチェインの存在確認
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust toolchain not found. Install from https://rustup.rs/" 1>&2
    return 1
  fi

  # リポジトリルートを探す
  script_dir="$(cd "$(dirname "$0")" && pwd)"
  if [ -f "$script_dir/../cli/Cargo.toml" ]; then
    repo_root="$script_dir/.."
  elif [ -f "./cli/Cargo.toml" ]; then
    repo_root="."
  else
    echo "Cannot find cli/Cargo.toml. Run from repository root or scripts/ directory." 1>&2
    return 1
  fi

  install_path="$(_resolve_install_path)"

  echo "Building release binary..."
  (cd "$repo_root" && cargo build -p invoice-lookup-cli --release)

  # ビルド成果物を探す
  binary_path=""
  for candidate in \
    "$repo_root/target/release/${BINARY_NAME}" \
    "$repo_root/target/x86_64-unknown-linux-musl/release/${BINARY_NAME}" \
    "$repo_root/target/x86_64-unknown-linux-gnu/release/${BINARY_NAME}" \
    "$repo_root/target/aarch64-apple-darwin/release/${BINARY_NAME}"
  do
    if [ -f "$candidate" ]; then
      binary_path="$candidate"
      break
    fi
  done

  if [ -z "$binary_path" ]; then
    echo "Build succeeded but binary not found." 1>&2
    return 1
  fi

  # 配置
  if [ ! -d "$install_path" ]; then
    mkdir -p "$install_path"
  fi

  _installed=0
  if cp "$binary_path" "$install_path/${BINARY_NAME}" 2>/dev/null; then
    chmod 755 "$install_path/${BINARY_NAME}"
    _installed=1
  else
    fallback_path="$HOME/.local/bin"
    if [ "$install_path" != "$fallback_path" ]; then
      echo "Permission denied for $install_path — falling back to $fallback_path"
      mkdir -p "$fallback_path"
      if cp "$binary_path" "$fallback_path/${BINARY_NAME}" 2>/dev/null; then
        chmod 755 "$fallback_path/${BINARY_NAME}"
        install_path="$fallback_path"
        _installed=1
      fi
    fi
  fi

  if [ "$_installed" -eq 0 ]; then
    echo "Failed to install binary. Try: sudo sh setup.sh build-install" 1>&2
    return 1
  fi

  echo ""
  echo "${BINARY_NAME} built and installed successfully!"
  echo "  Binary: $install_path/${BINARY_NAME}"
  echo ""
  echo "Run '${BINARY_NAME} --help' to get started."
}

# ---------- ヘルプ ----------

usage() {
  cat <<EOF
invoice-cli setup script

Usage: sh setup.sh <command>

Commands:
  install          Download and install the latest release binary
  uninstall        Remove installed binary
  build-install    Build from source and install (requires Rust toolchain)
  help             Show this help message

Environment Variables:
  INVOICE_CLI_VERSION       Version to install (default: latest)
  INVOICE_CLI_INSTALL_PATH  Install directory (default: /usr/local/bin)
  DEBUG                     Enable verbose output

Examples:
  # Install latest release
  curl -sSLf https://raw.githubusercontent.com/${REPO}/main/scripts/setup.sh | sh -s install

  # Install specific version
  curl -sSLf https://raw.githubusercontent.com/${REPO}/main/scripts/setup.sh | INVOICE_CLI_VERSION=v0.1.0 sh -s install

  # Install to custom path
  curl -sSLf https://raw.githubusercontent.com/${REPO}/main/scripts/setup.sh | INVOICE_CLI_INSTALL_PATH=~/.local/bin sh -s install

  # Build from source and install
  sh scripts/setup.sh build-install

  # Uninstall
  curl -sSLf https://raw.githubusercontent.com/${REPO}/main/scripts/setup.sh | sh -s uninstall
EOF
}

# ---------- エントリポイント ----------

main() {
  command="${1:-}"

  case "$command" in
    install)        cmd_install ;;
    uninstall)      cmd_uninstall ;;
    build-install)  cmd_build_install ;;
    -h|--help|help) usage ;;
    "")
      # サブコマンドなし → デフォルトで install (後方互換)
      cmd_install
      ;;
    *)
      echo "Unknown command: $command" 1>&2
      echo "" 1>&2
      usage 1>&2
      return 1
      ;;
  esac
}

main "$@"
