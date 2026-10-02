//! # Template Pallet
//!
//! A pallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pallet. It is typically used in beginner tutorials or in Substrate template
//! nodes as a starting point for creating a new pallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pallet section is annotated with an attribute using the `#[pallet::...]` procedural macro.
//! This macro generates the necessary code for a pallet to be aggregated into a FRAME runtime.
//!
//! Learn more about FRAME macros [here](https://docs.substrate.io/reference/frame-macros/).
//!
//! ### Pallet Sections
//!
//! The pallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pallet depends on
//!   (denoted by the `#[pallet::config]` attribute). See: [`Config`].
//! - A **means to store pallet-specific data** (denoted by the `#[pallet::storage]` attribute).
//!   See: [`storage_types`].
//! - A **declaration of the events** this pallet emits (denoted by the `#[pallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pallet can throw (denoted by the `#[pallet::error]`
//!   attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pallet's functionality (denoted by the
//!   `#[pallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pallet-template --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// FRAME pallets require their own "mock runtimes" to be able to run unit tests. This module
// contains a mock runtime specific for testing this pallet's functionality.
#[cfg(test)]
mod mock;

// This module contains the unit tests for this pallet.
// Learn about pallet unit testing here: https://docs.substrate.io/test/unit-testing/
#[cfg(test)]
mod tests;

