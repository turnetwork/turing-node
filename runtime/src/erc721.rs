use parity_codec::Encode;
/// A simple implementation of the ERC721, not include ERC165
use rstd::prelude::Vec;
use runtime_primitives::traits::{Hash, Zero};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ERC721 {
        OwnerOf get(owner_of) : map T::Hash => Option<T::AccountId>;
        Balances get(balance_of): map T::AccountId => u64;

        // Used to query approval
        Approvals get(get_approved): map T::Hash => Option<T::AccountId>;
        OperatorApprovals get(is_approved_for_all): map (T::AccountId, T::AccountId) => bool;

        // Optional metadata
        Name get(name) config(): Vec<u8>;
        Symbol get(symbol) config(): Vec<u8>;
        Decimal get(decimal) : u16 = 18;

        // Optional ERC721Enumerable
        TotalSupply get(total_supply): u64;
        Tokens get(token_by_index): map u64 => T::Hash;
        OwnedTokens get(token_of_owner_by_index): map (T::AccountId, u64) => T::Hash;

        // Not a part of the ERC721 specification, but used for ERC721Enumerable
        TokensIndex: map T::Hash => u64;
        OwnedTokensIndex: map T::Hash => u64;

        // Not a part of the ERC721 specification, but used in random token generation
        Nonce: u64;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event<T>() = default;

        /// Transfers the ownership of an NFT from one address to another by allowance
        fn transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
            // Need to be authorized first
            let caller = ensure_signed(origin)?;
            ensure!(Self::is_approved(caller, token_id), "You can not transfer this token");

            Self::transfer_from_impl(from, to, token_id)?;

            Ok(())
        }

        /// Check follows (Etherum):
        /// 1. sender is the current owner, an authorized operator, or the approved address for this NFT.
        /// 2. 'from' is the owner of the NFT.
        /// 3. 'token_id' is a valid NFT.
        /// 4. 'to' is not zero address.
        /// 5. if 'to' is a smart contract, calls 'onERC721Receive'.
        /// But this function is not exactly the same as Ethereum
        fn safe_transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
            let balance_to = <balances::Module<T>>::free_balance(&to);
            ensure!(!balance_to.is_zero(), "'to' does not satisfy the `ExistentialDeposit` requirement");

            Self::transfer_from(origin, from, to, token_id)?;

            Ok(())
        }

        // fn safe_transfer_from(origin, from: T::AccountId, to: T::AccountId, token_id: T::Hash, data: Vec<u8>) -> Result{}

        /// Approve the passed address to spend the specified amount of tokens on the behalf of the message's sender.
        fn approve(origin, spender: T::AccountId, token_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;
            let owner = Self::owner_of(token_id)
               .ok_or("No owner for this token")?;

            ensure!(spender != owner, "Owner is implicitly approved");
            ensure!(sender == owner || Self::is_approved_for_all((owner.clone(), sender.clone())), "You are not allowed to approve for this token");

            <Approvals<T>>::insert(token_id, spender.clone());

            Self::deposit_event(RawEvent::Approval(owner, spender, token_id));

            Ok(())
        }

        fn set_approve_for_all(origin, to: T::AccountId, approved: bool) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(to != sender, "You are already implicity approved for your own actions");
            <OperatorApprovals<T>>::insert((sender.clone(), to.clone()), approved);

            Self::deposit_event(RawEvent::ApprovalForAll(sender, to, approved));

            Ok(())
        }

        // Not part of ERC721, but allows you to play with the runtime
        fn create_token(origin) -> Result {
            let sender = ensure_signed(origin)?;
            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), sender.clone(), nonce).using_encoded(<T as system::Trait>::Hashing::hash);

            Self::mint(sender, random_hash)?;
            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Transfer(Option<AccountId>, Option<AccountId>, Hash),
        Approval(AccountId, AccountId, Hash),
        ApprovalForAll(AccountId, AccountId, bool),
    }
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    /// internal transfer_from function
    fn transfer_from_impl(from: T::AccountId, to: T::AccountId, token_id: T::Hash) -> Result {
        // Check first
        let owner = Self::owner_of(token_id).ok_or("'token_id' is a invalid NFT")?;

        ensure!(owner == from, "'from' is not the owner of the NFT.");

        let balance_of_from = Self::balance_of(&from);
        let balance_of_to = Self::balance_of(&to);

        let new_balance_of_from = balance_of_from
            .checked_sub(1)
            .ok_or("Transfer causes underflow of 'from' token balance")?;
        let new_balance_of_to = balance_of_to
            .checked_add(1)
            .ok_or("Transfer causes overflow of 'to' token balance")?;

        // Write later
        Self::remove_token_from_owner_enumeration(from.clone(), token_id)?;
        Self::add_token_to_owner_enumeration(to.clone(), token_id)?;
        Self::remove_approval(token_id)?;

        <Balances<T>>::insert(from.clone(), new_balance_of_from);
        <Balances<T>>::insert(to.clone(), new_balance_of_to);
        <OwnerOf<T>>::insert(token_id.clone(), to.clone());

        Self::deposit_event(RawEvent::Transfer(Some(from), Some(to), token_id));

        Ok(())
    }

    fn remove_approval(token_id: T::Hash) -> Result {
        <Approvals<T>>::remove(token_id);

        Ok(())
    }

    fn is_approved(caller: T::AccountId, token_id: T::Hash) -> bool {
        let owner = Self::owner_of(&token_id);
        let approved_user = Self::get_approved(&token_id);

        let approved_as_owner = match owner {
            Some(ref o) => o == &caller,
            None => false,
        };

        let approved_as_delegate = match owner {
            Some(d) => Self::is_approved_for_all((d, caller.clone())),
            None => false,
        };

        let approved_as_user = match approved_user {
            Some(u) => u == caller,
            None => false,
        };

        return approved_as_owner || approved_as_user || approved_as_delegate;
    }

    // Start ERC721 : Enumerable : Internal Functions //
    fn add_token_to_owner_enumeration(to: T::AccountId, token_id: T::Hash) -> Result {
        let new_token_index = Self::balance_of(&to);

        <OwnedTokensIndex<T>>::insert(token_id.clone(), new_token_index);
        <OwnedTokens<T>>::insert((to, new_token_index), token_id);

        Ok(())
    }

    fn add_token_to_all_tokens_enumeration(token_id: T::Hash) -> Result {
        let total_supply = Self::total_supply();

        let new_total_supply = total_supply
            .checked_add(1)
            .ok_or("Overflow when adding new token to total supply")?;

        let new_token_index = total_supply;

        <TokensIndex<T>>::insert(token_id.clone(), new_token_index);
        <Tokens<T>>::insert(new_token_index, token_id);
        <TotalSupply<T>>::put(new_total_supply);

        Ok(())
    }

    fn remove_token_from_owner_enumeration(from: T::AccountId, token_id: T::Hash) -> Result {
        let balance_of_from = Self::balance_of(&from);
        let last_token_index = balance_of_from
            .checked_sub(1)
            .ok_or("Underflow in subtracting 'from' token balance")?;
        let token_index = <OwnedTokensIndex<T>>::get(&token_id);

        // Swap and pop
        if token_index != last_token_index {
            let last_token_id = <OwnedTokens<T>>::get((from.clone(), last_token_index));
            <OwnedTokens<T>>::insert((from.clone(), token_index), last_token_id);
            <OwnedTokensIndex<T>>::insert(last_token_id, token_index);
        }

        <OwnedTokens<T>>::remove((from, last_token_index));
        <OwnedTokensIndex<T>>::remove(token_id);

        Ok(())
    }
    // End ERC721 : Enumerable : Internal Functions //

    /// Internal function to mint a new token.
    fn mint(to: T::AccountId, token_id: T::Hash) -> Result {
        ensure!(
            !<OwnerOf<T>>::exists(token_id),
            "ERC721: token already minted"
        );

        let balance_of = Self::balance_of(&to);

        let new_balance_of = balance_of
            .checked_add(1)
            .ok_or("Overflow adding a new token to account balance")?;

        Self::add_token_to_all_tokens_enumeration(token_id)?;
        Self::add_token_to_owner_enumeration(to.clone(), token_id)?;

        <OwnerOf<T>>::insert(token_id.clone(), to.clone());
        <Balances<T>>::insert(to.clone(), new_balance_of);

        Self::deposit_event(RawEvent::Transfer(None, Some(to), token_id));

        Ok(())
    }
}
