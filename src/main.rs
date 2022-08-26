pub mod builtin_words;
use clap::App;
use console;
use core::panic;
use std::io::{self, Write};
use colored::*;
use rand::SeedableRng;
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter};
use serde::{Serialize, Deserialize};

// To definite relevant constants.
const WORD_LENGTH: usize = 5;
const MAX_TRIES: usize = 6;
const DEFAULT_SEED: u64 = 20031007;
const CHAR_LIST: &[char; 26] = &['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'];

// To sanitize words to simpler forms.
fn sanitize(word: &str) -> String {
    word.trim().to_lowercase().chars().filter(|c| c.is_ascii_alphabetic()).collect()
}


// To load, filter and sort the word list.
// Use the data structure "BTreeSet" to realize automatic sorting.
fn load_data(path: &str, dic: &mut Vec<String>) -> BTreeSet<String> {
    let mut word_set: BTreeSet<String> = BTreeSet::new();
    let filename = path;
    if let Err(e) = File::open(filename) {
        panic!("{}", e);
    }
    let file = File::open(filename).unwrap();
    let fin = BufReader::new(file);
    for line in fin.lines() {
        let word = (&line.unwrap()).clone();
        word_set.insert(word);
    }
    for i in &word_set {
        dic.push(i.to_string());
    }
    word_set
}

// To record the state of each Wordle Game.
struct WordleGame {
    word: String,
    guesses: Vec<String>,
    conditions: HashMap<char, char>,
    green_pos: [bool; 5],
    yellow_num: HashMap<char, u64>,
    win: u64,
    lose: u64,
    tries: u64,
    used_words: HashMap<String, u64>,
}

// Use struct "Games" and "Game" to parse json files.
#[derive(Debug, Serialize, Deserialize)]
struct Games {
    #[serde(default = "default_total_rounds")]
    total_rounds: u64,
    #[serde(default = "default_games")]
    games: Vec<Game>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Game {
    #[serde(default = "default_answer")]
    answer: String,
    #[serde(default = "default_guesses")]
    guesses: Vec<String>
}

// Use struct "Configuration" to record default configurations.
#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
    #[serde(default = "default_word")]
    word: String,
    #[serde(default = "default_random")]
    random: bool,
    #[serde(default = "default_difficult")]
    difficult: bool,
    #[serde(default = "default_stats")]
    stats: bool,
    #[serde(default = "default_day")]
    day: u64,
    #[serde(default = "default_seed")]
    seed: u64,
    #[serde(default = "default_final_set")]
    final_set: String,
    #[serde(default = "default_acceptable_set")]
    acceptable_set: String,
    #[serde(default = "default_state")]
    state: String,
}

fn default_total_rounds() -> u64 { 0 }
fn default_games() -> Vec<Game> { let _vec: Vec<Game> = Vec::new(); _vec }
fn default_answer() -> String { let _str: String = String::new(); _str }
fn default_guesses() -> Vec<String> { let _vec:Vec<String> = Vec::new(); _vec }
fn default_word() -> String { let _str: String = String::new(); _str }
fn default_random() -> bool { false }
fn default_difficult() -> bool { false }
fn default_stats() -> bool { false }
fn default_day() -> u64 { 1 }
fn default_seed() -> u64 { DEFAULT_SEED }
fn default_final_set() -> String { let _str: String = String::new(); _str }
fn default_acceptable_set() -> String { let _str: String = String::new(); _str }
fn default_state() -> String { let _str: String = String::new(); _str }

impl Game {
    fn new() -> Self {
        Self { 
            answer: default_answer(), 
            guesses: default_guesses(),
        }
    }
}

impl Configuration {
    fn new() -> Self {
        Self { 
            word: default_word(), 
            random: default_random(), 
            difficult: default_difficult(), 
            stats: default_stats(), 
            day: default_day(), 
            seed: default_seed(), 
            final_set: default_final_set(), 
            acceptable_set: default_acceptable_set(), 
            state: default_state(),
        }
    }    
    fn clone(&self) -> Self {
        Self {
            word: self.word.clone(), 
            random: self.random.clone(), 
            difficult: self.difficult.clone(), 
            stats: self.stats.clone(), 
            day: self.day.clone(), 
            seed: self.seed.clone(), 
            final_set: self.final_set.clone(), 
            acceptable_set: self.acceptable_set.clone(), 
            state: self.state.clone(),
        }
    }
}

