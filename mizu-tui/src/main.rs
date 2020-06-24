use cursive::align::{Align, HAlign};
use cursive::event::{Event, Key};
use cursive::menu::MenuTree;
use cursive::theme;
use cursive::theme::Effect;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::view::SizeConstraint;
use cursive::views::*;
use cursive::Cursive;
use diesel::prelude::*;
use mizu_driver::Driver;
use mizu_sqlite::MizuConnection;
use mizu_tezos_interface::{BoxedTezos, Tezos};
use mizu_tezos_mock::TezosMock;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use structopt::StructOpt;

type DynamicDriver = Driver<BoxedTezos<'static>>;
type DynamicError = Box<dyn Error + Send + Sync + 'static>;
type Drivers = HashMap<String, DynamicDriver>;
// address * secret_key -> Tezos
type TezosFactory = Rc<dyn Fn(&str, &str) -> BoxedTezos<'static>>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const IDENTITY_MENU_INDEX: usize = 1;
const LEFT_WIDTH: usize = 45;
const IDENTITY_HEIGHT: usize = 4;

struct CursiveData {
    current_identity_id: Option<i32>,
    current_contact_id: Option<i32>,
    drivers: Drivers,
    user_db: Rc<MizuConnection>,
    factory: TezosFactory,
}

impl CursiveData {
    /// returns a driver for the current identity
    fn current_driver(&mut self) -> Option<&DynamicDriver> {
        match self.current_identity_id {
            Some(identity_id) => {
                let identity = self.user_db.find_identity(identity_id).ok()?;
                let user_db = Rc::clone(&self.user_db);
                let factory = Rc::clone(&self.factory);
                Some(
                    self.drivers
                        .entry(identity.name.to_string())
                        .or_insert_with(|| {
                            let tezos = (factory)(&identity.address, &identity.secret_key);
                            Driver::new(user_db, tezos)
                        }),
                )
            }
            None => None,
        }
    }
}

fn render_identity(identity: &Option<mizu_sqlite::identity::Identity>) -> impl View {
    // id. **name**
    //     tezos_address
    match identity {
        Some(identity) => {
            let mut styled = StyledString::plain(format!("{:>3}. ", identity.id));
            styled.append_styled(format!("{}\n", identity.name), Effect::Bold);
            styled.append(format!("     {}", identity.address));

            Panel::new(TextView::new(styled))
                .title("Your identity")
                .fixed_size((LEFT_WIDTH, IDENTITY_HEIGHT))
        }
        None => {
            let mut styled = StyledString::plain("Click ");
            styled.append_styled("Identity", Effect::Bold);
            styled.append(" menu");

            Panel::new(TextView::new(styled).align(Align::center()))
                .title("Your identity")
                .fixed_size((LEFT_WIDTH, IDENTITY_HEIGHT))
        }
    }
}

fn render_contact(client: &mizu_sqlite::contact::Contact) -> (StyledString, i32) {
    // contact_id. **name**       timestamp
    //             tezos_address
    // TODO: show last message like Signal?
    let mut styled = StyledString::plain(format!("{:>3}. ", client.id));
    styled.append_styled(format!("{:<15}", client.name), Effect::Bold);
    /*match client.latest_message_timestamp {
        Some(ts) => styled.append(format!("{}\n", ts)),
        None => styled.append("\n"),
    }*/
    styled.append(format!("     {}", client.address));
    (styled, client.id)
}

