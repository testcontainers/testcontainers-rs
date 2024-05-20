# Contributing to documentation

The Testcontainers for Rust documentation is a static site built with [MkDocs](https://www.mkdocs.org/).
We use the [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/) theme, which offers a number of useful extensions to MkDocs.

In addition we use a [custom plugin](https://github.com/rnorth/mkdocs-codeinclude-plugin) for inclusion of code snippets.

We publish our documentation using Netlify.

## Previewing rendered content

### Using Python locally

* Ensure that you have Python 3.8.0 or higher.
* Create a Python virtualenv. E.g. `python3 -m venv tc-venv`.
* Activate the virtualenv. E.g. `source tc-venv/bin/activate`.
* Run `pip3 install -r requirements.txt && ./tc-venv/bin/mkdocs serve` from the `testcontainers-rs` root directory. It will start a local auto-updating MkDocs server.

### PR Preview deployments

Note that documentation for pull requests will automatically be published by Netlify as 'deploy previews'.
These deployment previews can be accessed via the `deploy/netlify` check that appears for each pull request.
