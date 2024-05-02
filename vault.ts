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

        state.owner = owner.key;
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
        state.deriveWithBump(['state', owner.key], state.stateBump)
        auth.deriveWithBump(['auth', state.key], state.authBump)
        vault.deriveWithBump(['vault', auth.key], state.vaultBump)

        SystemProgram.transfer(
            owner.toAccountInfo(),
            vault.toAccountInfo(),
            amount
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

        SystemProgram.transfer(
            vault.toAccountInfo(),
            owner.toAccountInfo(),
            amount,
            ['auth', state.key, state.authBump]
        )
    }
}

export interface Vault extends Account {
    owner: Pubkey
    stateBump: u8
    authBump: u8
    vaultBump: u8
}
