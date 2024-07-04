import { Account, Pubkey, Result, Signer, SystemAccount, SystemProgram, UncheckedAccount, u64, u8 } from "@3thos/poseidon";

export default class VaultProgram {
    static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

    initialize(
        owner: Signer,
        state: Vault,
        auth: UncheckedAccount,
        vault: SystemAccount
    ): Result {
        
        auth.derive(['auth', state.key])
        state.derive(['state', owner.key]).init()
        vault.derive(['vault', auth.key])

        // assigning a arguement of type Pubkey to the custom_Acc(state) by calling the key property
        state.owner = owner.key;

        // to store bumps in the custom_Acc(state), we can simply call getBump on the custom_Acc(state)
        state.stateBump = state.getBump()
        state.authBump = auth.getBump()
        state.vaultBump = vault.getBump()
    }

    deposit(
        owner: Signer,
        state: Vault,
        auth: UncheckedAccount,
        vault: SystemAccount,
        amount: u64
    ) {
        // if we have stored bump in the custom_Acc(state), we can derive PDAs with stored bumps by passing that as the 2nd arguement
        state.deriveWithBump(['state', owner.key], state.stateBump)
        auth.deriveWithBump(['auth', state.key], state.authBump)
        vault.deriveWithBump(['vault', auth.key], state.vaultBump)

        // we support a number for functions from SystemProgram and TokenProgram
        SystemProgram.transfer(
            owner, // from
            vault, // to
            amount // amount to be sent
        )
    }

    withdraw(
        owner: Signer,
        state: Vault,
        auth: UncheckedAccount,
        vault: SystemAccount,
        amount: u64
    ) {        
        state.deriveWithBump(['state', owner.key], state.stateBump)
        auth.deriveWithBump(['auth', state.key], state.authBump)
        vault.deriveWithBump(['vault', auth.key], state.vaultBump)

        // since here we are transfering from a PDA we have give seeds of the PDA as the last arguement
        SystemProgram.transfer(
            vault,
            owner,
            amount,
            ['vault', state.key, state.authBump]
        )
    }
}

export interface Vault extends Account {
    owner: Pubkey
    stateBump: u8
    authBump: u8
    vaultBump: u8
}
