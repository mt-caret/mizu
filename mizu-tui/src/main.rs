use chrono::naive::NaiveDate;
use cursive::align::{Align, HAlign};
use cursive::event::Key;
use cursive::menu::MenuTree;
use cursive::theme;
use cursive::theme::Effect;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::*;
use cursive::Cursive;
use cursive::View;
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

fn render_identity(identity: &mizu_sqlite::identity::Identity) -> impl View {
    // id. **name**
    //     tezos_address
    let mut styled = StyledString::plain(format!("{:>3}. ", identity.id));
    styled.append_styled(format!("{}\n", identity.name), Effect::Bold);
    styled.append(format!("     {}", identity.address));
    Panel::new(TextView::new(styled)).title("Your identity")
}

fn render_contacts<I: Iterator<Item = mizu_sqlite::client::ClientInfo>>(iter: I) -> impl View {
    // contact_id. **name**       timestamp
    //             tezos_address
    // TODO: show last message like Signal?
    let contacts = iter.fold(LinearLayout::vertical(), |view, client| {
        let mut styled = StyledString::plain(format!("{:>3}. ", client.contact_id));
        styled.append_styled(format!("{:<15}", client.name), Effect::Bold);
        match client.latest_message_timestamp {
            Some(ts) => styled.append(format!("{}\n", ts)),
            None => styled.append("\n"),
        }
        styled.append(format!("     {}", client.address));
        view.child(TextView::new(styled))
    });

    Panel::new(contacts).title("Contacts")
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
        let timestamp = message.created_at.to_string();
        let mut styled = StyledString::new();
        styled.append_styled(content, Effect::Bold);
        styled.append(timestamp);

        view.child(TextView::new(styled).h_align(if message.my_message {
            HAlign::Right
        } else {
            HAlign::Left
        }))
    })
}

/*
fn aligned_inputs<S: Into<String>>(labels: Vec<S>) -> impl View {
    const DEFAULT_LENGTH: usize = 15;

    let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
    let max_len = labels.iter().map(|s| s.len()).max().unwrap();

    labels
        .into_iter()
        .fold(LinearLayout::vertical(), |view, label| {
            view.child(
                LinearLayout::horizontal()
                    .child(TextView::new(format!("{0:<1$}", label, max_len)))
                    .child(EditView::new().min_width(DEFAULT_LENGTH)),
            )
        })
}
*/

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
            .child(EditView::new().with_name(IDENTITY_FILE_EDIT).min_width(15));

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
                            c.with_user_data(|drivers: &mut Drivers| {
                                drivers.insert(name.clone(), driver)
                            })
                            .unwrap();

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
    tree.add_leaf("register", register_callback(user_db, factory));

    if !identities.is_empty() {
        tree.add_delimiter();
    }
    for identity in identities.iter() {
        tree.add_leaf(&identity.name, |_c| {});
    }

    Ok(())
}

#[derive(StructOpt)]
struct Opt {
    db: String,
    #[structopt(long)]
    tezos_mock: String,
    #[structopt(long)]
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
    let args = Opt::from_args();
    let user_db = Rc::new(MizuConnection::connect(&args.db)?);
    let mock_db = Rc::new(SqliteConnection::establish(&args.tezos_mock)?);
    let mock_factory: TezosFactory = {
        let mock_db = Rc::clone(&mock_db);
        Rc::new(move |pkh, _secret_key| TezosMock::new(pkh, Rc::clone(&mock_db)).boxed())
    };
    let drivers: Drivers = HashMap::new();

    let opt = Opt::from_args();

    let identity = mizu_sqlite::identity::Identity {
        id: 1,
        name: "Alice".into(),
        address: "tz1alice".into(),
        x3dh_client: vec![],
        created_at: "".into(),
    };
    let client_info = vec![
        mizu_sqlite::client::ClientInfo {
            contact_id: 2,
            address: "tz1hogehoge".into(),
            name: "Bob".into(),
            latest_message_timestamp: Some(NaiveDate::from_ymd(1996, 5, 24).and_hms(1, 23, 0)),
        },
        mizu_sqlite::client::ClientInfo {
            contact_id: 13,
            address: "tz1fugafuga".into(),
            name: "Chris".into(),
            latest_message_timestamp: Some(NaiveDate::from_ymd(2038, 12, 11).and_hms(7, 23, 54)),
        },
    ];
    let left_view = LinearLayout::vertical()
        .child(render_identity(&identity))
        .child(render_contacts(client_info.into_iter()));

    let messages = vec![
        mizu_sqlite::message::Message {
            id: 1,
            identity_id: 1,
            contact_id: 1,
            content: b"Hi!"[..].into(),
            my_message: false,
            created_at: NaiveDate::from_ymd(1996, 5, 24).and_hms(1, 23, 0),
        },
        mizu_sqlite::message::Message {
            id: 1,
            identity_id: 1,
            contact_id: 1,
            content: b"Hellooooooooo"[..].into(),
            my_message: false,
            created_at: NaiveDate::from_ymd(1996, 5, 24).and_hms(1, 23, 13),
        },
        mizu_sqlite::message::Message {
            id: 1,
            identity_id: 1,
            contact_id: 1,
            content: b"Are you here?"[..].into(),
            my_message: false,
            created_at: NaiveDate::from_ymd(1996, 5, 24).and_hms(1, 23, 58),
        },
        mizu_sqlite::message::Message {
            id: 1,
            identity_id: 1,
            contact_id: 1,
            content: b"what?"[..].into(),
            my_message: true,
            created_at: NaiveDate::from_ymd(1996, 5, 24).and_hms(1, 25, 1),
        },
    ];
    let right_view = Panel::new(
        LinearLayout::vertical()
            .child(render_messages(messages.into_iter()))
            .child(TextArea::new().min_height(3)),
    )
    .title("Messages");

    let view = LinearLayout::horizontal()
        .child(left_view)
        .child(DummyView)
        .child(right_view.full_screen());

    let mut siv = cursive::default();
    siv.set_user_data(drivers);

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
    siv.add_fullscreen_layer(view);
    siv.add_global_callback(Key::Esc, |c| c.select_menubar());
    siv.run();

    Ok(())
}
