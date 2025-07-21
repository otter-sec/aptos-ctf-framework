use std::env;
use std::fs;
use std::path::Path;
use move_model::metadata::LanguageVersion;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // Compile the Aptos framework
    let named_address_mapping_strings: Vec<String> = aptos_framework::named_addresses()
        .iter()
        .map(|(string, num_addr)| format!("{}={}", string, num_addr))
        .collect();

    let all_sources = aptos_cached_packages::head_release_bundle()
        .files()
        .unwrap();

    let options = move_compiler_v2::Options {
        sources: all_sources.clone(),
        dependencies: vec![],
        named_address_mapping: named_address_mapping_strings.clone(),
        known_attributes: aptos_framework::extended_checks::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest()),
        ..move_compiler_v2::Options::default()
    };

    let (_global_env, modules) = move_compiler_v2::run_move_compiler_to_stderr(options.clone())
        .expect("framework compilation succeeds");

    // Create a serializable representation of AnnotatedCompiledUnit (no serde derive in the original code)
    #[derive(serde::Serialize)]
    struct SerializableUnit {
        name: String,
        address: Vec<u8>,
        module_bytes: Vec<u8>,
        source_map: Option<Vec<u8>>,
        loc_file: String,
        loc_start: u32,
        loc_end: u32,
        module_name_loc_file: String,
        module_name_loc_start: u32,
        module_name_loc_end: u32,
        address_name: Option<String>,
        package_name: Option<String>,
    }

    // Extract and serialize AnnotatedCompiledUnit data
    let serializable_units: Vec<SerializableUnit> = modules
        .into_iter()
        .filter_map(|unit| {
            match unit {
                legacy_move_compiler::compiled_unit::CompiledUnitEnum::Module(module) => {
                    let mut module_bytes = Vec::new();
                    module.named_module.module
                        .serialize_for_version(Some(move_binary_format::file_format_common::VERSION_8), &mut module_bytes)
                        .expect("Failed to serialize module");
                    
                    Some(SerializableUnit {
                        name: module.named_module.name.to_string(),
                        address: module.named_module.address.into_inner().to_vec(),
                        module_bytes,
                        source_map: None,
                        loc_file: module.loc.file_hash().to_string(),
                        loc_start: module.loc.start(),
                        loc_end: module.loc.end(),
                        module_name_loc_file: module.module_name_loc.file_hash().to_string(),
                        module_name_loc_start: module.module_name_loc.start(),
                        module_name_loc_end: module.module_name_loc.end(),
                        address_name: module.address_name.map(|s| s.value.as_str().to_string()),
                        package_name: module.named_module.package_name.map(|s| s.as_str().to_string()),
                    })
                }
                _ => None, // Skip scripts for now
            }
        })
        .collect();

    let serialized_data = bcs::to_bytes(&(all_sources, serializable_units))
        .expect("Failed to serialize framework data");

    // Create output directory
    let out_dir = env::var("FRAMEWORK_OUT_DIR").unwrap_or_else(|_| {
        env::var("OUT_DIR").expect("Neither FRAMEWORK_OUT_DIR nor OUT_DIR is set")
    });
    let cache_path = Path::new(&out_dir).join("aptos_framework_cache.bcs");
    
    // Write the serialized data to file
    fs::write(&cache_path, &serialized_data)
        .expect("Failed to write framework cache");

    // Set environment variable for the library to find the cache
    println!("cargo:rustc-env=APTOS_FRAMEWORK_CACHE_PATH={}", cache_path.display());
    
    println!("cargo:warning=Framework compiled and cached at: {}", cache_path.display());
} 