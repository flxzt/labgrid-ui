<!--
SPDX-FileCopyrightText: 2025 Duagon Germany GmbH

SPDX-License-Identifier: CC0-1.0
-->

# labgrid-ui developing

## Architecture

The code is split into the following parts:
- `core` : core code that implements a gRPC client for communicating with the labgrid coordinator.
    only has minimal dependencies, is completely free of UI specific code and should be treated as it's own
    independent library.
- `testcli` : ad-hoc coded CLI that takes `core` as dependency to test it's functionality.
- `ui` : the UI itself. Takes `core` as dependency and utilizes the iced UI toolkit.
    It should be kept as rust-only crate so that it can be compiled into a single binary
    without additional external runtime dependencies.
    It provides some additional files for desktop integration that are located
    in `crates/ui/data`. Notably:
    - `fonts` : folder containing fonts that are embedded into the binary.
    - `icons` : folder containing icons used by the desktop file and also by the UI itself.
        Needed for the flatpak build.
    - `com.duagon.labgrid-ui.desktop` : file that gets installed to the `applications` directory
        so that the app appears in app launchers and desktop environments.
        Needed for the flatpak build.
    - `com.duagon.labgrid-ui.metainfo.xml` : XDG AppData file that gets installed to the `metainfo` directory
        so that info about the app appears in app stores.
        Needed for the flatpak build.
    Translation files are located in `crates/ui/i18n` (i18n stands for internationalization).
    They are also directly embedded into the binary.

### UI

The UI uses [iced](https://github.com/iced-rs/iced) as UI-Toolkit. It is inspired by the [Elm-Architecture](https://guide.elm-lang.org/architecture/)
which essentially separates app state from the declaration of UI elements and uses message-passing for updates.

Check out iced's [book](https://book.iced.rs/architecture.html) for detailed explanation of the toolkit.

## Translations

For translations [fluent](https://projectfluent.org/) is used.
It uses a different architecture than what developers might be used to with 'gettext'.
Instead of a english reference string which is the basis for additional languages, key-value pairs are used.

For a complete list of used keys check `crates/ui/i18n/en-US/labgrid_ui.ftl`, it is the fallback language.
Note that the values also can contain variables/placeholders set dynamically by the application,
denoted by syntax `{$variable}`.
If the fallback langauge contains a variable in its translation string,
new translations must use them in the same way as well.

When adding translations to a language or add a new language you should take the fallback language as reference,
adding missing key-value pairs ideally in the same order to make comparisons easier.

## Prerequisites

First the [Rust](https://www.rust-lang.org/) programming language must be installed.
Installation Instructions can be found on the website.

For development the [just](https://just.systems/) command runner is used.
It should be available through package managers of the latest distros, but can also be installed through cargo.
The recipes are defined in `justfile`.

Then, execute:

```bash
just prerequisites-dev
```

which installs necessary build-time dependencies (such as the protobuf compiler) and sets up the environment
for developing like installing the flatpak SDK and extensions and adding a code-style pre-commit hook.

## Building

Execute:

```bash
just build
```

To build all binary crates in the repo. You can also specify building only a specific crate
by passing it as additional argument to cargo.
For example, to only build the ui:

```bash
just build -p labgrid-ui
```

You can of course also invoke cargo directly yourself.

### Installation

To install the UI execute:

```bash
just install-ui
```

This install the binary to `.cargo/bin/` which should be in your `PATH` after installing rust.
It also install the app icon to `.local/share/icons/`, the desktop file to `.local/share/applications/`
and the AppData to `.local/share/metainfo` which should make the app appear in app launchers.

### Flatpak

Install prerequisites needed for flatpak builds:

```bash
just prerequisites-flatpak
```

To build the flatpak execute:

```bash
just build-ui-flatpak
```

It builds the flatpak bundle `com.duagon.labgrid-ui.flatpak` and places it directly into the repository root directory. 

## Debugging

To get more verbose logs while running the application, use environment variable `RUST_LOG=<crate-name>=<log-level>`.
So for example for the UI and with the `debug` log level this becomes `RUST_LOG=labgrid_ui=debug`.
Note that dashes must be replaced by underscores.
To get more verbose logs for all dependencies that also use `RUST_LOG`
(`tracing_subscriber` or `logging`) emit the `=<crate-name>` part, so for example `RUST_LOG=debug`.

For debugging in VSCode [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
can be used.

## Pre-Commit hooks

Pre-commit hooks will be installed when running `just prerequisites-dev`
Make sure to follow the outlined code-style from above to make them pass.
But as an escape-hatch circumvent the pre-commit hooks by passing the `--no-verify` flag:

```bash
git commit -m <message> --no-verify
```

## Docs

To build and launch docs for the UI crate execute:

```bash
just docs-ui-open
```

The same docs will be hosted through gitlab pages and the README will be updated with a link to them.
To view docs of released crate dependencies, navigate to [docs.rs](docs.rs).
To build and view docs of unreleased crate dependencies (e.g. `iced`'s github development version) use:

```bash
just docs-open
```

## Licensing

Licensing compliancy checks always runs in the git pre-commit hook and CI to ensure compliancy at all times.

To check it manually run

```bash
just licensing-check
```

To check the licenses of all dependencies run

```bash
just licensing-dependencies-check
```

To annotate a single file with a SPDX license identifier, use

```bash
just licensing-annotate-owned <file>
```

If this command recognizes a file format incorrectly or the licensing information should not be inlined,
pass argument `--force-dot-license`.
To overwrite the used license (for example `CC0-1.0` should be used for text files) execute the recipe like this

```bash
just license=<spdx-license-identifier> licensing-annotate-owned <file>
```

For external file with certain licenses and copyright holders use

```bash
just licensing-annotate-external <file> -c <copyright-holder> -l <spdx-license-identifier>
```

To quickly annotate all *.rs and *.py files in the repository use

```bash
find . ! -path "./target/*" -name *.rs -o *.py -exec just licensing-annotate-owned {} \;
```
