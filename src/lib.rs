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

use tempfile::NamedTempFile;
use once_cell::sync::Lazy;
use std::error;
use std::collections::HashMap;

static PRECOMPILED_APTOS_FRAMEWORK_V2_WITH_EXPERIMENTAL: Lazy<PrecompiledFilesModules> =
    Lazy::new(|| {
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

        PrecompiledFilesModules::new(all_sources, modules)
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

    let (adapter, _result_opt) = AptosTestAdapter::init(default_syntax, run_config, v2_lib, init_opt);
    println!("[*] Initialization Result: {:#?}", _result_opt);
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