impl WordleGame {
    fn new() -> Self {
        let mut cond: HashMap<char, char> = HashMap::new();
        let mut y_num: HashMap<char, u64> = HashMap::new();
        for c in CHAR_LIST { 
            cond.insert(*c, 'X');
            y_num.insert(*c, 0); 
        }
        Self {
            word: String::new(),
            guesses: Vec::new(),
            conditions: cond,
            green_pos: [false, false, false, false, false],
            yellow_num: y_num,
            win: 0,
            lose: 0,
            tries: 0,
            used_words: HashMap::new(),
        }
    }

    fn update(&mut self) {
        self.guesses = Vec::new();
        self.conditions = HashMap::new();
        self.green_pos = [false, false, false, false, false];
        self.yellow_num = HashMap::new();
    }

    // In "Interactive Mode", display the result of each guess.
    fn display_guesses(&mut self) {
        self.guesses.iter().enumerate().for_each(|(guess_number, guess)| {
            print!("{}: ", guess_number + 1);
            let mut green_word_update: [bool; 5] = [false, false, false, false, false];
            let mut yellow_word_update: HashMap<char, u64> = HashMap::new();
            let mut yellow_word_nums: HashMap<char, u64> = HashMap::new();
            for c in CHAR_LIST {
                yellow_word_nums.insert(*c, 0);
                yellow_word_update.insert(*c, 0);
            }
            self.word.trim().chars().enumerate().for_each(|(pos, c)| { 
                if guess.chars().nth(pos).unwrap() != c{
                    let count = yellow_word_nums.entry(c).or_insert(0);
                    *count += 1;
                }
            });
            guess.chars().enumerate().for_each(|(pos, c)| {
                let count = yellow_word_nums.entry(c).or_insert(0);
                let display = if self.word.chars().nth(pos).unwrap() == c {
                    green_word_update[pos] = true;
                    self.conditions.insert(c, 'G');
                    format!("{c}").to_uppercase().bright_green()
                } else if self.word.chars().any(|wc| wc == c) && *count != 0 {
                    *count -= 1;
                    let cnt = yellow_word_update.entry(c).or_insert(0);
                    *cnt += 1;
                    if self.conditions.get(&c).unwrap() != &('G'){
                        self.conditions.insert(c, 'Y');
                    }
                    format!("{c}").to_uppercase().bright_yellow()
                } else {
                    if self.conditions.get(&c).unwrap() == &('X'){
                        self.conditions.insert(c, 'R');
                    }
                    format!("{c}").to_uppercase().red()
                };
                print!("{}", display);
            });
            println!();
            for c in CHAR_LIST {
                let cnt = yellow_word_update.entry(*c).or_insert(0);
                let cur = self.yellow_num.entry(*c).or_insert(0);
                if *cnt > *cur {
                    *cur = *cnt;
                }
            }
            for i in 0..5 {
                if green_word_update[i] == true {
                    self.green_pos[i] = true;
                }
                if self.green_pos[i] == true {
                    let c = self.word.chars().nth(i).unwrap();
                    let cur = self.yellow_num.entry(c).or_insert(0);
                    if *cur > 0 {
                        *cur -= 1;
                    }
                }
            }
        });
        println!("The state of all letters are shown below: ");
        self.display_letters_state();    
    }

