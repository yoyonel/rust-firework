#![cfg(feature = "interactive_tests")]

use fireworks_sim::simulator::gui_settings::{GuiSettings, GuiTheme};
use std::cell::RefCell;
use std::rc::Rc;

mod helpers;
use helpers::{TestAudio, TestPhysic};

#[test]
fn test_gui_settings_session_sync() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log);

    let mut settings = GuiSettings::new();
    settings.active_tab = 0;
    assert_eq!(settings.active_tab, 0);

    let mut diag = false;
    let mut overlay = true;
    settings.apply_session_to_audio(&mut audio, &mut diag, &mut overlay);
    settings.apply_session_to_physic(&mut physic);

    settings.set_status("Test status message");
    assert!(settings.status_message.is_some());
    let (msg, _) = settings.status_message.as_ref().unwrap();
    assert_eq!(msg, "Test status message");

    settings.save_session_state(&audio, &physic, diag, overlay, false);
}

#[test]
fn test_gui_settings_tab_navigation() {
    let mut settings = GuiSettings::new();
    settings.open = true;

    settings.active_tab = 1;
    assert_eq!(settings.active_tab, 1);

    settings.active_tab = 2;
    assert_eq!(settings.active_tab, 2);

    settings.active_tab = 3;
    assert_eq!(settings.active_tab, 3);
}

#[test]
fn test_gui_settings_theme_changes() {
    let mut settings = GuiSettings::new();
    settings.theme = GuiTheme::CyberpunkCyan;
    assert_eq!(settings.theme, GuiTheme::CyberpunkCyan);

    settings.theme = GuiTheme::ClassicDark;
    assert_eq!(settings.theme, GuiTheme::ClassicDark);
}
