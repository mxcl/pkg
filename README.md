# The PackageManaginat0r

[![Coverage Status](https://coveralls.io/repos/github/mxcl/the-package-managinat0r/badge.svg?branch=main)](https://coveralls.io/github/mxcl/the-package-managinat0r?branch=main)

Execution, performance and security focused package manager for macOS.

## Install

The PackageManaginat0r is a single binary with no dependencies.

```sh
gh release download --repo mxcl/the-package-managinat0r --pattern 'ThePackageManaginat0r*.tar.gz'
sudo tar xzf ThePackageManaginat0r*.tar.gz -C /usr/local/bin
```

Here’s a one-liner via [`yoink`](https://github.com/mxcl/yoink):

```sh
sh <(curl -fsSL https://yoink.sh) --stream mxcl/the-package-managinat0r | sudo tar -xzC /usr/local/bin
```

## Overview

- Installs as root (like the good ol’ days)
- Installs to `/opt/$PKGNAME`
- Installs from vendor when possible
- Installs Homebrew packages otherwise
- Never touches `/opt/homebrew`
- Dependencies of Homebrew packages are installed alongside, ie. a self
  contained sandbox
- Installs as little as possible to `/usr/local/bin` (no deps)
- `p0r run PKG` can run anything ephemerally (downloads fresh every time)
- Agent focused, eg. we package `qmd` and support npm installs like
  `npm:openclaw`

## Usage

```sh
$ p0r run zopflipng in.png out.png  # alias: x
## ^^ emphermeral; downloads fresh every time

$ sudo p0r install npm:openclaw
/usr/local/bin/openclaw
# ^^ humans don’t let Claws modify themselves

$ sudo p0r uninstall npm:openclaw  # alias: rm

$ p0r list  # alias: ls

$ p0r outdated

$ sudo p0r update  # alias: up
```

## Is This Ready For Me?

No. Do not use this as a replacement for Homebrew. I whipped it up in a few
days. Homebrew is 16 years old.

## But I Wanna!

That’s fine. I like it. I think it's good. Maybe you will too.

## Caveats

- `p0r run` is ephemeral. It always downloads and it always downloads the
  latest version unless you specify, eg. `p0r run zopflipng@1.0.3 …`
  > This is a feature. We are operating in an agentic world where agents
  > can literally modify binaries if they want to be malicious. Everything
  > must be installed by a human and if not then the tool that is installed
  > by root that is executing things should never trust a user-writable cache
- there is no `p0r services` command. Use `brew`.
- some Homebrew formulae are not supported. If you come across them, report
  this as a bug.
- We do not and will likely never support casks.

## Why Did You Do This?

- I made Homebrew sudo-less since I assumed devs were the only users of their
  computers. Which was a safe bet at the time.
- Nowadays we are running agents all over the place. Our users contain other
  entities that are not even human. Best we secure things better now.
- However, I want agents to be able to run anythiing they need without it
  messing with the rest of the system.
- Hence `p0r run` executes in a sandbox that can only be configured by the
  root user. If you run it without configuring it first it can only write to
  `/tmp`
- I trust Vendors *the most* to maintain their own packages.
  - Because their reputation is on the line if they mess it up.
  - Because they know how to package their own software the best.
- Packaging is an awful job and I don’t miss it so we don’t do any of it.

## Technical Details

- For Homebrew packages we rewrite `/opt/homebrew/` in the binaries and any
  text files.
  - Indeed this may prove stupid and/or flakey
  - Indeed we do not recommend you depend on `p0r` in any substantial way
- We are overzealously rejecting any Homebrew packages with pre or post
  install steps at this time.
