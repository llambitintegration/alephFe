//! Transport stage: abstract live event sources.
//!
//! Hosts the `LiveEventSource` abstraction and its concrete SSE / MQTT
//! implementations, normalizing each into a single internal event stream so the
//! rest of the pipeline is transport-agnostic. Stub — no behavior yet.
