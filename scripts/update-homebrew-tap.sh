#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 <tap_repo_dir> <app_repo> <tag> <linux_sha256> <macos_sha256>" >&2
  exit 1
fi

tap_repo_dir="$1"
app_repo="$2"
tag="$3"
linux_sha="$4"
macos_sha="$5"

formula_dir="${tap_repo_dir}/Formula"
mkdir -p "${formula_dir}"

bash "$(dirname "${BASH_SOURCE[0]}")/render-homebrew-formula.sh" \
  "${app_repo}" \
  "${tag}" \
  "${linux_sha}" \
  "${macos_sha}" \
  > "${formula_dir}/crondrop.rb"

cat > "${tap_repo_dir}/README.md" <<EOF
# homebrew-crondrop

Homebrew tap for installing Cron Drop.

## Install

\`\`\`bash
brew tap AlTosterino/crondrop
brew install crondrop
\`\`\`

The formula installs release archives published from:

- https://github.com/${app_repo}
EOF
