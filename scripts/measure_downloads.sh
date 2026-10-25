#!/bin/bash
# The kill test, made mechanical. Run monthly; the numbers are the only
# evidence that matters for whether this tool is finding users.
set -u
echo "--- crates.io ---"
curl -s -H 'User-Agent: imgsmith-measure' https://crates.io/api/v1/crates/imgsmith |
  jq -r '"  downloads total: \(.crate.downloads)\n  latest version: \(.crate.max_version)"'

echo "--- release assets (binary downloads) ---"
gh release list --repo marceanlabs/imgsmith --limit 5 --json tagName --jq '.[].tagName' |
while read -r tag; do
  total=$(gh release view "$tag" --repo marceanlabs/imgsmith --json assets \
    --jq '[.assets[].downloadCount] | add // 0')
  printf '  %-10s %s\n' "$tag" "$total"
done

echo "--- github signal ---"
gh api repos/marceanlabs/imgsmith --jq '"  stars: \(.stargazers_count)  forks: \(.forks_count)  issues open: \(.open_issues_count)"'
gh api repos/marceanlabs/homebrew-imgsmith --jq '"  tap stars: \(.stargazers_count)"'

echo
echo "Kill test, 23 January 2027: under 5,000 crate downloads and under 10,000"
echo "binary downloads means the tool is not finding users, and the next work is"
echo "distribution, not features."
