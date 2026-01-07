# justfile for labgrid-ui

ci := "false"
cargo_profile := "dev"
flatpak_build_dir := "flatpak-build"
flatpak_repo_dir := "flatpak-repo"
license := "GPL-3.0-or-later"

[private]
sudo_cmd := if ci == "true" {
    ""
} else {
    "sudo"
}
[private]
nextest_ci_args := if ci == "true" {
    "--profile ci"
} else {
    ""
}
[private]
linux_distr := `lsb_release -ds | tr '[:upper:]' '[:lower:]'`
[private]
cargo_out_profile := if cargo_profile == "dev" { "debug" } else { cargo_profile }

default:
    just --list

prerequisites:
    #!/usr/bin/env bash
    set -euxo pipefail
    if [[ '{{linux_distr}}' =~ 'fedora' ]]; then
        {{sudo_cmd}} dnf install -y gcc protobuf-compiler protobuf-devel vulkan-loader curl git
    elif [[ '{{linux_distr}}' =~ 'debian' ]] || [[ '{{linux_distr}}' =~ 'ubuntu' ]]; then
        {{sudo_cmd}} apt-get update
        {{sudo_cmd}} apt-get install -y build-essential protobuf-compiler libprotobuf-dev libvulkan1 curl git
    else
        echo "Can't install system dependencies, unsupported distro."
        exit 1
    fi
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"

prerequisites-flatpak:
    #!/usr/bin/env bash
    set -euxo pipefail
    if [[ '{{linux_distr}}' =~ 'fedora' ]]; then
        {{sudo_cmd}} dnf install -y flatpak flatpak-builder appstream
    elif [[ '{{linux_distr}}' =~ 'debian' ]] || [[ '{{linux_distr}}' =~ 'ubuntu' ]]; then
        {{sudo_cmd}} apt-get update
        {{sudo_cmd}} apt-get install -y flatpak flatpak-builder appstream
    else
        echo "Can't install system dependencies, unsupported distro."
        exit 1
    fi
    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak install --user -y \
        org.freedesktop.Platform//25.08 \
        org.freedesktop.Sdk//25.08 \
        org.freedesktop.Sdk.Extension.rust-stable//25.08

prerequisites-dev: prerequisites
    #!/usr/bin/env bash
    set -euxo pipefail
    if [[ '{{linux_distr}}' =~ 'fedora' ]]; then
        {{sudo_cmd}} dnf install -y python3 uv
    elif [[ '{{linux_distr}}' =~ 'debian' ]] || [[ '{{linux_distr}}' =~ 'ubuntu' ]]; then
        {{sudo_cmd}} apt-get update
        {{sudo_cmd}} apt-get install -y python3
    else
        echo "Can't install system dependencies, unsupported distro."
        exit 1
    fi
    export PATH="$HOME/.cargo/bin:$PATH"
    curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    cargo binstall -y cargo-nextest cargo-edit cargo-deny cargo-cyclonedx
    if [[ {{ci}} != "true" ]]; then
        ln -sf auxiliary/git-hooks/pre-commit.hook .git/hooks/pre-commit
    fi

clean:
    cargo clean
    rm -rf {{flatpak_build_dir}}
    rm -rf {{flatpak_repo_dir}}

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

check:
    cargo check

lint:
    cargo clippy

test *NEXTEST_ARGS:
    cargo nextest run --workspace --no-tests warn {{nextest_ci_args}} {{NEXTEST_ARGS}}

build *ARGS:
    cargo build --profile {{cargo_profile}} {{ARGS}}

install-ui:
    cargo install --profile {{cargo_profile}} --path crates/ui
    mkdir -p ~/.local/share/metainfo || true
    mkdir -p ~/.local/share/applications || true
    mkdir -p ~/.local/share/icons || true
    install -Dm644 ./crates/ui/data/com.duagon.labgrid-ui.metainfo.xml -t ~/.local/share/metainfo/
    install -Dm755 ./crates/ui/data/com.duagon.labgrid-ui.desktop -t ~/.local/share/applications/
    install -Dm644 ./crates/ui/data/icons/com.duagon.labgrid-ui.svg -t ~/.local/share/icons/

