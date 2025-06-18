#[cfg(test)]
mod test {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        sysvar::clock,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_todo_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) =
            ProgramTest::new("todo_program", program_id, processor!(process_instruction))
                .start()
                .await;

        // Create a new keypair for the to-do account
        let todo_account_keypair = Keypair::new();
        let todo_name = "Buy groceries".to_string();

        // Step 1: Test NewTodo
        println!("Testing new to-do creation...");

        // Create instruction data for NewTodo
        let mut new_todo_data = vec![0]; // 0 = NewTodo instruction
        new_todo_data
            .extend_from_slice(&borsh::to_vec(&todo_name).expect("Failed to serialize todo name"));

        let new_todo_instruction = Instruction::new_with_bytes(
            program_id,
            &new_todo_data,
            vec![
                AccountMeta::new(todo_account_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(clock::id(), false),
            ],
        );

        // Send transaction
        let mut transaction =
            Transaction::new_with_payer(&[new_todo_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &todo_account_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(todo_account_keypair.pubkey())
            .await
            .expect("Failed to get todo account");

        if let Some(account_data) = account {
            let todo_account: TodoAccount = TodoAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize todo account");
            assert_eq!(todo_account.todos.len(), 1);
            assert_eq!(todo_account.todos[0].name, todo_name);
            assert_eq!(todo_account.todos[0].done, false);
            println!("✅ New to-do added: {}", todo_account.todos[0].name);
        }

        // Step 2: Test MarkDone
        println!("Testing mark to-do as done...");

        // Create instruction data for MarkDone
        let mut mark_done_data = vec![1]; // 1 = MarkDone instruction
        mark_done_data
            .extend_from_slice(&borsh::to_vec(&todo_name).expect("Failed to serialize todo name"));

        let mark_done_instruction = Instruction::new_with_bytes(
            program_id,
            &mark_done_data,
            vec![AccountMeta::new(todo_account_keypair.pubkey(), true)],
        );

        // Send transaction
        let mut transaction =
            Transaction::new_with_payer(&[mark_done_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &todo_account_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(todo_account_keypair.pubkey())
            .await
            .expect("Failed to get todo account");

        if let Some(account_data) = account {
            let todo_account: TodoAccount = TodoAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize todo account");
            assert_eq!(todo_account.todos.len(), 1);
            assert_eq!(todo_account.todos[0].name, todo_name);
            assert_eq!(todo_account.todos[0].done, true);
            println!("✅ To-do marked as done: {}", todo_account.todos[0].name);
        }
    }
}
