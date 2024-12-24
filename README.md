# DeArrow Browser
An explorer for the [DeArrow](https://dearrow.ajay.app/) database as a web application.
Inspired by [Lartza's SBbrowser](https://github.com/Lartza/SBbrowser).

Public instance available at [dearrow.minibomba.pro](https://dearrow.minibomba.pro/).

This repository is split into 4 main crates:
- dearrow-parser - definitions of structures in source .csv files, reading & merging those into a single structure per detail
- dearrow-browser-server - the backend, keeps the database loaded in memory, provides data for the backend
- dearrow-browser-api - definitions of API structures
- dearrow-browser-frontend - the frontend, uses Yew, functions as a single page application

This repository previously hosted 1 utility crate:
- error_handling - basically the core of anyhow, written from scratch to make errors cloneable. Renamed, moved to [it's own repository](https://github.com/mini-bomba/cloneable_errors) and published on [crates.io as `cloneable_errors`](https://crates.io/crates/cloneable_errors)

## SponsorBlockServer emulation
DeArrow Browser can emulate a limited set of SponsorBlockServer endpoints, specifically used by the DeArrow extension, allowing it to be used as an API mirror.<br>
Emulation must be enabled in `config.toml` by setting `enable_sbserver_emulation` to `true`.<br>
Emulated endpoints live under the `/sbserver` path. You can use them in the extension by setting the API URL to `https://<your dab domain>/sbserver` (`https://dearrow.minibomba.pro/sbserver` for the main instance)

**Voting/Submission endpoints are not and will not be supported!** - support for redirecting these needs to be added in the extension itself, **sending votes/submissions to mirrors leaks your private ID**.

Supported endpoints:
- `GET /api/branding` and `GET /api/branding/:sha256HashPrefix`
  - `randomTime` and `videoDuration` fields appear to be different on videos with segments without video duration info
    - appears to work on new videos tho
- `GET /api/userInfo`
  - `userID` param (lookup by private id) not supported
  - `value(s)` params ignored completely
    - endpoint will always return `userID`, `userName`, `vip`, `titleSubmissionCount`, `thumbnailSubmissionCount`, `warnings`, `warningReason` and `deArrowWarningReason` fields.

Unsupported endpoints:
- `POST /api/branding`
  - will not be supported, needs a solution in the extension
  - sending votes/submissions to mirrors leaks your private ID

## Starting a development server
To run a local development server without docker, you'll need:
- cargo
- trunk (cargo install trunk)

1. Grab a copy of the DeArrow database from a mirror of choice. Required files:
  - `thumbnails.csv`
  - `thumbnailTimestamps.csv`
  - `thumbnailVotes.csv`
  - `titles.csv`
  - `titleVotes.csv`
  - `userNames.csv`
  - `vipUsers.csv`
  - `sponsorTimes.csv`
  - `warnings.csv`
2. Build the frontend:
  - `trunk build` in the `dearrow-browser-frontend` directory to make a one-time build
  - `trunk watch` in the `dearrow-browser-frontend` directory to rebuild every time source files are updated
3. Build & start the server
  - `cargo run --bin dearrow-browser-server` in the root project dir, or
  - `cargo run` in the `dearrow-browser-server` directory
  - optionally use something like `cargo-watch` to rebuild on source file changes

## Building the container image
The main `Dockerfile` requires a custom "builder base" image defined in `builder_base.Dockerfile`.
This helps cache some layers in the builder stage that are less commonly changed, even when the `image prune` command is issued after building.

To build the main image:
1. Build the helper "builder base" image and tag it as `dearrow-browser:builder-base`
```sh
docker build -f builder_base.Dockerfile -t dearrow-browser:builder-base .
```
2. Build the main image
```sh
docker build -t dearrow-browser .
```

## Running an instance
1. Build the image (see above)
2. Create a config.toml file. Static content (frontend) is available at /static in the container.
3. Run the container
```sh
docker run -h dearrow-browser --name dearrow-browser -v <path to mirror>:/mirror -v <path to config.toml>:/config.toml:ro -p 9292 dearrow-browser
```

If you've got a proper mirror set up (instead of manually sourced .csv files), make it make a POST request to `/api/reload` with the auth secret as the `auth` URL parameter to reload the database.
DeArrow Browser should remain usable while the database is reloaded. (assuming we don't run out of RAM)

## Note about the internal API crate
The API provided by `dearrow-browser-server` and used by `dearrow-browser-frontend` is considered to be internal.

While API structures are publicly defined in the `dearrow-browser-api` crate (which can be used in other projects), breaking changes may be made to the API at any time with no backwards compatibility and without a major version number change.

The `dearrow-browser-api` crate provides `sync` (threadsafe, `Arc<>` based), `unsync` (not threadsafe, `Rc<>` based), `boxed` (`Box<>` based) and `string` (`String` based) implementations of the API structures.
These implementations can be enabled or disabled using respective features and are available in separate modules.
The `sync` implementation is enabled by default.

Any errors from the API will be returned as human-readable plaintext unless the client had explicitly requested `application/json` as one of the accepted formats.
If a client explicitly requests `application/json` by including it in the `Accept` request header (`*/*` does not count), any errors will be sent as json-encoded `SerializableError` from the `error_handling` crate.
All endpoints will always return json on success, even if the client requests a different format.

## Credits

The DeArrow Browser logo is a combination of the DeArrow logo (which is based on Twemoji) and the magnifying glass emoji from [Twemoji](https://github.com/twitter/twemoji) and is licensed under the [CC-BY 4.0 license](https://github.com/mini-bomba/DeArrowBrowser/blob/master/dearrow-browser-frontend/icon/LICENSE-CC-BY-4.0.txt).

Images located in the [`/dearrow-browser-frontend/icon/`](https://github.com/mini-bomba/DeArrowBrowser/tree/master/dearrow-browser-frontend/icon) directory are either based on or directly copied icons from [Twemoji](https://github.com/twitter/twemoji) and are licensed under the [CC-BY 4.0 license](https://github.com/mini-bomba/DeArrowBrowser/blob/master/dearrow-browser-frontend/icon/LICENSE-CC-BY-4.0.txt).

The DeArrow Browser source code is licensed under the [GNU Affero General Public License version 3](https://github.com/mini-bomba/DeArrowBrowser/blob/master/LICENSE).

© mini_bomba & [contributors](https://github.com/mini-bomba/DeArrowBrowser/graphs/contributors) 2023-2024

