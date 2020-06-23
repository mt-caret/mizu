use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::Effect;
use cursive::theme::Style;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, DummyView, LinearLayout, TextView};
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

    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());
    siv.add_fullscreen_layer(left_view);
    siv.run();
}
