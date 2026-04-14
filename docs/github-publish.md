# Publishing to GitHub

## Create the repo

```bash
git init
git add .
git commit -m "Initial AXIOM public scaffold"
gh repo create axiom --public --source=. --remote=origin --push
```

## Enable repository protections

- require pull-request review
- require passing CI before merge
- protect the `main` branch
- require signed commits for release branches if practical
- turn on dependency alerts and secret scanning
- publish a `SECURITY.md` and bug-report channel

## Release hygiene

- tag every testnet release
- attach build artifacts and checksums
- pin the genesis hash in release notes
- keep testnet and mainnet configs separate
