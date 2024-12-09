import { 
    Account, 
    Pubkey, 
    type Result, 
    i64, 
    u8, 
    Signer,
    Str
} from "@solanaturbine/poseidon";

export default class ChatProgram {
    static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

    initialize(
        authority: Signer,
        boardState: BoardState
    ): Result {
        boardState.derive(["board"])
                 .init(authority);
        
        boardState.authority = authority.key;
        boardState.messageCount = new i64(0);
        boardState.bump = boardState.getBump();
    }

    postMessage(
        author: Signer,
        message: Message,
        boardState: BoardState,
        title: Str<64>,
        content: Str<1024>
    ): Result {
        boardState.derive(["board"]);
        
        message.derive([
            "message",
            boardState.messageCount.toBytes(),
            author.key
        ]).init(author);
        
        message.author = author.key;
        message.title = title;
        message.content = content;
        message.messageIndex = boardState.messageCount;
        message.bump = message.getBump();
        
        boardState.messageCount = boardState.messageCount.add(1);
    }

    editMessage(
        author: Signer,
        message: Message,
        boardState: BoardState,
        newTitle: Str<64>,
        newContent: Str<1024>
    ): Result {
        boardState.derive(["board"]);
        
        message.derive([
            "message",
            message.messageIndex.toBytes(),
            author.key
        ]);

        // verify author
        if (message.author != author.key) {
            throw new Error("Only the author can edit this message");
        }
        
        // update message
        message.title = newTitle;
        message.content = newContent;
    }

    deleteMessage(
        author: Signer,
        message: Message,
        boardState: BoardState
    ): Result {
        boardState.derive(["board"]);
        
        message.derive([
            "message",
            message.messageIndex.toBytes(),
            author.key
        ]);

        if (message.author != author.key) {
            throw new Error("Only the author can delete this message");
        }
        
        message.close(author);
    }
}

export interface Message extends Account {
    author: Pubkey;
    title: Str<64>;
    content: Str<1024>;
    messageIndex: i64;
    bump: u8;
}

export interface BoardState extends Account {
    authority: Pubkey;
    messageCount: i64;
    bump: u8;
}