    // In "Test Mode", display the result of each guess.
    fn display_guesses_test(&mut self, guess: &str) {
        let mut green_word_update: [bool; 5] = [false, false, false, false, false];
        let mut yellow_word_update: HashMap<char, u64> = HashMap::new();
        let mut yellow_word_nums: HashMap<char, u64> = HashMap::new();
        for c in CHAR_LIST {
            yellow_word_nums.insert(*c, 0);
            yellow_word_update.insert(*c, 0);
        }
        self.word.trim().chars().enumerate().for_each(|(pos, c)| { 
            if guess.chars().nth(pos).unwrap() != c{
                let count = yellow_word_nums.entry(c).or_insert(0);
                *count += 1;
            }
        });
        guess.chars().enumerate().for_each(|(pos, c)| {
            let count = yellow_word_nums.entry(c).or_insert(0);
            let display = if self.word.chars().nth(pos).unwrap() == c {
                green_word_update[pos] = true;
                self.conditions.insert(c, 'G');
                'G'
            } else if self.word.chars().any(|wc| wc == c) && *count != 0 {
                *count -= 1;
                let cnt = yellow_word_update.entry(c).or_insert(0);
                *cnt += 1;
                if self.conditions.get(&c).unwrap() != &('G'){
                    self.conditions.insert(c, 'Y');
                }
                'Y'
            } else {
                if self.conditions.get(&c).unwrap() == &('X'){
                    self.conditions.insert(c, 'R');
                }
                'R'
            };
            print!("{display}");
        });
        print!(" ");
        for c in CHAR_LIST {
            print!("{}", self.conditions.get(&c).unwrap());
        }
        println!();
        self.green_pos = green_word_update.clone();
        self.yellow_num = yellow_word_update.clone();
    }

    // In "Difficult Mode", display the result of each guess.
    fn display_guesses_difficult(&mut self) {
        self.guesses.iter().enumerate().for_each(|(guess_number, guess)| {
            print!("{}: ", guess_number + 1);
            let mut green_word_update: [bool; 5] = [false, false, false, false, false];
            let mut yellow_word_update: HashMap<char, u64> = HashMap::new();
            let mut yellow_word_nums: HashMap<char, u64> = HashMap::new();
            for c in CHAR_LIST {
                yellow_word_nums.insert(*c, 0);
                yellow_word_update.insert(*c, 0);
            }
            self.word.trim().chars().enumerate().for_each(|(pos, c)| { 
                if guess.chars().nth(pos).unwrap() != c{
                    let count = yellow_word_nums.entry(c).or_insert(0);
                    *count += 1;
                }
            });
            guess.chars().enumerate().for_each(|(pos, c)| {
                let count = yellow_word_nums.entry(c).or_insert(0);
                let display = if self.word.chars().nth(pos).unwrap() == c {
                    green_word_update[pos] = true;
                    self.conditions.insert(c, 'G');
                    format!("{c}").to_uppercase().bright_green()
                } else if self.word.chars().any(|wc| wc == c) && *count != 0 {
                    *count -= 1;
                    let cnt = yellow_word_update.entry(c).or_insert(0);
                    *cnt += 1;
                    if self.conditions.get(&c).unwrap() != &('G'){
                        self.conditions.insert(c, 'Y');
                    }
                    format!("{c}").to_uppercase().bright_yellow()
                } else {
                    if self.conditions.get(&c).unwrap() == &('X'){
                        self.conditions.insert(c, 'R');
                    }
                    format!("{c}").to_uppercase().red()
                };
                print!("{}", display);
            });
            println!();
            for i in 0..5 {
                if green_word_update[i] == true {
                    self.green_pos[i] = true;
                }
            }
            self.green_pos = green_word_update.clone();
            self.yellow_num = yellow_word_update.clone();
        });
        println!("The state of all letters are shown below: ");
        self.display_letters_state();    
    }

    // In "Interactive Mode", display the state of each letter.
    fn display_letters_state(&self) {
        for c in CHAR_LIST {
            if self.conditions.get(&c).unwrap() == &('G') {
                print!("{} ", format!("{c}").to_uppercase().bright_green());
            } else if self.conditions.get(&c).unwrap() == &('Y') {
                print!("{} ", format!("{c}").to_uppercase().bright_yellow());
            } else if self.conditions.get(&c).unwrap() == &('R') {
                print!("{} ", format!("{c}").to_uppercase().red());
            } else {
                print!("{} ", format!("{c}").to_uppercase());
            }
        }
        println!();
    }

