# `pkg`

Execution, performance and security focused package manager.

## Install

`pkg` is a single binary with no dependencies.

```sh
gh release download --repo mxcl/pkg --pattern 'pkg*.tar.gz'
sudo tar xzf pkg*.tar.gz -C /usr/local/bin
```

Here’s a one-liner via [`yoink`](https://github.com/mxcl/yoink):

```sh
sh <(curl -fsSL https://yoink.sh) --stream mxcl/pkg | sudo tar -xzC /usr/local/bin
```

## Overview

- Installs to `/opt`
- Installs from vendor when possible
- Installs Homebrew packages otherwise
- Never touches `/opt/homebrew`
- Dependencies of Homebrew packages are installed alongside, ie. a self contained sandbox
- Installs as little as possible to `/usr/local/bin` (no deps)
- `pkg use PKG` can run anything ephemerally (downloads fresh every time)

## Usage

```sh
$ pkg use zopflipng in.png out.png

$ sudo pkg install openclaw
/usr/local/bin/openclaw -> /opt/openclaw/bin/openclaw
# we shouldn’t let OpenClaw modify itself!

$ sudo pkg uninstall openclaw
$ sudo pkg rm openclaw

$ pkg outdated

$ sudo pkg update

$ sudo pkg list
$ sudo pkg ls
```

## Is This Ready For Me?

No. Do not use this as a replacement for Homebrew. I whipped it up in a few
days. Homebrew is 16 years old.

## Caveats

- Mostly we are not going to package things from eg. `npm`, so you will need
  to `pkg use npx` or `pkg install npm && npm install -g` yourself.
- We make exceptions arbitarily
  - eg. OpenClaw is a special case because we do not think it’s a great idea
    to let OpenClaw modify itself and `npm` is not a package manager: it’s
    a *dependency manager*. In general you should not globally install with
    `npm` (in our humble opinion) and instead always `npx`.
- `pkg use` always does an update check unless you run with a specific
  version, eg. `pkg use zopflipng@1.0.3 …`
  - notably `npx` does not behave this way and requires eg. `npx foo@latest`
    but we do not have the same scope—all our packages are ephemeral
- Homebrew formula with:
  - `post_install` steps are not supported via `pkg install` or `pkg use`
  - `pre_install` steps are not supported via `pkg install` or `pkg use`
  We may figure out how to support these. But for now we’re just not going to
  do this because we assume we will screw it up.
  - `service` metadata does not block install, but `pkg` does not manage
    those services for you

### Caveats Relative to Brew Specifically

- We do not and never will support casks
- Many vast formula like imagemagick-full and ffmpeg-full just aint gunna
  install until we go through all the deps with complex install hooks and
  rewrite them to be more self contained. We may never do this.

## Why Did You Do This?

- I made Homebrew sudo-less since I assumed devs were the only users of their
  computers. Which was a safe bet at the time.
- Nowadays we are running agents all over the place. Our users contain other
  entities that are not even human. Best we secure things better now.
- However, I want agents to be able to run anythiing they need without it
  messing with the rest of the system.
- Hence `pkg use` executes in a sandbox that can only be configured by the
  root user. If you run it without configuring it first it can only write to
  `/tmp/pkg`
- I trust Vendors *the most* to maintain their own packages.
  - Because their reputation is on the line if they mess it up.
  - Because they know how to package their own software the best.
- Packaging is an awful job and I don’t miss it so we don’t do any of it.

## Technical Details

- For Homebrew packages we rewrite `/opt/homebrew/` in the binaries and any
  text files.
  - Indeed this may prove stupid and/or flakey
  - Indeed we do not recommend you depend on `pkg` in any substantial way
- We are overzealously rejecting any Homebrew packages with pre or post
  install steps at this time.
