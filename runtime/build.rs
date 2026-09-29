#[cfg(all(feature = "std", feature = "metadata-hash"))]
fn main() {
	substrate_wasm_builder::WasmBuilder::init_with_defaults()
		// This SDK imports node host functions implicitly; Rust 1.96+ requires opt-in.
		.append_to_rust_flags("-Clink-arg=--allow-undefined")
		.enable_metadata_hash("UNIT", 12)
		.build();
}

#[cfg(all(feature = "std", not(feature = "metadata-hash")))]
fn main() {
	substrate_wasm_builder::WasmBuilder::init_with_defaults()
		// This SDK imports node host functions implicitly; Rust 1.96+ requires opt-in.
		.append_to_rust_flags("-Clink-arg=--allow-undefined")
		.build();
}

/// The wasm builder is deactivated when compiling
/// this crate for wasm to speed up the compilation.
#[cfg(not(feature = "std"))]
fn main() {}
