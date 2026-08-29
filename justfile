set dotenv-load

skip_check := env('SKIP_CHECK', 'false')
tracy_feature := if env('AUTOMANCY_TRACY', 'false') == 'true' { "profile-with-tracy" } else { "" }

default:
    @just --choose

[private]
[no-exit-message]
cargo_cmd cmd profile:
    -BUILD_PROFILE='{{profile}}' cargo '{{cmd}}' --package=automancy --profile='{{profile}}' --features '{{tracy_feature}}'

buildscript:
    @echo 'Running build script.'
    SKIP_CHECK='{{skip_check}}' cargo run --package=build_script --release

build profile="dev": buildscript
    @echo
    @echo "Building profile '{{profile}}'."
    just cargo_cmd build '{{profile}}'

run profile="dev": buildscript
    @echo
    @echo "Running profile '{{profile}}'."
    just cargo_cmd run '{{profile}}'

flamegraph profile="dev": buildscript
    @echo
    @echo "Running profile '{{profile}}'."
    just cargo_cmd flamegraph '{{profile}}'

miri run profile="dev": buildscript
    @echo
    @echo "Running profile '{{profile}}'."
    just cargo_cmd miri '{{profile}}'

sort:
    cargo sort --grouped --workspace

license:
    cargo about -L debug generate --workspace about.hbs -o README-LICENSE.html