fn render_contacts(contacts: Vec<mizu_sqlite::contact::Contact>) -> impl View {
    // -----Contacts-----
    // | contacts here  |
    // ------------------
    // |   Add contact  |
    fn update_messages(c: &mut Cursive, contact_id: i32) {
        c.with_user_data(|data: &mut CursiveData| {
            data.current_contact_id = Some(contact_id);
        })
        .unwrap();
        render_world(c);
    }

    fn on_select(c: &mut Cursive, contact_id: &i32) {
        eprintln!("selected contact: {}", contact_id);
        update_messages(c, *contact_id);
    }

    fn on_submit(c: &mut Cursive, contact_id: &i32) {
        eprintln!("submitted contact: {}", contact_id);
        update_messages(c, *contact_id);
    }

    let contacts = Panel::new(
        SelectView::new()
            .with_all(contacts.iter().map(render_contact))
            .on_select(on_select)
            .on_submit(on_submit)
            .with_name("SELECTON"),
    )
    .title("Contacts")
    .min_height(5);
    let add_contact = Panel::new(Button::new("Add contact", |c| {
        if c.with_user_data(|data: &mut CursiveData| data.current_identity_id.is_none())
            .unwrap()
        {
            c.add_layer(
                Dialog::around(TextView::new("Please select an identity")).dismiss_button("Ok"),
            );
            return;
        }

        const CONTACT_NAME_EDIT: &str = "CONTACT_NAME_EDIT";
        const CONTACT_ADDRESS_EDIT: &str = "CONTACT_ADDRESS_EDIT";

        let content = LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("   Name: "))
                    .child(EditView::new().with_name(CONTACT_NAME_EDIT).min_width(40)),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("Address: "))
                    .child(
                        EditView::new()
                            .with_name(CONTACT_ADDRESS_EDIT)
                            .min_width(40),
                    ),
            );
        c.add_layer(
            Dialog::around(content)
                .title("Enter contact name and address")
                .dismiss_button("Cancel")
                .button("Ok", |c| {
                    let name: ViewRef<EditView> = c.find_name(CONTACT_NAME_EDIT).unwrap();
                    let address: ViewRef<EditView> = c.find_name(CONTACT_ADDRESS_EDIT).unwrap();
                    c.pop_layer();

                    match c
                        .with_user_data(|data: &mut CursiveData| {
                            let driver = data.current_driver().unwrap();
                            driver.add_contact(&name.get_content(), &address.get_content())?;
                            driver
                                .find_contact_by_address(&address.get_content())
                                .map(|contact| {
                                    data.current_contact_id = Some(contact.id);
                                })
                        })
                        .unwrap()
                    {
                        Ok(()) => render_world(c),
                        Err(e) => eprintln!("failed to add contact: {:?}", e),
                    };
                })
                .h_align(HAlign::Center),
        )
    }))
    .fixed_height(3);
    LinearLayout::vertical()
        .child(contacts)
        .child(add_contact)
        .fixed_width(LEFT_WIDTH)
}

fn render_messages<I: Iterator<Item = mizu_sqlite::message::Message>>(iter: I) -> impl View {
    // messages from me:
    // <right align> content
    //             timestamp

    // messages from the other guy in the conversation:
    // content   <left align>
    // timestamp

    iter.fold(LinearLayout::vertical(), |view, message| {
        let content = format!("{}\n", String::from_utf8_lossy(&message.content));
        let timestamp = message.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut styled = StyledString::new();
        styled.append_styled(content, Effect::Bold);
        styled.append(timestamp);

        view.child(TextView::new(styled).h_align(if message.my_message {
            HAlign::Right
        } else {
            HAlign::Left
        }))
    })
    .min_height(5)
    .full_width()
    .scrollable()
}

fn send_message(s: &mut Cursive) {
    let content = s
        .call_on_name("textarea", |t: &mut TextArea| t.get_content().to_string())
        .expect("textarea should always exists");
    if content.trim().is_empty() {
        return;
    }

    if let Some(dialog) = s
        .with_user_data(|data: &mut CursiveData| {
            match (data.current_identity_id, data.current_contact_id) {
                (None, _) => Some(Dialog::info("Please select an identity").title("Error")),
                (_, None) => Some(Dialog::info("Please select a contact").title("Error")),
                (Some(our_identity_id), Some(their_contact_id)) => match data
                    .current_driver()
                    .unwrap()
                    .post_message(&mut OsRng, our_identity_id, their_contact_id, &content)
                {
                    Ok(()) => None,
                    Err(e) => Some(
                        Dialog::info(format!("failed to send message: {:?}", e)).title("Error"),
                    ),
                },
            }
        })
        .unwrap()
    {
        // Rerender the world BEFORE showing a dialog
        render_world(s);
        s.add_layer(dialog);
    } else {
        render_world(s);
    };
}

