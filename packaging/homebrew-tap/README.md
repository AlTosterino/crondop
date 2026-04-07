# Homebrew Tap Notes

This directory contains the tap-ready contents for Homebrew distribution.

To support:

```bash
brew tap AlTosterino/crondrop
brew install crondrop
```

place the generated formula at:

```text
Formula/crondrop.rb
```

The release workflow in the main app repository can update the tap automatically after each tagged release if you configure:

- `HOMEBREW_TAP_REPOSITORY`
- `HOMEBREW_TAP_GITHUB_TOKEN`

Suggested value for `HOMEBREW_TAP_REPOSITORY`:

```text
AlTosterino/crondrop
```
