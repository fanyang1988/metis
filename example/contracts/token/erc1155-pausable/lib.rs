#![cfg_attr(not(feature = "std"), no_std)]

#[metis_lang::contract]
pub mod contract {
    use ink_prelude::{
        string::String,
        vec::Vec,
    };
    use metis_erc1155 as erc1155;
    pub use metis_erc1155::{
        Error,
        Result,
        TokenId,
    };
    use metis_lang::{
        import,
        metis,
    };
    use metis_ownable as ownable;
    use metis_pausable as pausable;

    #[ink(storage)]
    #[import(erc1155, ownable, pausable)]
    pub struct Erc1155 {
        erc1155: erc1155::Data<Erc1155>,
        ownable: ownable::Data<Erc1155>,
        pausable: pausable::Data,
    }

    // TODO: gen by marco with Erc1155 component
    impl erc1155::pausable::Impl<Erc1155> for Erc1155 {
        fn _before_token_transfer(
            &mut self,
            _operator: &AccountId,
            _from: &Option<&AccountId>,
            _to: &Option<&AccountId>,
            _ids: &Vec<TokenId>,
            _amounts: &Vec<Balance>,
            _data: &Vec<u8>,
        ) -> Result<()> {
            Ok(())
        }
    }

    /// Emitted when `value` tokens of token type `id` are
    /// transferred from `from` to `to` by `operator`.
    #[ink(event)]
    #[metis(erc1155)]
    pub struct TransferSingle {
        #[ink(topic)]
        pub operator: AccountId,
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub id: TokenId,
        pub value: Balance,
    }

    /// @dev Equivalent to multiple {TransferSingle} events,
    /// where `operator`, `from` and `to` are the same for all
    /// transfers.
    #[ink(event)]
    #[metis(erc1155)]
    pub struct TransferBatch {
        #[ink(topic)]
        pub operator: AccountId,
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        pub id: Vec<TokenId>,
        pub value: Vec<Balance>,
    }

