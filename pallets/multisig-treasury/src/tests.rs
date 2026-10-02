use crate::{mock::*, Error, Event, Something};
use frame_support::{assert_noop, assert_ok};

#[test]
fn veto_role_is_designated_per_multisig() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64, <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers.clone(), 2,
        ));
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));

        // Only test setup assigns the role here; there is no public setter yet.
        crate::VetoAuthorities::<Test>::insert(0, 9u64);
        assert_ok!(MultisigTreasury::ensure_veto_authority(RuntimeOrigin::signed(9), 0), 9);

        // Being a signer does not automatically grant veto authority.
        assert_noop!(
            MultisigTreasury::ensure_veto_authority(RuntimeOrigin::signed(1), 0),
            Error::<Test>::NotVetoAuthority
        );
        // Authority for multisig 0 does not extend to multisig 1.
        assert_noop!(
            MultisigTreasury::ensure_veto_authority(RuntimeOrigin::signed(9), 1),
            Error::<Test>::NotVetoAuthority
        );
        assert_noop!(
            MultisigTreasury::ensure_veto_authority(RuntimeOrigin::none(), 0),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn zero_proposal_lifetime_changes_nothing() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));

        // Simulate an invalid runtime configuration, only in this test.
        ProposalLifetime::set(0);
        assert_noop!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1), 0, sp_core::H256::repeat_byte(7),
            ),
            Error::<Test>::InvalidProposalLifetime
        );

        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
        assert!(!crate::NextProposalId::<Test>::contains_key(0));
        assert!(!crate::PendingProposals::<Test>::contains_key(0, 0));
        assert!(!crate::ProposalExpiry::<Test>::contains_key(0, 0));
        assert!(!crate::ApprovalVotes::<Test>::contains_key(0, 0));
        ProposalLifetime::set(100);
    });
}

#[test]
fn missing_deadline_blocks_approval_and_execution() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        System::set_block_number(10);
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));

        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );
        let action_hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_ok!(MultisigTreasury::approve(RuntimeOrigin::signed(1), 0, 0));
        assert_ok!(MultisigTreasury::approve(RuntimeOrigin::signed(2), 0, 0));

        // Simulate incomplete proposal data after collecting valid approvals.
        assert!(crate::ProposalExpiry::<Test>::contains_key(0, 0));
        crate::ProposalExpiry::<Test>::remove(0, 0);

        assert_noop!(
            MultisigTreasury::approve(RuntimeOrigin::signed(3), 0, 0),
            Error::<Test>::ProposalExpiryMissing
        );
        assert_noop!(
            MultisigTreasury::execute(RuntimeOrigin::signed(3), 0, 0, Box::new(call)),
            Error::<Test>::ProposalExpiryMissing
        );

        // Neither request changes storage or runs the action.
        assert_eq!(Something::<Test>::get(), None);
        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), Some((1, action_hash)));
        assert_eq!(crate::ApprovalVotes::<Test>::get(0, 0).as_slice(), &[1u64, 2u64]);
        assert!(!crate::ProposalExpiry::<Test>::contains_key(0, 0));
    });
}

#[test]
fn proposal_deadline_overflow_changes_nothing() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));

        // Adding the configured 100-block lifetime would exceed u64::MAX.
        System::set_block_number(u64::MAX - 50);
        assert_noop!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1), 0, sp_core::H256::repeat_byte(7),
            ),
            Error::<Test>::ProposalExpiryOverflow
        );

        // Rejection must not allocate an ID or leave partial proposal data.
        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
        assert!(!crate::NextProposalId::<Test>::contains_key(0));
        assert!(!crate::PendingProposals::<Test>::contains_key(0, 0));
        assert!(!crate::ProposalExpiry::<Test>::contains_key(0, 0));
        assert!(!crate::ApprovalVotes::<Test>::contains_key(0, 0));
    });
}

