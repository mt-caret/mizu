use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::Effect;
use cursive::theme::Style;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};
use cursive::View;
use structopt::StructOpt;

fn render_identity(identity: &mizu_sqlite::identity::Identity) -> impl View {
    // id. **name**
    //     tezos_address
    let mut styled = StyledString::plain(format!("{:>3}. ", identity.id));
    styled.append_styled(format!("{}\n", identity.name), Effect::Bold);
    styled.append(format!("     tz1blabla")); // TODO: store Tezos address in DB
    TextView::new(styled)
}

fn render_clients<I: Iterator<Item = mizu_sqlite::client::Client>>(iter: I)
-> impl Iterator<Item = impl View> {
    // contact_id. **name**       timestamp
    //             tezos_address
    // TODO: show last message like Signal?
    iter.map(|client| {
        let mut styled = StyledString::plain(format!("{:>3}. ", client.contact_id));
        styled.append_styled()
    })
}

fn main() {
    let identity = mizu_sqlite::identity::Identity {
        id: 1,
        name: "Alice".into(),
        x3dh_client: vec![],
        created_at: "".into(),
    };
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());
    siv.add_fullscreen_layer(render_identity(&identity));
    siv.run();
}
