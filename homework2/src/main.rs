fn most_frequent_word(text: &str) -> (String, usize) {
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut max_word = String::new();
    let mut max_count = 0;

    // pick each word
    for i in 0..words.len() {
        let current = words[i];
        let mut count = 0;

        //count occurrences
        for j in 0..words.len() {
            if words[j] == current {
                count += 1;
            }
        }

        // update
        if count > max_count {
            max_count = count;
            max_word = current.to_string();
        }
    }

    (max_word, max_count)
}

fn main() {
    let text = "the quick brown fox jumps over the lazy dog the quick brown fox";
    let (word, count) = most_frequent_word(text);
    println!("Most frequent word: \"{}\" ({} times)", word, count);
}
