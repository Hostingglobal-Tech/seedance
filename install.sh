#!/usr/bin/env bash
# seedance one-shot installer for Linux / macOS / Termux
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Hostingglobal-Tech/seedance/main/install.sh | bash
set -euo pipefail

REPO="https://github.com/Hostingglobal-Tech/seedance"
BIN_NAME="seedance"

say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
err() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

# 1) Rust toolchain
if ! command -v cargo >/dev/null 2>&1; then
  say "Rust 툴체인이 없습니다. rustup 으로 설치합니다."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

command -v cargo >/dev/null 2>&1 || err "rustup 설치 실패 — 수동 재시도 필요 (https://rustup.rs)"

# 2) Build + install seedance
say "seedance 을 빌드·설치합니다 ($(cargo --version))"
cargo install --git "$REPO" --locked

BIN_PATH="$HOME/.cargo/bin/$BIN_NAME"
[[ -x "$BIN_PATH" ]] || err "설치 후 $BIN_PATH 를 찾을 수 없습니다."

# 3) PATH 점검
if ! command -v "$BIN_NAME" >/dev/null 2>&1; then
  RC="$HOME/.bashrc"
  [[ "${SHELL##*/}" == "zsh" ]] && RC="$HOME/.zshrc"
  if ! grep -q 'cargo/bin' "$RC" 2>/dev/null; then
    printf '\nexport PATH="$HOME/.cargo/bin:$PATH"\n' >> "$RC"
    say "PATH 에 ~/.cargo/bin 추가 → $RC"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"
fi

# 4) FAL_KEY
if [[ -z "${FAL_KEY:-}" ]]; then
  RC="$HOME/.bashrc"
  [[ "${SHELL##*/}" == "zsh" ]] && RC="$HOME/.zshrc"
  if grep -q '^export FAL_KEY=' "$RC" 2>/dev/null; then
    say "FAL_KEY 가 이미 $RC 에 있습니다."
  else
    say "OpenAI API 키 입력 (sk-로 시작, 화면에 표시되지 않음)"
    printf "key: "
    read -rs KEY
    printf "\n"
    [[ -n "$KEY" ]] || err "빈 키로는 설치를 완료할 수 없습니다."
    printf '\nexport FAL_KEY="%s"\n' "$KEY" >> "$RC"
    export FAL_KEY="$KEY"
    say "$RC 에 FAL_KEY 저장 완료"
  fi
fi

# 5) Smoke test
say "설치 확인 중 — 샘플 이미지 1장 생성 (low quality)"
"$BIN_PATH" "a tiny friendly mascot, minimal vector style" --quality low || err "샘플 호출 실패. 키·네트워크 확인 필요."

say "설치 완료. 이제 아래처럼 쓰세요:"
printf "    %s '프롬프트' [--size 1024x1024] [--quality high]\n\n" "$BIN_NAME"
