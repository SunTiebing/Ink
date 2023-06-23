#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod erc20 {
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowance: Mapping<(AccountId, AccountId), Balance>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BalanceTooLow,
        AllowanceTooLow,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: AccountId,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        value: Balance,
    }

    type Result<T> = core::result::Result<T, Error>;

    impl Erc20 {
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            balances.insert(Self::env().caller(), &total_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Self::env().caller(),
                value: total_supply,
            });

            Self {
                total_supply,
                balances,
                ..Default::default()
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, who: AccountId) -> Balance {
            self.balances.get(&who).unwrap_or_default()
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.do_transfer(&sender, &to, value)
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let sender = self.env().caller();
            let allowance = self.allowance.get(&(from, sender)).unwrap_or_default();

            if allowance < value {
                return Err(Error::AllowanceTooLow);
            }

            self.allowance.insert(&(from, sender), &(allowance - value));

            self.do_transfer(&from, &to, value)
        }

        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.allowance.insert(&(sender, to), &value);

            self.env().emit_event(Approval {
                from: sender,
                to,
                value,
            });

            Ok(())
        }

        pub fn do_transfer(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<()> {
            let balance_sender = self.balance_of(*from);
            let balance_to = self.balance_of(*to);

            if value > balance_sender {
                return Err(Error::BalanceTooLow);
            }

            self.balances.insert(from, &(balance_sender - value));
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer {
                from: Some(*from),
                to: *to,
                value,
            });

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        type Event = <Erc20 as ink::reflect::ContractEventBase>::Type;

        #[ink::test]
        fn constructor_works() {
            let erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.total_supply(), 10000);
            assert_eq!(erc20.balance_of(accounts.alice), 10000);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emitted_events[0];

            let decoded =
                <Event as scale::Decode>::decode(&mut &event.data[..]).expect("decoded error");
            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert!(from.is_none(), "mint from error");
                    assert_eq!(to, accounts.alice, "mint from error");
                    assert_eq!(value, 10000, "mint from error");
                }
                _ => panic!("match error"),
            }
        }

        #[ink::test]
        fn balance_of_works() {
            let erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(erc20.balance_of(accounts.alice), 10000);
        }

        #[ink::test]
        fn transfer_works() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(erc20.transfer(accounts.bob, 500), Ok(()));
            assert_eq!(erc20.balance_of(accounts.alice), 9500);
            assert_eq!(erc20.balance_of(accounts.bob), 500);
        }

        #[ink::test]
        fn transfer_fail() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                erc20.transfer(accounts.charlie, 500),
                Err(Error::BalanceTooLow)
            );
        }

        #[ink::test]
        fn transfer_from_works() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(erc20.approve(accounts.bob, 500), Ok(()));

            // Switch to Bob's account
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            assert_eq!(
                erc20.transfer_from(accounts.alice, accounts.charlie, 500),
                Ok(())
            );
            assert_eq!(erc20.balance_of(accounts.alice), 9500);
            assert_eq!(erc20.balance_of(accounts.charlie), 500);
        }

        #[ink::test]
        fn approve_works() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(erc20.approve(accounts.bob, 500), Ok(()));
            let allowance = erc20
                .allowance
                .get(&(accounts.alice, accounts.bob))
                .unwrap_or_default();
            assert_eq!(allowance, 500);
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;

        type E2EResults<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn transfer_works(mut client: ink_e2e::Client<C, E>) -> E2EResults<()> {
            let total_apply = 123;
            let constructor = Erc20Ref::new(total_apply);
            let contract_acc_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;
            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = build_message::<Erc20Ref>(contract_acc_id.clone())
                .call(|erc20| erc20.transfer(bob_acc, 2));
            let res = client.call(&ink_e2e::alice(), transfer_msg, 0, None).await;

            assert!(res.is_ok());

            let balance_of_msg = build_message::<Erc20Ref>(contract_acc_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc));
            let balance_of_alice = client
                .call_dry_run(&ink_e2e::alice(), &balance_of_msg, 0, None)
                .await;

            assert_eq!(balance_of_alice.return_value(), 121);
            Ok(())
        }
    }
}
