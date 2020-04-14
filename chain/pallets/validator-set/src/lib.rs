//! # Validator Set Pallet
//!
//! The Validator Set Pallet provides functionality to add/remove validators in a Substrate-based
//! PoA network.
//!
//! The pallet is based on the Substrate session pallet and implements related traits for session
//! management when validators are added or removed.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, StorageValue, traits::EstimateNextSessionRotation};
use sp_runtime::traits::Convert;
use sp_std::prelude::*;
use system::{self as system, ensure_root};

pub trait Trait: system::Trait + session::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as ValidatorSet {
        pub Validators get(fn validators) config(): Option<Vec<T::AccountId>>;
        Flag get(fn flag): bool;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        // New validator added.
        ValidatorAdded(AccountId),

        // Validator removed.
        ValidatorRemoved(AccountId),
    }
);

decl_error! {
    /// Errors for the module.
    pub enum Error for Module<T: Trait> {
        NoValidators,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Add a new validator using root/sudo privileges.
        ///
        /// New validator's session keys should be set in session module before calling this.
        #[weight = frame_support::weights::SimpleDispatchInfo::default()]
        pub fn add_validator(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
            validators.push(validator_id.clone());
            <Validators<T>>::put(validators);
            // Calling rotate_session to queue the new session keys.
            <session::Module<T>>::rotate_session();
            Self::deposit_event(RawEvent::ValidatorAdded(validator_id));

            // Triggering rotate session again for the queued keys to take effect.
            Flag::put(true);
            Ok(())
        }

        /// Remove a validator using root/sudo privileges.
        #[weight = frame_support::weights::SimpleDispatchInfo::default()]
        pub fn remove_validator(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
            // Assuming that this will be a PoA network for enterprise use-cases,
            // the validator count may not be too big; the for loop shouldn't be too heavy.
            // In case the validator count is large, we need to find another way.
            for (i, v) in validators.clone().into_iter().enumerate() {
                if v == validator_id {
                    validators.swap_remove(i);
                }
            }
            <Validators<T>>::put(validators);
            // Calling rotate_session to queue the new session keys.
            <session::Module<T>>::rotate_session();
            Self::deposit_event(RawEvent::ValidatorRemoved(validator_id));

            // Triggering rotate session again for the queued keys to take effect.
            Flag::put(true);
            Ok(())
        }
    }
}

/// Indicates to the session module if the session should be rotated.
/// We set this flag to true when we add/remove a validator.
impl<T: Trait> session::ShouldEndSession<T::BlockNumber> for Module<T> {
    fn should_end_session(_now: T::BlockNumber) -> bool {
        Self::flag()
    }
}

/// Something that can predict the next session rotation. This should typically come from the
/// same logical unit that provides [`ShouldEndSession`], yet, it gives a best effort estimate.
impl<T: Trait> EstimateNextSessionRotation<T::BlockNumber> for Module<T> {
    fn estimate_next_session_rotation(now: T::BlockNumber) -> Option<T::BlockNumber> {
        let now_block_number = <system::Module<T>>::block_number();
        Some(now_block_number)
    }
}

type SessionIndex = u32;
/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Trait> session::SessionManager<T::AccountId> for Module<T> {
    fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
        // Flag is set to false so that the session doesn't keep rotating.
        Flag::put(false);

        Self::validators()
    }

    fn start_session(_: SessionIndex) {}
    fn end_session(_: SessionIndex) {}
}

/// Implementation of Convert trait for mapping ValidatorId with AccountId.
/// This is mainly used to map stash and controller keys.
/// In this module, for simplicity, we just return the same AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
    fn convert(account: T::AccountId) -> Option<T::AccountId> {
        Some(account)
    }
}
