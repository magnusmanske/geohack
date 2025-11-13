# GeoHack
A Rust rewrite of the PHP GeoHack tool.

The PHP version has run into resource limits, and this rewrite aims to address those issues.

## Compatibility
The Rust code was based on the PHP code, and was, to a large degree, "translated" line by line. This was done to preserve compatibility, but results in some untypical Rust constructs. This will be rectified over time.

Besides various method tests, the Rust version also rests all 18 examples from the PHP version's `testcases.html` file. Input templates and expected HTML are part of this repo, and were manually verified to be identical to the PHP results (barring spacing, which does not matter in HTML rendering, and the occasional rounding artefact).

## Changed behaviour
To increase speed and reduce server load, the Rust version caches Wikipedia templates for up to 1 hour. This means that changes to the template may not show immediately.
- The `/sandbox` pages are never cached, as they serve a testing setup, and should always be live. Also, they do not create significant server load.
- Adding `&purge=1` to the URL will force an immediate cache refresh for the used template.
