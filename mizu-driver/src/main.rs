use mizu_crypto::{
    keys::{IdentityPublicKey, PrekeyPublicKey},
    x3dh::X3DHClient,
    Client,
};
use mizu_sqlite::MizuConnection;
use tezos_interface::Tezos;
use tezos_mock::TezosMock;

use std::convert::TryInto;

fn main() {
    let mut rng = rand::rngs::OsRng;
    let user_data = MizuConnection::connect(&std::env::var("MIZU_DB").unwrap()).unwrap();
    let tezos = TezosMock::new(&std::env::var("MIZU_TEZOS_MOCK").unwrap());
    let address = std::env::var("TEZOS_ADDRESS").unwrap();
    let address = address.as_bytes();

    let mut rl = rustyline::Editor::<()>::new();
    while let Ok(line) = rl.readline("> ") {
        rl.add_history_entry(line.as_str());
        let line = line.trim();
        let mut words = line.split(' ');
        match words.next() {
            Some(s) if s.to_lowercase() == "list" => match words.next() {
                Some(s) if s.to_lowercase() == "identity" => {
                    for id in user_data.list_identities().into_iter() {
                        println!("{}\t{}\t{}", id.id, id.name, id.created_at);
                    }
                }
                Some(s) if s.to_lowercase() == "contact" => {
                    for contact in user_data.list_contacts().into_iter() {
                        println!("{}\t{}\t{}", contact.id, contact.name, contact.created_at);
                    }
                }
                Some(s) if s.to_lowercase() == "message" => {
                    if let Some(Ok(our_identity_id)) = words.next().map(str::parse) {
                        if let Some(Ok(their_contact_id)) = words.next().map(str::parse) {
                            for message in
                                user_data.find_messages(our_identity_id, their_contact_id)
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
                        }
                    }
                }
                _ => {}
            },
            Some(s) if s.to_lowercase() == "generate" => match words.next() {
                Some(s) if s.to_lowercase() == "identity" => {
                    let x3dh = X3DHClient::new(&mut rng);
                    if let Some(name) = words.next() {
                        user_data.create_identity(name, &x3dh);
                        println!("generated X3DHClient as {}", name);
                    }
                }
                _ => {}
            },
            Some(s) if s.to_lowercase() == "register" => match words.next() {
                Some(s) if s.to_lowercase() == "identity" => {
                    if let Some(Ok(identity_id)) = words.next().map(str::parse) {
                        let identity = user_data.find_identity(identity_id);
                        let x3dh: X3DHClient = bincode::deserialize(&identity.x3dh_client).unwrap();
                        let identity_key = x3dh.identity_key.public_key;
                        let prekey = x3dh.prekey.public_key;
                        tezos.register(
                            &address,
                            Some(identity_key.0.as_bytes()),
                            prekey.0.as_bytes(),
                        );
                        println!("registered {}", identity_id);
                    }
                }
                Some(s) if s.to_lowercase() == "contact" => {
                    if let Some(name) = words.next() {
                        if let Some(address) = words.next() {
                            user_data.create_contact(name, address.as_bytes());
                        }
                    }
                }
                _ => {}
            },
            Some(s) if s.to_lowercase() == "exist" => {
                if let Some(s) = words.next() {
                    if let Some(_data) = tezos.retrieve_user_data(s.as_bytes()) {
                        println!("{} exists", s);
                    } else {
                        println!("{} doesn't exist", s);
                    }
                }
            }
            Some(s) if s.to_lowercase() == "post" => {
                // POST identity_id contact_id message
                if let Some(Ok(our_identity_id)) = words.next().map(str::parse) {
                    if let Some(Ok(their_contact_id)) = words.next().map(str::parse) {
                        if let Some(message) = words.next() {
                            let our_identity = user_data.find_identity(our_identity_id);
                            let our_x3dh: X3DHClient =
                                bincode::deserialize(&our_identity.x3dh_client).unwrap();

                            let their_contact = user_data.find_contact(their_contact_id);

                            if let Some(their_data) =
                                tezos.retrieve_user_data(&their_contact.address)
                            {
                                let identity_key: [u8; 32] =
                                    their_data.identity_key.as_slice().try_into().unwrap();
                                let identity_key = IdentityPublicKey(identity_key.into());
                                let prekey: [u8; 32] =
                                    their_data.prekey.as_slice().try_into().unwrap();
                                let prekey = PrekeyPublicKey(prekey.into());
                                match user_data.find_client(our_identity_id, their_contact_id) {
                                    Some(client) => {
                                        eprintln!("using existing Client");
                                        let mut client: Client =
                                            bincode::deserialize(&client.client_data).unwrap();
                                        let message = client
                                            .create_message(
                                                &mut rng,
                                                &identity_key,
                                                &prekey,
                                                message.as_bytes(),
                                            )
                                            .unwrap();
                                        let payload = bincode::serialize(&message).unwrap();
                                        tezos.post(address, &[&payload], &[]);
                                        user_data.update_client(
                                            our_identity_id,
                                            their_contact_id,
                                            &client,
                                        );
                                    }
                                    None => {
                                        eprintln!("creating new Client");
                                        let mut client = Client::with_x3dh_client(
                                            our_x3dh,
                                            address,
                                            &their_contact.address,
                                        );
                                        let message = client
                                            .create_message(
                                                &mut rng,
                                                &identity_key,
                                                &prekey,
                                                message.as_bytes(),
                                            )
                                            .unwrap();
                                        let payload = bincode::serialize(&message).unwrap();
                                        tezos.post(address, &[&payload], &[]);
                                        user_data.create_client(
                                            our_identity_id,
                                            their_contact_id,
                                            &client,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(s) if s.to_lowercase() == "get" => {
                // GET identity_id contact_id
                if let Some(Ok(our_identity_id)) = words.next().map(str::parse) {
                    if let Some(Ok(their_contact_id)) = words.next().map(str::parse) {
                        let our_identity = user_data.find_identity(our_identity_id);
                        let our_x3dh: X3DHClient =
                            bincode::deserialize(&our_identity.x3dh_client).unwrap();

                        let their_contact = user_data.find_contact(their_contact_id);

                        if let Some(their_data) = tezos.retrieve_user_data(&their_contact.address) {
                            match user_data.find_client(our_identity_id, their_contact_id) {
                                Some(client) => {
                                    eprintln!("using existing Client");
                                    let mut client: Client =
                                        bincode::deserialize(&client.client_data).unwrap();
                                    for message in their_data.postal_box.into_iter() {
                                        let message =
                                            bincode::deserialize(&message.content).unwrap();
                                        if let Ok(message) =
                                            client.attempt_message_decryption(&mut rng, message)
                                        {
                                            user_data.create_message(
                                                our_identity_id,
                                                their_contact_id,
                                                &message,
                                            );
                                        }
                                    }
                                    user_data.update_client(
                                        our_identity_id,
                                        their_contact_id,
                                        &client,
                                    );
                                }
                                None => {
                                    eprintln!("creating new Client");
                                    let mut client = Client::with_x3dh_client(
                                        our_x3dh,
                                        address,
                                        &their_contact.address,
                                    );
                                    for message in their_data.postal_box.into_iter() {
                                        let message =
                                            bincode::deserialize(&message.content).unwrap();
                                        if let Ok(message) =
                                            client.attempt_message_decryption(&mut rng, message)
                                        {
                                            user_data.create_message(
                                                our_identity_id,
                                                their_contact_id,
                                                &message,
                                            );
                                        }
                                    }
                                    user_data.create_client(
                                        our_identity_id,
                                        their_contact_id,
                                        &client,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
