/// This module provides the foundation for transferring of Tokens
module AptosFramework::TokenTransfers {
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::TokenV1::{Self, Token, TokenId};

    struct TokenTransfers has key {
        pending_claims: Table<address, Table<TokenId, Token>>,
    }

    fun initialize_token_transfers(account: &signer) {
        move_to(
            account,
            TokenTransfers {
                pending_claims: Table::new<address, Table<TokenId, Token>>(),
            }
        )
    }

    public(script) fun offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: vector<u8>,
        name: vector<u8>,
        edition: u64,
        amount: u64,
    ) acquires TokenTransfers {
        let token_id = TokenV1::create_token_id_raw(creator, collection, name, edition);
        offer(&sender, receiver, token_id, amount);
    }

    // Make an entry into pending transfers and extract from gallery
    public fun offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
        amount: u64,
    ) acquires TokenTransfers {
        let sender_addr = Signer::address_of(sender);
        if (!exists<TokenTransfers>(sender_addr)) {
            initialize_token_transfers(sender)
        };

        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender_addr).pending_claims;
        if (!Table::contains(pending_claims, receiver)) {
            Table::add(pending_claims, receiver, Table::new())
        };
        let addr_pending_claims = Table::borrow_mut(pending_claims, receiver);

        let token = TokenV1::withdraw_token(sender, token_id, amount);
        let token_id = *TokenV1::token_id(&token);
        if (Table::contains(addr_pending_claims, token_id)) {
            let dst_token = Table::borrow_mut(addr_pending_claims, token_id);
            TokenV1::merge(dst_token, token)
        } else {
            Table::add(addr_pending_claims, token_id, token)
        }
    }

    public(script) fun claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: vector<u8>,
        edition: u64,
        name: vector<u8>,
    ) acquires TokenTransfers {
        let token_id = TokenV1::create_token_id_raw(creator, collection, name, edition);
        claim(&receiver, sender, token_id);
    }

    // Pull from someone else's pending transfers and insert into our gallery
    public fun claim(
        receiver: &signer,
        sender: address,
        token_id: TokenId,
    ) acquires TokenTransfers {
        let receiver_addr = Signer::address_of(receiver);
        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender).pending_claims;
        let pending_tokens = Table::borrow_mut(pending_claims, receiver_addr);
        let token = Table::remove(pending_tokens, token_id);

        if (Table::length(pending_tokens) == 0) {
            let real_pending_claims = Table::remove(pending_claims, receiver_addr);
            Table::destroy_empty(real_pending_claims)
        };

        TokenV1::deposit_token(receiver, token)
    }

    public(script) fun cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: vector<u8>,
        serial_number: u64,
        name: vector<u8>,
    ) acquires TokenTransfers {
        let token_id = TokenV1::create_token_id_raw(creator, collection, name, serial_number);
        cancel_offer(&sender, receiver, token_id);
    }

    // Extra from our pending_claims and return to gallery
    public fun cancel_offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
    ) acquires TokenTransfers {
        let sender_addr = Signer::address_of(sender);
        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender_addr).pending_claims;
        let pending_tokens = Table::borrow_mut(pending_claims, receiver);
        let token = Table::remove(pending_tokens, token_id);

        if (Table::length(pending_tokens) == 0) {
            let real_pending_claims = Table::remove(pending_claims, receiver);
            Table::destroy_empty(real_pending_claims)
        };

        TokenV1::deposit_token(sender, token)
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun test_nft(creator: signer, owner: signer) acquires TokenTransfers {
        let token_id = create_token(&creator, 1);

        let creator_addr = Signer::address_of(&creator);
        let owner_addr = Signer::address_of(&owner);
        offer(&creator, owner_addr, token_id, 1);
        claim(&owner, creator_addr, token_id);

        offer(&owner, creator_addr, token_id, 1);
        cancel_offer(&owner, creator_addr, token_id);
    }

    #[test(creator = @0x1, owner0 = @0x2, owner1 = @0x3)]
    public fun test_editions(
        creator: signer,
        owner0: signer,
        owner1: signer,
    ) acquires TokenTransfers {
        let token_id = create_token(&creator, 2);

        let creator_addr = Signer::address_of(&creator);
        let owner0_addr = Signer::address_of(&owner0);
        let owner1_addr = Signer::address_of(&owner1);

        offer(&creator, owner0_addr, token_id, 1);
        assert!(Table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 1);
        offer(&creator, owner1_addr, token_id, 1);
        assert!(Table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 2, 2);
        claim(&owner0, creator_addr, token_id);
        assert!(Table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 3);
        claim(&owner1, creator_addr, token_id);
        assert!(Table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 0, 4);

        offer(&owner0, owner1_addr, token_id, 1);
        claim(&owner1, owner0_addr, token_id);

        offer(&owner1, creator_addr, token_id, 1);
        offer(&owner1, creator_addr, token_id, 1);
        claim(&creator, owner1_addr, token_id);
    }

    #[test_only]
    fun create_token(creator: &signer, amount: u64): TokenId {
        use Std::ASCII::{String, Self};

        let collection_name = ASCII::string(b"Hello, World");
        TokenV1::create_collection(
            creator,
            *&collection_name,
            ASCII::string(b"Collection: Hello, World"),
            ASCII::string(b"https://aptos.dev"),
            amount,
            vector<bool>[false, false, false]
        );

        let default_keys = vector<String>[ASCII::string(b"attack"), ASCII::string(b"num_of_use")];
        let default_vals = vector<vector<u8>>[b"10", b"5"];
        let default_types = vector<String>[ASCII::string(b"integer"), ASCII::string(b"integer")];
        let mutate_setting = vector<bool>[false, false, false, false, false];

        TokenV1::create_token(
            creator,
            *&collection_name,
            ASCII::string(b"Token: Hello, Token"),
            ASCII::string(b"Hello, Token"),
            amount,
            amount,
            ASCII::string(b"https://aptos.dev"),
            Signer::address_of(creator),
            100,
            0,
            mutate_setting,
            default_keys,
            default_vals,
            default_types,
        )
    }
}
