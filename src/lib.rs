use aptos_transactional_test_harness::{
    AptosTestAdapter, 
    AptosInitArgs, 
    RawPrivateKey,
    AptosPublishArgs,
    AptosRunArgs,
};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey,
};
use move_binary_format::{
    file_format::CompiledModule,
};
use move_transactional_test_runner::{
    framework::MoveTestAdapter,
    tasks::{ InitCommand, SyntaxChoice, TaskInput },
    vm_test_harness::{ PrecompiledFilesModules, TestRunConfig },
};
use legacy_move_compiler::shared::NumericalAddress;
use move_core_types::{
    identifier::{ IdentStr, Identifier },
    account_address::AccountAddress,
    language_storage::{ ModuleId, TypeTag },
    value::MoveValue,
};
use move_command_line_common::{
    address::ParsedAddress,
};
use move_model::metadata::LanguageVersion;
use move_symbol_pool::Symbol;
use move_ir_types::location::{Loc, Spanned};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;

use tempfile::NamedTempFile;
use once_cell::sync::Lazy;
use std::error;
use std::collections::HashMap;
use std::env;

static PRECOMPILED_APTOS_FRAMEWORK_V2_WITH_EXPERIMENTAL: Lazy<PrecompiledFilesModules> =
    Lazy::new(|| {
        // Allow overriding the framework cache path at runtime, but fall back to the
        // path determined at build time.
        let cache_path = env::var("APTOS_FRAMEWORK_CACHE_PATH_OVERRIDE")
            .unwrap_or_else(|_| env!("APTOS_FRAMEWORK_CACHE_PATH").to_string());
        
        let serialized_data = std::fs::read(&cache_path)
            .expect("Failed to read framework cache");
        
        // Define the serializable struct (must match build.rs)
        #[derive(serde::Deserialize)]
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

        let (all_sources, serializable_units): (Vec<String>, Vec<SerializableUnit>) = 
            bcs::from_bytes(&serialized_data)
                .expect("Failed to deserialize framework data");

        // Reconstruct AnnotatedCompiledUnit objects
        let annotated_units: Vec<legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit> = serializable_units
            .into_iter()
            .map(|unit| {
                let module = move_binary_format::file_format::CompiledModule::deserialize(&unit.module_bytes)
                    .expect("Failed to deserialize module");
                
                let address = move_core_types::account_address::AccountAddress::new(unit.address.try_into().unwrap());
                let name = move_core_types::identifier::Identifier::new(unit.name).unwrap();
                
                // Create default location (0, 0)
                let loc = Loc::new(FileHash::new(&unit.loc_file), unit.loc_start as u32, unit.loc_end as u32);
                let module_name_loc = Loc::new(FileHash::new(&unit.module_name_loc_file), unit.module_name_loc_start as u32, unit.module_name_loc_end as u32);
                let address_bytes: [u8; 32] = address.into();
                legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(
                    legacy_move_compiler::compiled_unit::AnnotatedCompiledModule {
                        loc,
                        module_name_loc,
                        address_name: unit.address_name.map(|s| Spanned::unsafe_no_loc(Symbol::from(s))),
                        named_module: legacy_move_compiler::compiled_unit::NamedCompiledModule {
                            package_name: unit.package_name.map(|s| Symbol::from(s)),
                            address: legacy_move_compiler::shared::NumericalAddress::new(address_bytes, legacy_move_compiler::shared::NumberFormat::Hex),
                            name: Symbol::from(name.as_str()),
                            module,
                            source_map: SourceMap::new(loc.clone(), None), // Empty source map
                        },
                    }
                )
            })
            .collect();

        PrecompiledFilesModules::new(all_sources, annotated_units)
    });

static APTOS_FRAMEWORK_FILES: Lazy<Vec<String>> = Lazy::new(|| {
    aptos_cached_packages::head_release_bundle()
        .files()
        .unwrap()
});

// Aptos CTF framework environment
pub struct AptosTF {
    adapter: AptosTestAdapter<'static>,
    account_map: HashMap<AccountAddress, String>,
    package_map: HashMap<String, AccountAddress>,
}