fn render_input_view() -> impl View {
    // We would like to use Shift+Enter or Ctrl+Enter like other messengers,
    // but terminals don't support this:
    // see https://github.com/gyscos/Cursive/issues/151#issuecomment-366578010.
    let textarea = OnEventView::new(TextArea::new().with_name("textarea"))
        .on_pre_event(Event::CtrlChar('s'), send_message);

    Dialog::around(
        LinearLayout::horizontal()
            .child(ResizedView::new(
                SizeConstraint::Full,
                SizeConstraint::AtLeast(3),
                textarea,
            ))
            .child(Button::new("send", send_message)),
    )
}

fn error_dialog<E: std::fmt::Debug>(error: E) -> impl View {
    Dialog::around(TextView::new(format!("{:?}", error)))
        .title("Error")
        .dismiss_button("Ok")
}

fn register_callback(
    user_db: Rc<MizuConnection>,
    factory: TezosFactory,
) -> impl Fn(&mut Cursive) + 'static {
    move |c| {
        const IDENTITY_FILE_EDIT: &str = "IDENTITY_FILE_EDIT";

        let content = LinearLayout::horizontal()
            .child(TextView::new("identity file: "))
            .child(EditView::new().with_name(IDENTITY_FILE_EDIT).min_width(50));

        c.add_layer(
            Dialog::around(content)
                .title("Register your identity with Mizu")
                .dismiss_button("Cancel")
                .button("Ok", {
                    let user_db = Rc::clone(&user_db);
                    let factory = Rc::clone(&factory);
                    move |c| {
                        let edit: ViewRef<EditView> = c.find_name(IDENTITY_FILE_EDIT).unwrap();
                        c.pop_layer();

                        match read_identity_file(edit.get_content().as_str()).and_then(|file| {
                            let name = file.name;
                            let tezos = factory(&file.pkh, &file.secret_key);
                            let driver = Driver::new(Rc::clone(&user_db), tezos);
                            driver.generate_identity(&mut OsRng, &name)?;
                            let identity = user_db.find_identity_by_name(&name)?;
                            driver.publish_identity(identity.id)?;
                            c.with_user_data(|data: &mut CursiveData| {
                                data.drivers.insert(name.clone(), driver);
                                data.current_identity_id = Some(identity.id);
                            })
                            .unwrap();

                            render_world(c);
                            // rerender the Identity menu
                            render_identity_menu(
                                // 1st subtree corresponds to "Identity" menu
                                c.menubar().get_subtree(IDENTITY_MENU_INDEX).unwrap(),
                                Rc::clone(&user_db),
                                Rc::clone(&factory),
                            )?;

                            Ok(name)
                        }) {
                            Ok(name) => c.add_layer(
                                Dialog::around({
                                    let mut styled = StyledString::plain("Registered yourself as ");
                                    styled.append_styled(name, Effect::Bold);
                                    TextView::new(styled)
                                })
                                .title("Registration succeeded")
                                .dismiss_button("Ok"),
                            ),
                            Err(e) => c.add_layer(error_dialog(e)),
                        }
                    }
                })
                .h_align(HAlign::Center),
        );
    }
}

fn render_identity_menu(
    tree: &mut MenuTree,
    user_db: Rc<MizuConnection>,
    factory: TezosFactory,
) -> Result<(), DynamicError> {
    // identity
    // --------
    // identity_1
    // identity_2

    let identities = user_db.list_identities()?;
    tree.clear();
    tree.add_leaf(
        "register",
        register_callback(Rc::clone(&user_db), Rc::clone(&factory)),
    );

    if !identities.is_empty() {
        tree.add_delimiter();
    }
    for identity in identities.iter() {
        let id = identity.id;
        tree.add_leaf(&identity.name, move |c| {
            c.with_user_data(move |data: &mut CursiveData| {
                data.current_identity_id = Some(id);
            });
            render_world(c);
        });
    }

    Ok(())
}

