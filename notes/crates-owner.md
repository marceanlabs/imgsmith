# Giving the company the crate

crates.io has no organisations and no email invites. An owner is a crates.io user
account, and a crates.io account is created by signing in with GitHub. An organisation
cannot sign in, so the company needs a GitHub *user* account of its own. That account
is the only way `imgsmith` can be owned by Marcean Labs rather than by a person.

## What only you can do (five minutes, because of the captcha and the email)

1. **Pick the address.** The account needs an email that is not already on GitHub.
   `releases@marcean.com` is the right kind of address; do not reuse a personal one.
2. **Create the GitHub account** at https://github.com/signup with the username
   **`marceanlabs-release`** (free on GitHub as of 25 Oct 2026) and that address.
   Verify the email.
3. **Sign in to crates.io** at https://crates.io with that GitHub account. crates.io
   creates the account and its username becomes `marceanlabs-release`.
4. **Tell me it exists.** The next two steps are mine.

Caveat worth knowing: GitHub's terms assume one account per person. A machine account
for an organisation is normal practice, but it can be flagged, so keep it for releases
only and do not use it to interact with other people's repos.

## What I do once the account exists

```console
# 1. wlkr73 invites the company as a co-owner (crates.io emails the invite)
cargo owner --add marceanlabs-release imgsmith

# 2. after the invite is accepted, confirm both owners are listed
curl -s https://crates.io/api/v1/crates/imgsmith/owners \
  -H 'User-Agent: imgsmith-measure' | jq -r '.users[].login'

# 3. the company account gets its own token, stored outside the repo
#    (cargo login, then publish as the company for the next release)
```

## What this changes

| Thing | Before | After |
|---|---|---|
| Crate owner | wlkr73 | wlkr73 + marceanlabs-release |
| Who can publish | the personal account | either account |
| Crate page ownership line | personal | reads as Marcean Labs |
| Bus factor | one login | two logins, one held by the company |

Ownership cannot be transferred away from an account, only shared, so the personal
account stays listed. That is normal and harmless: the point is that the company is not
locked out.
