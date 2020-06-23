//! TODO: all deserialization shouldn't unwrap
//! TODO: consider error conditions of encryption

use diesel::prelude::*;
use mizu_driver::*;
use mizu_sqlite::MizuConnection;
use mizu_tezos_interface::Tezos;
use mizu_tezos_mock::TezosMock;
use rand::rngs::OsRng;
use std::path::PathBuf;
use structopt::StructOpt;

fn uncons(input: &str) -> Option<(&str, &str)> {
    let start = input.find(|c: char| !c.is_whitespace())?;
    match input[start..].find(char::is_whitespace) {
        Some(len) => Some((&input[start..start + len], &input[start + len..])),
        None => Some((&input[start..], "")),
    }
}

fn uncons_parse<'a, T, H>(input: &'a str, message: &'static str) -> DriverResult<T, (H, &'a str)>
where
    T: Tezos,
    H: std::str::FromStr,
{
    use DriverError::*;

    let (head, rest) = uncons(input).ok_or_else(|| NotFound)?;
    let head = head.parse().map_err(|_| ParseFail(message.into()))?;
    Ok((head, rest))
}

type Command<'a, T> = Box<dyn Fn(&str) -> DriverResult<T, ()> + 'a>;

fn subcommands<'a, T: Tezos>(subcommands: Vec<(&'a str, Command<'a, T>)>) -> Command<'a, T>
where
    T::ReadError: 'a,
    T::WriteError: 'a,
{
    Box::new(move |input| {
        if let Some((head, rest)) = uncons(input) {
            for (key, f) in subcommands.iter() {
                if head.eq_ignore_ascii_case(key) {
                    return f(rest);
                }
            }

            if head.eq_ignore_ascii_case("help") {
                let keys: Vec<&str> = subcommands.iter().map(|(key, _)| key.clone()).collect();
                println!("available commands: {}", keys.join(", "));
            }
        }

        Err(DriverError::NotFound)
    })
}

fn list<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    subcommands::<T>(vec![
        (
            "identity",
            Box::new(move |_input: &str| {
                for id in driver.list_identities()? {
                    println!("{}\t{}\t{}", id.id, id.name, id.created_at);
                }

                Ok(())
            }) as Command<T>,
        ),
        (
            "contact",
            Box::new(move |_input: &str| {
                for contact in driver.list_contacts()? {
                    println!("{}\t{}\t{}", contact.id, contact.name, contact.created_at);
                }

                Ok(())
            }),
        ),
        (
            "message",
            Box::new(move |input: &str| {
                let (our_identity_id, input) =
                    uncons_parse::<T, _>(input, "failed to parse identity id")?;
                let (their_contact_id, _input) =
                    uncons_parse::<T, _>(input, "failed to parse contact id")?;
                for message in driver.list_messages(our_identity_id, their_contact_id)? {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        message.id,
                        message.identity_id,
                        message.contact_id,
                        String::from_utf8_lossy(&message.content),
                        message.created_at
                    );
                }

                Ok(())
            }),
        ),
    ])
}

fn generate<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    use DriverError::*;

    subcommands::<T>(vec![(
        "identity",
        Box::new(move |input: &str| {
            let mut rng = OsRng;

            let (name, _) = uncons(input).ok_or_else(|| NotFound)?;
            driver.generate_identity(&mut rng, name)?;
            println!("generated X3DHClient as {}", name);

            Ok(())
        }) as Command<T>,
    )])
}

fn publish<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    subcommands::<T>(vec![(
        "identity",
        Box::new(move |input: &str| {
            let (identity_id, _input) = uncons_parse::<T, _>(input, "failed to parse identity id")?;
            driver.publish_identity(identity_id)?;
            println!("registered {}", identity_id);

            Ok(())
        }) as Command<T>,
    )])
}

fn add<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    use DriverError::*;

    subcommands::<T>(vec![(
        "contact",
        Box::new(move |input: &str| {
            let (name, rest) = uncons(input).ok_or(NotFound)?;
            let (address, _rest) = uncons(rest).ok_or(NotFound)?;
            driver.add_contact(name, address)
        }) as Command<T>,
    )])
}

