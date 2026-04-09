pub use {
    bytemuck,
    lotic_macros::{account_state, declare_program, instruction, InstructionAccounts},
    pinocchio, solana_address,
    std::{
        marker::PhantomData,
        ops::{Deref, DerefMut},
    },
};

pub struct Context<'a, T> {
    pub program_id: &'a pinocchio::Address,
    pub accounts: &'a mut T,
}

pub struct Account<'a, T> {
    view: &'a mut pinocchio::AccountView,
    pub state: &'a mut T,
}

impl<'a, T: bytemuck::Pod> Account<'a, T> {
    pub fn new(view: &'a mut pinocchio::AccountView) -> Self {
        let data: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(view.data_mut_ptr(), std::mem::size_of::<T>())
        };
        let state: &mut T = bytemuck::from_bytes_mut(data);

        Self { view, state }
    }
}

impl<'a, T> Deref for Account<'a, T> {
    type Target = pinocchio::AccountView;
    fn deref(&self) -> &Self::Target {
        self.view
    }
}

impl<'a, T> DerefMut for Account<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.view
    }
}
