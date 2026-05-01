use {
    lotic::pinocchio::Address,
    mollusk_svm::{program, result::Check, Mollusk},
    solana_sdk::{
        account,
        instruction::{AccountMeta, Instruction},
    },
};

#[test]
fn test_init_success() {
    let program_id = Address::from_str_const("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");
    let mollusk = Mollusk::new(&program_id, "../../target/deploy/test_processor_init");

    let account = Address::new_unique();
    let new_account = Address::new_unique();
    let system_program = program::keyed_account_for_system_program();
    let mut accounts = vec![
        (account, account::Account::default()),
        (new_account, account::Account::default()),
        system_program.clone(),
    ];

    accounts[0].1.lamports = 1000000;

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(account, true),
            AccountMeta::new(new_account, true),
            AccountMeta::new_readonly(system_program.0, false),
        ],
        data: vec![0],
    };

    let result =
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);
    assert!(
        result.get_account(&new_account).unwrap().data.len() == 10,
        "Account data length is wrong"
    );
}
