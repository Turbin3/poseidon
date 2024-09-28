# Program

In TypeScript, the program is defined as a class with a `static PROGRAM_ID` to specify the program ID.

```typescript
import { Pubkey } from "@solanaturbine/poseidon";
export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");
}
```

And `poseidon` will transpile it into the following Rust code.

```rust,ignore
use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod escrow_program {
    use super::*;
}
```

Notice that Anchor will generate the program ID for you.

Get your program IDs with this command inside your Anchor project.

```bash
$ anchor keys list
# Output
# <program_name>: <program_id>
```
