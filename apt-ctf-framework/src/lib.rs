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
use move_compiler::{
    FullyCompiledProgram,
    shared::{ NumericalAddress, PackagePaths },
};
use move_core_types::{
    identifier::{ IdentStr, Identifier },
    account_address::AccountAddress,
    language_storage::{ ModuleId, TypeTag },
    value::MoveValue,
};
use move_command_line_common::{
    address::ParsedAddress,
};

use tempfile::NamedTempFile;
use once_cell::sync::Lazy;
use std::error;

static PRECOMPILED_APTOS_FRAMEWORK_V2: Lazy<PrecompiledFilesModules> = Lazy::new(|| {
    let named_address_mapping_strings: Vec<String> = aptos_framework::named_addresses()
        .iter()
        .map(|(string, num_addr)| format!("{}={}", string, num_addr))
        .collect();

    let options = move_compiler_v2::Options {
        sources: aptos_cached_packages::head_release_bundle()
            .files()
            .unwrap(),
        dependencies: vec![],
        named_address_mapping: named_address_mapping_strings,
        known_attributes: aptos_framework::extended_checks::get_all_attribute_names().clone(),
        language_version: None,
        ..move_compiler_v2::Options::default()
    };

    let (_global_env, modules) = move_compiler_v2::run_move_compiler_to_stderr(options)
        .expect("stdlib compilation succeeds");
    PrecompiledFilesModules::new(APTOS_FRAMEWORK_FILES.clone(), modules)
});

static APTOS_FRAMEWORK_FILES: Lazy<Vec<String>> = Lazy::new(|| {
    aptos_cached_packages::head_release_bundle()
        .files()
        .unwrap()
});

pub fn initialize<'a>( 
    named_addresses: Vec<(String, NumericalAddress)>,
    account_priv_keys : Vec<(Identifier, Ed25519PrivateKey)>,
) -> AptosTestAdapter<'a> {

    let default_syntax = SyntaxChoice::Source;
    let comparison_mode = false;
    let run_config = TestRunConfig::CompilerV1;
    let v1_lib: Option<&(FullyCompiledProgram, Vec<PackagePaths>)> = None;
    let v2_lib: Option<&PrecompiledFilesModules> = Some(&*PRECOMPILED_APTOS_FRAMEWORK_V2);

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

    let (adapter, _result_opt) = AptosTestAdapter::init(default_syntax, comparison_mode, run_config, v1_lib, v2_lib, init_opt);
    println!("[*] Initialization Result: {:#?}", _result_opt);
    println!("[*] Successfully Initialized");
    
    adapter
}

pub fn publish_compiled_module(
    adapter: &mut AptosTestAdapter, 
    module: CompiledModule, 
    signer: String,
    module_named_address: String,
) -> AccountAddress {
    let gas_budget: Option<u64> = None;
    let extra: AptosPublishArgs = AptosPublishArgs { 
        private_key: Some(RawPrivateKey::Named(Identifier::new(signer).unwrap())), 
        expiration_time: None, 
        sequence_number: None,
        gas_unit_price: None,
        override_signer: None
    };
    let named_addr_opt = Some(Identifier::new(module_named_address).unwrap());

    let (_output, module) = adapter
        .publish_module(module, named_addr_opt, gas_budget, extra)
        .unwrap();

    let published_address = module.address_identifiers[0];

    println!(
        "[*] Successfully published at {:#?}",
        published_address
    );
    // println!("[*] Output: {:#?} \n", output.unwrap());

    published_address
}

pub fn call_function(
    adapter: &mut AptosTestAdapter,
    mod_addr: AccountAddress,
    mod_name: &str,
    fun_name: &str,
    signer: String,
    args: Vec<MoveValue>,
    type_args: Vec<TypeTag>,
) -> Result<(), Box<dyn error::Error>> {
    let module_id: ModuleId = ModuleId::new(mod_addr, Identifier::new(mod_name).unwrap());
    let function: &IdentStr = IdentStr::new(fun_name).unwrap();
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

    let (output, _return_values) = adapter.call_function(
        &module_id, function, type_args, signers, args, gas_budget, extra_args,
    ).unwrap();

    println!("[*] Successfully called {:#?}", fun_name);
    println!("[*] Output Call: {:#?}", output.unwrap());

    Ok(())
}

pub fn view_object(
    adapter: &mut AptosTestAdapter, 
    address: AccountAddress,
    module: &ModuleId,
    resource: &IdentStr,
    type_args: Vec<TypeTag>
) -> String {

    let output = adapter.view_data(address, module, resource, type_args);

    println!("[*] Successfully viewed object");
    // println!("[*] Output Call: {:#?}", output.unwrap());

    output.unwrap()
}