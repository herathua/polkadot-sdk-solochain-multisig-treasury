use crate::{mock::*, Error, Event, Something};
use frame_support::{assert_noop, assert_ok};

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
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(MultisigTreasury::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
