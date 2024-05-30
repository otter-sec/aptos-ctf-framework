module solution::exploit  {

    //
    // [*] Dependencies
    //
    use challenge::welcome;

    //
    // [*] Public functions
    //
    public entry fun solve(account: &signer) {
        let secret_number = 1337;
        welcome::submit(account, secret_number);
    }
}