    // In "Interactive Mode", get the player's input and determine if it is valid.
    fn ask_for_guess(&mut self, acceptable_dic: &Vec<String>) -> String {
        println!("{}", format!("Enter your guess (5 letters) and press ENTER: {} tries left", MAX_TRIES - self.guesses.len()).cyan());
        let mut guess = String::new();
        let mut valid_guess = false;
        while !valid_guess {
            guess = String::new();
            std::io::stdin().read_line(&mut guess).unwrap();
            guess = sanitize(&guess);
            if guess.len() != WORD_LENGTH {
                if guess == "hint" {
                    self.word_hint(acceptable_dic);
                } else {
                    println!("{}", format!("INVALID! Your guess must be 5 letters.").red())
                }
            } else if !acceptable_dic.iter().any(|word| word==&guess) {
                println!("{} {} {}", "INVALID! The word".red(), guess.to_uppercase().red(), "isn't in the Wordle dictionary.".red())
            } else {
                self.guesses.push(guess.clone());
                *self.used_words.entry(guess.clone()).or_insert(0) += 1;
                valid_guess = true;
            }
        }
        guess
    }

    // In "Test Mode", get the player's input and determine if it is valid.
    fn ask_for_guess_test(&mut self, acceptable_dic: &Vec<String>) -> String {
        let mut guess = String::new();
        let mut valid_guess = false;
        while !valid_guess {
            guess = String::new();
            std::io::stdin().read_line(&mut guess).unwrap();
            guess = sanitize(&guess);
            if guess.trim().len() != WORD_LENGTH {
                println!("INVALID")
            } else if !acceptable_dic.iter().any(|word| word.trim()==&guess) {
                println!("INVALID")
            } else {
                self.guesses.push(guess.clone());
                *self.used_words.entry(guess.clone()).or_insert(0) += 1;
                valid_guess = true;
            }
        }
        guess
    }

    // In "Interactive Difficult Mode", get the player's input and determine if it is valid.
    fn ask_for_guess_difficult(&mut self, acceptable_dic: &Vec<String>) -> String {
        println!("{}", format!("Enter your guess (5 letters) and press ENTER: {} tries left", MAX_TRIES - self.guesses.len()).cyan());
        let mut guess = String::new();
        let mut valid_guess = false;
        while !valid_guess {
            guess = String::new();
            std::io::stdin().read_line(&mut guess).unwrap();
            guess = sanitize(&guess);
            if guess.len() != WORD_LENGTH {
                if guess == "hint" {
                    self.word_hint(acceptable_dic);
                } else {
                    println!("{}", format!("INVALID! Your guess must be {} letters.", WORD_LENGTH).red())
                }
            } else if !acceptable_dic.iter().any(|word| word==&guess) {
                println!("{} {} {}", "INVALID! The word".red(), guess.to_uppercase().red(), "isn't in the Wordle dictionary.".red())
            } else if self.check_guess_difficult(&guess) == false {
                println!("{}", "INVALID! Please ensure that you follow the rules of difficult mode.".red())
            } else {
                self.guesses.push(guess.clone());
                *self.used_words.entry(guess.clone()).or_insert(0) += 1;
                valid_guess = true;
            }
        }
        guess
    }
    
    // In "Test Difficult Mode", get the player's input and determine if it is valid.
    fn ask_for_guess_test_difficult(&mut self, acceptable_dic: &Vec<String>) -> String {
        let mut guess = String::new();
        let mut valid_guess = false;
        while !valid_guess {
            guess = String::new();
            std::io::stdin().read_line(&mut guess).unwrap();
            guess = sanitize(&guess);
            if guess.len() != WORD_LENGTH {
                println!("INVALID")
            } else if !acceptable_dic.iter().any(|word| word.trim()==&guess) {
                println!("INVALID")
            } else if self.check_guess_difficult(&guess) == false {
                println!("INVALID")
            } else {
                self.guesses.push(guess.clone());
                *self.used_words.entry(guess.clone()).or_insert(0) += 1;
                valid_guess = true;
            }
        }
        guess
    }

