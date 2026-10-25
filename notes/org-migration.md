# Moving imgsmith under Marcean Labs

Decided 25 October 2026. Handle `marceanlabs` is taken by someone else, `marcean-labs` is
free, so the organisation is **Marcean Labs** with handle **marcean-labs**.

## The one step that cannot be automated

GitHub creates organisations from the web UI only; `POST /orgs` returns 404 for a user
token. So this click is yours:

1. https://github.com/account/organizations/new?plan=free_org
2. Account name: `marcean-labs` (this is the handle, it cannot be changed later)
3. Display name: `Marcean Labs`
4. Owner: your account, and leave the org public.

## Then, in order

```console
# 1. transfer the repo (GitHub redirects the old URLs, including release assets)
gh api -X POST repos/wlkr73/imgsmith/transfer -f new_owner=marcean-labs

# 2. confirm the assets still download after the transfer
gh release view v0.1.1 --repo marcean-labs/imgsmith

# 3. point the clone at the new home
cd /root/work/imgtool && git remote set-url origin https://github.com/marcean-labs/imgsmith.git

# 4. create the tap and push the prepared formula
gh repo create marcean-labs/homebrew-imgsmith --public \
  --description "Homebrew tap for imgsmith"
# then: Formula/imgsmith.rb from notes/homebrew-formula.rb
```

## crates.io is the exception

crates.io has no organisations. A crate is owned by user accounts, and accounts are
created by signing in with GitHub, which an organisation cannot do. Two honest options:

| Option | What it means |
|---|---|
| Keep wlkr73 as the owner | simplest; the crate page points at the marcean-labs repo, which redirects fine |
| A company user account | create a GitHub user for the company, sign in to crates.io with it, then `cargo owner --add <that user> imgsmith` |

The second is the only way the company itself holds the crate. Do it before the tool has
users, not after.

## What changes and what does not

| Thing | Effect |
|---|---|
| GitHub repo URL | redirects; old links keep working |
| Release asset URLs | redirect; the formula keeps working after the transfer |
| Homebrew tap | becomes `marcean-labs/imgsmith`, installed with `brew install marcean-labs/imgsmith/imgsmith` |
| Cargo.toml `repository` field | update on the next publish, not worth a version bump alone |
| Adoption | unchanged. This decides ownership, not distribution. |
