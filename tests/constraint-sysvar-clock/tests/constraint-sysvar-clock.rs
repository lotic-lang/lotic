use {
    lotic::pinocchio::{error::ProgramError, Address},
    mollusk_svm::{result::Check, Mollusk},
    solana_sdk::{
        account,
        instruction::{AccountMeta, Instruction},
    },
};

#[test]
fn test_sysvar_clock_success() {
    let program_id = Address::from_str_const("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");
    let mollusk = Mollusk::new(
        &program_id,
        "../../target/deploy/test_constraint_sysvar_clock",
    );

    let sysvar_clock = Address::from_str_const("SysvarC1ock11111111111111111111111111111111");
    let accounts = vec![(sysvar_clock, account::Account::default())];
    let instruction = Instruction {
        program_id,
        accounts: vec![AccountMeta::new(sysvar_clock, true)],
        data: vec![0],
    };

    mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()]);
}

#[test]
fn test_sysvar_clock_failure() {
    let program_id = Address::from_str_const("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");
    let mollusk = Mollusk::new(
        &program_id,
        "../../target/deploy/test_constraint_sysvar_clock",
    );

    let account = Address::new_unique();
    let accounts = vec![(account, account::Account::default())];

    let instruction = Instruction {
        program_id,
        accounts: vec![AccountMeta::new_readonly(account, true)],
        data: vec![0],
    };

    mollusk.process_and_validate_instruction(
        &instruction,
        &accounts,
        &[Check::err(ProgramError::IncorrectProgramId)],
    );
}