install-scripts:
    install -Dm755 ./scripts/*.py ~/.local/share/labgrid-ui/scripts/

uninstall-ui:
    rm ~/.local/share/metainfo/com.duagon.labgrid-ui.metainfo.xml || true
    rm ~/.local/share/applications/com.duagon.labgrid-ui.desktop || true
    rm ~/.local/share/icons/com.duagon.labgrid-ui.svg || true

run-ui *ARGS:
    cargo run --profile {{cargo_profile}} -p labgrid-ui -- {{ARGS}}

build-ui-flatpak:
    flatpak-builder --force-clean --repo={{flatpak_repo_dir}} {{flatpak_build_dir}} auxiliary/com.duagon.labgrid-ui.yaml
    flatpak build-bundle {{flatpak_repo_dir}} com.duagon.labgrid-ui.flatpak com.duagon.labgrid-ui

run-testcli *ARGS:
    cargo run --profile {{cargo_profile}} -p labgrid-ui-testcli -- {{ARGS}}

deploy-ui-remote target:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo build --profile release
    tmpdir=$(ssh {{target}} "mktemp -d")
    scp target/release/labgrid-ui "{{target}}:${tmpdir}/labgrid-ui"
    ssh -t {{target}} "sudo mv \"${tmpdir}/labgrid-ui\" /usr/local/bin/labgrid-ui"

# prepares the ui docs for deployment by CI with gitlab pages
docs-ui-prepare:
    cargo doc -p labgrid-ui
    mkdir -p public
    cp -r target/doc/* public/.
    echo '<meta http-equiv="refresh" content="0; url=labgrid_ui">' > public/index.html

docs-ui-open:
    cargo doc -p labgrid-ui --open

docs-open package:
    cargo doc -p {{package}} --open

check-outdated-dependencies:
    cargo upgrade --dry-run -vv

licensing-check:
    uvx reuse lint

[doc('Annotates a source file with the copyright holders and licensing information found in the `COPYRIGHT_HOLDERS` file
and the `license` variable respectively.
To use a different license for a single file, invoke this recipe with
`just license=<spdx-license-identifier> licensing-annotate <file>`.
If the file format is not recognized or it is not desired to write inline license information comments,
supply `--force-dot-license` to write a `<file>.license` license file')]
licensing-annotate-owned file *REUSE_ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t copyright_holders < COPYRIGHT_HOLDERS
    cmd_array=( uvx reuse annotate )
    cmd_array+=( --merge-copyrights )
    for elem in "${copyright_holders[@]}"; do
        cmd_array+=( -c "$elem" )
    done
    cmd_array+=( -l {{license}} )
    echo "executing \"${cmd_array[@]@Q}\" {{REUSE_ARGS}} {{file}}"
    "${cmd_array[@]}" {{REUSE_ARGS}} {{file}}

[doc('Annotates an external file with information supplied by `REUSE_ARGS`.
To pass author information, use argument `-c <author> (can be repeated, author must be enclosed in escaped quotes),
to specify the license use `-l <spdx-license-identifier>')]
licensing-annotate-external file *REUSE_ARGS:
    #!/usr/bin/env bash
    set -euxo pipefail
    uvx reuse annotate --merge-copyrights {{REUSE_ARGS}} {{file}}

licensing-dependencies-check:
    cargo deny check --hide-inclusion-graph

# as soon as JSON output of the `reuse spdx` command is available (see: https://github.com/fsfe/reuse-tool/issues/1164)
# a converter tool like `cyclonedx-cli` could be used to convert to CycloneDX format
[doc('Export licensing document of the project itself in SPDX format')]
licensing-doc-export path="./project.spdx":
    uvx reuse spdx -o {{path}} 

# Export SBOMs of project dependencies in CycloneDX format
sbom-export:
    cargo cyclonedx