impl AptosTF {
    pub fn initialize( 
        named_addresses: Vec<(String, NumericalAddress)>,
        account_priv_keys : Vec<(Identifier, Ed25519PrivateKey)>,
    ) -> Result<AptosTF, Box<dyn error::Error>> {

    let default_syntax = SyntaxChoice::Source;
    // let language_version = LanguageVersion::latest();
    let v2_lib: &PrecompiledFilesModules = &*PRECOMPILED_APTOS_FRAMEWORK_V2_WITH_EXPERIMENTAL;

    let run_config = TestRunConfig::compiler_v2(LanguageVersion::latest(), vec![("attach-compiled-module".to_owned(),true)]);

    // Build account map before moving named_addresses
    let mut account_map = HashMap::new();
    for (name, num_addr) in named_addresses.iter() {
        let addr: AccountAddress = num_addr.into_inner();
        account_map.insert(addr, name.clone());
    }

    let command = (
        InitCommand { named_addresses }, 
        AptosInitArgs { 
            private_keys: Some(account_priv_keys),
            initial_coins: None, // Some(1)
        }
    );

    let name: String = "init".to_string();
    let number: usize = 0;
    let start_line: usize = 1;
    let command_lines_stop: usize = 1;
    let stop_line: usize = 1;
    let data: Option<NamedTempFile> = None;

    let init_opt: Option<TaskInput<(InitCommand, <AptosTestAdapter<'_> as MoveTestAdapter>::ExtraInitArgs)>> = Some(TaskInput {
        command,
        name,
        number,
        start_line,
        command_lines_stop,
        stop_line,
        data,
    });

    let (adapter, result_opt) = AptosTestAdapter::init(default_syntax, run_config, v2_lib, init_opt);
    println!("[*] Initialization Result: {:#?}", result_opt);
    println!("[*] Successfully Initialized");

    let aptos_tf = AptosTF {
        adapter,
        account_map,
        package_map: HashMap::new(),
    };

