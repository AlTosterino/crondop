# homebrew-crondrop

This directory contains the tap-ready contents for a dedicated Homebrew tap repository.

To support:

```bash
brew tap AlTosterino/crondrop
brew install crondrop
```

create a separate GitHub repository named:

```text
AlTosterino/homebrew-crondrop
```

and place the generated formula at:

```text
Formula/crondrop.rb
```

The release workflow in the main app repository can update that repo automatically after each tagged release if you configure:

- `HOMEBREW_TAP_REPOSITORY`
- `HOMEBREW_TAP_GITHUB_TOKEN`

Suggested value for `HOMEBREW_TAP_REPOSITORY`:

```text
AlTosterino/homebrew-crondrop
```
