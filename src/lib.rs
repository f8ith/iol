use sdl2::{
    controller::{Axis, Button, GameController},
    keyboard::Scancode,
};
use serde::{Deserialize, Serialize};
use vigem_client::{XGamepad, Xbox360Wired};

pub struct GamepadState {
    state: XGamepad,
    target: Xbox360Wired,
}

impl GamepadState {
    pub fn new(target: XGamepad, target: Xbox360Wired) -> Self {
        GamepadState { state, target }
    }

    pub fn state(&mut self) -> &mut XGamepad {
        &mut self.state
    }

    pub fn controller(&self) -> &mut Xbox360Wired {
        &mut self.controller
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum IolEvent {
    ButtonUp {
        id: u32,
        #[serde(with = "sdl2_button_serde")]
        button: Button,
    },
    ButtonDown {
        id: u32,
        #[serde(with = "sdl2_button_serde")]
        button: Button,
    },
    AxisMotion {
        id: u32,
        #[serde(with = "sdl2_axis_serde")]
        axis: Axis,
        value: i16,
    },
    KeyDown {
        #[serde(with = "sdl2_scancode_serde")]
        scancode: Scancode,
        repeat: bool,
    },
    KeyUp {
        #[serde(with = "sdl2_scancode_serde")]
        scancode: Scancode,
    },
    PhysicalDeviceAdded {
        which: u32,
    },
    PhysicalDeviceRemoved {
        id: u32,
    },
    VirtualDeviceAdded {
        id: u32,
        which: u32,
    },
}

pub(crate) mod sdl2_scancode_serde {
    use std::fmt;

    use sdl2::keyboard::Scancode as Sdl2Scancode;
    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(scancode: &Sdl2Scancode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*scancode as i32)
    }

    pub struct ScancodeVisitor;

    impl<'de> Visitor<'de> for ScancodeVisitor {
        type Value = Sdl2Scancode;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid SDL2 Scancode integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            use std::i32;
            if value >= i64::from(i32::MIN) && value <= i64::from(i32::MAX) {
                Sdl2Scancode::from_i32(value as i32)
                    .ok_or(E::custom(format!("scancode not recognized by SDL2")))
            } else {
                Err(E::custom(format!("i32 out of range: {}", value)))
            }
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            i64::try_from(v)
                .map_err(|_| E::custom(format!("out of range: {}", v)))
                .and_then(|v| self.visit_i64(v))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Sdl2Scancode, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i64(ScancodeVisitor)
    }
}

pub(crate) mod sdl2_button_serde {
    use std::fmt;

    use sdl2::controller::Button;
    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(button: &Button, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&button.string())
    }
    pub struct ButtonVisitor;

    impl<'de> Visitor<'de> for ButtonVisitor {
        type Value = Button;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid SDL2 Scancode integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Button::from_string(value).ok_or(E::custom(format!("button not recognized by SDL2")))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Button, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ButtonVisitor)
    }
}

pub(crate) mod sdl2_axis_serde {
    use std::fmt;

    use sdl2::controller::Axis;
    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(axis: &Axis, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&axis.string())
    }
    pub struct AxisVisitor;

    impl<'de> Visitor<'de> for AxisVisitor {
        type Value = Axis;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid SDL2 Scancode integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Axis::from_string(value).ok_or(E::custom(format!("Axis not recognized by SDL2")))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Axis, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AxisVisitor)
    }
}
