use std::{collections::HashMap, rc::Rc};

use sdl2::controller::{Axis, Button};

pub struct ViGEMState {
    target: vigem_client::Xbox360Wired<Rc<vigem_client::Client>>,
    pub button_state: HashMap<u16, bool>,
    pub gamepad: vigem_client::XGamepad,
    pub socd_horizontal: bool,
    pub socd_vertical: bool,
}

impl ViGEMState {
    pub fn new(client: Rc<vigem_client::Client>) -> Self {
        // Create the virtual controller target
        let id = vigem_client::TargetId::XBOX360_WIRED;
        let mut target = vigem_client::Xbox360Wired::new(client, id);

        // Plugin the virtual controller
        target.plugin().unwrap();

        // Wait for the virtual controller to be ready to accept updates
        target.wait_ready().unwrap();

        // The input state of the virtual controller
        let gamepad = vigem_client::XGamepad {
            buttons: vigem_client::XButtons!(),
            ..Default::default()
        };

        let _ = target.update(&gamepad);

        let mut button_state = HashMap::default();
        button_state.insert(vigem_client::XButtons::UP, false);
        button_state.insert(vigem_client::XButtons::DOWN, false);
        button_state.insert(vigem_client::XButtons::RIGHT, false);
        button_state.insert(vigem_client::XButtons::LEFT, false);
        button_state.insert(vigem_client::XButtons::START, false);
        button_state.insert(vigem_client::XButtons::BACK, false);
        button_state.insert(vigem_client::XButtons::LTHUMB, false);
        button_state.insert(vigem_client::XButtons::RTHUMB, false);
        button_state.insert(vigem_client::XButtons::LB, false);
        button_state.insert(vigem_client::XButtons::RB, false);
        button_state.insert(vigem_client::XButtons::A, false);
        button_state.insert(vigem_client::XButtons::B, false);
        button_state.insert(vigem_client::XButtons::X, false);
        button_state.insert(vigem_client::XButtons::Y, false);

        ViGEMState {
            target: target,
            button_state: button_state,
            gamepad: gamepad,
            socd_vertical: false,
            socd_horizontal: false,
        }
    }

    pub fn submit_report(&mut self) {
        let mut computed_button_state = 0;
        for (k, v) in self.button_state.clone().into_iter() {
            if v {
                computed_button_state |= k;
            }
        }
        self.gamepad.buttons = vigem_client::XButtons(computed_button_state);
        let _ = self.target.update(&self.gamepad);
    }

    pub fn update_button(&mut self, button: &u16, value: bool) {
        *self.button_state.get_mut(button).unwrap() = value
    }

    pub fn from_sdl2_button(&mut self, button: Button, value: bool) {
        match button {
            Button::A => self.update_button(&vigem_client::XButtons::A, value),
            Button::B => self.update_button(&vigem_client::XButtons::B, value),
            Button::X => self.update_button(&vigem_client::XButtons::X, value),
            Button::Y => self.update_button(&vigem_client::XButtons::Y, value),
            Button::LeftShoulder => self.update_button(&vigem_client::XButtons::LB, value),
            Button::RightShoulder => self.update_button(&vigem_client::XButtons::RB, value),
            Button::LeftStick => self.update_button(&vigem_client::XButtons::LTHUMB, value),
            Button::RightStick => self.update_button(&vigem_client::XButtons::RTHUMB, value),
            Button::DPadLeft => self.update_button(&vigem_client::XButtons::LEFT, value),
            Button::DPadRight => self.update_button(&vigem_client::XButtons::RIGHT, value),
            Button::DPadUp => self.update_button(&vigem_client::XButtons::UP, value),
            Button::DPadDown => self.update_button(&vigem_client::XButtons::DOWN, value),
            Button::Guide => self.update_button(&vigem_client::XButtons::GUIDE, value),
            Button::Back => self.update_button(&vigem_client::XButtons::BACK, value),
            Button::Start => self.update_button(&vigem_client::XButtons::START, value),
            _ => {}
        }
        self.submit_report();
    }

    pub fn from_sdl2_axis(&mut self, axis: Axis, value: i16) {
        match axis {
            Axis::LeftX => self.gamepad.thumb_lx = value,
            Axis::LeftY => self.gamepad.thumb_ly = value,
            Axis::RightX => self.gamepad.thumb_rx = value,
            Axis::RightY => self.gamepad.thumb_ry = value,
            Axis::TriggerLeft => self.gamepad.left_trigger = (value / (32767 / 255)) as u8,
            Axis::TriggerRight => self.gamepad.right_trigger = (value / (32767 / 255)) as u8,
        }
        let _ = self.target.update(&self.gamepad);
    }
}
