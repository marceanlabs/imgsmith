# Crate ownership, decided

**Decision, 25 October 2026: one account, renamed.** No company account. The crate stays
owned by the personal account, which is renamed to a neutral handle so it carries no name
trace. A second GitHub user would only add a login to maintain, and GitHub's terms assume
one account per person.

## The two clicks only you can do

1. **Rename the GitHub account**: github.com/settings/admin → Change username →
   `lumenyr`. The signup captcha and the settings page sit behind a login I do not have.
2. **Rename the crates.io account**: crates.io → Settings → username → `lumenyr`.
   crates.io keeps its own username; renaming GitHub alone leaves the crate page reading
   `wlkr73`.

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
