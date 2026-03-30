# Substrate

[![Coverage Status](https://coveralls.io/repos/github/mxcl/substrate/badge.svg?branch=main)](https://coveralls.io/github/mxcl/substrate?branch=main)

Execution, performance and security oriented multi-source package manager for
macOS.

## Install

Substrate is a single rust binary with no dependencies.

```sh
gh release download --repo mxcl/substrate --pattern 'substrate*.tar.gz'
sudo tar xzf substrate*.tar.gz -C /usr/local/bin
```

Here’s a one-liner via [`yoink`](https://github.com/mxcl/yoink):

```sh
sh <(curl -fsSL https://yoink.sh) --stream mxcl/substrate | sudo tar -xzC /usr/local/bin
```

## Usage Overview

```sh
$ sudo subs install node  # alias: i
/usr/local/bin/node
## ^^ installs homebrew packages

$ sudo subs i npm:openclaw
/usr/local/bin/openclaw
## ^^ installs npm packages

$ sudo subs i npm:@tobilu/qmd
◇ installing brew:sqlite dependency…
/usr/local/bin/qmd
## ^^ knows when npm packages need homebrew dependencies

$ sudo subs i pip:psycopg2
◇ installing brew:libpg dependency…
/usr/local/bin/psycopg2
## ^^ same for pypi. virtual environment and everything

$ sudo subs i bun
/usr/local/bin/bun
## ^^ bun is not in homebrew, but the vendor provides a package and we
##    know how to check there when brew doesn't have it.
```

Packages are installed as root with all dependencies side-by-side in
`/opt/PKGNAME`. Self-contained, isolated and reliable. Also it’s fast.

We also can run anything:

```sh
$ subs run zopflipng in.png out.png  # alias: x
## ^^ run anything ephemerally; downloads fresh every time to an unpredictable location

$ subs run npx cowsay hello
 _______
< hello >
 -------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## Overview

- Securely installs packages as root.
- Sources from multiple package managers and vendor provided packages.
- Can mix package managers when eg. npm packages need brew or vendor provided
  dependencies.
- Never touches /opt/homebrew.
- Dependencies are installed alongside; everything goes in the same
  self-contained prefix at `/opt/PKGNAME`.
- Installs as little as possible to /usr/local (ie. nothing from deps, only
  what you asked for, no jank).
- `subs run PKG` can run anything ephemerally (downloads fresh every time)

### Designed for the Era of Agents

I know what you’re thinking: this douchebag made Homebrew sudo-less and now
he’s making a new package manager that’s not only sudo but also has a name
that sounds like it should be a JavaScript library.

When I made Homebrew sudo-less, computers were pretty much single user and the
users were pretty much human. That’s not the case anymore. We are running
agents all over the place.

> Don’t let OpenClaw modify itself.

This is also why `subs run` is ephemeral and *always* downloads the latest
version. The download destination is unpredictable. The window for
expliotation is small but not zero. We want agents to be able to run anything
but we don’t want them to to potentially modify binaries as they are
downloading for who knows what nefarious reason. Then once an agent (or you)
has used this user-writable software it vanishes.

## Is This Ready For Me?

Look: I’m using it. But I’m also the one who made it.

I'm going to be honest with you. Substrate does some *mad tricks* to make
Homebrew bottles relocatable. There *will definitely be some issues*.

OTOH maybe that excites you? You’re the type who wants to get involved in
that?

## Caveats

- There is no `subs services` command. Use `brew`.
- Some Homebrew formulae are not supported. If you come across them, report
  this as a bug.
- We do not and will likely never support casks (use `brew`!)

## Technical Details

- For Homebrew packages we rewrite `/opt/homebrew/` in the binaries and any
  text files.
  - Indeed this may prove stupid and/or flakey
  - Indeed we do not recommend you depend on `subs` in any substantial way
- We are overzealously rejecting any Homebrew packages with pre or post
  install steps at this time.
