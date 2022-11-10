pub mod build_adjust_account_transaction;
pub mod build_dao_transaction;
pub mod build_simple_transfer_transaction;
pub mod build_sudt_issue_transaction;
pub mod build_transfer_transaction_ckb;
pub mod build_transfer_transaction_udt;
pub mod built_in_pw_lock;
pub mod exec_instructions;
pub mod extension_omni_lock;
pub mod get_balance;

#[derive(Debug)]
pub struct IntegrationTest {
    pub name: &'static str,
    pub test_fn: fn(),
}

impl IntegrationTest {
    pub fn all_test_names() -> Vec<&'static str> {
        inventory::iter::<IntegrationTest>
            .into_iter()
            .map(|x| x.name)
            .collect::<Vec<&str>>()
    }

    pub fn from_name<S: AsRef<str>>(test_name: S) -> Option<&'static IntegrationTest> {
        inventory::iter::<IntegrationTest>
            .into_iter()
            .find(|t| t.name == test_name.as_ref())
    }
}

inventory::collect!(IntegrationTest);
