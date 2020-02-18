# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    # TODO: verify this works
    if which apt-get > /dev/null; then
        # this is Ubuntu
        sudo -E apt-get -yq --no-install-suggests --no-install-recommends $(travis_apt_get_options) install libudev-dev libasound2-dev
    fi

    cross build --target $TARGET
    cross build --target $TARGET --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    cross test --target $TARGET
    cross test --target $TARGET --release
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi

# vim:set ts=4 sw=4 softtabstop=4 et:
