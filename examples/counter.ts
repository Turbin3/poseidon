import { Account, Pubkey, Result, i64, u8, Signer } from "@turbin3/poseidon";

// creating a class VoteProgram is similar to creating a creating a mod in anchor with all the instructions inside
export default class VoteProgram {

    // define the progam id as a static constant like bellow
    static PROGRAM_ID = new Pubkey("HC2oqz2p6DEWfrahenqdq2moUcga9c9biqRBcdK3XKU1");

    // we can pass in standard Accounts(Signer, TokenAccount, Mint, UncheckedAccount and so on), Custom Accounts(state in this case) and IX arguements(hash in this case) as parameters.
    initialize(state: VoteState, hash: Uint8Array, user: Signer): Result {

        // PDAs can be derived like <custom_Acc>.derive([...])
        // where inside array we can pass string, Uint8Array, pubkey
        // we can also derive PDAs which are token account, associated token account which will be covered in vault and escrow 
        state.derive(["vote", hash])
            .init() // we can initialise PDA just by chaining a init method to the derive method

        // defining properties(vote) of custom_Acc(state)
        state.vote = new i64(0)
    }

    upvote(state: VoteState, hash: Uint8Array): Result {
        state.derive(["vote", hash])
        // to do arithemtics we can chain methods like add, sub, mul, div, eq(equal), neq(not equal), lt(less than), lte(less than or equal) and so on
        state.vote = state.vote.add(1)
    }

    downvote(state: VoteState, hash: Uint8Array): Result {
        state.derive(["vote", hash])
        state.vote = state.vote.sub(1)
    }
}

// define custom accounts by creating an interface which extends class Account
export interface VoteState extends Account {
    // a variety of types are available like u8-u128, i8-i128, usize, boolean, string, Pubkey, etc...
    vote: i64
    bump: u8
}