use super::*;
use crate::rpc_impl::utils;

use ckb_types::core::EpochNumberWithFraction;

#[tokio::test]
async fn test_is_dao_withdraw_unlock() {
    let deposit_epoch = RationalU256::from_u256(0u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(180u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(res);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(184u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(!res);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(186u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(res);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(186u64.into());
    let tip_epoch = RationalU256::from_u256(364u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(!res);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(186u64.into());
    let tip_epoch = RationalU256::from_u256(366u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(res);

    let deposit_epoch = RationalU256::from_u256(360u64.into());
    let withdraw_epoch = RationalU256::from_u256(386u64.into());
    let tip_epoch = RationalU256::from_u256(387u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(!res);

    let deposit_epoch = RationalU256::from_u256(360u64.into());
    let withdraw_epoch = RationalU256::from_u256(386u64.into());
    let tip_epoch = RationalU256::from_u256(541u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert!(res);

    let deposit_epoch = EpochNumberWithFraction::new(2, 648, 1677);
    let withdraw_epoch = EpochNumberWithFraction::new(47, 382, 1605);
    let tip_epoch = EpochNumberWithFraction::new(47, 382, 1605);
    let res = utils::is_dao_withdraw_unlock(
        deposit_epoch.to_rational(),
        withdraw_epoch.to_rational(),
        Some(tip_epoch.to_rational()),
    );
    assert!(!res);

    let deposit_epoch = EpochNumberWithFraction::new(2, 648, 1677);
    let withdraw_epoch = EpochNumberWithFraction::new(47, 382, 1605);
    let tip_epoch = EpochNumberWithFraction::new(182, 648, 1677);
    let res = utils::is_dao_withdraw_unlock(
        deposit_epoch.to_rational(),
        withdraw_epoch.to_rational(),
        Some(tip_epoch.to_rational()),
    );
    assert!(res);
}

#[tokio::test]
async fn test_epoch_number_into_u256() {
    let epoch = EpochNumberWithFraction::new(2, 648, 1677).to_rational();
    let epoch_rebuild = RationalU256::from_u256(epoch.clone().into_u256());
    assert_ne!(epoch, epoch_rebuild);

    let epoch_number_rational_u256 = RationalU256::new(3201u32.into(), 1600u32.into());
    let epoch_number: EpochNumberWithFraction = EpochNumberWithFraction::new(0, 3201, 1600);
    assert_eq!(epoch_number_rational_u256, epoch_number.to_rational());
}
