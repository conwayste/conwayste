# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    # XXX run in docker: sudo apt-get -yq --no-install-suggests --no-install-recommends install libudev-dev libasound2-dev
    if [[ "$TARGET" = *linux* ]]; then
        cross build --target $TARGET --release
    else
        # don't attempt to build the dissector for windows/darwin (missing dependencies)
        cross build --target $TARGET --release --workspace --exclude dissect-netwayste
    fi

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    # TODO: once submodules for openbsd support are removed (PR #145) can delete exclusions below
    EXCLUSIONS="--exclude ggez --exclude rodio"
    if [[ "$TARGET" != *linux* ]]; then
        # don't attempt to build the dissector for windows/darwin (missing dependencies)
        EXCLUSIONS="$EXCLUSIONS --exclude dissect-netwayste"
    fi
    cross test --target $TARGET --release --workspace $EXCLUSIONS
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi

# vim:set ts=4 sw=4 softtabstop=4 et:
