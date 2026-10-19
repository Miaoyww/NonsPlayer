# NonsPlayer transport patch

Origin: https://github.com/SPlayer-Dev/ncm-api-rs, commit `133b65bfe482e41ebccf018870d3fce07bf58eb3` (the previous Cargo pin). Upstream source and WTFPL license are preserved; server capability remains unused by the host.

- `request.rs`: clients disable automatic redirects; body reads validate Content-Length and accumulated chunks before JSON/EAPI decoding. Default cap is 16MiB for platform business calls (including large playlist metadata before host pagination); a clone sharing the connection pool uses 2MiB for single-track and playback resolution. `set_response_limit` adjusts that per-client cap.
- `error.rs`: typed `ResponseTooLarge`, mapped to the uniform `invalidData` error without response text.
- `api/playlist_detail.rs`: honor an explicit `n` song-body limit (0–100000), retaining the upstream 100000 default for other callers. The adapter requests `n=0,s=0` when reading metadata/IDs and fetches song bodies only for the requested page. Full ID lists, metadata, transport limits and credential handling remain intact.
- No new platform protocol or cryptography implementation. Rebase these small changes when updating upstream, and run the bounded-body tests here plus the host adapter/playback checks.

The cap bounds response payload accumulation, not TLS/HTTP allocations, JSON expansion, or process memory. Cancellation drops the async body reader; no response or credentials are logged by the patch.
