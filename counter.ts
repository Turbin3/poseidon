import { Account, Pubkey, Result, i64, u8, Signer } from "@turbin3/poseidon";

export default class VoteProgram {
    static PROGRAM_ID = new Pubkey("HC2oqz2p6DEWfrahenqdq2moUcga9c9biqRBcdK3XKU1");

    public initialize(state: VoteState, hash: Uint8Array, user: Signer): Result {
        state.derive(["vote", hash])
            .init()
        state.vote = new i64(0)
    }

    public upvote(state: VoteState, hash: Uint8Array): Result {
        state.derive(["vote", hash])
        state.vote = state.vote.add(1)
    }

    public downvote(state: VoteState, hash: Uint8Array): Result {
        state.derive(["vote", hash])
        state.vote = state.vote.sub(1)
    }
}

export interface VoteState extends Account {
    vote: i64
    // bump: u8
}