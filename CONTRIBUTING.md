# Contributing

We love pull requests from everyone. By participating in this project, you agree to abide by the [Contributor Covenant] Code of Conduct, version 1.4.

Fork, then clone the repository:

`$ git clone git@github.com:your-username/conwayste.git`

Build the code as is to ensure it is not broken out the gate:

`$ cargo build`

Make sure the tests pass:

`$ cargo test`

Make your change. Add tests for your change. Make the tests pass:

`$ cargo build`
`$ cargo test`

Make sure no new warnings are introduced as a result of your change.

Run `rust-format` on your changes before submitting a PR.

Push to your fork and [submit a pull request][pr].

[pr]: https://github.com/conwayste/conwayste/compare/

At this point you're waiting on us. We like to at least comment on pull requests within three-to-five business days (and, typically, one business day). We may suggest some changes, improvements, or alternatives.

Some things that will increase the chance that your pull request is accepted:

* Write tests.
* Follow our [style guide][style] by simply running `rust-format`.
* Write a [good commit message][commit].

[commit]: http://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html