    /// Emitted when `owner` enables or disables (`approved`) `operator` to manage all of its assets.
    #[ink(event)]
    #[metis(erc1155)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        pub owner: AccountId,
        #[ink(topic)]
        pub operator: AccountId,
        pub approved: bool,
    }

    /// @dev Emitted when the URI for token type `id` changes to `value`, if it is a non-programmatic URI.
    ///
    /// If an {URI} event was emitted for `id`, the standard
    /// https://eips.ethereum.org/EIPS/eip-1155#metadata-extensions[guarantees] that `value` will equal the value
    /// returned by {IERC1155MetadataURI-uri}.
    #[ink(event)]
    #[metis(erc1155)]
    pub struct Url {
        pub value: String,
        #[ink(topic)]
        pub id: TokenId,
    }

    /// Event emitted when Owner AccountId Transferred
    #[ink(event)]
    #[metis(ownable)]
    pub struct OwnershipTransferred {
        /// previous owner account id
        #[ink(topic)]
        previous_owner: Option<AccountId>,
        /// new owner account id
        #[ink(topic)]
        new_owner: Option<AccountId>,
    }

    /// Event emitted when Pause
    #[ink(event)]
    #[metis(pausable)]
    pub struct Paused {
        /// paused caller
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when unPause
    #[ink(event)]
    #[metis(pausable)]
    pub struct Unpaused {
        /// unpaused caller
        #[ink(topic)]
        account: AccountId,
    }

    // for test message
    impl Erc1155 {
        /// For test to mint
        #[ink(message)]
        pub fn mint_test(
            &mut self,
            to: AccountId,
            id: TokenId,
            amount: Balance,
            data: Vec<u8>,
        ) -> Result<()> {
            erc1155::Impl::_mint(self, to, id, amount, data)
        }

        /// For test to burn
        #[ink(message)]
        pub fn burn_test(
            &mut self,
            account: AccountId,
            id: TokenId,
            amount: Balance,
        ) -> Result<()> {
            erc1155::Impl::_burn(self, account, id, amount)
        }
    }

    // impl
    impl Erc1155 {
        #[ink(constructor)]
        pub fn new(url: String) -> Self {
            let mut instance = Self {
                erc1155: erc1155::Data::new(),
                ownable: ownable::Data::new(),
                pausable: pausable::Data::new(),
            };

            erc1155::Impl::init(&mut instance, url);
            ownable::Impl::init(&mut instance);
            pausable::Impl::init(&mut instance);

            instance
        }

        /// @dev See {IERC1155MetadataURI-uri}.
        ///
        /// This implementation returns the same URI for *all* token types. It relies
        /// on the token type ID substitution mechanism
        /// https://eips.ethereum.org/EIPS/eip-1155#metadata[defined in the EIP].
        ///
        /// Clients calling this function must replace the `\{id\}` substring with the
        /// actual token type ID.
        #[ink(message)]
        pub fn url(&self, token_id: TokenId) -> String {
            erc1155::Impl::url(self, token_id)
        }

        /// @dev See {IERC1155-balanceOf}.
        ///
        /// Requirements:
        ///
        /// - `account` cannot be the zero address.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, id: TokenId) -> Balance {
            erc1155::Impl::balance_of(self, &owner, &id)
        }

        /// @dev See {IERC1155-balanceOfBatch}.
        ///
        /// Requirements:
        ///
        /// - `accounts` and `ids` must have the same length.
        #[ink(message)]
        pub fn balance_of_batch(
            &self,
            accounts: Vec<AccountId>,
            ids: Vec<TokenId>,
        ) -> Vec<Balance> {
            erc1155::Impl::balance_of_batch(self, accounts, ids)
        }

        /// @dev See {IERC1155-isApprovedForAll}.
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            erc1155::Impl::is_approved_for_all(self, &owner, &operator)
        }

        /// @dev See {IERC1155-setApprovalForAll}.
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) {
            erc1155::Impl::set_approval_for_all(self, operator, approved)
        }

        /// @dev See {IERC1155-safeTransferFrom}.
        #[ink(message)]
        pub fn safe_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
            amount: Balance,
            data: Vec<u8>,
        ) -> Result<()> {
            erc1155::Impl::safe_transfer_from(self, from, to, id, amount, data)
        }

        /// @dev See {IERC1155-safeBatchTransferFrom}.
        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Vec<TokenId>,
            amount: Vec<Balance>,
            data: Vec<u8>,
        ) -> Result<()> {
            erc1155::Impl::safe_batch_transfer_from(self, from, to, id, amount, data)
        }

        // Owner messages

        /// Return the owner AccountId
        #[ink(message)]
        pub fn get_ownership(&self) -> Option<AccountId> {
            *ownable::Impl::owner(self)
        }

        /// Leaves the contract without owner. It will not be possible to call
        /// `ensure_xxx` functions anymore. Can only be called by the current owner.
        /// NOTE: Renouncing ownership will leave the contract without an owner,
        /// thereby removing any functionality that is only available to the owner.
        #[ink(message)]
        pub fn renounce_ownership(&mut self) {
            ownable::Impl::renounce_ownership(self)
        }

        /// Transfers ownership of the contract to a new account (`new_owner`).
        /// Can only be called by the current owner.
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) {
            ownable::Impl::transfer_ownership(self, &new_owner)
        }

        // Pausable messages

        /// Returns true if the contract is paused, and false otherwise
        #[ink(message)]
        pub fn paused(&self) -> bool {
            pausable::Impl::paused(self)
        }

        /// Returns to normal state.
        ///
        /// Requirements:
        ///
        /// - The contract must be paused.
        #[ink(message)]
        pub fn pause(&mut self) {
            ownable::Impl::ensure_caller_is_owner(self);
            pausable::Impl::_pause(self)
        }

        /// Transfers ownership of the contract to a new account (`new_owner`).
        /// Can only be called by the current owner.
        #[ink(message)]
        pub fn unpause(&mut self) {
            ownable::Impl::ensure_caller_is_owner(self);
            pausable::Impl::_unpause(self)
        }
    }
}
