# DeArrow Browser
An explorer for the [DeArrow](https://dearrow.ajay.app/) database as a web application.
Inspired by [Lartza's SBbrowser](https://github.com/Lartza/SBbrowser).

Public instance available at [dearrow.minibomba.pro](https://dearrow.minibomba.pro/).

This repository is split into 4 crates:
- dearrow-parser - definitions of structures in source .csv files, reading & merging those into a single structure per detail
- dearrow-browser-server - the backend, keeps the database loaded in memory, provides data for the backend
- dearrow-browser-api - definitions of API structures
- dearrow-browser-frontend - the frontend, uses Yew, functions as a single page application

The logo is a combination of the DeArrow logo and the magnifying glass emoji from [twemoji](https://github.com/twitter/twemoji)

## SponsorBlockServer emulation
DeArrow Browser can emulate a limited set of SponsorBlockServer endpoints, specifically used by the DeArrow extension, allowing it to be used as an API mirror.
Emulation must be enabled in `config.toml` by setting `enable_sbserver_emulation` to `true`.
Emulated endpoints live under the `/sbserver` path. You can use them in the extension by setting the API URL to `https://<your dab domain>/sbserver` (`https://dearrow.minibomba.pro/sbserver` for the main instance)
**Voting/Submission endpoints are not and will not be supported!** - support for redirecting these needs to be added in the extension itself, **sending votes/submissions to mirrors leaks your private ID**.

Supported endpoints:
- `GET /api/branding` and `GET /api/branding/:sha256HashPrefix`
  - `videoDuration` field not implemented (yet)
  - `randomTime` field might be different than server by the main server if SponsorBlock segments exist (to be fixed)
- `GET /api/userInfo`
  - `userID` param (lookup by private id) not supported
  - `value(s)` params ignored completely
    - endpoint will always return `userID`, `userName`, `vip`, `titleSubmissionCount` and `thumbnailSubmissionCount` fields.

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
2. Build the frontend:
  - `trunk build` in the `dearrow-browser-frontend` directory to make a one-time build
  - `trunk watch` in the `dearrow-browser-frontend` directory to rebuild every time source files are updated
3. Build & start the server
  - `cargo run --bin dearrow-browser-server` in the root project dir, or
  - `cargo run` in the `dearrow-browser-server` directory
  - optionally use something like `cargo-watch` to rebuild on source file changes

## Running an instance
1. Build the image
```sh
docker build -t dearrow-browser .
```
2. Create a config.toml file. Static content (frontend) is available at /static in the container.
3. Run the container
```sh
docker run -h dearrow-browser --name dearrow-browser -v <path to mirror>:/mirror -v <path to config.toml>:/config.toml:ro -p 9292 dearrow-browser
```

If you've got a proper mirror set up (instead of manually sourced .csv files), make it make a POST request to `/api/reload` with the auth secret as the `auth` URL parameter to reload the database.
DeArrow Browser should remain usable while the database is reloaded. (assuming we don't run out of RAM)
