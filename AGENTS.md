# fx Agents Notes

These notes capture project-specific behavior that is easy to miss from
the code alone.

## Product Rules

- `x` is Homebrew-only.
- `i` checks compiled vendor packages first, then falls back to Homebrew.
- `i ffmpeg` intentionally installs `ffmpeg-full`.
- Real `i` installs must be run as root.

## Install Layout

- `x` installs ephemerally under `/tmp/x/<formula>`.
- `i` installs into `/opt/<package>`.
- `i` does not encode versions in the install directory name.
- `i` does not create `/opt/<package>` symlinks to versioned dirs.
- For `i`, Homebrew dependencies are merged into the main prefix rather than
  installed under `pkgs/`.
- When merged files conflict during `i` staging, later files overwrite
  earlier ones.
- For `i`, only the explicit package's executables should get stubs in
  `/usr/local/bin`.

## Stub Behavior

- Stub PATH entries are deduplicated.
- Only add an `sbin` PATH entry if that directory exists.
- For Homebrew `i` installs, the root executable set is tracked via
  `.xi-root-executables.json`.

## Homebrew Rules

- Homebrew path relocation still rewrites `/opt/homebrew/...` references.
- Homebrew formulas with `service` metadata are unsupported.
- Homebrew formulas with `post_install` are unsupported except:
  `openssl@3`, `ca-certificates`.
- Unsupported Homebrew formulas should fail with:
  `Unsupported formula: use \`brew install foo\``.

## Vendor Rules

- Vendor `version()` returns `Result<semver::Version, String>`.
- Vendor code owns version normalization such as stripping a leading `v`.
- Treat the vendor inventory as code, not documentation: list `./vendor`
  to see the current package modules, and check `src/vendor.rs` for the
  registry that is actually compiled in.
- When adding or removing a vendor package, keep `src/vendor.rs` and its
  tests in sync with the files under `vendor/`.

## Vendor Package Notes

- `gh` downloads GitHub release zips from `cli/cli`.
- `node` uses GitHub releases for version discovery from `nodejs/node`, but
  downloads tarballs from `https://nodejs.org/dist/...`.
- `node` currently exposes `node`, `npm`, and `npx`.
- `node` does not currently expose `corepack`.

## Local Tooling Notes

- In this environment, `cargo` may not be on `PATH`.
- If that happens, use `/Users/mxcl/.cargo/bin/cargo`.
