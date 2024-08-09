# the error_handling crate

this is basically the core of anyhow, but written from scratch, in safe rust.

simpler, hopefully less annoying when trying to cache errors (errors use Arc<> internally, and there's a serializable variant that drops a lot of data, but makes it easy to send error info between workers)

no required dependencies
serde optionally required for serializing the serializable error variant
anyhow optionally required for turning anyhow errors into serializable errors
