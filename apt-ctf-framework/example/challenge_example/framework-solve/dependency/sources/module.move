module challenge::welcome  {

    //
    // [*] Dependencies
    //

    use std::signer;
    // use aptos_std::debug;
    

    //
    // [*] Constants
    //
    const ERR_NOT_ADMIN : u64 = 0x4000;
    const ERR_NOT_INITIALIZED : u64 = 0x4001;
    const ERR_NOT_SOLVED : u64 = 0x4002;


    //
    // [*] Structures
    //
    struct ChallengeStatus has key, store {
        is_solved : bool
    }


    //
    // [*] Module Initialization 
    //
    public entry fun initialize(account: &signer) {
        assert!(signer::address_of(account) == @challenger, ERR_NOT_ADMIN);
        move_to(account, ChallengeStatus{ is_solved: false });
    }


    //
    // [*] Public functions
    //
    public entry fun submit(_account: &signer, secret_number : u64) acquires ChallengeStatus {
        if (secret_number == 1337) {
            if (exists<ChallengeStatus>(@challenger)) {
                let challenge_status = borrow_global_mut<ChallengeStatus>(@challenger);
                challenge_status.is_solved = true;
            };
        };
    }

    public entry fun is_solved(_account: &signer) acquires ChallengeStatus {
        assert!(exists<ChallengeStatus>(@challenger), ERR_NOT_INITIALIZED);
        let challenge_status = borrow_global_mut<ChallengeStatus>(@challenger);
        assert!(challenge_status.is_solved == true, ERR_NOT_SOLVED);
    }
}