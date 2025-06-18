use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

// Program entrypoint
entrypoint!(process_instruction);

// Function to route instructions to the correct handler
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Unpack instruction data
    let instruction = TodoInstruction::unpack(instruction_data)?;

    // Match instruction type
    match instruction {
        TodoInstruction::NewTodo { todo } => process_new_todo(program_id, accounts, todo)?,
        TodoInstruction::MarkDone { todo } => process_mark_done(program_id, accounts, todo)?,
    };
    Ok(())
}

// Instructions that our program can execute
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TodoInstruction {
    NewTodo { todo: String },  // Variant 0: Add a new to-do
    MarkDone { todo: String }, // Variant 1: Mark a to-do as done
}

impl TodoInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Get the instruction variant and remaining bytes
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        // Match instruction type and parse the remaining bytes
        match variant {
            0 => {
                // Parse string for NewTodo
                let todo = String::deserialize(&mut &rest[..])
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::NewTodo { todo })
            }
            1 => {
                // Parse string for MarkDone
                let todo = String::deserialize(&mut &rest[..])
                    .map_err(|_| ProgramError::InvalidInstructionData)?;
                Ok(Self::MarkDone { todo })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// Struct representing the to-do account's data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TodoAccount {
    todos: Vec<Todo>,
}

// Struct representing a single to-do item
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Todo {
    name: String,
    done: bool,
    publish_date: u64,
}

// Initialize a new to-do account or add a new to-do item
fn process_new_todo(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    todo_name: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    // Extract accounts
    let todo_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let clock = next_account_info(accounts_iter)?;

    // Verify system program
    if system_program.key != &solana_program::system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Get current timestamp from Clock sysvar
    let clock = Clock::from_account_info(clock)?;
    let current_timestamp = clock.unix_timestamp as u64;

    // Check if the account is already initialized
    let mut todo_account_data = todo_account.data.borrow_mut();
    let todo_account_struct = if todo_account_data.is_empty() {
        // Account is not initialized, create a new account
        let account_space = 1024; // Allocate 1KB for the account (adjust as needed)
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(account_space);

        // Create the to-do account
        invoke(
            &system_instruction::create_account(
                payer_account.key,
                todo_account.key,
                required_lamports,
                account_space as u64,
                program_id,
            ),
            &[
                payer_account.clone(),
                todo_account.clone(),
                system_program.clone(),
            ],
        )?;

        // Initialize with an empty to-do list
        TodoAccount { todos: vec![] }
    } else {
        // Deserialize existing account data
        TodoAccount::try_from_slice(&todo_account_data)
            .map_err(|_| ProgramError::InvalidAccountData)?
    };

    // Verify account ownership
    if todo_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Add new to-do item
    let new_todo = Todo {
        name: todo_name.clone(),
        done: false,
        publish_date: current_timestamp,
    };
    let mut updated_todos = todo_account_struct.todos;
    updated_todos.push(new_todo);

    // Update the account data
    let updated_account = TodoAccount {
        todos: updated_todos,
    };
    updated_account.serialize(&mut &mut todo_account_data[..])?;

    msg!("Added new to-do: {}", todo_name);
    Ok(())
}

// Mark a to-do item as done
fn process_mark_done(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    todo_name: String,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    // Extract accounts
    let todo_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if todo_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize account data
    let mut todo_account_data = todo_account.data.borrow_mut();
    let mut todo_account_struct = TodoAccount::try_from_slice(&todo_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Find and mark the to-do item as done
    let todo = todo_account_struct
        .todos
        .iter_mut()
        .find(|todo| todo.name == todo_name)
        .ok_or(ProgramError::InvalidInstructionData)?;

    if todo.done {
        return Err(ProgramError::InvalidAccountData); // Already marked as done
    }

    todo.done = true;

    // Serialize updated account data
    todo_account_struct.serialize(&mut &mut todo_account_data[..])?;

    msg!("Marked to-do as done: {}", todo_name);
    Ok(())
}
