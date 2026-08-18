# justfile

export RUSTFLAGS := '-D warnings'
export RUSTDOCFLAGS := '-D warnings'

MUTE := justfile_directory() + '/scripts/mute'
CARGO := 'cargo --quiet'
CARGO_NIGHTLY := 'cargo +nightly --quiet'

# Make all recipes quiet by default since `mute` will print commands when necessary.
set quiet := true

# NOTE: Dependencies are ordered by execution speed: faster checks (formatting, static lints) run first.
[default]
[doc('Comprehensive workspace checks: formatting check, clippy, doc, and tests')]
[group('verification')]
verify: lint doc test

[doc('Same as `verify` but with coverage tests')]
[group('verification')]
verify-full: lint doc test coverage

[doc('Run code formatting, and clippy checks')]
[group('style')]
lint: fmt clippy

[doc('Format the Rust code specified in args using nightly rustfmt')]
[group('style')]
fmt +args='--all':
    {{ MUTE }} {{ CARGO_NIGHTLY }} fmt {{ args }}

[doc('Check formatting without modifying files')]
[group('style')]
fmt-check +args='--all':
    {{ MUTE }} {{ CARGO_NIGHTLY }} fmt --check {{ args }}

[doc('Run clippy on all targets, denying warnings')]
[group('style')]
clippy +args='--workspace --all-targets':
    {{ MUTE }} {{ CARGO }} clippy {{ args }}

[doc('Checks that the code compiles')]
[group('test')]
check +args='--workspace --all-targets':
    {{ MUTE }} {{ CARGO }} check {{ args }}

[doc('Run tests on the workspace by default, or individual targets')]
[group('test')]
test +args='--workspace':
    {{ MUTE }} {{ CARGO }} test --quiet --lib --tests {{ args }}

nextest_args := "--status-level fail --show-progress none --no-output-indent --cargo-quiet"
[doc('Run tests using nextest')]
[group('test')]
nextest +args='--workspace':
    {{ MUTE }} cargo nextest run {{ nextest_args }} --lib --tests {{ args }}

export COVERAGE_TARGET := "80"
coverage_args := '--quiet'

[doc('Test coverage greater than {{ COVERAGE_TARGET }}%')]
[group('test')]
coverage +args='--workspace':
    #!/bin/zsh
    {{ MUTE }} {{ CARGO }} llvm-cov {{ coverage_args }} \
        --fail-under-lines {{ COVERAGE_TARGET }} \
        --text \
        --output-path target/llvm-cov-target/coverage.txt \
        test {{ args }}
    exit_status=$?
    if [ $exit_status -ne 0 ]; then
        print
        print -P "    %F{red}%BCoverage failed!%b%f"
        print -P "    You can view the report with: %Bjust coverage-report%b"
        print
        exit $exit_status
    else
        print -P "    %F{green}Coverage passed!%f Report: %Bjust coverage-report%b"
    fi

[group('test')]
coverage-report compact='':
    #!/bin/zsh
    B="$(print -f "%s" -P "%B")"
    b="$(print -f "%s" -P "%b")"
    print -P "%BCoverage Report:%b"
    {{ CARGO }} llvm-cov {{ coverage_args }} report --show-missing-lines \
        | gawk '
        function emoji(coverage) {
            if (coverage >= target) return "✅"
            if (coverage >= 70) return "🟡"
            return "🔴"
        }
        BEGIN {
            target = {{ COVERAGE_TARGET }}
            compact = match("{{ compact }}", /--compact/)
        }
        /^-+$/ {
            section=1
            next
        }
        /^Uncovered Lines:$/ {
            section=2
            next
        }
        section==1 {
            line_coverage[$1]=$10
        }
        section==2 {
            gsub(/'${PWD//\//\\\/}'\/|^\/|:/, "", $1)
            cov = line_coverage[$1] + 0.0
            printf "%s  %.2f%%\t%s\n", emoji(cov), cov, $1
            if (compact) next
            printf "\n"
            gsub(/,/, "", $2)
            start = prev = $2
            for (i = 3; i <= NF; i++) {
                sub(/,/, "", $i)
                if ($i == prev + 1) {
                    prev = $i
                } else {
                    printf "%s, ", (start == prev ? start : start "-" prev)
                    start = prev = $i
                }
            }
            printf "%s\n\n", (start == prev ? start : start "-" prev)
        }
        END {
            total = line_coverage["TOTAL"] + 0.0
            print "'$B'TOTAL: ", total, " (", emoji(total), ")'$b'"
        }'

[doc('Build according to arguments')]
[group('build')]
build +args='--workspace':
    {{ MUTE }} {{ CARGO }} build {{ args }}
    [ -f target/debug ] && ln -sfh debug target/latest-build

[doc('Build release binaries optimized for size')]
[group('build')]
release +args='--workspace':
    {{ MUTE }} {{ CARGO_NIGHTLY }} -Zbuild-std-features=panic_immediate_abort build {{ args }} --bins --release
    [ -f target/release ] && ln -sfh release target/latest-build

[doc('Build documentation, treating warnings as errors')]
[group('build')]
doc +args='--no-deps':
    {{ MUTE }} {{ CARGO }} doc {{ args }}
