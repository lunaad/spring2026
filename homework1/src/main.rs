// Assignment 3: Guessing Game

// check the guess
fn check_guess(guess: i32, secret: i32) -> i32 {
    if guess == secret {
        0      // correct
    } else if guess > secret {
        1      // too high
    } else {
        -1     // too low
    }
}

fn main() {
    // Secret number
    let secret: i32 = 7;

    // Guess counter
    let mut attempts: i32 = 0;

    // Simulated guesses (instead of user input)
    let guesses: [i32; 5] = [3, 5, 9, 7, 10];
    let mut index: usize = 0;

    // Loop until the correct guess is found
    loop {
        let guess = guesses[index];   // simulate user guess
        attempts += 1;

        let result = check_guess(guess, secret);

        if result == 0 {
            println!("Guess {guess}: Correct!");
            break; // exit loop
        } else if result == 1 {
            println!("Guess {guess}: Too high");
        } else {
            println!("Guess {guess}: Too low");
        }

        index += 1; // move to next guess
    }

    // Print number of attempts
    println!("It took {attempts} guesses to find the secret number.");
}


// Assignment 2: Number Analyzer

// check if a number is even
fn is_even(n: i32) -> bool {
    n % 2 == 0
}

fn main() {
    // array of 10 integers
    let numbers: [i32; 10] = [12, 7, 9, 20, 15, 3, 8, 30, 5, 11];

    println!("Number Analysis:");

    // For loop to analyze each number
    for &num in numbers.iter() {
        if num % 3 == 0 && num % 5 == 0 {
            println!("{num}: FizzBuzz");
        } else if num % 3 == 0 {
            println!("{num}: Fizz");
        } else if num % 5 == 0 {
            println!("{num}: Buzz");
        } else if is_even(num) {
            println!("{num}: Even");
        } else {
            println!("{num}: Odd");
        }
    }

    // calculate the sum
    let mut sum = 0;
    let mut i = 0;
    while i < numbers.len() {
        sum += numbers[i];
        i += 1;
    }
    println!("\nSum of all numbers: {sum}");

    // Loop to find the largest number
    let mut largest = numbers[0];
    for &num in numbers.iter() {
        if num > largest {
            largest = num;
        }
    }
    println!("Largest number: {largest}");
}


// Assignment 1: Temperature Converter

// freezing point of water in Fahrenheit
const FREEZING_F: f64 = 32.0;

// fahrenheit to Celsius
fn fahrenheit_to_celsius(f: f64) -> f64 {
    (f - FREEZING_F) * 5.0 / 9.0
}

// Celsius to Fahrenheit
fn celsius_to_fahrenheit(c: f64) -> f64 {
    c * 9.0 / 5.0 + FREEZING_F
}

fn main() {
    
    let mut temp_f: f64 = 32.0;

    // Convert starting temperature
    let temp_c = fahrenheit_to_celsius(temp_f);
    println!("{temp_f}°F = {temp_c:.2}°C");

    // Loop to convert the next 5 integer temperatures
    for _ in 1..=5 {
        temp_f += 1.0;
        let temp_c = fahrenheit_to_celsius(temp_f);
        println!("{temp_f}°F = {temp_c:.2}°C");
    }

    // Example of Celsius to Fahrenheit conversion (optional test)
    let temp_c = 0.0;
    let temp_f = celsius_to_fahrenheit(temp_c);
    println!("{temp_c}°C = {temp_f}°F");
}
