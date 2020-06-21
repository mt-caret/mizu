use mizu_crypto::{
    keys::{IdentityPublicKey, PrekeyPublicKey},
    x3dh::X3DHClient,
    Client,
};
use mizu_sqlite::MizuConnection;
use rand::rngs::OsRng;
use tezos_interface::Tezos;
use tezos_mock::TezosMock;
use thiserror::Error;

use std::convert::TryInto;

type DieselError = diesel::result::Error;

#[derive(Debug, Error)]
enum DriverError {
    #[error("failed to parse command: {0}")]
    ParseFail(String),
    #[error("something not found")]
    NotFound,
    #[error("persistency layer: {0}")]
    UserData(DieselError),
    #[error("Tezos: {0}")]
    Tezos(DieselError),
}

fn uncons(input: &str) -> Option<(&str, &str)> {
    let start = input.find(|c: char| !c.is_whitespace())?;
    match input[start..].find(char::is_whitespace) {
        Some(len) => Some((&input[start..start + len], &input[start + len..])),
        None => Some((&input[start..], "")),
    }
}

fn uncons_parse<'a, T: std::str::FromStr>(
    input: &'a str,
    message: &str,
) -> Result<(T, &'a str), DriverError> {
    use DriverError::*;

    let (head, rest) = uncons(input).ok_or_else(|| NotFound)?;
    let head = head.parse().map_err(|_| ParseFail(message.into()))?;
    Ok((head, rest))
}

type Command<'a> = Box<dyn Fn(&str) -> Result<(), DriverError> + 'a>;

fn subcommands<'a, I: IntoIterator<Item = (&'a str, Command<'a>)>>(subcommands: I) -> Command<'a> {
    let subcommands: Vec<_> = subcommands.into_iter().collect();
    Box::new(move |input| {
        if let Some((head, rest)) = uncons(input) {
            for (key, f) in subcommands.iter() {
                if head.eq_ignore_ascii_case(key) {
                    return f(rest);
                }
            }
        }

        Err(DriverError::NotFound)
    })
}

