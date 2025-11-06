//! HTTP version agnostic API

// H1 and H2 logic is separated
//
// when a connection is received, an auto detection version is run first,
//
// either starts with H1 or directly to H2
//
// when a H1 server perform a protocol upgrade, a new tokio task is spawned

