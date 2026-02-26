# ExOpenDirectory

Elixir bindings to macOS OpenDirectory.framework for directory services.

Provides native access to local accounts, LDAP servers, and Active Directory
domains via Apple's `opendirectoryd` daemon. Uses [Rustler](https://github.com/rusterlium/rustler)
NIFs backed by [objc2-open-directory](https://crates.io/crates/objc2-open-directory).

## Why Not Just Use `:eldap`?

OpenDirectory provides capabilities that raw LDAP cannot:

- **AD integration** — Apple's own AD plugin handles site-aware DC discovery,
  Kerberos ticket acquisition, nested group resolution, and GPO awareness
- **Local directory** — Query local macOS users/groups, not just networked ones
- **Password policy** — Reads `msDS-UserPasswordExpiryTimeComputed` and local
  password policy, computing days-until-expiry correctly
- **Authentication** — `ODRecord.verifyPassword` triggers the full auth chain
  (Kerberos on AD-bound nodes) rather than a simple LDAP bind

If you're building a NoMAD/Jamf Connect replacement, you need this framework.
If you just need basic LDAP queries, `:eldap` or `exldap` may suffice.

## Installation

```elixir
def deps do
  [{:ex_open_directory, "~> 0.1.0"}]
end
```

Requires Rust toolchain (`rustup`). macOS only.

## Usage

```elixir
# Connect to the search node (searches all configured directories)
{:ok, node} = ExOpenDirectory.connect(:search)

# Find a user
{:ok, record} = ExOpenDirectory.find_user(node, "jsmith")

# Get attributes
{:ok, attrs} = ExOpenDirectory.get_attributes(record, [
  "dsAttrTypeStandard:RealName",
  "dsAttrTypeStandard:EMailAddress"
])

# Check group membership (handles AD nested groups)
true = ExOpenDirectory.member?(node, "jsmith", "Engineering")

# Authenticate (triggers Kerberos on AD-bound nodes)
:ok = ExOpenDirectory.authenticate(node, "jsmith", "password123")

# Check password expiry
{:ok, policy} = ExOpenDirectory.password_policy(node, "jsmith")
policy.days_until_expiry #=> 14
```

## License

MIT
