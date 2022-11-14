pub fn compute_score(scoring_algorithm: ScoringAlgorithm, user: User) -> Score {
  let User {
      age,
      income,
      account_balance,
      current_loans,
      credit_card_limit,
      ever_bankrupt,
      loan_payment_failures,
      credit_payment_failures,
      surety_income,
  } = user;
  let ScoringAlgorithm {
      age_score,
      income_score,
      account_balance_score,
      current_loans_score,
      credit_card_score,
      bankruptcy_score,
      loan_payment_history_score,
      credit_payment_history_score,
      surety_income_score,
      score_limits,
  } = scoring_algorithm;

  let age_points = compute_age_points(age, age_score);

  let income_points = compute_income_points(income, income_score);

  let account_balance_points =
      compute_account_balance_points(account_balance, account_balance_score);

  let current_loans_points = compute_current_loans_points(current_loans, current_loans_score);

  let credit_card_points = compute_credit_card_points(credit_card_limit, credit_card_score);

  let bankruptcy_points = compute_bankruptcy_points(ever_bankrupt, bankruptcy_score);

  let loan_payment_history_points =
      compute_loan_payment_history_points(loan_payment_failures, loan_payment_history_score);

  let credit_payment_history_points = compute_credit_payment_history_points(
      credit_payment_failures,
      credit_payment_history_score,
  );

  let surety_income_points = compute_surety_income_points(surety_income, surety_income_score);

  let total_points = age_points
      + income_points
      + account_balance_points
      + current_loans_points
      + credit_card_points
      + bankruptcy_points
      + loan_payment_history_points
      + credit_payment_history_points
      + surety_income_points;

  compute_final_score(total_points, score_limits);
}

fn compute_age_points(age: u8, age_score: [MatchClause; 4]) -> i32 {
  let mut age_points = 0i32;
  for clause in age_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;
              if age as i64 >= min && (age as i64) < max {
                  age_points = age_points + inc
              }
          }
          _ => {}
      }
  }
  age_points
}

fn compute_income_points(income: u32, income_score: [MatchClause; 4]) -> i32 {
  let mut income_points = 0i32;
  for clause in income_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;
              if income as i64 >= min && (income as i64) < max {
                  income_points = income_points + inc
              }
          }
          _ => {}
      }
  }
  income_points
}

fn compute_account_balance_points(
  account_balance: i64,
  account_balance_score: [MatchClause; 4],
) -> i32 {
  let mut account_balance_points = 0i32;
  for clause in account_balance_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;
              if account_balance >= min && account_balance < max {
                  account_balance_points = account_balance_points + inc
              }
          }
          _ => {}
      }
  }
  account_balance_points
}

fn compute_current_loans_points(current_loans: u64, current_loans_score: [MatchClause; 4]) -> i32 {
  let mut current_loans_points = 0i32;
  for clause in current_loans_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;
              if current_loans as i64 >= min && (current_loans as i64) < max {
                  current_loans_points = current_loans_points + inc
              }
          }
          _ => {}
      }
  }
  current_loans_points
}

fn compute_credit_card_points(credit_card_limit: u32, credit_card_score: [MatchClause; 4]) -> i32 {
  let mut credit_card_points = 0i32;
  for clause in credit_card_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;
              if credit_card_limit as i64 >= min && (credit_card_limit as i64) < max {
                  credit_card_points = credit_card_points + inc;
              }
          }
          _ => {}
      }
  }
  credit_card_points
}

fn compute_bankruptcy_points(ever_bankrupt: bool, bankruptcy_score: [MatchClause; 2]) -> i32 {
  let mut bankruptcy_points = 0i32;
  for clause in bankruptcy_score {
      match clause {
          MatchClause::Bool(boolean, points) => {
              let Points { inc } = points;
              if ever_bankrupt == boolean {
                  bankruptcy_points = bankruptcy_points + inc;
              }
          }
          _ => {}
      }
  }
  bankruptcy_points
}

fn compute_loan_payment_history_points(
  loan_payment_failures: u8,
  loan_payment_history_score: [MatchClause; 4],
) -> i32 {
  let mut loan_payment_history_points = 0i32;

  for clause in loan_payment_history_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;

              if loan_payment_failures as i64 >= min && (loan_payment_failures as i64) < max {
                  loan_payment_history_points = loan_payment_history_points + inc
              }
          }
          _ => {}
      }
  }
  loan_payment_history_points
}

fn compute_credit_payment_history_points(
  credit_payment_failures: u8,
  credit_payment_history_score: [MatchClause; 4],
) -> i32 {
  let mut credit_payment_history_points = 0i32;
  for clause in credit_payment_history_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;

              if credit_payment_failures as i64 >= min && (credit_payment_failures as i64) < max {
                  credit_payment_history_points = credit_payment_history_points + inc
              }
          }
          _ => {}
      }
  }
  credit_payment_history_points
}

fn compute_surety_income_points(surety_income: u32, surety_income_score: [MatchClause; 4]) -> i32 {
  let mut surety_income_points = 0i32;

  for clause in surety_income_score {
      match clause {
          MatchClause::Range(range, points) => {
              let Range { min, max } = range;
              let Points { inc } = points;

              if surety_income as i64 >= min && (surety_income as i64) < max {
                  surety_income_points = surety_income_points + inc
              }
          }
          _ => {}
      }
  }
  surety_income_points
}

fn compute_final_score(total_points: i32, score_limits: ScoreLimits) -> Score {
  if total_points <= score_limits.min {
      Score::Bad(0u8)
  } else if total_points >= score_limits.max {
      Score::Good(100u8)
  } else {
      let score = (total_points * 100i32) / score_limits.max;
      if score < 50i32 {
          Score::Bad(score as u8)
      } else {
          Score::Good(score as u8)
      }
  }
}

struct User {
  age: u8,
  income: u32,
  account_balance: i64,
  current_loans: u64,
  credit_card_limit: u32,
  ever_bankrupt: bool,
  loan_payment_failures: u8,
  credit_payment_failures: u8,
  surety_income: u32,
}

struct ScoringAlgorithm {
  age_score: [MatchClause; 4],
  income_score: [MatchClause; 4],
  account_balance_score: [MatchClause; 4],
  current_loans_score: [MatchClause; 4],
  credit_card_score: [MatchClause; 4],
  bankruptcy_score: [MatchClause; 2],
  loan_payment_history_score: [MatchClause; 4],
  credit_payment_history_score: [MatchClause; 4],
  surety_income_score: [MatchClause; 4],
  score_limits: ScoreLimits,
}

enum MatchClause {
  Range(Range, Points),
  Bool(bool, Points),
  None,
}

struct Range {
  min: i64,
  max: i64,
}

struct Points {
  inc: i32,
}

struct ScoreLimits {
  min: i32,
  max: i32,
}

enum Score {
  Good(u8),
  Bad(u8),
}
