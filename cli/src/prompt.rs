use crossterm::{input, InputEvent, KeyEvent, RawScreen, TerminalInput};
use std::io::{self, stdout, Write};

fn raw_read_line(input: &mut TerminalInput, _raw: &RawScreen) -> Option<String> {
    let mut stdin = input.read_sync();

    let mut string = String::new();
    
    for event in stdin.next() {
        if let InputEvent::Keyboard(KeyEvent::Char(c)) = event {
            match c {
                '\0' | '\x03' | '\x04' => return None,
                '\x7f' => {
                    string.pop();
                }
                '\n' | '\r' => break,
                c => string.push(c),
            }
        }
    }
    
    Some(string)
}

fn read_passwd(input: &mut TerminalInput) -> io::Result<Option<String>> {
    let raw = RawScreen::into_raw_mode()?;
    Ok(raw_read_line(input, &raw))
}


fn username_password() -> (String, String) {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    let mut input = input();

    stdout.write_all(b"Username: ").unwrap();
    stdout.flush().expect("Error");
    let username = input.read_line().expect("Fail").to_lowercase();

    stdout.write_all(b"Password: ").unwrap();
    stdout.flush().expect("Error");
    let password_input = read_passwd(&mut input);


    if let Ok(Some(password)) = password_input {
        stdout.write_all(b"\n").unwrap();
        (username, password)
    } else {
        stdout.write_all(b"Error\n").unwrap();
        panic!("Failed to get password");
    }
}

fn email() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    let input = input();

    stdout.write_all(b"Email: ").unwrap();
    stdout.flush().expect("Error");
    input.read_line().expect("Fail").to_lowercase()
}

#[derive(Debug, Clone)]
pub struct SignupValues {
    pub email: String,
    pub username: String,
    pub password: String,
}

pub fn signup() -> SignupValues {
    println!("Please enter the following information");
    let (username, password) = username_password();
    let email = email();

    SignupValues {
        email: email.to_owned(),
        username: username.to_owned(),
        password: password.to_owned()
    }
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn login() -> Credentials {
    print!("Please login to continue: ");
    let (username, password) = username_password();

    Credentials {
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

pub fn is_email_verified() -> bool {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    stdout.write_all(b"Have you clicked the link in the verification email? [y/n] ")
        .unwrap();
    stdout.flush().expect("Error");

    if let Ok(_raw) = RawScreen::into_raw_mode() {
        let input = input();

        let mut stdin = input.read_sync();
        
        let mut out = false;
        for event in stdin.next() {
            if let InputEvent::Keyboard(c) = event {
                match c {
                    KeyEvent::Char('y') => out = true,
                    KeyEvent::Char('n') => out = false,
                    _ => {println!("Sorry didn't catch that"); out = false},
                };
                stdout.flush().expect("Error");
                break
            }
        };
        out
    } else {
        false
    }
}
