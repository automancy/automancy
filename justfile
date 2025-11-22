set dotenv-load

default:
    @just --choose

buildscript:
    @echo 'Running build script.'
    cargo run --package=build_script --release

build profile="dev": buildscript
    @echo
    @echo "Building profile '{{profile}}'."
    BUILD_PROFILE='{{profile}}' cargo build --package=automancy --profile={{profile}}

run profile="dev": buildscript
    @echo
    @echo "Running profile '{{profile}}'."
    BUILD_PROFILE='{{profile}}' cargo run --package=automancy --profile={{profile}}

sort:
    cargo sort --grouped --workspace

license:
    cargo about generate --workspace about.hbs -o README-LICENSE.html
