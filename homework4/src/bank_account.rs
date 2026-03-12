#[derive(Debug)]
pub struct BankAccount {
    balance: f64,
}

impl BankAccount {
    pub fn new(initial_balance: f64) -> BankAccount {
        BankAccount {
            balance: initial_balance
        }
    }

    pub fn deposit(&mut self, amount: f64) {
        if amount >= 0.0 {
            self.balance += amount;
        }
    }

    pub fn withdraw(&mut self, amount: f64) {
        if amount >= 0.0 && amount <= self.balance {
            self.balance -= amount;
        }
    }

    pub fn balance(&self) -> f64 {
        self.balance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_new_account() {
        let account = BankAccount::new(100.0);
        assert!((account.balance() - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_deposit() {
        let mut account = BankAccount::new(50.0);
        account.deposit(25.0);

        assert!((account.balance() - 75.0).abs() < EPSILON);
    }

    #[test]
    fn test_withdraw() {
        let mut account = BankAccount::new(100.0);
        account.withdraw(40.0);

        assert!((account.balance() - 60.0).abs() < EPSILON);
    }

    #[test]
    fn test_withdraw_more_than_balance() {
        let mut account = BankAccount::new(100.0);
        account.withdraw(200.0);

        assert!((account.balance() - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_deposit_negative_amount() {
        let mut account = BankAccount::new(100.0);
        account.deposit(-50.0);

        assert!((account.balance() - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_withdraw_negative_amount() {
        let mut account = BankAccount::new(100.0);
        account.withdraw(-30.0);

        assert!((account.balance() - 100.0).abs() < EPSILON);
    }

    #[test]
    fn test_balance() {
        let account = BankAccount::new(250.0);

        assert!((account.balance() - 250.0).abs() < EPSILON);
    }
}