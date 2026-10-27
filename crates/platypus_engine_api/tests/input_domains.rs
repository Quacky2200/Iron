// SPDX-License-Identifier: Apache-2.0

//! `input` per event domain (windows design 3a). IDs are in TESTS.md (M9-API-14xx).
use platypus_engine_api::input::{self, input_api};
use platypus_engine_api::window::{self, DomainId};

// M9-API-1497
#[test]
fn keys_of_an_isolated_domain_do_not_reach_the_main_game() {
    window::reset();
    input::reset();
    let inspector = DomainId(5);
    input::key_down_in(inspector, "KeyA", false);
    input::mouse_move_in(inspector, 12.0, 34.0);
    // The main game (no domain entered) sees nothing.
    assert!(!input_api::get_key("KeyA"));
    assert_eq!(input_api::mouse_position().x, 0.0);
    // Reads with the inspector's domain current see its state.
    {
        let _g = window::enter_domain(inspector);
        assert!(input_api::get_key("KeyA"));
        assert_eq!(input_api::mouse_position().x, 12.0);
        // "This frame" only starts with its own tick.
        assert!(!input_api::get_key_down("KeyA"));
    }
    input::tick_domain(DomainId::MAIN);
    {
        let _g = window::enter_domain(inspector);
        assert!(
            !input_api::get_key_down("KeyA"),
            "the main tick is not its tick"
        );
    }
    input::tick_domain(inspector);
    {
        let _g = window::enter_domain(inspector);
        assert!(input_api::get_key_down("KeyA"));
    }
    assert!(!input_api::get_key_down("KeyA"));

    // Gamepads go where the host sends them (the focused domain).
    input::gamepad_connected_in(inspector, 0, "pad");
    input::gamepad_button_in(inspector, 0, "South", 1.0);
    assert!(!input_api::is_button_down("South"));
    {
        let _g = window::enter_domain(inspector);
        assert!(input_api::is_button_down("South"));
        assert_eq!(input_api::gamepad_count(), 1.0);
    }
    assert_eq!(input_api::gamepad_count(), 0.0);

    // Blur lets go of one domain's keys only.
    input::key_down_in(DomainId::MAIN, "KeyB", false);
    input::blur_domain(inspector);
    assert!(input_api::get_key("KeyB"));
    {
        let _g = window::enter_domain(inspector);
        assert!(!input_api::get_key("KeyA"));
    }
    input::reset();
    window::reset();
}
