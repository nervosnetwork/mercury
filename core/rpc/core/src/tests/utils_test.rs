use super::*;
use crate::r#impl::utils::{self, calculate_cell_capacity};
use ckb_jsonrpc_types::OutPoint;
use core_rpc_types::{JsonItem, SinceConfig, SinceFlag, SinceType};

use ckb_types::core::EpochNumberWithFraction;

#[test]
fn test_is_dao_withdraw_unlock() {
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

#[test]
fn test_calculate_unlock_epoch_number() {
    let deposit_epoch = EpochNumberWithFraction::new(2, 648, 1677);
    let withdraw_epoch = EpochNumberWithFraction::new(47, 382, 1605);
    let unlock_epoch_number = utils::calculate_unlock_epoch_number(
        deposit_epoch.full_value(),
        withdraw_epoch.full_value(),
    );
    assert_eq!(
        unlock_epoch_number,
        EpochNumberWithFraction::new(182, 648, 1677).full_value()
    );

    let deposit_epoch = EpochNumberWithFraction::new(2, 0, 1);
    let withdraw_epoch = EpochNumberWithFraction::new(100, 0, 1);
    let unlock_epoch_number = utils::calculate_unlock_epoch_number(
        deposit_epoch.full_value(),
        withdraw_epoch.full_value(),
    );
    assert_eq!(
        unlock_epoch_number,
        EpochNumberWithFraction::new(182, 0, 1).full_value()
    );
}

#[test]
fn test_epoch_number_into_u256() {
    let epoch = EpochNumberWithFraction::new(2, 648, 1677).to_rational();
    let epoch_rebuild = RationalU256::from_u256(epoch.clone().into_u256());
    assert_ne!(epoch, epoch_rebuild);

    let epoch_number_rational_u256 = RationalU256::new(3201u32.into(), 1600u32.into());
    let epoch_number: EpochNumberWithFraction = EpochNumberWithFraction::new(0, 3201, 1600);
    assert_eq!(epoch_number_rational_u256, epoch_number.to_rational());
}

#[test]
fn test_to_since() {
    let deposit_epoch = EpochNumberWithFraction::new(2, 648, 1677);
    let withdraw_epoch = EpochNumberWithFraction::new(47, 382, 1605);
    let unlock_epoch_number = utils::calculate_unlock_epoch_number(
        deposit_epoch.full_value(),
        withdraw_epoch.full_value(),
    );
    assert_eq!(0x68d02880000b6u64, unlock_epoch_number);
    let since = utils::to_since(SinceConfig {
        type_: SinceType::EpochNumber,
        flag: SinceFlag::Absolute,
        value: unlock_epoch_number.into(),
    });
    assert_eq!(0x20068d02880000b6u64, since.unwrap());
}

#[test]
fn test_dedup_items() {
    let a1 = JsonItem::Identity("abc".to_string());
    let b1 = JsonItem::Identity("bcd".to_string());
    let c1 = JsonItem::OutPoint(OutPoint {
        index: 0.into(),
        tx_hash: H256::from_str("365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17")
            .unwrap(),
    });
    let c2 = JsonItem::OutPoint(OutPoint {
        index: 1.into(),
        tx_hash: H256::from_str("365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17")
            .unwrap(),
    });
    let c3 = JsonItem::OutPoint(OutPoint {
        index: 1.into(),
        tx_hash: H256::from_str("365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17")
            .unwrap(),
    });
    let b2 = JsonItem::Identity("bcd".to_string());

    let mut items = vec![a1, b1, c1, c2, c3, b2];
    utils::dedup_json_items(&mut items);

    assert_eq!(
        vec![
            JsonItem::Identity("abc".to_string()),
            JsonItem::Identity("bcd".to_string()),
            JsonItem::OutPoint(OutPoint {
                index: 0.into(),
                tx_hash: H256::from_str(
                    "365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17"
                )
                .unwrap(),
            }),
            JsonItem::OutPoint(OutPoint {
                index: 1.into(),
                tx_hash: H256::from_str(
                    "365698b50ca0da75dca2c87f9e7b563811d3b5813736b8cc62cc3b106faceb17"
                )
                .unwrap(),
            })
        ],
        items
    );
}

#[test]
fn test_dedup_items_identity() {
    let a = JsonItem::Identity("bcd".to_string());
    let b = JsonItem::Identity("bcd".to_string());
    let c = JsonItem::Identity("abc".to_string());
    let e = JsonItem::Identity("bcd".to_string());

    let mut items = vec![a, b, c, e];
    utils::dedup_json_items(&mut items);

    assert_eq!(
        vec![
            JsonItem::Identity("bcd".to_string()),
            JsonItem::Identity("abc".to_string()),
        ],
        items
    );
}

#[test]
fn test_calculate_the_percentage() {
    assert_eq!(
        "0.00000%".to_string(),
        utils::calculate_the_percentage(0, 0)
    );
    assert_eq!(
        "0.00000%".to_string(),
        utils::calculate_the_percentage(0, 1)
    );
    assert_eq!(
        "0.00000%".to_string(),
        utils::calculate_the_percentage(3, 0)
    );
    assert_eq!(
        "50.00000%".to_string(),
        utils::calculate_the_percentage(1, 2)
    );
    assert_eq!(
        "66.66667%".to_string(),
        utils::calculate_the_percentage(2, 3)
    );
    assert_eq!(
        "75.00000%".to_string(),
        utils::calculate_the_percentage(3, 4)
    );
    assert_eq!(
        "99.99516%".to_string(),
        utils::calculate_the_percentage(3741740, 3741921)
    );
    assert_eq!(
        "99.98987%".to_string(),
        utils::calculate_the_percentage(3742181, 3742560)
    );
    assert_eq!(
        "99.99997%".to_string(),
        utils::calculate_the_percentage(3741920, 3741921)
    );
    assert_eq!(
        "99.99999%".to_string(),
        utils::calculate_the_percentage(6741920, 6741921)
    );
    assert_eq!(
        "99.99999%".to_string(),
        utils::calculate_the_percentage(16741920, 16741921)
    );
    assert_eq!(
        "100.00000%".to_string(),
        utils::calculate_the_percentage(2, 2)
    );
    assert_eq!(
        "150.00000%".to_string(),
        utils::calculate_the_percentage(3, 2)
    );
}

#[test]
fn test_calculate_cell_capacity() {
    let address = Address::from_str("ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq4nnw7qkdnnclfkg59uzn8umtfd2kwxceqcydzyt").unwrap();
    let capacity = calculate_cell_capacity(
        &address_to_script(address.payload()),
        &packed::ScriptOpt::default(),
        Capacity::bytes(0).expect("generate capacity"),
    );
    assert_eq!(6100000000, capacity);
}
