/// An example of an on-chain raffle using randomness
///
/// This example requires CLI version 3.1.0 or later.
module raffle::raffle {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::randomness;
    use aptos_std::smart_vector;
    use aptos_std::smart_vector::SmartVector;
    use aptos_framework::coin::Coin;
    use std::signer;

    // We need this friend declaration so our tests can call `init_module`.
    friend raffle::raffle_test;

    /// Error code for when a user tries to initate the drawing but no users bought any tickets.
    const E_NO_TICKETS: u64 = 2;

    /// Error code for when the somebody tries to draw an already-closed raffle
    const E_RAFFLE_HAS_CLOSED: u64 = 3;

    /// The minimum price of a raffle ticket, in APT.
    const TICKET_PRICE: u64 = 10_000;

    /// A raffle: a list of users who bought tickets.
    struct Raffle has key {
        // A list of users who bought raffle tickets (repeats allowed).
        tickets: SmartVector<address>,
        coins: Coin<AptosCoin>,
        is_closed: bool,
    }

    /// Initializes the `Raffle` resource, which will maintain the list of raffle tickets bought by users.
    fun init_module(deployer: &signer) {
        move_to(
            deployer,
            Raffle {
                tickets: smart_vector::empty(),
                coins: coin::zero(),
                is_closed: false,
            }
        );
    }

    #[test_only]
    public(friend) fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    /// The price of buying a raffle ticket.
    public fun get_ticket_price(): u64 { TICKET_PRICE }

    /// Any user can call this to purchase a ticket in the raffle.
    public entry fun buy_a_ticket(user: &signer) acquires Raffle {
        let raffle = borrow_global_mut<Raffle>(@raffle);

        // Charge the price of a raffle ticket from the user's balance, and
        // accumulate it into the raffle's bounty.
        let coins = coin::withdraw<AptosCoin>(user, TICKET_PRICE);
        coin::merge(&mut raffle.coins, coins);

        // Issue a ticket for that user
        smart_vector::push_back(&mut raffle.tickets, signer::address_of(user))
    }

    #[randomness]
    /// Can only be called as a top-level call from a TXN, preventing **test-and-abort** attacks (see
    /// [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    entry fun randomly_pick_winner() acquires Raffle {
        randomly_pick_winner_internal();
    }

    /// Insecurely wraps around `randomly_pick_winner_internal` allowing this function to
    /// be called from a Move script or another module, leaving it vulnerable to
    /// **test-and-abort** attacks (see [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    ///
    /// Commented out for security.
    //public fun randomly_pick_winner() acquires Raffle, Credentials {
    //    randomly_pick_winner_internal();
    //}

    /// Allows anyone to close the raffle (if enough time has elapsed & more than
    /// 1 user bought tickets) and to draw a random winner.
    public(friend) fun randomly_pick_winner_internal(): address acquires Raffle {
        let raffle = borrow_global_mut<Raffle>(@raffle);
        assert!(!raffle.is_closed, E_RAFFLE_HAS_CLOSED);
        assert!(!smart_vector::is_empty(&raffle.tickets), E_NO_TICKETS);

        // Pick a random winner in [0, |raffle.tickets|)
        let winner_idx = randomness::u64_range(0, smart_vector::length(&raffle.tickets));
        let winner = *smart_vector::borrow(&raffle.tickets, winner_idx);

        // Pay the winner
        let coins = coin::extract_all(&mut raffle.coins);
        coin::deposit<AptosCoin>(winner, coins);
        raffle.is_closed = true;

        winner
    }
}
