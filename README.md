# GeoHack
A Rust rewrite of the [PHP GeoHack tool](https://geohack.toolforge.org).

The PHP version has run into resource limits, and this rewrite aims to address those issues.

## Installation
- [Install Rust](https://rust-lang.org/tools/install/)
- Clone this repository
- Run `cargo build --release`

## Compatibility
The Rust code was based on the PHP code, and was, to a large degree, "translated" line by line. This was done to preserve compatibility, but results in some untypical Rust constructs. This will be rectified over time.

Besides various method tests, the Rust version also rests all 18 examples from the PHP version's `testcases.html` file. Input templates and expected HTML are part of this repo, and were manually verified to be identical to the PHP results (barring spacing, which does not matter in HTML rendering, and the occasional rounding artefact).

The `region.php` file has not been ported yet. It relies on PHP include files and a database, both of which seem to not exist. It is unclear whether it is still used or not.

## Improvements
- Self-contained web server based on `axum`.
- No dependency on a file system (eg `nfs`) or database. All file-like data is included in the binary at compile time.
- Rust speed and memory safety.
- Caching of templates to reduce server load and improve performance.

## Changed behaviour
To increase speed and reduce server load, the Rust version caches Wikipedia templates for up to 1 hour. This means that changes to the template may not show immediately.
- The `/sandbox` pages are never cached, as they serve a testing setup, and should always be live. Also, they do not create significant server load.
- Adding `&purge=1` to the URL will force an immediate cache refresh for the used template.
