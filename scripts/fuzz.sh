#!/usr/bin/env bash
# Run a compactp fuzz target locally for a user-specified duration.
#
# Usage:
#   scripts/fuzz.sh --target <lex|parse> --duration <minutes> [-- <extra libFuzzer args>]
#
# Examples:
#   scripts/fuzz.sh --target lex --duration 30
#   scripts/fuzz.sh --target parse --duration 480   # 8-hour overnight session
#   scripts/fuzz.sh --target parse --duration 60 -- -jobs=4 -workers=4
set -euo pipefail

target=""
duration=""
extra=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)   target="$2"; shift 2 ;;
    --duration) duration="$2"; shift 2 ;;
    --)         shift; extra=("$@"); break ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0 ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 2 ;;
  esac
done

if [[ -z "$target" || -z "$duration" ]]; then
  echo "Usage: $0 --target <lex|parse> --duration <minutes> [-- <extra libFuzzer args>]" >&2
  exit 2
fi

if [[ "$target" != "lex" && "$target" != "parse" ]]; then
  echo "--target must be 'lex' or 'parse', got: $target" >&2
  exit 2
fi

if ! [[ "$duration" =~ ^[0-9]+$ ]]; then
  echo "--duration must be a positive integer (minutes), got: $duration" >&2
  exit 2
fi

seconds=$(( duration * 60 ))

export PATH="$HOME/.cargo/bin:$PATH"

echo "Fuzzing target '$target' for $duration minute(s) ($seconds seconds)..."
exec cargo +nightly fuzz run "$target" -- -max_total_time="$seconds" "${extra[@]}"