    // In "Random Mode", determine if the player has guessed correctly.
    fn is_game_over(&mut self, guess: &str) -> bool {
        self.display_guesses();
        let n_tries = self.guesses.len();
        if guess.to_string().trim() == self.word.trim() {
            println!("CORRECT! You guessed the word in {} tries.", n_tries);
            self.win += 1;
            self.tries += n_tries as u64;
            true
        } else if n_tries >= MAX_TRIES {
            println!("{}", format!("SHAME! You ran out of tries! The word was {}", self.word).bright_red().trim());
            self.lose += 1;
            true
        } else { false }
    }

    // In "Difficult Mode", get the player's input and determine if it is valid.
    fn is_game_over_difficult(&mut self, guess: &str) -> bool {
        self.display_guesses_difficult();
        let n_tries = self.guesses.len();
        if guess.to_string().trim() == self.word.trim() {
            println!("CORRECT! You guessed the word in {} tries!", n_tries);
            self.win += 1;
            self.tries += n_tries as u64;
            true
        } else if n_tries >= MAX_TRIES {
            println!("{}", format!("WRONG! You ran out of tries! The word was {}.", self.word).bright_red().trim());
            self.lose += 1;
            true
        } else { false }
    }

    // In "Test Mode", get the player's input and determine if it is valid.
    fn is_game_over_test(&mut self, guess: &str) -> bool {
        let n_tries = self.guesses.len();
        if guess.to_string().trim() == self.word.trim() {
            self.win += 1;
            self.tries += n_tries as u64;
            println!("CORRECT {}", n_tries);
            true
        } else if n_tries >= MAX_TRIES {
            self.lose += 1;
            println!("{}", format!("FAILED {}", self.word.to_uppercase()).bright_red().trim());
            true
        } else { false }
    }

    // With the parameter "-t/--stats", print relevant statistics.
    fn print_info(&mut self) {
        print!("{} ", self.win);
        print!("{} ", self.lose);
        if self.win == 0 { println!("0.00"); }
        else {
            let average: f32 = self.tries as f32 / self.win as f32;
            println!("{:.2}", average);
        }
        let mut cnt_words: Vec<(&String, &u64)> = self.used_words.iter().collect();
        cnt_words.sort_by(|a, b| b.1.cmp(a.1));
        let length = cnt_words.len();
        let mut index: Vec<usize> = Vec::new();
        index.push(0);
        for i in 0..(length - 1) {
            if cnt_words[i].1 != cnt_words[i + 1].1 {
                index.push(i + 1);
            }
        }
        index.push(length);
        for i in 0..index.len() - 1 {
            let slice = &mut cnt_words[index[i]..index[i + 1]];
            slice.sort_by(|a, b| a.0.cmp(b.0));
        }
        if length <= WORD_LENGTH {
            for i in 0..(length - 1) {
                print!("{} {} ", cnt_words[i].0.to_uppercase(), cnt_words[i].1);
            }
            print!("{} {}", cnt_words[length - 1].0.to_uppercase(), cnt_words[length - 1].1);
        }
        else {
            for i in 0..4 {
                print!("{} {} ", cnt_words[i].0.to_uppercase(), cnt_words[i].1);
            }
            print!("{} {}", cnt_words[4].0.to_uppercase(), cnt_words[4].1);
        }
        println!();
    }

    // In "Difficult Mode", check if the guessed word follows the rules.
    fn check_guess_difficult(&mut self, guess: &str) -> bool {
        for c in 0..WORD_LENGTH {
            if self.green_pos[c] == true {
                if guess.trim().chars().nth(c).unwrap() != self.word.trim().chars().nth(c).unwrap() {
                    return false;
                }
            }
        }
        let mut check_yellow_num = self.yellow_num.clone();
        for c in CHAR_LIST {
            let cnt = check_yellow_num.entry(*c).or_insert(0);
            for i in 0..WORD_LENGTH {
                if guess.trim().chars().nth(i).unwrap() == *c && self.green_pos[i] == false {
                    if *cnt > 0 { *cnt -= 1; }
                }
            }
            if *cnt > 0 { return false; }
        }
        true
    }
    
