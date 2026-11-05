# Dependency licences

Generated from the locked Cargo metadata, including build and inactive platform dependencies.
An OR expression is used under the permitted alternative recorded below. Every term of an
AND expression must be permitted. Data from the Carbon Intensity API is separately CC BY 4.0. Boundary data is separately
under the NESO Open Data Licence; see data/SOURCES.md.

The local `unicode-ident` compatibility adapter is clean-room MIT code calling the
MIT/Apache-2.0 `unicode-xid` implementation. No upstream unicode-ident code or tables are copied.
`lru` and `unicode-normalization` remain constrained by the project licence policy.
Patched `url`/IDNA uses ICU4X (`icu_collections`, `icu_locale_core`, `icu_normalizer`,
`icu_properties`, `icu_provider` and their data crates) under permissive Unicode-3.0.
Keep the lockfile and run this check when upgrading.

| Package | Version | Declared SPDX | Selected licence(s) |
| --- | --- | --- | --- |
| ahash | 0.8.12 | MIT OR Apache-2.0 | MIT |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | MIT |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 | MIT |
| anstream | 1.0.0 | MIT OR Apache-2.0 | MIT |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | MIT |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | MIT |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 | MIT |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 | MIT |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | MIT |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | Apache-2.0 |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | Apache-2.0 |
| base64 | 0.22.1 | MIT OR Apache-2.0 | MIT |
| base64 | 0.23.1 | MIT OR Apache-2.0 | MIT |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | MIT |
| block2 | 0.6.2 | MIT | MIT |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | MIT |
| bytes | 1.12.1 | MIT | MIT |
| cassowary | 0.3.0 | MIT / Apache-2.0 | MIT |
| castaway | 0.2.4 | MIT | MIT |
| cc | 1.10.0 | MIT OR Apache-2.0 | MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | MIT |
| cfg_aliases | 0.2.2 | MIT | MIT |
| chacha20 | 0.10.2 | MIT OR Apache-2.0 | MIT |
| chrono | 0.4.45 | MIT OR Apache-2.0 | MIT |
| clap | 4.6.7 | MIT OR Apache-2.0 | MIT |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 | MIT |
| clap_derive | 4.6.7 | MIT OR Apache-2.0 | MIT |
| clap_lex | 1.1.1 | MIT OR Apache-2.0 | MIT |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | MIT |
| compact_str | 0.8.2 | MIT | MIT |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | MIT |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | MIT |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 | MIT |
| crossterm | 0.28.1 | MIT | MIT |
| crossterm_winapi | 0.9.1 | MIT | MIT |
| ctrlc | 3.5.2 | MIT/Apache-2.0 | MIT |
| darling | 0.24.1 | MIT | MIT |
| darling_core | 0.24.1 | MIT | MIT |
| darling_macro | 0.24.1 | MIT | MIT |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | Apache-2.0 |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | MIT |
| either | 1.19.0 | MIT OR Apache-2.0 | MIT |
| errno | 0.3.14 | MIT OR Apache-2.0 | MIT |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | MIT |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | MIT |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | MIT |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | MIT |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | MIT |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | MIT |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | MIT |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | MIT |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | MIT |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | MIT |
| gridtop | 0.1.0 | MIT | MIT |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | MIT |
| heck | 0.5.0 | MIT OR Apache-2.0 | MIT |
| http | 1.5.0 | MIT OR Apache-2.0 | MIT |
| http-body | 1.1.0 | MIT | MIT |
| http-body-util | 0.1.5 | MIT | MIT |
| httparse | 1.10.1 | MIT OR Apache-2.0 | MIT |
| hyper | 1.12.0 | MIT | MIT |
| hyper-rustls | 0.27.10 | Apache-2.0 OR ISC OR MIT | Apache-2.0 |
| hyper-util | 0.1.21 | MIT | MIT |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 | MIT |
| iana-time-zone-haiku | 0.1.2 | MIT OR Apache-2.0 | MIT |
| icu_collections | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_locale_core | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_normalizer | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_normalizer_data | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_properties | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_properties_data | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| icu_provider | 2.4.0 | Unicode-3.0 | Unicode-3.0 |
| ident_case | 1.0.1 | MIT/Apache-2.0 | MIT |
| idna | 1.1.0 | MIT OR Apache-2.0 | MIT |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | Apache-2.0 |
| indoc | 2.0.8 | MIT OR Apache-2.0 | MIT |
| instability | 0.3.14 | MIT | MIT |
| ipnet | 2.12.2 | MIT OR Apache-2.0 | MIT |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | MIT |
| itertools | 0.13.0 | MIT OR Apache-2.0 | MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 | MIT |
| js-sys | 0.3.107 | MIT OR Apache-2.0 | MIT |
| libc | 0.2.191 | MIT OR Apache-2.0 | MIT |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Apache-2.0 |
| litemap | 0.8.3 | Unicode-3.0 | Unicode-3.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | MIT |
| log | 0.4.35 | MIT OR Apache-2.0 | MIT |
| lru | 0.12.4 | MIT | MIT |
| lru-slab | 0.1.3 | MIT OR Apache-2.0 OR Zlib | MIT |
| memchr | 2.8.3 | Unlicense OR MIT | MIT |
| mio | 1.2.4 | MIT | MIT |
| nix | 0.31.3 | MIT | MIT |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | MIT |
| objc2 | 0.6.5 | MIT | MIT |
| objc2-encode | 4.1.0 | MIT | MIT |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | MIT |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 | MIT |
| openssl-probe | 0.2.2 | MIT OR Apache-2.0 | MIT |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | MIT |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | MIT |
| paste | 1.0.15 | MIT OR Apache-2.0 | MIT |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | Apache-2.0 |
| potential_utf | 0.1.6 | Unicode-3.0 | Unicode-3.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | MIT |
| quinn | 0.11.12 | MIT OR Apache-2.0 | MIT |
| quinn-proto | 0.11.19 | MIT OR Apache-2.0 | MIT |
| quinn-udp | 0.5.16 | MIT OR Apache-2.0 | MIT |
| quote | 1.0.47 | MIT OR Apache-2.0 | MIT |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | MIT |
| rand | 0.10.3 | MIT OR Apache-2.0 | MIT |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | MIT |
| rand_pcg | 0.10.2 | MIT OR Apache-2.0 | MIT |
| ratatui | 0.29.0 | MIT | MIT |
| redox_syscall | 0.5.18 | MIT | MIT |
| reqwest | 0.12.28 | MIT OR Apache-2.0 | MIT |
| ring | 0.17.14 | Apache-2.0 AND ISC | Apache-2.0 AND ISC |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | Apache-2.0 |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Apache-2.0 |
| rustls | 0.23.46 | Apache-2.0 OR ISC OR MIT | Apache-2.0 |
| rustls-native-certs | 0.8.5 | Apache-2.0 OR ISC OR MIT | Apache-2.0 |
| rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 | MIT |
| rustls-webpki | 0.103.16 | ISC | ISC |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | MIT |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | Apache-2.0 |
| schannel | 0.1.30 | MIT | MIT |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | MIT |
| security-framework | 3.7.0 | MIT OR Apache-2.0 | MIT |
| security-framework-sys | 2.17.0 | MIT OR Apache-2.0 | MIT |
| serde | 1.0.229 | MIT OR Apache-2.0 | MIT |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | MIT |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | MIT |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | MIT |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | MIT |
| shlex | 2.0.1 | MIT OR Apache-2.0 | MIT |
| signal-hook | 0.3.18 | Apache-2.0/MIT | Apache-2.0 |
| signal-hook-mio | 0.2.5 | MIT OR Apache-2.0 | MIT |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | MIT |
| slab | 0.4.12 | MIT | MIT |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | MIT |
| socket2 | 0.6.5 | MIT OR Apache-2.0 | MIT |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | MIT |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | MIT |
| strsim | 0.11.1 | MIT | MIT |
| strum | 0.26.3 | MIT | MIT |
| strum_macros | 0.26.4 | MIT | MIT |
| subtle | 2.6.1 | BSD-3-Clause | BSD-3-Clause |
| syn | 2.0.119 | MIT OR Apache-2.0 | MIT |
| syn | 3.0.9 | MIT OR Apache-2.0 | MIT |
| sync_wrapper | 1.0.2 | Apache-2.0 | Apache-2.0 |
| synstructure | 0.14.0 | MIT | MIT |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | MIT |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | MIT |
| tinystr | 0.8.5 | Unicode-3.0 | Unicode-3.0 |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT | Apache-2.0 |
| tokio | 1.54.1 | MIT | MIT |
| tokio-rustls | 0.26.6 | MIT OR Apache-2.0 | MIT |
| tower | 0.5.3 | MIT | MIT |
| tower-http | 0.6.11 | MIT | MIT |
| tower-layer | 0.3.3 | MIT | MIT |
| tower-service | 0.3.3 | MIT | MIT |
| tracing | 0.1.44 | MIT | MIT |
| tracing-core | 0.1.36 | MIT | MIT |
| try-lock | 0.2.5 | MIT | MIT |
| unicode-ident | 1.0.27 | MIT | MIT |
| unicode-normalization | 0.1.24 | MIT/Apache-2.0 | MIT |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | MIT |
| unicode-truncate | 1.1.0 | MIT OR Apache-2.0 | MIT |
| unicode-width | 0.1.14 | MIT OR Apache-2.0 | MIT |
| unicode-width | 0.2.0 | MIT OR Apache-2.0 | MIT |
| unicode-xid | 0.2.6 | MIT OR Apache-2.0 | MIT |
| untrusted | 0.9.0 | ISC | ISC |
| url | 2.5.8 | MIT OR Apache-2.0 | MIT |
| utf8_iter | 1.1.0 | Apache-2.0 OR MIT | Apache-2.0 |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | Apache-2.0 |
| utf_types | 1.0.0 | Apache-2.0 OR MIT | Apache-2.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 | MIT |
| want | 0.3.2 | MIT | MIT |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | Apache-2.0 |
| wasm-bindgen | 0.2.130 | MIT OR Apache-2.0 | MIT |
| wasm-bindgen-futures | 0.4.80 | MIT OR Apache-2.0 | MIT |
| wasm-bindgen-macro | 0.2.130 | MIT OR Apache-2.0 | MIT |
| wasm-bindgen-macro-support | 0.2.130 | MIT OR Apache-2.0 | MIT |
| wasm-bindgen-shared | 0.2.130 | MIT OR Apache-2.0 | MIT |
| web-sys | 0.3.107 | MIT OR Apache-2.0 | MIT |
| web-time | 1.1.0 | MIT OR Apache-2.0 | MIT |
| winapi | 0.3.9 | MIT/Apache-2.0 | MIT |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | MIT |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | MIT |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | MIT |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | MIT |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | MIT |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | MIT |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | MIT |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | MIT |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | MIT |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | MIT |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | MIT |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | MIT |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | MIT |
| writeable | 0.6.4 | Unicode-3.0 | Unicode-3.0 |
| yoke | 0.8.4 | Unicode-3.0 | Unicode-3.0 |
| yoke-derive | 0.8.4 | Unicode-3.0 | Unicode-3.0 |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | BSD-2-Clause |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | BSD-2-Clause |
| zerofrom | 0.1.8 | Unicode-3.0 | Unicode-3.0 |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | Unicode-3.0 |
| zeroize | 1.9.1 | Apache-2.0 OR MIT | Apache-2.0 |
| zerotrie | 0.2.5 | Unicode-3.0 | Unicode-3.0 |
| zerovec | 0.11.9 | Unicode-3.0 | Unicode-3.0 |
| zerovec-derive | 0.11.7 | Unicode-3.0 | Unicode-3.0 |
| zmij | 1.0.23 | MIT | MIT |