#[test]
fn approved_proposal_cannot_execute_at_its_deadline() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        System::set_block_number(10);
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));
        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );
        let action_hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_ok!(MultisigTreasury::approve(RuntimeOrigin::signed(1), 0, 0));
        assert_ok!(MultisigTreasury::approve(RuntimeOrigin::signed(2), 0, 0));
        assert_eq!(crate::ProposalExpiry::<Test>::get(0, 0), Some(110));

        // Enough approvals and a matching call are valid before expiry.
        System::set_block_number(109);
        assert_ok!(MultisigTreasury::ensure_ready_to_execute(0, 0, &call));

        // At the deadline, execution must fail without changing storage.
        System::set_block_number(110);
        assert_noop!(
            MultisigTreasury::execute(RuntimeOrigin::signed(3), 0, 0, Box::new(call)),
            Error::<Test>::ProposalExpired
        );
        assert_eq!(Something::<Test>::get(), None);
        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), Some((1, action_hash)));
        assert_eq!(crate::ApprovalVotes::<Test>::get(0, 0).as_slice(), &[1u64, 2u64]);
        assert_eq!(crate::ProposalExpiry::<Test>::get(0, 0), Some(110));
    });
}

#[test]
fn signer_cannot_approve_at_proposal_deadline() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");
        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));
        let action_hash = sp_core::H256::repeat_byte(7);
        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_eq!(crate::ProposalExpiry::<Test>::get(0, 0), Some(110));

        // Approval still works one block before expiry.
        System::set_block_number(109);
        assert_ok!(MultisigTreasury::approve(RuntimeOrigin::signed(1), 0, 0));

        // A different signer cannot add a vote at the deadline.
        System::set_block_number(110);
        assert_noop!(
            MultisigTreasury::approve(RuntimeOrigin::signed(2), 0, 0),
            Error::<Test>::ProposalExpired
        );
        assert_eq!(crate::ApprovalVotes::<Test>::get(0, 0).as_slice(), &[1u64]);
        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), Some((1, action_hash)));
        assert_eq!(crate::ProposalExpiry::<Test>::get(0, 0), Some(110));
    });
}