    // Used to give hints.
    fn check_guess_hint(&mut self, guess: &str) -> bool {
        for c in 0..WORD_LENGTH {
            if self.green_pos[c] == true {
                if guess.trim().chars().nth(c).unwrap() != self.word.trim().chars().nth(c).unwrap() {
                    return false;
                }
            }
            let c = guess.trim().chars().nth(c).unwrap();
            if self.conditions.get(&c).unwrap() == &('R') {
                return false;
            }
        }
        let mut check_yellow_num = self.yellow_num.clone();
        for c in CHAR_LIST {
            let cnt = check_yellow_num.entry(*c).or_insert(0);
            for i in 0..WORD_LENGTH {
                if guess.trim().chars().nth(i).unwrap() == *c && self.green_pos[i] == false {
                    if *cnt > 0 { *cnt -= 1; }
                }
            }
            if *cnt > 0 { return false; }
        }
        true
    }
    
    // In "Interactive Mode", execute the game.
    fn execute_game(&mut self, game_config: &Configuration, acceptable_dic: &Vec<String>) {
        loop {
            let mut guess = String::new();
            if game_config.difficult == false {
                guess = self.ask_for_guess(acceptable_dic);
                if self.is_game_over(&guess) {
                    if game_config.stats == true {
                        self.print_info();
                    }
                    break;
                }
                println!();
            } else {
                guess = self.ask_for_guess_difficult(acceptable_dic);
                if self.is_game_over_difficult(&guess) {
                    if game_config.stats == true {
                        self.print_info();
                    }
                    break;
                }
                println!();
            }
        }
    }
    
    // In "Test Mode", execute the game.
    fn execute_game_test(&mut self, game_config: &Configuration, acceptable_dic: &Vec<String>) {
        loop {
            let mut guess = String::new();
            if game_config.difficult == false {
                guess = self.ask_for_guess_test(acceptable_dic);
            } else {
                guess = self.ask_for_guess_test_difficult(acceptable_dic);
            }
            self.display_guesses_test(&guess);
            if self.is_game_over_test(&guess) {
                if game_config.stats == true {
                    self.print_info();
                }
                break;
            }
        }
    }

    fn color_initialization(&mut self) {
        for c in CHAR_LIST { 
            self.conditions.insert(*c, 'X');
            self.yellow_num.insert(*c, 0); 
        }
    }

