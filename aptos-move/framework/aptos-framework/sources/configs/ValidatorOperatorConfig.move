/// Stores the string name of a ValidatorOperator account.
module AptosFramework::ValidatorOperatorConfig {
    use Std::Errors;
    use Std::Signer;
    use AptosFramework::Timestamp;

    struct ValidatorOperatorConfig has key {
        /// The human readable name of this entity. Immutable.
        human_name: vector<u8>,
    }

    /// The `ValidatorOperatorConfig` was not in the required state
    const EVALIDATOR_OPERATOR_CONFIG: u64 = 0;

    public fun publish(
        validator_operator_account: &signer,
        human_name: vector<u8>,
    ) {
        Timestamp::assert_operating();

        assert!(
            !has_validator_operator_config(Signer::address_of(validator_operator_account)),
            Errors::already_published(EVALIDATOR_OPERATOR_CONFIG)
        );

        move_to(validator_operator_account, ValidatorOperatorConfig {
            human_name,
        });
    }

    /// Get validator's account human name
    /// Aborts if there is no ValidatorOperatorConfig resource
    public fun get_human_name(validator_operator_addr: address): vector<u8> acquires ValidatorOperatorConfig {
        assert!(has_validator_operator_config(validator_operator_addr), Errors::not_published(EVALIDATOR_OPERATOR_CONFIG));
        *&borrow_global<ValidatorOperatorConfig>(validator_operator_addr).human_name
    }

    public fun has_validator_operator_config(validator_operator_addr: address): bool {
        exists<ValidatorOperatorConfig>(validator_operator_addr)
    }
}