#[test]
fn creating_proposal_stores_its_deadline() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1), signers, 2,
        ));
        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, sp_core::H256::repeat_byte(7),
        ));

        // Creation block 10 + lifetime 100 = deadline 110.
        assert_eq!(crate::ProposalExpiry::<Test>::get(0, 0), Some(110u64));
    });
}

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(MultisigTreasury::do_something(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(Something::<Test>::get(), Some(42));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}
#[test]
fn increment_changes_42_to_43() {
    new_test_ext().execute_with(|| {
        // Store 42 using test account 1.
        assert_ok!(MultisigTreasury::do_something(RuntimeOrigin::signed(1), 42));

        // This function increments the stored number when one exists.
        assert_ok!(MultisigTreasury::cause_error(RuntimeOrigin::signed(1)));

        // Check that blockchain storage now contains 43.
        assert_eq!(Something::<Test>::get(), Some(43));
    });
}
#[test]
fn increment_fails_at_maximum_value() {
    new_test_ext().execute_with(|| {
        // Store the largest number a u32 can hold.
        assert_ok!(MultisigTreasury::do_something(
            RuntimeOrigin::signed(1),
            u32::MAX
        ));

        // Incrementing must return StorageOverflow and leave storage unchanged.
        assert_noop!(
            MultisigTreasury::cause_error(RuntimeOrigin::signed(1)),
            Error::<Test>::StorageOverflow
        );

        // Confirm the original number is still stored.
        assert_eq!(Something::<Test>::get(), Some(u32::MAX));
    });
}
#[test]
fn unsigned_caller_cannot_store_value() {
    new_test_ext().execute_with(|| {
        // Try to store 42 without a signed account.
        assert_noop!(
            MultisigTreasury::do_something(RuntimeOrigin::none(), 42),
            sp_runtime::DispatchError::BadOrigin
        );

        // Confirm that nothing was stored.
        assert_eq!(Something::<Test>::get(), None);
    });
}
#[test]
fn another_signed_account_can_replace_value() {
    new_test_ext().execute_with(|| {
        // Account 1 stores 42.
        assert_ok!(MultisigTreasury::do_something(RuntimeOrigin::signed(1), 42));

        // Account 2 replaces it with 99.
        assert_ok!(MultisigTreasury::do_something(RuntimeOrigin::signed(2), 99));

        // The shared storage now contains 99.
        assert_eq!(Something::<Test>::get(), Some(99));
    });
}
#[test]
fn signer_list_rejects_a_fourth_account() {
    new_test_ext().execute_with(|| {
        let mut signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert!(signers.try_push(4).is_err());
        assert_eq!(signers.as_slice(), &[1, 2, 3]);
    });
}
#[test]
fn zero_approval_threshold_is_rejected() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_noop!(
            MultisigTreasury::validate_threshold(&signers, 0),
            Error::<Test>::InvalidThreshold
        );
    });
}
#[test]
fn threshold_above_signer_count_is_rejected() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_noop!(
            MultisigTreasury::validate_threshold(&signers, 4),
            Error::<Test>::InvalidThreshold
        );
    });
}
#[test]
fn two_of_three_threshold_is_accepted() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::validate_threshold(&signers, 2)
        );
    });
}
#[test]
fn duplicate_signers_are_rejected() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 1, 2].try_into().expect("Three entries should fit");

        assert_noop!(
            MultisigTreasury::validate_unique_signers(&signers),
            Error::<Test>::DuplicateSigner
        );
    });
}
#[test]
fn distinct_signers_are_accepted() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::validate_unique_signers(&signers)
        );
    });
}
#[test]
fn multisig_configuration_can_be_stored() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        // No configuration exists for ID 0 yet.
        assert_eq!(crate::MultisigAccounts::<Test>::get(0), None);

        // Save the signers and a threshold of 2.
        crate::MultisigAccounts::<Test>::insert(
            0,
            (signers.clone(), 2u32),
        );

        // Read back exactly what we saved.
        assert_eq!(
            crate::MultisigAccounts::<Test>::get(0),
            Some((signers, 2u32))
        );
    });
}
#[test]
fn next_multisig_id_starts_at_zero() {
    new_test_ext().execute_with(|| {
        assert_eq!(crate::NextMultisigId::<Test>::get(), 0);
    });
}#[test]
fn creating_multisig_stores_configuration_and_advances_id() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::do_create_multisig(signers.clone(), 2)
        );

        assert_eq!(
            crate::MultisigAccounts::<Test>::get(0),
            Some((signers, 2u32))
        );

        assert_eq!(crate::NextMultisigId::<Test>::get(), 1);
    });
}
#[test]
fn creating_multisig_with_duplicates_changes_nothing() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 1, 2].try_into().expect("Three entries should fit");

        // Creation must reject the duplicate account.
        assert_noop!(
            MultisigTreasury::do_create_multisig(signers, 2),
            Error::<Test>::DuplicateSigner
        );

        // No configuration should have been saved.
        assert_eq!(crate::MultisigAccounts::<Test>::get(0), None);

        // The ID should still be available.
        assert_eq!(crate::NextMultisigId::<Test>::get(), 0);
    });
}
#[test]
fn creating_two_multisigs_preserves_both_configurations() {
    new_test_ext().execute_with(|| {
        let first_signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        let second_signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![4, 5].try_into().expect("Two signers should fit");

        assert_ok!(
            MultisigTreasury::do_create_multisig(first_signers.clone(), 2)
        );

        assert_ok!(
            MultisigTreasury::do_create_multisig(second_signers.clone(), 2)
        );

        // Creating the second configuration must not overwrite the first.
        assert_eq!(
            crate::MultisigAccounts::<Test>::get(0),
            Some((first_signers, 2u32))
        );

        assert_eq!(
            crate::MultisigAccounts::<Test>::get(1),
            Some((second_signers, 2u32))
        );

        // The next available ID is now 2.
        assert_eq!(crate::NextMultisigId::<Test>::get(), 2);
    });
}
#[test]
fn creating_multisig_at_id_limit_changes_nothing() {
    new_test_ext().execute_with(|| {
        // Simulate the counter reaching its maximum.
        crate::NextMultisigId::<Test>::put(u32::MAX);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_noop!(
            MultisigTreasury::do_create_multisig(signers, 2),
            Error::<Test>::MultisigIdOverflow
        );

        // Nothing was created at the maximum ID.
        assert_eq!(
            crate::MultisigAccounts::<Test>::get(u32::MAX),
            None
        );

        // The counter remains unchanged.
        assert_eq!(
            crate::NextMultisigId::<Test>::get(),
            u32::MAX
        );
    });
}
#[test]
fn creating_multisig_emits_event() {
    new_test_ext().execute_with(|| {
        // Events are recorded after the genesis block.
        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::do_create_multisig(signers, 2)
        );

        System::assert_last_event(
            Event::MultisigCreated {
                multisig_id: 0,
                threshold: 2,
            }
            .into(),
        );
    });
}
#[test]
fn signer_can_create_multisig() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers.clone(),
                2,
            )
        );

        assert_eq!(
            crate::MultisigAccounts::<Test>::get(0),
            Some((signers, 2u32))
        );
    });
}
#[test]
fn outsider_cannot_create_multisig() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        // Account 4 is signed, but is not in [1, 2, 3].
        assert_noop!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(4),
                signers,
                2,
            ),
            Error::<Test>::NotASigner
        );

        // Rejection must not create a configuration or consume an ID.
        assert_eq!(crate::MultisigAccounts::<Test>::get(0), None);
        assert_eq!(crate::NextMultisigId::<Test>::get(), 0);
    });
}
#[test]
fn unsigned_caller_cannot_create_multisig() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_noop!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::none(),
                signers,
                2,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        assert_eq!(crate::MultisigAccounts::<Test>::get(0), None);
        assert_eq!(crate::NextMultisigId::<Test>::get(), 0);
    });
}
#[test]
fn proposal_storage_is_scoped_to_its_multisig() {
    new_test_ext().execute_with(|| {
        // A sample hash for testing storage, not a real encoded action.
        let action_hash = sp_core::H256::repeat_byte(7);

        // Save proposal 0 for multisig 0, proposed by account 1.
        crate::PendingProposals::<Test>::insert(
            0,
            0,
            (1u64, action_hash),
        );

        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((1u64, action_hash))
        );

        // Multisig 1 must not see multisig 0's proposal.
        assert_eq!(
            crate::PendingProposals::<Test>::get(1, 0),
            None
        );
    });
}
#[test]
fn proposal_id_counters_are_independent() {
    new_test_ext().execute_with(|| {
        // Both counters start at zero.
        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
        assert_eq!(crate::NextProposalId::<Test>::get(1), 0);

        // Change only multisig 0's counter.
        crate::NextProposalId::<Test>::insert(0, 1);

        assert_eq!(crate::NextProposalId::<Test>::get(0), 1);
        assert_eq!(crate::NextProposalId::<Test>::get(1), 0);
    });
}
#[test]
fn signer_can_open_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        assert_ok!(
            MultisigTreasury::do_propose(0, 1, action_hash)
        );

        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((1u64, action_hash))
        );
        assert_eq!(crate::NextProposalId::<Test>::get(0), 1);
    });
}
#[test]
fn outsider_cannot_open_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        // Account 4 is not a signer of multisig 0.
        assert_noop!(
            MultisigTreasury::do_propose(0, 4, action_hash),
            Error::<Test>::NotASigner
        );

        // Rejection must not store a proposal or consume an ID.
        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), None);
        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
    });
}
#[test]
fn proposal_for_missing_multisig_is_rejected() {
    new_test_ext().execute_with(|| {
        let action_hash = sp_core::H256::repeat_byte(7);

        // We have not created multisig 99.
        assert_noop!(
            MultisigTreasury::do_propose(99, 1, action_hash),
            Error::<Test>::MultisigNotFound
        );

        assert_eq!(crate::PendingProposals::<Test>::get(99, 0), None);
        assert_eq!(crate::NextProposalId::<Test>::get(99), 0);
    });
}
#[test]
fn proposal_id_overflow_changes_nothing() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        // Simulate this multisig reaching the proposal-ID limit.
        crate::NextProposalId::<Test>::insert(0, u32::MAX);

        let action_hash = sp_core::H256::repeat_byte(7);

        assert_noop!(
            MultisigTreasury::do_propose(0, 1, action_hash),
            Error::<Test>::ProposalIdOverflow
        );

        assert_eq!(
            crate::PendingProposals::<Test>::get(0, u32::MAX),
            None
        );
        assert_eq!(
            crate::NextProposalId::<Test>::get(0),
            u32::MAX
        );
    });
}
#[test]
fn opening_proposal_emits_event() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        assert_ok!(
            MultisigTreasury::do_propose(0, 1, action_hash)
        );

        System::assert_last_event(
            Event::ProposalOpened {
                multisig_id: 0,
                proposal_id: 0,
                proposer: 1,
                action_hash,
            }
            .into(),
        );
    });
}
#[test]
fn signed_proposal_records_actual_caller() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        // Account 2 submits the proposal.
        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(2),
                0,
                action_hash,
            )
        );

        // The stored proposer must be account 2, not the creator.
        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((2u64, action_hash))
        );
        assert_eq!(crate::NextProposalId::<Test>::get(0), 1);
    });
}
#[test]
fn unsigned_caller_cannot_submit_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        assert_noop!(
            MultisigTreasury::propose(
                RuntimeOrigin::none(),
                0,
                action_hash,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), None);
        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
    });
}
#[test]
fn signed_outsider_cannot_submit_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        let action_hash = sp_core::H256::repeat_byte(7);

        // Account 4 is signed, but is not an authorised signer.
        assert_noop!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(4),
                0,
                action_hash,
            ),
            Error::<Test>::NotASigner
        );

        assert_eq!(crate::PendingProposals::<Test>::get(0, 0), None);
        assert_eq!(crate::NextProposalId::<Test>::get(0), 0);
    });
}
#[test]
fn opening_proposal_does_not_automatically_approve_it() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // The proposal exists, but nobody has approved it yet.
        assert!(crate::PendingProposals::<Test>::contains_key(0, 0));
        assert!(crate::ApprovalVotes::<Test>::get(0, 0).is_empty());
    });
}
#[test]
fn signer_can_approve_existing_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // Signer 2 approves proposal 0 belonging to multisig 0.
        assert_ok!(MultisigTreasury::do_approve(0, 0, 2));

        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[2u64]
        );
    });
}
#[test]
fn signer_cannot_approve_same_proposal_twice() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // The first approval succeeds.
        assert_ok!(MultisigTreasury::do_approve(0, 0, 2));

        // The same signer cannot approve this proposal again.
        assert_noop!(
            MultisigTreasury::do_approve(0, 0, 2),
            Error::<Test>::AlreadyApproved
        );

        // Exactly one approval remains.
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[2u64]
        );
    });
}
#[test]
fn outsider_cannot_approve_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // Record a valid approval first.
        assert_ok!(MultisigTreasury::do_approve(0, 0, 2));

        // Account 4 is not a signer.
        assert_noop!(
            MultisigTreasury::do_approve(0, 0, 4),
            Error::<Test>::NotASigner
        );

        // The valid approval remains; the outsider was not added.
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[2u64]
        );
    });
}
#[test]
fn signer_cannot_approve_missing_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        // Multisig 0 exists, but proposal 99 does not.
        assert_noop!(
            MultisigTreasury::do_approve(0, 99, 2),
            Error::<Test>::ProposalNotFound
        );

        // No approval entry should have been created.
        assert!(
            !crate::ApprovalVotes::<Test>::contains_key(0, 99)
        );
    });
}
#[test]
fn two_distinct_signers_can_approve_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        assert_ok!(MultisigTreasury::do_approve(0, 0, 1));
        assert_ok!(MultisigTreasury::do_approve(0, 0, 2));

        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[1u64, 2u64]
        );

        // Collecting approvals does not execute or remove the proposal.
        assert!(crate::PendingProposals::<Test>::contains_key(0, 0));
    });
}
#[test]
fn approving_proposal_emits_event() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        assert_ok!(MultisigTreasury::do_approve(0, 0, 2));

        System::assert_last_event(
            Event::ProposalApproved {
                multisig_id: 0,
                proposal_id: 0,
                approver: 2,
            }
            .into(),
        );
    });
}
#[test]
fn signed_approval_records_actual_caller() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // Account 2 submits the approval transaction.
        assert_ok!(
            MultisigTreasury::approve(
                RuntimeOrigin::signed(2),
                0,
                0,
            )
        );

        // The vote belongs to the caller, account 2.
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[2u64]
        );
    });
}
#[test]
fn unsigned_caller_cannot_approve_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        assert_noop!(
            MultisigTreasury::approve(
                RuntimeOrigin::none(),
                0,
                0,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        // No approval entry should have been created.
        assert!(
            !crate::ApprovalVotes::<Test>::contains_key(0, 0)
        );
    });
}
#[test]
fn signed_outsider_cannot_submit_approval() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(
            MultisigTreasury::create_multisig(
                RuntimeOrigin::signed(1),
                signers,
                2,
            )
        );

        assert_ok!(
            MultisigTreasury::propose(
                RuntimeOrigin::signed(1),
                0,
                sp_core::H256::repeat_byte(7),
            )
        );

        // Account 4 is signed, but is not in the signer list.
        assert_noop!(
            MultisigTreasury::approve(
                RuntimeOrigin::signed(4),
                0,
                0,
            ),
            Error::<Test>::NotASigner
        );

        assert!(
            !crate::ApprovalVotes::<Test>::contains_key(0, 0)
        );
    });
}
#[test]
fn one_approval_does_not_meet_two_of_three_threshold() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            sp_core::H256::repeat_byte(7),
        ));

        // Only account 1 approves. The threshold requires two accounts.
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1),
            0,
            0,
        ));

        assert_noop!(
            MultisigTreasury::ensure_enough_approvals(0, 0),
            Error::<Test>::NotEnoughApprovals
        );
    });
}
#[test]
fn two_approvals_meet_two_of_three_threshold() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            sp_core::H256::repeat_byte(7),
        ));

        // Two different signers approve the same proposal.
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1),
            0,
            0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2),
            0,
            0,
        ));

        // Two approvals satisfy the required threshold.
        assert_ok!(MultisigTreasury::ensure_enough_approvals(0, 0));
    });
}
#[test]
fn approval_check_rejects_missing_proposal() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        // Multisig 0 exists, but proposal 0 was never created.
        assert_noop!(
            MultisigTreasury::ensure_enough_approvals(0, 0),
            Error::<Test>::ProposalNotFound
        );
    });
}
#[test]
fn different_action_hash_is_rejected() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        // The proposal stores hash 7.
        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            sp_core::H256::repeat_byte(7),
        ));

        // A different hash must be rejected without changing storage.
        assert_noop!(
            MultisigTreasury::ensure_matching_action(
                0,
                0,
                sp_core::H256::repeat_byte(8),
            ),
            Error::<Test>::ActionHashMismatch
        );
    });
}
#[test]
fn matching_action_hash_is_accepted() {
    new_test_ext().execute_with(|| {
        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        let action_hash = sp_core::H256::repeat_byte(7);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            action_hash,
        ));

        // The supplied hash matches the hash stored in the proposal.
        assert_ok!(MultisigTreasury::ensure_matching_action(
            0,
            0,
            action_hash,
        ));
    });
}
#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(MultisigTreasury::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
#[test]
fn treasury_accounts_are_stable_and_distinct() {
    new_test_ext().execute_with(|| {
        let first = MultisigTreasury::treasury_account(0);
        let repeated = MultisigTreasury::treasury_account(0);
        let second = MultisigTreasury::treasury_account(1);

        // The same multisig ID gives the same account.
        assert_eq!(first, repeated);

        // These two multisig IDs give different accounts.
        assert_ne!(first, second);
    });
}
#[test]
fn approved_proposal_rejects_a_changed_call() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        // Propose a specific action.
        let original_call = RuntimeCall::System(
            frame_system::Call::remark {
                remark: vec![1, 2, 3],
            },
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(
                &original_call,
            );

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            action_hash,
        ));

        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // The original action passes both readiness checks.
        assert_ok!(MultisigTreasury::ensure_ready_to_execute(
            0, 0, &original_call,
        ));

        // Changing the action must fail, even with enough approvals.
        let changed_call = RuntimeCall::System(
            frame_system::Call::remark {
                remark: vec![9, 9, 9],
            },
        );

        assert_noop!(
            MultisigTreasury::ensure_ready_to_execute(
                0, 0, &changed_call,
            ),
            Error::<Test>::ActionHashMismatch
        );
    });
}
#[test]
fn approved_action_executes_as_treasury_once() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(&call);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            action_hash,
        ));

        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // Confirm that a deadline exists before execution.
        assert!(crate::ProposalExpiry::<Test>::contains_key(0, 0));

        assert_ok!(MultisigTreasury::do_execute(
            0, 0, call.clone(),
        ));

        // Successful execution also removes the deadline.
        assert!(!crate::ProposalExpiry::<Test>::contains_key(0, 0));

        // The action actually changed storage.
        assert_eq!(Something::<Test>::get(), Some(42));

        // The treasury account executed the action.
        System::assert_last_event(
            Event::<Test>::SomethingStored {
                something: 42,
                who: MultisigTreasury::treasury_account(0),
            }
            .into(),
        );

        // Successful execution removes the proposal and votes.
        assert!(!crate::PendingProposals::<Test>::contains_key(0, 0));
        assert!(!crate::ApprovalVotes::<Test>::contains_key(0, 0));

        // The same proposal cannot execute again.
        assert_noop!(
            MultisigTreasury::do_execute(0, 0, call),
            Error::<Test>::ProposalNotFound
        );
    });
}
#[test]
fn failed_execution_preserves_proposal_and_approvals() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        // This action fails when Something has no stored value.
        let call = RuntimeCall::MultisigTreasury(
            crate::Call::cause_error {},
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(&call);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1),
            0,
            action_hash,
        ));

        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // Remember the deadline so rollback must restore the same value.
        let expiry_before = crate::ProposalExpiry::<Test>::get(0, 0)
            .expect("The proposal must have a deadline before execution");

        // Execution fails, and all storage must remain unchanged.
        assert_noop!(
            MultisigTreasury::do_execute(0, 0, call),
            Error::<Test>::NoneValue
        );

        // The deadline, proposal and both approvals are still available.
        assert_eq!(
            crate::ProposalExpiry::<Test>::get(0, 0),
            Some(expiry_before)
        );
        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((1, action_hash))
        );
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[1u64, 2u64]
        );
        assert_eq!(Something::<Test>::get(), None);
    });
}
#[test]
fn signer_can_submit_execution() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        System::set_block_number(1);

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(&call);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // Signer 3 submits execution after signers 1 and 2 approve.
        assert_ok!(MultisigTreasury::execute(
            RuntimeOrigin::signed(3),
            0,
            0,
            Box::new(call),
        ));

        assert_eq!(Something::<Test>::get(), Some(42));

        // The action runs as the treasury, not the submitting signer.
        System::assert_last_event(
            Event::<Test>::SomethingStored {
                something: 42,
                who: MultisigTreasury::treasury_account(0),
            }
            .into(),
        );
    });
}
#[test]
fn outsider_cannot_submit_execution() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(&call);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // Account 4 is not a signer of this multisig.
        assert_noop!(
            MultisigTreasury::execute(
                RuntimeOrigin::signed(4),
                0,
                0,
                Box::new(call),
            ),
            Error::<Test>::NotASigner
        );

        // The action did not run.
        assert_eq!(Something::<Test>::get(), None);

        // The approved proposal remains available.
        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((1, action_hash))
        );
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[1u64, 2u64]
        );
    });
}
#[test]
fn unsigned_caller_cannot_submit_execution() {
    new_test_ext().execute_with(|| {
        use sp_runtime::traits::Hash;

        let signers: frame_support::BoundedVec<
            u64,
            <Test as crate::Config>::MaxSigners,
        > = vec![1, 2, 3].try_into().expect("Three signers should fit");

        assert_ok!(MultisigTreasury::create_multisig(
            RuntimeOrigin::signed(1),
            signers,
            2,
        ));

        let call = RuntimeCall::MultisigTreasury(
            crate::Call::do_something { something: 42 },
        );

        let action_hash =
            <Test as frame_system::Config>::Hashing::hash_of(&call);

        assert_ok!(MultisigTreasury::propose(
            RuntimeOrigin::signed(1), 0, action_hash,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(1), 0, 0,
        ));
        assert_ok!(MultisigTreasury::approve(
            RuntimeOrigin::signed(2), 0, 0,
        ));

        // No signed account submitted this request.
        assert_noop!(
            MultisigTreasury::execute(
                RuntimeOrigin::none(),
                0,
                0,
                Box::new(call),
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        // Execution was rejected without consuming the proposal.
        assert_eq!(Something::<Test>::get(), None);
        assert_eq!(
            crate::PendingProposals::<Test>::get(0, 0),
            Some((1, action_hash))
        );
        assert_eq!(
            crate::ApprovalVotes::<Test>::get(0, 0).as_slice(),
            &[1u64, 2u64]
        );
    });
}
#[test]
fn proposal_expiry_is_scoped_to_its_multisig() {
    new_test_ext().execute_with(|| {
        // Store a deadline for proposal 0 of multisig 0.
        crate::ProposalExpiry::<Test>::insert(0, 0, 100u64);

        assert_eq!(
            crate::ProposalExpiry::<Test>::get(0, 0),
            Some(100u64)
        );

        // Proposal 0 of a different multisig has no deadline.
        assert_eq!(
            crate::ProposalExpiry::<Test>::get(1, 0),
            None
        );
    });
}
#[test]
fn proposal_expires_at_its_deadline() {
    new_test_ext().execute_with(|| {
        // Set up proposal storage directly to test the expiry boundary.
        crate::PendingProposals::<Test>::insert(
            0,
            0,
            (1u64, sp_core::H256::repeat_byte(7)),
        );
        crate::ProposalExpiry::<Test>::insert(0, 0, 100u64);

        // One block before the deadline: still valid.
        System::set_block_number(99);
        assert_ok!(
            MultisigTreasury::ensure_proposal_not_expired(0, 0)
        );

        // At the deadline: expired.
        System::set_block_number(100);
        assert_noop!(
            MultisigTreasury::ensure_proposal_not_expired(0, 0),
            Error::<Test>::ProposalExpired
        );

        // After the deadline: still expired.
        System::set_block_number(101);
        assert_noop!(
            MultisigTreasury::ensure_proposal_not_expired(0, 0),
            Error::<Test>::ProposalExpired
        );
    });
}