fn render_world(siv: &mut Cursive) {
    let world = siv
        .with_user_data(|data: &mut CursiveData| {
            let identity =
                match data.current_identity_id.map(|id| data.user_db.find_identity(id)) {
                    Some(Ok(identity)) => Some(identity),
                    Some(Err(e)) => {
                        eprintln!("current identity not found: {:?}", e);
                        data.current_identity_id = None;
                        None
                    }
                    None => None,
                };
            // TODO: contacts are shared among identities
            // list_talking_clients searches for `Client`s, so contacts are not listed if no conversation happened
            let contacts = data.user_db.list_contacts().unwrap_or_else(|e| {
                eprintln!("failed to retrieve contacts from local DB: {:?}", e);
                vec![]
            });
            let messages = match (data.current_identity_id, data.current_contact_id) {
                (Some(current_identity_id), Some(current_contact_id)) => {
                    // update messages
                    data.current_driver().unwrap().get_messages(&mut OsRng, current_identity_id, current_contact_id)
                        .unwrap_or_else(|e| {
                            eprintln!("failed to retrieve messages from Tezos: identity = {}, contact = {}, {:?}", current_identity_id, current_contact_id, e);
                            vec![]
                        });
                    data.user_db.find_messages(current_identity_id, current_contact_id)
                        .unwrap_or_else(|e| {
                            eprintln!("failed to retrieve messages from local DB: identity = {}, contact = {}, {:?}", current_identity_id, current_contact_id, e);
                            vec![]
                        })
                }
                _ => vec![],
            };

            let identity = render_identity(&identity);
            let contacts = render_contacts(contacts);
            let left = LinearLayout::vertical().child(identity).child(contacts);

            let refresh = Panel::new(Button::new("Refresh", render_world))
                .fixed_height(3);

            let messages = render_messages(messages.into_iter());
            let input_view = render_input_view();
            let messages_title = match data.current_contact_id.map(|id| data.user_db.find_contact(id)) {
                Some(Ok(contact)) => format!("Conversation with {}", contact.name),
                Some(Err(e)) => {
                    eprintln!("current contact not found: {:?}", e);
                    data.current_contact_id = None;
                    "Conversation".into()
                },
                None => "Conversation".into(),
            };
            let messages = Panel::new(
                LinearLayout::vertical()
                    .child(messages)
                    .child(input_view),
            )
            .title(messages_title);

            let right = LinearLayout::vertical()
                .child(refresh)
                .child(messages);

            LinearLayout::horizontal()
                .child(left)
                .child(DummyView)
                .child(right.full_screen())
        })
        .unwrap();

    let layers = siv.screen_mut();
    match layers.len() {
        0 => layers.add_fullscreen_layer(world),
        1 => {
            layers.pop_layer();
            layers.add_fullscreen_layer(world);
        }
        _ => eprintln!("too many layers"),
    }
}

#[derive(StructOpt)]
struct Opt {
    db: String,
    #[structopt(long)]
    tezos_mock: Option<String>,
    #[structopt(long)]
    /// Path to theme TOML file (see
    /// https://docs.rs/cursive/0.15.0/cursive/theme/index.html#themes)
    theme: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
struct IdentityFile {
    name: String,
    pkh: String,
    secret_key: String,
}

fn read_identity_file<P: AsRef<Path>>(
    path: P,
) -> Result<IdentityFile, Box<dyn Error + Send + Sync + 'static>> {
    let content = std::fs::read_to_string(path)?;
    let identity_file = serde_json::from_str(&content)?;
    Ok(identity_file)
}

