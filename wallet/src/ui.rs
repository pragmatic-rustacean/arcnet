use std::{
    clone, fmt,
    sync::{Arc, Mutex},
};

use crate::{
    core::Core,
    tasks::{Unit, convert_amount},
};
use anyhow::Result;
use cursive::{
    Cursive, CursiveExt,
    event::{Event, Key},
    view::Nameable,
    views::{Button, Dialog, EditView, LinearLayout, Panel, ResizedView, TextContent, TextView},
};
use tracing::*;

/// Initialize and run the user interface.
pub fn run_ui(core: Arc<Core>, balance_content: TextContent) -> Result<()> {
    info!("Initializing UI");
    let mut siv = Cursive::default();
    setup_siv(&mut siv, core.clone(), balance_content);
    info!("Starting UI event loop");
    siv.run();
    info!("UI event loop stopped");
    Ok(())
}

/// Set up the cursive interface with all necessary componenent.
pub fn setup_siv(siv: &mut Cursive, core: Arc<Core>, balance_content: TextContent) {
    siv.set_autorefresh(true);
    siv.set_fps(30);
    siv.set_window_title("Arc Coin wallet".to_string());
    siv.add_global_callback('q', |ev| {
        info!("Quit command received");
        ev.quit();
    });
    setup_menubar(siv, core.clone());
    setup_layout(siv, core.clone(), balance_content);
    siv.add_global_callback(Event::Key(Key::Esc), |ev| {
        ev.select_menubar();
    });
    siv.select_menubar();
}

/// Set up menu bar with send and quit options.
fn setup_menubar(siv: &mut Cursive, core: Arc<Core>) {
    siv.menubar()
        .add_leaf("Send", move |s| {
            show_send_transactions(s, core.clone());
        })
        .add_leaf("Quit", |q| q.quit());

    siv.set_autohide_menu(false);
}
/// Set up the main layout of the wallet
fn setup_layout(siv: &mut Cursive, core: Arc<Core>, balance_content: TextContent) {
    let instructions = TextView::new("Press escape to select the top menu");
    let balance_panel = Panel::new(TextView::new_with_content(balance_content)).title("Balance");
    let info_layout = create_info_layout(&core);
    let layout = LinearLayout::vertical()
        .child(instructions)
        .child(balance_panel)
        .child(info_layout);

    siv.add_layer(layout);
}
/// Create an information layout containing keys and contacts.
fn create_info_layout(core: &Arc<Core>) -> LinearLayout {
    let mut info_layout = LinearLayout::horizontal();
    let keys_content = core
        .config
        .keys
        .iter()
        .map(|key| format!("{}", key.private.display()))
        .collect::<Vec<String>>()
        .join("\n");
    info_layout.add_child(ResizedView::with_full_width(
        Panel::new(TextView::new(keys_content)).title("Your keys"),
    ));

    let contact_content = core
        .config
        .contacts
        .iter()
        .map(|cont| cont.name.clone())
        .collect::<Vec<String>>()
        .join("\n");
    info_layout.add_child(ResizedView::with_full_width(
        Panel::new(TextView::new(contact_content)).title("Contacts"),
    ));

    info_layout
}
/// Display the send transaction
fn show_send_transactions(siv: &mut Cursive, core: Arc<Core>) {
    info!("Showing send transaction dialog");
    let unit = Arc::new(Mutex::new(Unit::Arcs));
    siv.add_layer(
        Dialog::around(create_transaction_layout(unit.clone()))
            .title("Send Transaction")
            .button("Send", move |siv| {
                send_transaction(siv, core.clone(), *unit.lock().unwrap());
            })
            .button("Cancel", |siv| {
                info!("Transaction cancelled");
                siv.pop_layer();
            }),
    );
}
/// Create layout for selecting transaction unit (Sats, Arcs)
fn create_transaction_layout(unit: Arc<Mutex<Unit>>) -> LinearLayout {
    LinearLayout::vertical()
        .child(TextView::new("Recipient"))
        .child(EditView::new().with_name("recipient"))
        .child(TextView::new("amount"))
        .child(EditView::new().with_name("amount"))
        .child(create_init_layout(unit))
}

fn create_init_layout(unit: Arc<Mutex<Unit>>) -> LinearLayout {
    LinearLayout::horizontal()
        .child(TextView::new("Unit"))
        .child(TextView::new_with_content(TextContent::new("Arcs")).with_name("unit_ dispay"))
        .child(Button::new("Switch", move |sw| {
            switch_units(sw, unit.clone());
        }))
}
/// switch units
fn switch_units(siv: &mut Cursive, unit: Arc<Mutex<Unit>>) {
    let mut unit = unit.lock().unwrap();
    *unit = match *unit {
        Unit::Sats => Unit::Arcs,
        Unit::Arcs => Unit::Sats,
    };
    siv.call_on_name("unit_display", |view: &mut TextView| {
        view.set_content(match *unit {
            Unit::Arcs => "Arcs",
            Unit::Sats => "Sats",
        });
    });
}
/// process the sent transaction
async fn send_transaction(siv: &mut Cursive, core: Arc<Core>, unit: Unit) {
    debug!("Send transaction button pressed");
    let recipient = siv
        .call_on_name("recipient", |view: &mut EditView| view.get_content())
        .unwrap();
    let amount = siv
        .call_on_name("amount", |view: &mut EditView| view.get_content())
        .unwrap()
        .parse()
        .unwrap_or(0.0);
    let amount_sats = convert_amount(amount, unit, Unit::Sats) as u64;
    info!(
        "Trying to send transaction to {} for {:?} satoshis",
        recipient, amount
    );
    match core
        .send_transaction_async(recipient.as_str(), amount_sats)
        .await
    {
        Ok(_) => show_success_dialog(siv),
        Err(error) => show_error_dialog(siv, error),
    };
}
/// Display a success dialog after a successful transaction
fn show_success_dialog(siv: &mut Cursive) {
    info!("Transaction sent successfully");
    siv.add_layer(
        Dialog::text("Transaction sent succesfully")
            .title("Success")
            .button("OK", |view| {
                debug!("Closing success dialog");
                view.pop_layer();
                view.pop_layer();
            }),
    );
}
/// Display an error when a transaction fails.
fn show_error_dialog(siv: &mut Cursive, error: impl fmt::Display) {
    error!("Failed to send transaction: {}", error);
    siv.add_layer(
        Dialog::text(format!("Failed to send transaction: {}", error))
            .title("Error")
            .button("OK", |view| {
                debug!("Closing error dialog");
                view.pop_layer();
            }),
    );
}
