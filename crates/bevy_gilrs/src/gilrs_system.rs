use crate::{
    converter::{convert_axis, convert_button}, Gilrs, GilrsGamepads, RawGilrsGamepadAxisChangedEvent, RawGilrsGamepadButtonChangedEvent, RawGilrsGamepadInputEvent
};
use bevy_ecs::event::EventWriter;
use bevy_ecs::prelude::Commands;
#[cfg(target_arch = "wasm32")]
use bevy_ecs::system::NonSendMut;
use bevy_ecs::system::ResMut;
use bevy_input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter};

pub fn gilrs_event_startup_system(
    mut commands: Commands,
    #[cfg(target_arch = "wasm32")] mut gilrs: NonSendMut<Gilrs>,
    #[cfg(not(target_arch = "wasm32"))] mut gilrs: ResMut<Gilrs>,
    mut gamepads: ResMut<GilrsGamepads>,
    mut events: EventWriter<GamepadConnectionEvent>,
) {
    for (id, gamepad) in gilrs.0.get().gamepads() {
        // Create entity and add to mapping
        let entity = commands.spawn_empty().id();
        gamepads.id_to_entity.insert(id, entity);
        gamepads.entity_to_id.insert(entity, id);

        events.send(GamepadConnectionEvent {
            gamepad: entity,
            connection: GamepadConnection::Connected {
                name: gamepad.name().to_string(),
                vendor_id: gamepad.vendor_id(),
                product_id: gamepad.product_id(),
            },
        });
    }
}

pub fn gilrs_event_system(
    mut commands: Commands,
    #[cfg(target_arch = "wasm32")] mut gilrs: NonSendMut<Gilrs>,
    #[cfg(not(target_arch = "wasm32"))] mut gilrs: ResMut<Gilrs>,
    mut gamepads: ResMut<GilrsGamepads>,
    mut events: EventWriter<RawGamepadEvent>,
    mut raw_gilrs_events: EventWriter<RawGilrsGamepadInputEvent>,
    mut connection_events: EventWriter<GamepadConnectionEvent>,
    mut button_events: EventWriter<RawGamepadButtonChangedEvent>,
    mut axis_event: EventWriter<RawGamepadAxisChangedEvent>,
) {
    let gilrs = gilrs.0.get();
    while let Some(gilrs_event) = gilrs.next_event().filter_ev(&axis_dpad_to_button, gilrs) {
        gilrs.update(&gilrs_event);
        match gilrs_event.event {
            EventType::Connected => {
                let pad = gilrs.gamepad(gilrs_event.id);
                let entity = gamepads.get_entity(gilrs_event.id).unwrap_or_else(|| {
                    let entity = commands.spawn_empty().id();
                    gamepads.id_to_entity.insert(gilrs_event.id, entity);
                    gamepads.entity_to_id.insert(entity, gilrs_event.id);
                    entity
                });

                let event = GamepadConnectionEvent::new(
                    entity,
                    GamepadConnection::Connected {
                        name: pad.name().to_string(),
                        vendor_id: pad.vendor_id(),
                        product_id: pad.product_id(),
                    },
                );

                events.send(event.clone().into());
                connection_events.send(event);
            }
            EventType::Disconnected => {
                let gamepad = gamepads
                    .id_to_entity
                    .get(&gilrs_event.id)
                    .copied()
                    .expect("mapping should exist from connection");
                let event = GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected);
                events.send(event.clone().into());
                connection_events.send(event);
            }
            EventType::ButtonChanged(gilrs_button, raw_value, code) => {
                let gamepad = gamepads
                    .id_to_entity
                    .get(&gilrs_event.id)
                    .copied()
                    .expect("mapping should exist from connection");

                let gilrs_axis_event = RawGilrsGamepadButtonChangedEvent::new(gamepad, code.into_u32(), raw_value);
                raw_gilrs_events.send(RawGilrsGamepadInputEvent::Button(gilrs_axis_event));

                let Some(button) = convert_button(gilrs_button) else {
                    continue;
                };
                events.send(RawGamepadButtonChangedEvent::new(gamepad, button, raw_value).into());
                button_events.send(RawGamepadButtonChangedEvent::new(
                    gamepad, button, raw_value,
                ));
            }
            EventType::AxisChanged(gilrs_axis, raw_value, code) => {
                let gamepad = gamepads
                    .id_to_entity
                    .get(&gilrs_event.id)
                    .copied()
                    .expect("mapping should exist from connection");

                let gilrs_axis_event = RawGilrsGamepadAxisChangedEvent::new(gamepad, code.into_u32(), raw_value);
                raw_gilrs_events.send(RawGilrsGamepadInputEvent::Axis(gilrs_axis_event));

                let Some(axis) = convert_axis(gilrs_axis) else {
                    continue;
                };                
                events.send(RawGamepadAxisChangedEvent::new(gamepad, axis, raw_value).into());
                axis_event.send(RawGamepadAxisChangedEvent::new(gamepad, axis, raw_value));
            }
            _ => (),
        };
    }
    gilrs.inc();
}
