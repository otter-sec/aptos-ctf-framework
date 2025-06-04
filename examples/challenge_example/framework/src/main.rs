use std::env;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::mem::drop;
use std::net::{TcpListener, TcpStream};

use tokio;

use aptos_crypto::ed25519::Ed25519PrivateKey;
use move_binary_format::file_format::CompiledModule;
use move_core_types::{
    ident_str, 
    identifier::{ Identifier },
    account_address::AccountAddress, 
    language_storage::{TypeTag, ModuleId },
    value::MoveValue,
};
use legacy_move_compiler::shared::NumericalAddress;

async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    
    let modules = vec!["welcome"];
    
    // Initialize Named Addresses
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

    // Create Accounts
    let mut account_priv_keys : Vec<(Identifier, Ed25519PrivateKey)> = Vec::new();
    let challenger_key = Ed25519PrivateKey::from_bytes_unchecked(&[
        0x56, 0xa2, 0x61, 0x40, 0xeb, 0x23, 0x37, 0x50, 0xcd, 0x14, 0xfb, 0x16, 0x8c, 0x3e, 0xb4, 0xbd, 0x07, 0x82, 0xb0, 0x99, 0xcd, 0xe6, 0x26, 0xec, 0x8a, 0xff, 0x7f, 0x3c, 0xce, 0xb6, 0x36, 0x4f
    ]).unwrap();
    let solver_key = Ed25519PrivateKey::from_bytes_unchecked(&[
        0x95, 0x2a, 0xaf, 0x3a, 0x98, 0xa2, 0x79, 0x03, 0xdd, 0x07, 0x8d, 0x76, 0xfc, 0x9e, 0x41, 0x17, 0x40, 0xd2, 0xae, 0x9d, 0xd9, 0xec, 0xb8, 0x7b, 0x96, 0xc7, 0xcd, 0x6b, 0x79, 0x1f, 0xfc, 0x69
    ]).unwrap();
    account_priv_keys.push((Identifier::new("challenger".to_string()).unwrap(), challenger_key));
    account_priv_keys.push((Identifier::new("solver".to_string()).unwrap(), solver_key));

    // Initialize CTF Framework (Adapter)
    let mut adapter = apt_ctf_framework::initialize(
        named_addresses,
        account_priv_keys
    );

    // Publish Challenge Module
    let mod_path = format!("./challenge/build/challenge/bytecode_modules/{}.mv", modules[0]);
    let mod_bytes: Vec<u8> = std::fs::read(mod_path)?;

    let module : CompiledModule = CompiledModule::deserialize(&mod_bytes).unwrap();

    let chall_addr = apt_ctf_framework::publish_compiled_module(
        &mut adapter,
        module,
        "challenger".to_string(),
        "challenge".to_string(),
    );
    println!("[SERVER] Module published at: {:?}", chall_addr); 

    // Read Solution Module
    let mut solution_data = [0 as u8; 2000];
    let _solution_size = stream.read(&mut solution_data)?;

    // Send Challenge Address
    let mut output = String::new();
    fmt::write(
        &mut output,
        format_args!(
            "[SERVER] Challenge modules published at: {}",
            chall_addr.to_string().as_str(),
        ),
    )
    .unwrap();
    stream.write(output.as_bytes()).unwrap();

    // Publish Solution Module
    let module_solve : CompiledModule = CompiledModule::deserialize(&solution_data).unwrap();

    let mut chall_dependencies: Vec<String> = Vec::new();
    chall_dependencies.push(String::from("challenge"));

    let sol_addr = apt_ctf_framework::publish_compiled_module(
        &mut adapter,
        module_solve,
        "solver".to_string(),
        "solve".to_string(),
    );
    println!("[SERVER] Module published at: {:?}", sol_addr); 

    // Send Solution Address
    output = String::new();
    fmt::write(
        &mut output,
        format_args!(
            "[SERVER] Solution published at {}",
            sol_addr.to_string().as_str()
        ),
    )
    .unwrap();
    stream.write(output.as_bytes()).unwrap();

    // Call initialize Function
    let args_init: Vec<MoveValue> = Vec::new();
    let mut type_args : Vec<TypeTag> = Vec::new();

    let ret_val = apt_ctf_framework::call_function(
        &mut adapter,
        chall_addr,
        "welcome",
        "initialize",
        "challenger".to_string(),
        args_init,
        type_args,
    );
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Check Resource (View Object)
    let mut owner_address = AccountAddress::from_hex_literal("0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6").unwrap();
    let mut module_id: ModuleId = ModuleId::new(chall_addr, Identifier::new(modules[0]).unwrap());
    let mut object_output = apt_ctf_framework::view_object(&mut adapter, owner_address, &module_id, &ident_str!("ChallengeStatus"), Vec::new());
    println!("Object Output: {:#?}", object_output);


    // Call solve Function
    let args_solve: Vec<MoveValue> = Vec::new();
    type_args = Vec::new();

    let ret_val = apt_ctf_framework::call_function(
        &mut adapter,
        sol_addr,
        "exploit",
        "solve",
        "solver".to_string(),
        args_solve,
        type_args,
    );
    println!("[SERVER] Return value {:#?}", ret_val);
    println!("");

    // Check Resource (View Object)
    owner_address = AccountAddress::from_hex_literal("0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6").unwrap();
    module_id = ModuleId::new(chall_addr, Identifier::new(modules[0]).unwrap());
    object_output = apt_ctf_framework::view_object(&mut adapter, owner_address, &module_id, &ident_str!("ChallengeStatus"), Vec::new());
    println!("Object Output: {:#?}", object_output);

    // Check Solution
    let args_solve: Vec<MoveValue> = Vec::new();
    let type_args : Vec<TypeTag> = Vec::new();

    // Call is_solved Function
    let sol_ret = apt_ctf_framework::call_function(
        &mut adapter,
        chall_addr,
        "welcome",
        "is_solved",
        "challenger".to_string(),
        args_solve,
        type_args,
    );
    println!("[SERVER] Return value {:#?}", sol_ret);
    println!("");

    // Validate Solution
    match sol_ret {
        Ok(()) => {
            println!("[SERVER] Correct Solution!");
            println!("");
            if let Ok(flag) = env::var("FLAG") {
                let message = format!("[SERVER] Congrats, flag: {}", flag);
                stream.write(message.as_bytes()).unwrap();
            } else {
                stream.write("[SERVER] Flag not found, please contact admin".as_bytes()).unwrap();
            }
        }
        Err(_error) => {
            println!("[SERVER] Invalid Solution!");
            println!("");
            stream.write("[SERVER] Invalid Solution!".as_bytes()).unwrap();
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create Socket - Port 31337
    let listener = TcpListener::bind("0.0.0.0:31337")?;
    println!("[SERVER] Starting server at port 31337!");

    let local = tokio::task::LocalSet::new();

    // Wait For Incoming Solution
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("[SERVER] New connection: {}", stream.peer_addr()?);
                    let result = local.run_until( async move {
                        tokio::task::spawn_local( async {
                            handle_client(stream).await.unwrap();
                        }).await.unwrap();
                    }).await;
                    println!("[SERVER] Result: {:?}", result);
            }
            Err(e) => {
                println!("[SERVER] Error: {}", e);
            }
        }
    }

    // Close Socket Server
    drop(listener);
    Ok(())
}
