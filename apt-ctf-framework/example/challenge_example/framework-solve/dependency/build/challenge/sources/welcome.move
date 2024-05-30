module aptosctf::welcome  {

    //
    // [*] Dependencies
    //

    use std::signer;
    // use aptos_std::debug;
    

    //
    // [*] Constants
    //
    const ERR_NOT_ADMIN : u64 = 1337;


    //
    // [*] Structures
    //
    struct ChallengeStatus has key {
        is_solved : bool
    }


    //
    // [*] Module Initialization 
    //
    fun init_module(creator: &signer) {
        assert!(signer::address_of(creator) == @challenge_admin, ERR_NOT_ADMIN);
        move_to(creator, ChallengeStatus{ is_solved: false })
    }


    //
    // [*] Public functions
    //
    public fun unlock(_account: &signer, secret_number : u64) : bool acquires ChallengeStatus {
        if (secret_number == 1337) {
            if (!exists<ChallengeStatus>(@challenge_admin)) {
                let challenge_status = borrow_global_mut<ChallengeStatus>(@challenge_admin);
                challenge_status.is_solved = true;
                return true
            }
            else {
                return false
            }
        }
        else {
            return false
        }
    }

    public fun is_solved(_account: &signer) : bool acquires ChallengeStatus {
        if (!exists<ChallengeStatus>(@challenge_admin)) {
            let challenge_status = borrow_global_mut<ChallengeStatus>(@challenge_admin);
            if (challenge_status.is_solved) {
                return true
            }
            else {
                return false
            }
        }
        else {
            return false
        }
    }
}

