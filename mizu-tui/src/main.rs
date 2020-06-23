use cursive::align::HAlign;
use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::Effect;
use cursive::theme::Style;
use cursive::traits::Resizable;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, DummyView, LinearLayout, TextArea, TextView};
use cursive::View;
use structopt::StructOpt;

fn render_identity(identity: &mizu_sqlite::identity::Identity) -> impl View {
    // id. **name**
    //     tezos_address
    let mut styled = StyledString::plain(format!("{:>3}. ", identity.id));
    styled.append_styled(format!("{}\n", identity.name), Effect::Bold);
    styled.append(format!("     {}", identity.address));
    TextView::new(styled)
}

fn render_clients<I: Iterator<Item = mizu_sqlite::client::ClientInfo>>(
    iter: I,
) -> impl Iterator<Item = impl View> {
    // contact_id. **name**       timestamp
    //             tezos_address
    // TODO: show last message like Signal?
    iter.map(|client| {
        let mut styled = StyledString::plain(format!("{:>3}. ", client.contact_id));
        styled.append_styled(format!("{:<15}", client.name), Effect::Bold);
        match client.latest_message_timestamp {
            Some(ts) => styled.append(format!("{}\n", ts)),
            None => styled.append("\n"),
        }
        styled.append(format!("     {}", client.address));
        TextView::new(styled)
    })
}

fn render_messages<I: Iterator<Item = mizu_sqlite::message::Message>>(
    iter: I,
) -> impl Iterator<Item = impl View> {
    // messages from me:
    // <right align> content
    //             timestamp

    // messages from the other guy in the conversation:
    // content   <left align>
    // timestamp

    iter.map(|message| {
        let content = format!("{}\n", String::from_utf8_lossy(&message.content));
        let timestamp = message.created_at.to_string();
        let mut styled = StyledString::new();
        styled.append_styled(content, Effect::Bold);
        styled.append(timestamp);

        TextView::new(styled).h_align(if message.my_message {
            HAlign::Right
        } else {
            HAlign::Left
        })
    })
}

fn main() {
    use chrono::naive::NaiveDate;

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
    let mut left_view = LinearLayout::vertical();
    left_view.add_child(render_identity(&identity));
    left_view.add_child(DummyView);
    for view in render_clients(client_info.into_iter()) {
        left_view.add_child(view);
    }

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
    let mut right_view = LinearLayout::vertical();
    for view in render_messages(messages.into_iter()) {
        right_view.add_child(view);
        right_view.add_child(DummyView);
    }
    right_view.add_child(TextArea::new());

    let view = LinearLayout::horizontal()
        .child(left_view)
        .child(DummyView)
        .child(right_view.full_screen());

    let mut siv = cursive::default();
    siv.add_fullscreen_layer(view);
    siv.run();
}
