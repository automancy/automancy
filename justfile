dev: buildscript
    cargo run -p automancy --profile dev

staging: buildscript
    BUILD_PROFILE="staging" cargo run -p automancy --profile staging

release: buildscript
    BUILD_PROFILE="release" cargo run -p automancy --profile release

buildscript:
    cargo run -p build_script --release

sort:
    cargo sort --grouped --workspace