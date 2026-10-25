# Crate ownership, decided

**Decision, 25 October 2026: one account, renamed.** No company account. The crate stays
owned by the personal account, which is renamed to a neutral handle so it carries no name
trace. A second GitHub user would only add a login to maintain, and GitHub's terms assume
one account per person.

## State, 25 October 2026

1. **GitHub account renamed to `lumenyr`** — done. `users/lumenyr` resolves to the same
   numeric ID (86650480), and `users/wlkr73` returns 404, so old links keep redirecting
   while the name is unclaimed.
2. **crates.io username renamed to `lumenyr` — done.** `users/lumenyr` now resolves to the
   same crates.io account (id 445415), `users/wlkr73` returns free, and both `imgsmith` and
   `pdfsurgeon` list `lumenyr` as owner.

The published metadata still carries `github.com/wlkr73/...` in the `repository` field,
because that is baked into each published version. Those URLs 301 to the org, so nothing
is broken, and the next publish of each crate refreshes them.

## Why nothing breaks

| Thing | Why it survives |
|---|---|
| Repos | already under `marceanlabs`, so the rename moves no code |
| Release assets | served from the org, not the personal account |
| Git authorship | the noreply address is keyed to the numeric user ID (86650480), not the name |
| Old links | GitHub redirects the profile and repos while nobody claims `wlkr73` |
| Crate ownership | ownership follows the account, not the display name |

## After the rename

```console
# local identity, both repos
git config --global user.name lumenyr
git config --global user.email 86650480+lumenyr@users.noreply.github.com

# confirm the old name is still unclaimed, so redirects keep working
gh api users/wlkr73 --jq .login  # expect 404
```

`imgsmith` and `pdfsurgeon` both have `wlkr73` baked into their published crates.io
metadata; that corrects itself the next time each is published, and the old URL redirects
until then.

## The honest limit

`marcean.com` carries the company number, and Companies House publishes the director of
Marcean Ltd as WALKER, Adam James. A neutral handle hides the casual link between the
software and a person, not a determined lookup.