fn default_theme() -> theme::Theme {
    use theme::*;

    let mut palette = Palette::default();
    let default_colors = vec![
        (PaletteColor::Background, Color::TerminalDefault),
        (PaletteColor::Shadow, Color::TerminalDefault),
        (PaletteColor::View, Color::TerminalDefault),
        (PaletteColor::Primary, Color::TerminalDefault),
        (PaletteColor::Secondary, Color::TerminalDefault),
        (PaletteColor::Tertiary, Color::TerminalDefault),
        (PaletteColor::TitlePrimary, Color::TerminalDefault),
        (PaletteColor::TitleSecondary, Color::TerminalDefault),
        (PaletteColor::Highlight, Color::TerminalDefault),
        (PaletteColor::HighlightInactive, Color::TerminalDefault),
        (PaletteColor::HighlightText, Color::TerminalDefault),
    ];
    palette.extend(default_colors);

    Theme {
        shadow: false,
        borders: BorderStyle::Simple,
        palette,
    }
}

fn main() -> Result<(), DynamicError> {
    let opt = Opt::from_args();
    let user_db = Rc::new(MizuConnection::connect(&opt.db)?);
    let mock_factory: TezosFactory = {
        let database_url = opt.tezos_mock.as_deref().unwrap_or(":memory:").to_string();
        let run_migration = database_url == ":memory:" || std::fs::metadata(&database_url).is_err();
        let mock_db = Rc::new(SqliteConnection::establish(&database_url)?);
        // Ideally, we want to perform this check in TezosMock like
        // MizuConnection::connect does, but since we use the connection
        // across multiple instances, we need to do this here.
        if run_migration {
            mizu_tezos_mock::run_migrations(&mock_db);
        }

        Rc::new(move |pkh, secret_key| {
            TezosMock::new(pkh.into(), secret_key.into(), Rc::clone(&mock_db)).boxed()
        })
    };
    let theme = opt
        .theme
        .and_then(|theme_path| match theme::load_theme_file(theme_path) {
            Ok(theme) => Some(theme),
            Err(theme::Error::Io(err)) => {
                eprintln!("error loading theme: {}", err);
                None
            }
            Err(theme::Error::Parse(err)) => {
                eprintln!("error parsing theme: {}", err);
                None
            }
        })
        .unwrap_or_else(default_theme);

    // TODO: persist current_ids in User DB
    let current_identity_id = user_db
        .list_identities()?
        .first()
        .map(|identity| identity.id);
    let current_contact_id = user_db.list_contacts()?.first().map(|contact| contact.id);

    let mut siv = cursive::default();
    siv.set_user_data(CursiveData {
        current_identity_id,
        current_contact_id,
        drivers: HashMap::new(),
        user_db: Rc::clone(&user_db),
        factory: Rc::clone(&mock_factory),
    });
    siv.set_theme(theme);

    siv.menubar()
        .add_subtree(
            "Application",
            MenuTree::new()
                .leaf("About Mizu", |c| {
                    let mut styled =
                        StyledString::plain(format!("💧 Mizu Messenger v{}\n", VERSION));
                    styled.append_styled("https://github.com/mt-caret/mizu", Effect::Underline);
                    let content = TextView::new(styled).align(Align::center());
                    let dialog = Dialog::around(content)
                        .dismiss_button("Ok")
                        .h_align(HAlign::Center);
                    c.add_layer(dialog);
                })
                .leaf("Exit", |c| c.quit()),
        )
        .add_subtree("Identity", MenuTree::new());

    render_identity_menu(
        // 1st subtree corresponds to "Identity" menu
        siv.menubar().get_subtree(IDENTITY_MENU_INDEX).unwrap(),
        Rc::clone(&user_db),
        Rc::clone(&mock_factory),
    )?;

    siv.set_autohide_menu(false);
    //siv.add_fullscreen_layer(view);
    render_world(&mut siv);
    siv.add_global_callback(Key::Esc, |c| c.select_menubar());
    siv.run();

    Ok(())
}