fn list(user_data: &MizuConnection) -> Command {
    use DriverError::*;

    subcommands(vec![
        (
            "identity",
            Box::new(move |_input: &str| {
                for id in user_data.list_identities().map_err(UserData)? {
                    println!("{}\t{}\t{}", id.id, id.name, id.created_at);
                }

                Ok(())
            }) as Command,
        ),
        (
            "contact",
            Box::new(move |_input: &str| {
                for contact in user_data.list_contacts().map_err(UserData)? {
                    println!("{}\t{}\t{}", contact.id, contact.name, contact.created_at);
                }

                Ok(())
            }),
        ),
        (
            "message",
            Box::new(move |input: &str| {
                let (our_identity_id, input) = uncons_parse(input, "failed to parse identity id")?;
                let (their_contact_id, _input) = uncons_parse(input, "failed to parse contact id")?;
                for message in user_data
                    .find_messages(our_identity_id, their_contact_id)
                    .map_err(UserData)?
                {
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

fn generate(user_data: &MizuConnection) -> Command {
    use DriverError::*;

    subcommands(vec![(
        "identity",
        Box::new(move |input: &str| {
            let mut rng = OsRng;

            let (name, _) = uncons(input).ok_or_else(|| NotFound)?;
            let x3dh = X3DHClient::new(&mut rng);
            user_data.create_identity(name, &x3dh).map_err(UserData)?;
            println!("generated X3DHClient as {}", name);

            Ok(())
        }) as Command,
    )])
}

fn register<'a>(
    address: &'a [u8],
    tezos: &'a TezosMock,
    user_data: &'a MizuConnection,
) -> Command<'a> {
    use DriverError::*;

    subcommands(vec![
        (
            "identity",
            Box::new(move |input: &str| {
                let (identity_id, _input) = uncons_parse(input, "failed to parse identity id")?;
                let identity = user_data.find_identity(identity_id).map_err(UserData)?;
                let x3dh: X3DHClient = bincode::deserialize(&identity.x3dh_client).unwrap();
                let identity_key = x3dh.identity_key.public_key;
                let prekey = x3dh.prekey.public_key;
                tezos
                    .register(
                        address,
                        Some(identity_key.0.as_bytes()),
                        prekey.0.as_bytes(),
                    )
                    .map_err(Tezos)?;
                println!("registered {}", identity_id);

                Ok(())
            }) as Command,
        ),
        (
            "contact",
            Box::new(move |input: &str| {
                let (name, rest) = uncons(input).ok_or(NotFound)?;
                let (address, _rest) = uncons(rest).ok_or(NotFound)?;
                user_data
                    .create_contact(name, address.as_bytes())
                    .map_err(UserData)
            }),
        ),
    ])
}

fn exist(tezos: &TezosMock) -> Command {
    use DriverError::*;

    Box::new(move |input: &str| {
        let (address, _rest) = uncons(input).ok_or(NotFound)?;
        match tezos
            .retrieve_user_data(address.as_bytes())
            .map_err(Tezos)?
        {
            Some(_) => println!("{} exists", address),
            None => println!("{} doesn't exist", address),
        }

        Ok(())
    })
}

fn post<'a>(address: &'a [u8], tezos: &'a TezosMock, user_data: &'a MizuConnection) -> Command<'a> {
    use DriverError::*;

    Box::new(move |input: &str| {
        let mut rng = OsRng;

        let (our_identity_id, input) = uncons_parse(input, "failed to parse identity id")?;
        let (their_contact_id, input) = uncons_parse(input, "failed to parse contact id")?;
        let (message, _input) = uncons(input).ok_or(NotFound)?;

        eprintln!("{}\t{}\t{}", our_identity_id, their_contact_id, message);

        let our_identity = user_data.find_identity(our_identity_id).map_err(UserData)?;
        let our_x3dh: X3DHClient = bincode::deserialize(&our_identity.x3dh_client).unwrap();

        let their_contact = user_data.find_contact(their_contact_id).map_err(UserData)?;

        if let Some(their_data) = tezos
            .retrieve_user_data(&their_contact.address)
            .map_err(Tezos)?
        {
            let identity_key: [u8; 32] = their_data.identity_key.as_slice().try_into().unwrap();
            let identity_key = IdentityPublicKey(identity_key.into());
            let prekey: [u8; 32] = their_data.prekey.as_slice().try_into().unwrap();
            let prekey = PrekeyPublicKey(prekey.into());
            match user_data
                .find_client(our_identity_id, their_contact_id)
                .map_err(UserData)?
            {
                Some(client) => {
                    eprintln!("using existing Client");
                    let latest_message_timestamp = client.latest_message_timestamp;
                    let mut client: Client = bincode::deserialize(&client.client_data).unwrap();
                    let message = client
                        .create_message(&mut rng, &identity_key, &prekey, message.as_bytes())
                        .unwrap();
                    eprintln!("message = '{:?}'", message);
                    let payload = bincode::serialize(&message).unwrap();
                    tezos.post(address, &[&payload], &[]).map_err(Tezos)?;
                    user_data
                        .update_client(
                            our_identity_id,
                            their_contact_id,
                            &client,
                            latest_message_timestamp.as_ref(),
                        )
                        .map_err(UserData)?;
                }
                None => {
                    eprintln!("creating new Client");
                    let mut client =
                        Client::with_x3dh_client(our_x3dh, address, &their_contact.address);
                    let message = client
                        .create_message(&mut rng, &identity_key, &prekey, message.as_bytes())
                        .unwrap();
                    eprintln!("message = '{:?}'", message);
                    let payload = bincode::serialize(&message).unwrap();
                    tezos.post(address, &[&payload], &[]).map_err(Tezos)?;
                    user_data
                        .create_client(our_identity_id, their_contact_id, &client, None)
                        .map_err(UserData)?;
                }
            }

            Ok(())
        } else {
            Err(NotFound)
        }
    })
}

fn get<'a>(address: &'a [u8], tezos: &'a TezosMock, user_data: &'a MizuConnection) -> Command<'a> {
    use DriverError::*;

    Box::new(move |input: &str| {
        let mut rng = OsRng;

        let (our_identity_id, input) = uncons_parse(input, "failed to parse identity id")?;
        let (their_contact_id, _input) = uncons_parse(input, "failed to parse contact id")?;
        let our_identity = user_data.find_identity(our_identity_id).map_err(UserData)?;
        let our_x3dh: X3DHClient = bincode::deserialize(&our_identity.x3dh_client).unwrap();

        let their_contact = user_data.find_contact(their_contact_id).map_err(UserData)?;

        if let Some(their_data) = tezos
            .retrieve_user_data(&their_contact.address)
            .map_err(Tezos)?
        {
            match user_data
                .find_client(our_identity_id, their_contact_id)
                .map_err(UserData)?
            {
                Some(client) => {
                    eprintln!("using existing Client");
                    let mut latest_message_timestamp = client.latest_message_timestamp;
                    let mut client: Client = bincode::deserialize(&client.client_data).unwrap();
                    for message in their_data.postal_box.iter() {
                        // assuming message is ordered
                        let timestamp = message.timestamp;
                        match latest_message_timestamp {
                            // if the message is older than recorded timestamp, skip it.
                            Some(latest_message_timestamp)
                                if latest_message_timestamp >= timestamp =>
                            {
                                continue;
                            }
                            _ => {
                                latest_message_timestamp = Some(timestamp);
                            }
                        }

                        let message = bincode::deserialize(&message.content).unwrap();
                        if let Ok(message) = client.attempt_message_decryption(&mut rng, message) {
                            user_data
                                .create_message(our_identity_id, their_contact_id, &message)
                                .map_err(UserData)?;
                            println!("received {:?}", message);
                        }
                    }
                    user_data
                        .update_client(
                            our_identity_id,
                            their_contact_id,
                            &client,
                            latest_message_timestamp.as_ref(),
                        )
                        .map_err(UserData)?;
                }
                None => {
                    eprintln!("creating new Client");
                    let mut client =
                        Client::with_x3dh_client(our_x3dh, address, &their_contact.address);
                    let mut latest_message_timestamp = None;
                    for message in their_data.postal_box.iter() {
                        latest_message_timestamp = Some(message.timestamp);
                        let message = bincode::deserialize(&message.content).unwrap();
                        if let Ok(message) = client.attempt_message_decryption(&mut rng, message) {
                            user_data
                                .create_message(our_identity_id, their_contact_id, &message)
                                .map_err(UserData)?;
                            // assuming message is ordered
                            println!("received {:?}", message);
                        }
                    }
                    user_data
                        .create_client(
                            our_identity_id,
                            their_contact_id,
                            &client,
                            latest_message_timestamp.as_ref(),
                        )
                        .map_err(UserData)?;
                }
            }

            Ok(())
        } else {
            Err(NotFound)
        }
    })
}

fn commands<'a>(
    address: &'a [u8],
    tezos: &'a TezosMock,
    user_data: &'a MizuConnection,
) -> Command<'a> {
    subcommands(vec![
        ("list", list(user_data)),
        ("generate", generate(user_data)),
        ("register", register(address, tezos, user_data)),
        ("exist", exist(tezos)),
        ("post", post(address, tezos, user_data)),
        ("get", get(address, tezos, user_data)),
    ])
}

fn main() {
    let user_data = MizuConnection::connect(&std::env::var("MIZU_DB").unwrap()).unwrap();
    let tezos = TezosMock::connect(&std::env::var("MIZU_TEZOS_MOCK").unwrap()).unwrap();
    let address = std::env::var("TEZOS_ADDRESS").unwrap();
    let address = address.as_bytes();
    let commands = commands(address, &tezos, &user_data);

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
