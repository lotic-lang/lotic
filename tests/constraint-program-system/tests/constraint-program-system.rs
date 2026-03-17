use {
    lotic::pinocchio::{error::ProgramError, Address},
    mollusk_svm::{result::Check, Mollusk},
    solana_sdk::{
        account,
        instruction::{AccountMeta, Instruction},
    },
};

#[test]
fn test_constraint_program_system_success() {
    let program_id = Address::from_str_const("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");
    let mollusk = Mollusk::new(
        &program_id,
        "../../target/deploy/test_constraint_program_system",
    );

    let user = Address::new_unique();
    let system_account = Address::from_str_const("11111111111111111111111111111111");
    let accounts = vec![
        (user, account::Account::default()),
        (system_account, account::Account::default()),
    ];

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(system_account, false),
        ],
        data: vec![0],
    };

    mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);
}

#[test]
fn test_constraint_program_system_failure() {
    let program_id = Address::from_str_const("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");
    let mollusk = Mollusk::new(
        &program_id,
        "../../target/deploy/test_constraint_program_system",
    );

    let user = Address::new_unique();
    let system_account = Address::new_unique();
    let accounts = vec![
        (user, account::Account::default()),
        (system_account, account::Account::default()),
    ];

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(user, true),
            AccountMeta::new(system_account, false),
        ],
        data: vec![0],
    };

    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[Check::err(ProgramError::IncorrectProgramId)],
    );
}