    Ok(aptos_tf)
    }

    pub fn publish_compiled_module(
        &mut self, 
        module: CompiledModule, 
        signer: String,
        module_named_address: String,
    ) -> Result<AccountAddress, Box<dyn error::Error>> {
    let gas_budget: Option<u64> = None;
    let extra: AptosPublishArgs = AptosPublishArgs { 
        private_key: Some(RawPrivateKey::Named(Identifier::new(signer).unwrap())), 
        expiration_time: None, 
        sequence_number: None,
        gas_unit_price: None,
        override_signer: None
    };
    let named_addr_opt = Some(Identifier::new(module_named_address).unwrap());

    let result = self.adapter
        .publish_module(module, named_addr_opt.clone(), gas_budget, extra);
    
    let (_output, module) = match result {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[!] Failed to publish module: {:?}", e);
            return Err(e.into());
        }
    };

    let published_address = module.address_identifiers[0];

    println!(
        "[*] Successfully published at {:#?}",
        published_address
    );
    // println!("[*] Output: {:#?} \n", output.unwrap());
    
    if let Some(package_name) = named_addr_opt {
        self.package_map.insert(package_name.to_string(), published_address);
    }

    Ok(published_address)
    }

    pub fn call_function(
        &mut self,
        mod_addr: AccountAddress,
        mod_name: &str,
        fun_name: &str,
        signer: String,
        args: Vec<MoveValue>,
        type_args: Vec<TypeTag>,
    ) -> Result<Option<String>, Box<dyn error::Error>> {
        let module_id: ModuleId = ModuleId::new(mod_addr, Identifier::new(mod_name).map_err(|e| -> Box<dyn error::Error> { e.into() })?);
        let function: &IdentStr = IdentStr::new(fun_name).map_err(|e| -> Box<dyn error::Error> { e.into() })?;
        let mut signers: Vec<ParsedAddress> = Vec::new();
        signers.push(ParsedAddress::Named(signer));

        let gas_budget: Option<u64> = None;
        let extra_args: AptosRunArgs = AptosRunArgs {
            private_key: None,
            script: false,
            expiration_time: None,
            sequence_number: None,
            gas_unit_price: None,
            show_events: true,
            secondary_signers: None,
        };

        match self.adapter.call_function(
            &module_id, function, type_args, signers, args, gas_budget, extra_args,
        ) {
            Ok((output, _return_values)) => {
                println!("[*] Successfully called {:#?}", fun_name);
                println!("[*] Output Call: {:#?}", output.clone().unwrap_or_else(|| "<empty>".to_string()));
                Ok(output)
            }
            Err(err) => {
                eprintln!("[!] Failed to call function: {:?}", err);
                Err(err.into())
            }
        }
    }

    pub fn view_object(
        &mut self, 
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>
    ) -> Result<String, Box<dyn error::Error>> {

        match self.adapter.view_data(address, module, resource, type_args) {
            Ok(output) => {
                println!("[*] Successfully viewed object");
                // println!("[*] Output Call: {:#?}", output);
                Ok(output)
            }
            Err(err) => {
                eprintln!("[!] Failed to view object: {:?}", err);
                Err(err.into())
            }
        }
    }

    pub fn get_account_address(
        &self, 
        account_name: &str
    ) -> Option<AccountAddress> {
        self.account_map.iter().find_map(|(&addr, name)| {
            if name == account_name {
                Some(addr)
            } else {
                None
            }
        })
    }

    pub fn get_package_address(
        &self, 
        package_name: &str
    ) -> Option<AccountAddress> {
        self.package_map.get(package_name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use legacy_move_compiler::shared::NumericalAddress;
    
    #[test]
    fn it_initializes_framework() -> Result<(), Box<dyn std::error::Error>> {
        /* 1. framework init
         * ───────────────── */
        let named_addresses = vec![
            (
                "challenger".to_string(),
                NumericalAddress::parse_str(
                    "0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6", 
                )?,
            ),
            (
                "solver".to_string(),
                NumericalAddress::parse_str(
                    "0x9c3b634ac05d0af393e0f93b9b19b61e7cac1c519f566276aa0c6fd15dac12aa",
                )?,
            ),
            (
                "challenge".to_string(),
                NumericalAddress::parse_str(
                    "0x1337",
                )?,
            ),
            (
                "solution".to_string(),
                NumericalAddress::parse_str(
                    "0x1338",
                )?,
            )
        ];

        // Create accounts
        let mut account_priv_keys: Vec<(Identifier, Ed25519PrivateKey)> = Vec::new();
        let challenger_key = Ed25519PrivateKey::from_bytes_unchecked(&[
            0x56, 0xa2, 0x61, 0x40, 0xeb, 0x23, 0x37, 0x50, 0xcd, 0x14, 0xfb, 0x16, 0x8c, 0x3e, 0xb4, 0xbd, 0x07, 0x82, 0xb0, 0x99, 0xcd, 0xe6, 0x26, 0xec, 0x8a, 0xff, 0x7f, 0x3c, 0xce, 0xb6, 0x36, 0x4f
        ])?;
        let solver_key = Ed25519PrivateKey::from_bytes_unchecked(&[
            0x95, 0x2a, 0xaf, 0x3a, 0x98, 0xa2, 0x79, 0x03, 0xdd, 0x07, 0x8d, 0x76, 0xfc, 0x9e, 0x41, 0x17, 0x40, 0xd2, 0xae, 0x9d, 0xd9, 0xec, 0xb8, 0x7b, 0x96, 0xc7, 0xcd, 0x6b, 0x79, 0x1f, 0xfc, 0x69
        ])?;
        account_priv_keys.push((Identifier::new("challenger")?, challenger_key));
        account_priv_keys.push((Identifier::new("solver")?, solver_key));

        // Initialize AptosTF
        let mut aptos_tf = AptosTF::initialize(named_addresses.clone(), account_priv_keys)?;

        // Check that AptosTestAdapter::init worked and v2_lib is loaded
        let challenger_addr = aptos_tf.get_account_address("challenger").unwrap();
        let framework_addr = AccountAddress::from_hex_literal("0x1")?;
        let module_id = ModuleId::new(framework_addr, Identifier::new("account")?);
        let resource_ident = IdentStr::new("Account")?;
        let view_result = aptos_tf.view_object(
            challenger_addr,
            &module_id,
            resource_ident,
            vec![],
        );
        assert!(view_result.is_ok(), "Should be able to view Account resource from framework");


        // Check that the account map is correctly populated
        assert_eq!(aptos_tf.account_map.len(), 4);
        assert!(aptos_tf.package_map.is_empty());

        for (name, num_addr) in &named_addresses {
            let expected_addr: AccountAddress = (*num_addr).into_inner();
            assert_eq!(aptos_tf.get_account_address(name), Some(expected_addr));
        }

        println!("✓ Framework initialized");
        Ok(())
    }
}