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
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(MultisigTreasury::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
