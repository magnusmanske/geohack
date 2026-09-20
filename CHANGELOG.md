# Changelog

## Unreleased

### Performance
- Templates are now served stale-while-revalidate. Previously the first request
  after the 1-hour TTL expired had to wait for the upstream wiki, measured at
  1.4-3.1 s; that request is now answered from cache in ~2 ms while a background
  task fetches the new copy. A stale copy is served for up to 24 hours if the
  wiki is unreachable, instead of returning HTTP 500.
- Background refreshes are capped at 4 concurrently, so the whole cache going
  stale at once (entries populated by one traffic burst also expire together)
  cannot stampede the wikis.
- Per-request CPU for the warm path dropped ~18% (736 us -> 605 us for the
  English template) by removing redundant copies of the ~200 kB page: the
  intermediate page is no longer stored on `GeoHack`, the three MediaWiki
  fixups run as one pass instead of three, the `{nztm*}` substitutions moved
  into the existing replacement map, body extraction trims in place, and
  `process` no longer clones the output when no `region:` is given.

## 0.1.1 - 2026-07-28

### Security
- Fixed a remote-triggerable panic (whole-process abort in release builds) on
  multi-byte UTF-8 in `region:` values.
- User-supplied `params`, `pagename`, and `title` now have quotes escaped,
  closing an attribute-injection XSS in `href="..."` contexts.
- The `project` parameter is URL-encoded before being placed in template
  fetch URLs, preventing query injection.
- Template cache is now bounded (100 entries), preventing memory exhaustion
  via arbitrary cache keys.

### Added
- Dark mode, following the OS/browser preference
  (`prefers-color-scheme`). Light mode is unchanged. (#3)

### Fixed
- Static assets were served with a duplicated, malformed `Content-Type`
  (`application/octet-stream,image/png`): `AppendHeaders` adds a second value
  instead of replacing the one axum derives from the body. `main.css` now also
  declares `charset=utf-8`.
- The celestial-body logo in the sidebar is no longer a black rectangle:
  Wikimedia now rejects hotlinked thumbnails whose width is not a standard
  step (<https://w.wiki/GHai>), and every URL in `data/logos.json` used 150px.
  All are now 250px, scaled down via `background-size:contain` to keep the
  previous 150px display size. Three stale Commons paths (`sun`, `titania`,
  `puck`) and the OpenStreetMap wiki logo URL were dead as well and have been
  updated; all 44 URLs verified to return 200.
- The Wikimedia Cloud Services logo has a black wordmark, which was
  illegible on the dark background introduced in this release. It now gets a
  light backing in dark mode.
- Wikis without their own `Template:GeoTemplate` now fall back to the English
  template instead of rendering MediaWiki's "page does not exist" body: a 404
  response was being used as the template. (#4)
- OSGB36 output now applies the official OSTN15 datum transformation
  (WGS84 → OSGB36) via the `lonlat_bng` crate. Previous values, inherited
  from the PHP original, skipped the datum shift and were ~100 m off across
  the UK. Coordinates outside Great Britain now yield `0`/empty instead of
  meaningless values.
- `{geoa1}` returns the actual subdivision code (`NY` for `region:US-NY`);
  it was off by one (a faithfully ported PHP bug).
- The default title (when `&title=` is absent) and `{params}` are no longer
  double-escaped.
- Invalid user input returns HTTP 400 with a brief message instead of a
  blank 500.

### Changed
- UTM projections delegated to the `utm` crate (output identical to sub-mm);
  hand-rolled projection code retained only for CH1903.
- Template cache replaced with `moka`: 1 h TTL, bounded, coalesces concurrent
  fetches, and shares cached templates without per-request copies.
- Templates are fetched over HTTPS.
- Server errors are logged via `tracing`.

## 0.1.0

- Initial Rust port of the PHP GeoHack tool.
