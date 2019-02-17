//! The cli used to interact with the woz service.
//!
//! # Usage
//!
//! Install from the [woz site](https://woz.sh) and run the following.
//!
//! ```
//! woz setup
//! ```
//!
//! Follow the instructions to activate your account. Now initialize the current directory to make a woz app.
//!
//! ```
//! woz init
//! ```
//!
//! To deploy to your space:
//!
//! ```
//! woz deploy
//! ```

use std::collections::HashMap;
use std::io;
use std::io::{stdin, stdout, Write};
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::fs;

#[macro_use]
extern crate clap;
use clap::App;

use rusoto_core::{Region, RusotoFuture};
use rusoto_cognito_identity::*;
use rusoto_cognito_idp::CognitoIdentityProvider;
use rusoto_cognito_idp::*;


const IDENTITY_POOL_ID: &'static str = "us-west-2:c2e83f1c-8a6a-482a-9318-2dcab7648fe0";
const IDENTITY_POOL_URL: &'static str = "cognito-idp.us-west-2.amazonaws.com/us-west-2_LAE0aVAzH";
const CLIENT_ID: &'static str = "fbg7q8rv3iu8d8r7n86isq4mg";

fn get_home_path() -> PathBuf {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or(std::env::var_os("HOME"))
        .map(|v| v.into_string().expect("Unable to parse $HOME to string"))
        .expect("No home");
    let mut buf = PathBuf::new();
    buf.push(home);
    buf.push(".woz");
    buf
}

#[test]
// TODO only compile on macOS
fn get_home_path_test() {
    let user = std::env::var_os("USER")
        .map(|v| v.into_string().expect("Could not parse $USER to string"))
        .expect("Could not get a $USER");
    let path_str = format!("/Users/{}/.woz", user);
    assert_eq!(PathBuf::from(path_str), get_home_path());
}

enum Command {
    Init,
    Setup,
    Deploy,
    Update,
    Unknown,
}

impl From<&str> for Command {
    fn from(s: &str) -> Command {
        match s {
            "init" => Command::Init,
            "setup" => Command::Setup,
            "deploy" => Command::Deploy,
            "update" => Command::Update,
            _ => Command::Unknown
        }
    }
}


fn signup(client: CognitoIdentityProviderClient, email: String, username: String, password: String) -> RusotoFuture<SignUpResponse, SignUpError> {
    // Build the request
    let mut request = SignUpRequest::default();
    request.username = username;
    request.password = password;
    request.client_id = String::from(CLIENT_ID);
    let email = AttributeType {
        name: String::from("email"),
        value: Some(email)
    };
    request.user_attributes = Some(vec![email]);

    // Make the request
    client.sign_up(request)
}

fn login(client: &CognitoIdentityProviderClient, username: String, password: String) -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut request = InitiateAuthRequest::default();
    request.auth_flow = String::from("USER_PASSWORD_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("USERNAME"), username);
    auth_params.insert(String::from("PASSWORD"), password);
    request.client_id = String::from(CLIENT_ID);
    request.auth_parameters = Some(auth_params);
    client.initiate_auth(request)
}

#[derive(Debug, Clone)]
struct SignupFormValues {
    email: String,
    username: String,
    password: String,
}

fn signup_form() -> SignupFormValues {
    // TODO validate input
    println!("Entering setup...");
    print!("Please enter a username: ");
    stdout().flush().expect("Error");
    let username_buffer = &mut String::new();
    stdin().read_line(username_buffer).expect("Fail");
    let username = username_buffer.trim_right();

    print!("Please enter a password: ");
    stdout().flush().expect("Error");
    let password_buffer = &mut String::new();
    stdin().read_line(password_buffer).expect("Fail");
    let password = password_buffer.trim_right();

    print!("Please enter a email: ");
    stdout().flush().expect("Error");
    let email_buffer = &mut String::new();
    stdin().read_line(email_buffer).expect("Fail");
    let email = email_buffer.trim_right();

    SignupFormValues {
        email: email.to_owned(),
        username: username.to_owned(),
        password: password.to_owned()
    }
}

fn prompt_login() -> Credentials {
    print!("Please enter a username: ");
    stdout().flush().expect("Error");
    let username_buffer = &mut String::new();
    stdin().read_line(username_buffer).expect("Fail");
    let username = username_buffer.trim_right();

    print!("Please enter a password: ");
    stdout().flush().expect("Error");
    let password_buffer = &mut String::new();
    stdin().read_line(password_buffer).expect("Fail");
    let password = password_buffer.trim_right();

    Credentials {
        username: username.to_owned(),
        password: password.to_owned(),
    }
}

struct Credentials {
    username: String,
    password: String,
}

type IdentityID = String;
type IDToken = String;
type RefreshToken = String;

fn refresh_auth(client: &CognitoIdentityProviderClient, refresh_token: RefreshToken)
                -> RusotoFuture<InitiateAuthResponse, InitiateAuthError> {
    let mut req = InitiateAuthRequest::default();
    req.client_id = CLIENT_ID.to_string();
    req.auth_flow = String::from("REFRESH_TOKEN_AUTH");
    let mut auth_params = HashMap::new();
    auth_params.insert(String::from("REFRESH_TOKEN"), refresh_token);
    req.auth_parameters = Some(auth_params);
    client.initiate_auth(req)
}