fn exist_user<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    use DriverError::*;

    Box::new(move |input: &str| {
        let (address, _rest) = uncons(input).ok_or(NotFound)?;
        match driver.find_user(address)? {
            Some(_) => println!("{} exists", address),
            None => println!("{} doesn't exist", address),
        }

        Ok(())
    })
}

fn post_message<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    use DriverError::*;

    Box::new(move |input: &str| {
        let mut rng = OsRng;

        let (our_identity_id, input) = uncons_parse::<T, _>(input, "failed to parse identity id")?;
        let (their_contact_id, input) = uncons_parse::<T, _>(input, "failed to parse contact id")?;
        let (message, _input) = uncons(input).ok_or(NotFound)?;

        eprintln!("{}\t{}\t{}", our_identity_id, their_contact_id, message);
        driver.post_message(&mut rng, our_identity_id, their_contact_id, message)
    })
}

fn get_messages<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    Box::new(move |input: &str| {
        let mut rng = OsRng;

        let (our_identity_id, input) = uncons_parse::<T, _>(input, "failed to parse identity id")?;
        let (their_contact_id, _input) = uncons_parse::<T, _>(input, "failed to parse contact id")?;

        for message in driver.get_messages(&mut rng, our_identity_id, their_contact_id)? {
            println!("message: {}", String::from_utf8_lossy(&message));
        }

        Ok(())
    })
}

fn commands<T: Tezos>(driver: &Driver<T>) -> Command<T> {
    subcommands::<T>(vec![
        ("list", list(driver)),
        ("generate", generate(driver)),
        ("publish", publish(driver)),
        ("add", add(driver)),
        ("exist", exist_user(driver)),
        ("post", post_message(driver)),
        ("get", get_messages(driver)),
    ])
}

#[derive(StructOpt, Debug)]
struct MockOpt {
    address: Option<String>,
    db_path: Option<String>,
    mock_db_path: Option<String>,
}

#[derive(StructOpt, Debug)]
struct RpcOpt {
    faucet_output: PathBuf,
    config: PathBuf,
    db_path: Option<String>,
}

#[derive(StructOpt, Debug)]
enum Opt {
    Mock(MockOpt),
    Rpc(RpcOpt),
}

fn main() {
    match Opt::from_args() {
        Opt::Mock(opt) => {
            let address = opt
                .address
                .unwrap_or_else(|| std::env::var("TEZOS_ADDRESS").expect("address not given"));
            let db_path = opt
                .db_path
                .unwrap_or_else(|| std::env::var("MIZU_DB").expect("db_path not given"));
            let conn = MizuConnection::connect(&db_path)
                .expect("MizuConnection: failed to establish connection");
            let mock_db_path = opt.mock_db_path.unwrap_or_else(|| {
                std::env::var("MIZU_TEZOS_MOCK").expect("mock_db_path not given")
            });
            let tezos_db_conn = SqliteConnection::establish(&mock_db_path)
                .expect("SqliteConnection: failed to establish connection");
            let tezos = TezosMock::new(&address, &tezos_db_conn);
            let driver = Driver::new(conn, tezos);
            let commands = commands(&driver);

            let mut rl = rustyline::Editor::<()>::new();
            while let Ok(line) = rl.readline("> ") {
                rl.add_history_entry(line.as_str());
                let line = line.trim();
                match commands(line) {
                    Ok(()) => {}
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
        Opt::Rpc(opt) => {
            let db_path = opt
                .db_path
                .unwrap_or_else(|| std::env::var("MIZU_DB").expect("db_path not given"));
            let driver = create_rpc_driver(&opt.faucet_output, &opt.config, &db_path)
                .expect("rpc driver creation should succeed");

            let commands = commands(&driver);

            let mut rl = rustyline::Editor::<()>::new();
            while let Ok(line) = rl.readline("> ") {
                rl.add_history_entry(line.as_str());
                let line = line.trim();
                match commands(line) {
                    Ok(()) => {}
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
    }
}