    // In "Interactive Mode", give hints about the answer word.
    fn word_hint(&mut self, acceptable_dic: &Vec<String>) {
        let mut ans = Vec::new();
        for possible_word in acceptable_dic {
            if self.check_guess_hint(&possible_word) {
                ans.push(possible_word.to_uppercase().clone());
            }
        }
        println!("Here are {} possible words to solve the Wordle game:", ans.len());
        println!();
        for possible_word in ans {
            println!("{}", possible_word);
        }
        println!();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialization of Wordle Game, Command Line Parameters and Configuration Parameters.
    let mut game = WordleGame::new();
    let yml = clap::load_yaml!("yaml.yml");
    let matches = App::from_yaml(yml).get_matches();
    let mut game_config = Configuration::new();

    // Initialization of available words.
    let mut final_dictionary: Vec<String> = builtin_words::FINAL.iter().map(|&x| x.to_string()).collect();
    let mut acceptable_dictionary: Vec<String> = builtin_words::ACCEPTABLE.iter().map(|&x| x.to_string()).collect();
    
    // Deal with parameter "-c".
    if let Some(path) = matches.value_of("load_configuration"){
        if let Err(e) = File::open(path) { panic!("{}", e); }
        let filename = File::open(path).unwrap();
        let config: Configuration = serde_json::from_reader(filename).unwrap();
        game_config = config.clone(); 
    }

    // Update configuration file "game_config".
    if let Some(word) = matches.value_of("input_word"){ game_config.word = word.to_string(); }
    if matches.occurrences_of("random_word") == 1 { game_config.random = true; }
    if matches.occurrences_of("difficult_word") == 1 { game_config.difficult = true; }
    if matches.occurrences_of("statistical_word") == 1 { game_config.stats = true; }
    if let Some(path_final) = matches.value_of("set_final_words") {
        if let Some(path_acceptable) = matches.value_of("set_acceptable_words") {
            game_config.final_set = path_final.to_string();
            game_config.acceptable_set = path_acceptable.to_string();
        }
    }
    if let Some(rand_day) = matches.value_of("rand_day") { game_config.day = rand_day.trim().parse().unwrap(); }
    if let Some(rand_seed) = matches.value_of("rand_seed") { game_config.seed = rand_seed.trim().parse().unwrap(); }
    if let Some(states) = matches.value_of("load_state") { game_config.state = states.to_string(); }

    // Deal with parameter "-a", "-f".
    if !game_config.final_set.is_empty() && !game_config.acceptable_set.is_empty() {
        let mut tmp_final_dic: Vec<String> = Vec::new();
        let mut tmp_acceptable_dic: Vec<String> = Vec::new();
        let set_final = load_data(&game_config.final_set, &mut tmp_final_dic);
        let set_acceptable = load_data(&game_config.acceptable_set, &mut tmp_acceptable_dic);
        assert_eq!(set_final.is_subset(&set_acceptable), true);
        final_dictionary = tmp_final_dic.clone();
        acceptable_dictionary = tmp_acceptable_dic.clone();
    }

    // Deal with parameter "-S".
    if !game_config.state.is_empty(){
        if let Err(e) = File::open(game_config.state.clone()) { panic!("{}", e); }
        let filename = File::open(game_config.state.clone()).unwrap();
        let game_json: Games = serde_json::from_reader(filename).unwrap();
        for single_game in &game_json.games {
            for guess in &single_game.guesses {
                *game.used_words.entry(guess.to_lowercase().clone()).or_insert(0) += 1;
            }
            if single_game.answer == single_game.guesses[single_game.guesses.len() - 1] {
                game.win += 1;
            } else {
                game.lose += 1;
            }
            game.tries += single_game.guesses.len() as u64;
        }
    }

    // Game Start: Interactive Mode.
    let is_tty = atty::is(atty::Stream::Stdout);
    if is_tty {
        print!("{}", console::style("Please enter your name: ").bold().red());
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        println!("Welcome to Wordle, {}!", line.trim());
        println!();

        if !game_config.word.is_empty() {
            if game_config.random == true || game_config.seed != DEFAULT_SEED || !game_config.state.is_empty() {
                panic!("Contradictory parameters!")
            }
            game.word = game_config.word.to_lowercase();
            game.execute_game(&game_config, &acceptable_dictionary);
        }
        else if game_config.random == false {   
            if game_config.seed != DEFAULT_SEED || !game_config.state.is_empty() {
                panic!("Contradictory parameters!")
            }
            loop {  
                println!("Input a word as the answer of this Wordle Game: ");
                let mut read_word = String::new();
                std::io::stdin().read_line(&mut read_word).unwrap();
                game.word = read_word.clone().to_lowercase();
                game.update();
                game.color_initialization();
                game.execute_game(&game_config, &acceptable_dictionary);
                println!();
                println!("Type in 'Y' to continue...");
                println!("Type in 'N' to quit...");
                
                let mut ans = String::new();
                io::stdin().read_line(&mut ans).unwrap();
                let ans = ans.trim();
                if ans == "Y" {
                    continue;
                }
                else if ans == "N" || ans.len() == 0 {
                    break;
                }
            }
        }
        else {
            let mut day: u64 = game_config.day;
            day -= 1;
            let seed: u64 = game_config.seed;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let mut array = final_dictionary.clone();
            array.shuffle(&mut rng);
            
            loop {
                if day > array.len() as u64 - 1 {
                    day -= array.len() as u64;
                }
                game.word = array[day as usize].to_string();
                game.update();
                game.color_initialization();
                game.execute_game(&game_config, &acceptable_dictionary);
                
                if !game_config.state.is_empty() {
                    if let Err(e) = File::open(&game_config.state) { panic!("{}", e); }
                    let filename = File::open(&game_config.state).unwrap();
                    let mut game_json: Games = serde_json::from_reader(filename).unwrap();
                    game_json.total_rounds += 1;
                    let mut single_game: Game = Game::new();
                    single_game.answer = game.word.to_uppercase();
                    single_game.guesses = game.guesses.iter().map(|x| x.to_uppercase()).collect();
                    game_json.games.push(single_game);
                    let file = OpenOptions::new().write(true).create(true).open(&game_config.state)?;
                    let buf_writer = BufWriter::new(file);
                    serde_json::to_writer_pretty(buf_writer, &game_json).unwrap();
                }
                println!();
                println!("Type in 'Y' to continue...");
                println!("Type in 'N' to quit...");
                
                let mut ans = String::new();
                io::stdin().read_line(&mut ans).unwrap();
                let ans = ans.trim();
                if ans == "Y" {
                    day += 1;
                    continue;
                }
                else if ans == "N" || ans.len() == 0 {
                    break;
                }
            }
        }
    } 

    // Game Start: Test Mode.
    if !is_tty {
        if !game_config.word.is_empty() {
            if game_config.random == true || game_config.seed != DEFAULT_SEED || !game_config.state.is_empty() {
                panic!("Contradictory parameters!")
            }
            game.word = game_config.word.to_lowercase();
            game.execute_game_test(&game_config, &acceptable_dictionary);
        }
        else if game_config.random == false {  
            if game_config.seed != DEFAULT_SEED || !game_config.state.is_empty() {
                panic!("Contradictory parameters!")
            }  
            loop {
                let mut read_word = String::new();
                std::io::stdin().read_line(&mut read_word).unwrap();
                game.word = read_word.clone().to_lowercase();
                game.update();
                game.color_initialization();
                game.execute_game_test(&game_config, &acceptable_dictionary);
                
                let mut ans = String::new();
                io::stdin().read_line(&mut ans).unwrap();
                let ans = ans.trim();
                if ans == "Y" {
                    continue;
                }
                else if ans == "N" || ans.len() == 0 {
                    break;
                }
            }
        }
        else {
            let mut day: u64 = game_config.day;
            day -= 1;
            let seed: u64 = game_config.seed;
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let mut array = final_dictionary.clone();
            array.shuffle(&mut rng);
            
            loop {
                if day > array.len() as u64 - 1 {
                    day -= array.len() as u64;
                }
                game.word = array[day as usize].to_string();
                game.update();
                game.color_initialization();
                game.execute_game_test(&game_config, &acceptable_dictionary);
                
                if !game_config.state.is_empty() {
                    if let Err(e) = File::open(&game_config.state) { panic!("{}", e); }
                    let filename = File::open(&game_config.state).unwrap();
                    let mut game_json: Games = serde_json::from_reader(filename).unwrap();
                    game_json.total_rounds += 1;
                    let mut single_game: Game = Game::new();
                    single_game.answer = game.word.to_uppercase();
                    single_game.guesses = game.guesses.iter().map(|x| x.to_uppercase()).collect();
                    game_json.games.push(single_game);
                    if let Err(e) = File::open(&game_config.state) { panic!("{}", e); }
                    let file = OpenOptions::new().write(true).create(true).open(&game_config.state)?;
                    let buf_writer = BufWriter::new(file);
                    serde_json::to_writer_pretty(buf_writer, &game_json).unwrap();
                }
                
                let mut ans = String::new();
                io::stdin().read_line(&mut ans).unwrap();
                let ans = ans.trim();
                if ans == "Y" {
                    day += 1;
                    continue;
                }
                else if ans == "N" || ans.len() == 0 {
                    break;
                }
            }
        }
    }
    Ok(())
}