fn identity_id(client: &CognitoIdentityClient, id_token: &IDToken)
               -> RusotoFuture<GetIdResponse, GetIdError> {
    let mut logins = HashMap::new();
    logins.insert(IDENTITY_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetIdInput::default();
    req.identity_pool_id = IDENTITY_POOL_ID.to_string();
    req.logins = Some(logins);
    client.get_id(req)
}

fn aws_credentials(client: &CognitoIdentityClient, identity_id: &IdentityID, id_token: &IDToken)
                   -> RusotoFuture<GetCredentialsForIdentityResponse,
                                  GetCredentialsForIdentityError> {
    let mut logins = HashMap::new();
    logins.insert(IDENTITY_POOL_URL.to_string(), id_token.to_owned());

    let mut req = GetCredentialsForIdentityInput::default();
    req.identity_id = identity_id.to_owned();
    req.logins = Some(logins);
    client.get_credentials_for_identity(req)
}

/// Attempt to get a refresh_token token, prompting the user to log in if
/// refresh token is expired and stores it locally.
fn ensure_refresh_token(client: &CognitoIdentityProviderClient) -> RefreshToken {
    let mut path = get_home_path();
    path.push(".refresh_token");
    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let creds = prompt_login();
            let token = login(&client, creds.username, creds.password).sync()
                .and_then(|resp| {
                    let token = resp
                        .authentication_result.expect("Failed")
                        .refresh_token.expect("Missing refresh token");
                    store_token(&token);
                    Ok(token)})
                .or_else::<io::Error, _>(|_| Ok(ensure_refresh_token(client)))
                .expect("Something went wrong");
            Ok(token)
        })
        .unwrap()
}

fn ensure_identity_id(client: &CognitoIdentityClient, id_token: &IDToken)
                      -> IdentityID {
    let mut path = get_home_path();
    path.push(".identity");
    fs::read_to_string(path)
        .or_else::<io::Error, _>(|_| {
            let id = identity_id(client, id_token)
                .sync()
                .expect("Failed to get identity ID")
                .identity_id.expect("No identity ID");
            store_identity_id(&id);
            Ok(id)
        })
        .unwrap()
}

fn store_token(refresh_token: &RefreshToken) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or(std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".refresh_token");

    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(refresh_token.as_bytes()).unwrap();
}

fn store_user_id(user_id: &String) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or(std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".user");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(user_id.as_bytes()).unwrap();
}

fn store_identity_id(id: &IdentityID) {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or(std::env::var_os("HOME"))
        .map(|v| v.into_string().unwrap())
        .expect("No home");
    let mut home_path = PathBuf::new();
    home_path.push(home);
    home_path.push(".woz");

    let mut file_path = home_path.clone();
    file_path.push(".identity");
    fs::create_dir_all(home_path).unwrap();
    let mut f = File::create(&file_path).unwrap();
    f.write_all(id.as_bytes()).unwrap();
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let app = App::from_yaml(yaml);
    let input = app.get_matches();

    if let Some(sub) = input.subcommand_name() {
        match Command::from(sub) {
            // Setup should result in
            // 1. An account
            // 2. A unique user ID
            // 3. A configured dotfile
            Command::Setup => {
                // TODO first check if there is an existing installation
                let values = signup_form();
                dbg!(values.clone());
                let client = CognitoIdentityProviderClient::new(Region::UsWest2);
                signup(client, values.email, values.username, values.password)
                    .sync()
                    .map_err(|e| {
                        let msg = match e {
                            SignUpError::InvalidParameter(msg) => msg,
                            _ => String::from("An unknown error has occurred")
                        };
                        println!("{}", msg);
                    })
                    .and_then(|resp| {
                        let user_id = resp.user_sub;
                        store_user_id(&user_id);
                        Ok(())
                    })
                    .expect("An error occured");
                println!("Please check your inbox for an email to verify your account.");
            },
            // Init should result in
            // 1. A config file in the current directory
            // 2. A new subdomain on woz
            Command::Init => {
                println!("Init...");
                let id_provider_client = CognitoIdentityProviderClient::new(Region::UsWest2);
                let id_client = CognitoIdentityClient::new(Region::UsWest2);
                let refresh_token = ensure_refresh_token(&id_provider_client);
                let id_token = refresh_auth(&id_provider_client, refresh_token)
                    .sync()
                    .expect("Failed to acquire access token")
                    .authentication_result.expect("No auth result")
                    .id_token.expect("No access token");
                let identity_id = ensure_identity_id(&id_client, &id_token);
                let aws_token = aws_credentials(&id_client, &identity_id, &id_token).sync();
                dbg!(aws_token);
                // Write refresh token to disk

                // TODO Acquire an access token
                // TODO Acquire AWS S3 credentials
                // TODO Create a local config file
                // TODO Create a project landing page in S3
                // Print the url
            },
            Command::Deploy => {
                println!("Deploying...");
            }
            // Sub command parsing will print the error and exit
            // before we get to this match statement so the only way
            // we can reach here is if there is a valid subcommand
            // specified, but it hasn't been implemented
            _ => unimplemented!()
        };
    }
}