// Every callable function or "dispatchable" a pallet exposes must have weight values that correctly
// estimate a dispatchable's execution time. The benchmarking module is used to calculate weights
// for each dispatchable and generates this pallet's weight.rs file. Learn more about benchmarking here: https://docs.substrate.io/test/benchmark/
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	// The `Pallet` struct serves as a placeholder to implement traits, methods and dispatchables
	// (`Call`s) in this pallet.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	///
	/// All our types and constants a pallet depends on must be declared here.
	/// These types are defined generically and made concrete when the pallet is declared in the
	/// `runtime/src/lib.rs` file of your chain.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// Maximum number of signers allowed in one multisig account.
		#[pallet::constant]
		type MaxSigners: Get<u32>;
	}

	/// A storage item for this pallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.substrate.io/build/runtime-storage/>
	#[pallet::storage]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	pub type MultisigAccounts<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    u32,
    (BoundedVec<T::AccountId, T::MaxSigners>, u32),
    OptionQuery,
	>;
	/// The ID to allocate to the next multisig configuration.
	#[pallet::storage]
	pub type NextMultisigId<T> = StorageValue<_, u32, ValueQuery>;
	/// Proposer and action hash, indexed by multisig ID and proposal ID.
	#[pallet::storage]
	pub type PendingProposals<T: Config> = StorageDoubleMap<
	    _,
	    Blake2_128Concat,
	    u32,
	    Blake2_128Concat,
	    u32,
	    (T::AccountId, T::Hash),
	    OptionQuery,
	>;
	/// The next proposal ID for each multisig.
	#[pallet::storage]
	pub type NextProposalId<T> = StorageMap<
	    _,
	    Blake2_128Concat,
	    u32,
	    u32,
	    ValueQuery,
	>;
	/// Accounts that approved a proposal.
	#[pallet::storage]
	pub type ApprovalVotes<T: Config> = StorageDoubleMap<
	    _,
	    Blake2_128Concat,
	    u32,
	    Blake2_128Concat,
	    u32,
	    BoundedVec<T::AccountId, T::MaxSigners>,
	    ValueQuery,
	>;
	/// Events emitted by this pallet.
	///
	/// The `generate_deposit` macro generates `deposit_event`, which converts this pallet's
	/// events into `RuntimeEvent` and records them through `frame_system::Pallet::deposit_event`.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user has successfully set a new value.
		SomethingStored {
			/// The new value set.
			something: u32,
			/// The account who set the new value.
			who: T::AccountId,
		},
		/// A multisig configuration was created.
		MultisigCreated {
    	/// The ID of the new multisig.
    	multisig_id: u32,
    	/// The approval threshold for the new multisig.
    	threshold: u32,
		},
		/// A signer opened a proposal.
		ProposalOpened {
		    multisig_id: u32,
		    proposal_id: u32,
		    proposer: T::AccountId,
		    action_hash: T::Hash,
		},
		/// A signer approved a proposal.
		ProposalApproved {
		    multisig_id: u32,
		    proposal_id: u32,
		    approver: T::AccountId,
		},
	}

	/// Errors that can be returned by this pallet.
	///
	/// Errors tell users that something went wrong so it's important that their naming is
	/// informative. Similar to events, error documentation is added to a node's metadata so it's
	/// equally important that they have helpful documentation associated with them.
	///
	/// This type of runtime error can be up to 4 bytes in size should you want to return additional
	/// information.
	#[pallet::error]
	pub enum Error<T> {
		/// The value retrieved was `None` as no value was previously set.
		NoneValue,
		/// There was an attempt to increment the value in storage over `u32::MAX`.
		StorageOverflow,
		/// The approval threshold is zero or exceeds the number of signers.
		InvalidThreshold,
		/// The same account appears more than once in the signer list.
		DuplicateSigner,
		/// No further multisig IDs can be allocated.
		MultisigIdOverflow,
		/// The caller is not included in the signer list.
		NotASigner,
		/// The requested multisig does not exist.
		MultisigNotFound,
		/// No further proposal IDs can be allocated for this multisig.
		ProposalIdOverflow,
		/// The requested proposal does not exist.
		ProposalNotFound,
			
		/// This signer has already approved the proposal.
		AlreadyApproved,
			
		/// The approval list has reached its maximum length.
		TooManyApprovals,

		/// The proposal has fewer approvals than required.
		NotEnoughApprovals,

		/// The action does not match the stored proposal.
		ActionHashMismatch,
	}

	/// The pallet's dispatchable functions ([`Call`]s).
	///
	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// They must always return a `DispatchResult` and be annotated with a weight and call index.
	///
	/// The [`call_index`] macro is used to explicitly
	/// define an index for calls in the [`Call`] enum. This is useful for pallets that may
	/// introduce new dispatchables over time. If the order of a dispatchable changes, its index
	/// will also change which will break backwards compatibility.
	///
	/// The [`weight`] macro is used to assign a weight to each call.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a single u32 value as a parameter, writes the value
		/// to storage and emits an event.
		///
		/// It checks that the _origin_ for this call is _Signed_ and returns a dispatch
		/// error if it isn't. Learn more about origins here: <https://docs.substrate.io/build/origins/>
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// Update storage.
			Something::<T>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });

			// Return a successful `DispatchResult`
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		///
		/// It checks that the caller is a signed origin and reads the current value from the
		/// `Something` storage item. If a current value exists, it is incremented by 1 and then
		/// written back to storage.
		///
		/// ## Errors
		///
		/// The function will return an error under the following conditions:
		///
		/// - If no value has been set ([`Error::NoneValue`])
		/// - If incrementing the value in storage causes an arithmetic overflow
		///   ([`Error::StorageOverflow`])
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::cause_error())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match Something::<T>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage. This will cause an error in the event
					// of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					Something::<T>::put(new);
					Ok(())
				},
			}
		}
			/// Create a multisig configuration with the caller as one of its signers.
		#[pallet::call_index(2)]
		#[pallet::weight(
		    Weight::from_parts(100_000_000, 10_000)
		        .saturating_add(T::DbWeight::get().reads_writes(1, 2))
		)]
		pub fn create_multisig(
		    origin: OriginFor<T>,
		    signers: BoundedVec<T::AccountId, T::MaxSigners>,
		    threshold: u32,
		) -> DispatchResult {
		    let who = ensure_signed(origin)?;
		
		    ensure!(
		        signers.contains(&who),
		        Error::<T>::NotASigner
		    );
		
		    Self::do_create_multisig(signers, threshold)
		}
		/// Open a proposal as an authorised signer.
		#[pallet::call_index(3)]
		#[pallet::weight(
		    Weight::from_parts(100_000_000, 10_000)
		        .saturating_add(T::DbWeight::get().reads_writes(2, 2))
		)]
		pub fn propose(
		    origin: OriginFor<T>,
		    multisig_id: u32,
		    action_hash: T::Hash,
		) -> DispatchResult {
		    let proposer = ensure_signed(origin)?;
		
		    Self::do_propose(multisig_id, proposer, action_hash)
		}

		/// Approve an existing proposal as an authorised signer.
		#[pallet::call_index(4)]
		#[pallet::weight(
		    Weight::from_parts(100_000_000, 10_000)
		        .saturating_add(T::DbWeight::get().reads_writes(3, 1))
		)]
		pub fn approve(
		    origin: OriginFor<T>,
		    multisig_id: u32,
		    proposal_id: u32,
		) -> DispatchResult {
		    let approver = ensure_signed(origin)?;
		
		    Self::do_approve(multisig_id, proposal_id, approver)
		}
	}
	impl<T: Config> Pallet<T> {
    	pub(crate) fn validate_threshold(
    	    signers: &BoundedVec<T::AccountId, T::MaxSigners>,
    	    threshold: u32,
    	) -> DispatchResult {
    	    ensure!(
    	        threshold > 0 && threshold as usize <= signers.len(),
    	        Error::<T>::InvalidThreshold
    	    );

    	    Ok(())
    	}
		pub(crate) fn validate_unique_signers(
    		signers: &BoundedVec<T::AccountId, T::MaxSigners>,
		) -> DispatchResult {
			for (index, signer) in signers.iter().enumerate() {
        		ensure!(
            		!signers[..index].contains(signer),
            		Error::<T>::DuplicateSigner
        	);
    	}

    	Ok(())
	}
		pub(crate) fn do_create_multisig(
	    signers: BoundedVec<T::AccountId, T::MaxSigners>,
	    threshold: u32,
	) -> DispatchResult {
	    // Reject invalid configurations before changing storage.
	    Self::validate_threshold(&signers, threshold)?;
	    Self::validate_unique_signers(&signers)?;

	    // Calculate the next counter value without allowing overflow.
	    let multisig_id = NextMultisigId::<T>::get();
	    let next_id = multisig_id
	        .checked_add(1)
	        .ok_or(Error::<T>::MultisigIdOverflow)?;

	    // All checks passed: save the configuration and advance the counter.
	    MultisigAccounts::<T>::insert(multisig_id, (signers, threshold));
	    NextMultisigId::<T>::put(next_id);
		Self::deposit_event(Event::MultisigCreated {
		    multisig_id,
		    threshold,
		});
	    Ok(())
		}
		pub(crate) fn do_propose(
	    multisig_id: u32,
	    proposer: T::AccountId,
	    action_hash: T::Hash,
	) -> DispatchResult {
	    let (signers, _threshold) = MultisigAccounts::<T>::get(multisig_id)
	        .ok_or(Error::<T>::MultisigNotFound)?;
	
	    ensure!(
	        signers.contains(&proposer),
	        Error::<T>::NotASigner
	    );
	
	    let proposal_id = NextProposalId::<T>::get(multisig_id);
	    let next_id = proposal_id
	        .checked_add(1)
	        .ok_or(Error::<T>::ProposalIdOverflow)?;
	
		PendingProposals::<T>::insert(
    		multisig_id,
    		proposal_id,
    		(proposer.clone(), action_hash),
		);
	    NextProposalId::<T>::insert(multisig_id, next_id);
		Self::deposit_event(Event::ProposalOpened {
    		multisig_id,
    		proposal_id,
    		proposer,
    		action_hash,
		});
	    Ok(())
	}
		pub(crate) fn do_approve(
	    multisig_id: u32,
	    proposal_id: u32,
	    approver: T::AccountId,
	) -> DispatchResult {
	    let (signers, _threshold) = MultisigAccounts::<T>::get(multisig_id)
	        .ok_or(Error::<T>::MultisigNotFound)?;
	
	    ensure!(
	        PendingProposals::<T>::contains_key(multisig_id, proposal_id),
	        Error::<T>::ProposalNotFound
	    );
	
	    ensure!(
	        signers.contains(&approver),
	        Error::<T>::NotASigner
	    );
	
	    let mut approvals = ApprovalVotes::<T>::get(multisig_id, proposal_id);
	
	    ensure!(
	        !approvals.contains(&approver),
	        Error::<T>::AlreadyApproved
	    );
	
	    approvals
	        .try_push(approver.clone())
	        .map_err(|_| Error::<T>::TooManyApprovals)?;
	
	    ApprovalVotes::<T>::insert(multisig_id, proposal_id, approvals);
		Self::deposit_event(Event::ProposalApproved {
		    multisig_id,
		    proposal_id,
		    approver,
		});
	    Ok(())
		}
		pub(crate) fn ensure_enough_approvals(
		    multisig_id: u32,
		    proposal_id: u32,
		) -> DispatchResult {
		    let (_, threshold) = MultisigAccounts::<T>::get(multisig_id)
		        .ok_or(Error::<T>::MultisigNotFound)?;

		    ensure!(
		        PendingProposals::<T>::contains_key(multisig_id, proposal_id),
		        Error::<T>::ProposalNotFound
		    );

		    let approvals = ApprovalVotes::<T>::get(multisig_id, proposal_id);

		    ensure!(
		        approvals.len() >= threshold as usize,
		        Error::<T>::NotEnoughApprovals
		    );

		    Ok(())
		}
		pub(crate) fn ensure_matching_action(
	    multisig_id: u32,
	    proposal_id: u32,
	    action_hash: T::Hash,
	) -> DispatchResult {
	    let (_, stored_hash) =
	        PendingProposals::<T>::get(multisig_id, proposal_id)
	            .ok_or(Error::<T>::ProposalNotFound)?;

	    ensure!(
	        action_hash == stored_hash,
	        Error::<T>::ActionHashMismatch
	    );

	    Ok(())
	}
	}
}
