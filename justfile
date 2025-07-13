dev_build: buildscript
    cargo build -p automancy --profile dev

dev: buildscript
    cargo run -p automancy --profile dev

staging_build: buildscript
    BUILD_PROFILE="staging" cargo build -p automancy --profile staging

staging: buildscript
    BUILD_PROFILE="staging" cargo run -p automancy --profile staging

release_build: buildscript
    BUILD_PROFILE="release" cargo build -p automancy --profile release

release: buildscript
    BUILD_PROFILE="release" cargo run -p automancy --profile release

buildscript:
    cargo run -p build_script --release

sort:
    cargo sort --grouped --workspace

license:
    cargo about generate --workspace about.hbs -o README-LICENSE.html
