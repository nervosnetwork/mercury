use super::*;
use crate::rpc_impl::utils;

#[tokio::test]
async fn test_is_dao_withdraw_unlock() {
    let deposit_epoch = RationalU256::from_u256(0u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(180u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, false);

    let deposit_epoch = RationalU256::from_u256(0u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(181u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, true);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(184u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, false);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(100u64.into());
    let tip_epoch = RationalU256::from_u256(186u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, true);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(186u64.into());
    let tip_epoch = RationalU256::from_u256(364u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, false);

    let deposit_epoch = RationalU256::from_u256(5u64.into());
    let withdraw_epoch = RationalU256::from_u256(186u64.into());
    let tip_epoch = RationalU256::from_u256(366u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, true);

    let deposit_epoch = RationalU256::from_u256(360u64.into());
    let withdraw_epoch = RationalU256::from_u256(386u64.into());
    let tip_epoch = RationalU256::from_u256(387u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, false);

    let deposit_epoch = RationalU256::from_u256(360u64.into());
    let withdraw_epoch = RationalU256::from_u256(386u64.into());
    let tip_epoch = RationalU256::from_u256(541u64.into());
    let res = utils::is_dao_withdraw_unlock(deposit_epoch, withdraw_epoch, Some(tip_epoch));
    assert_eq!(res, true);
}
