/*
 * Copyright (c) 2018 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use std::cell::RefCell;
use std::rc::Rc;

use enigo::{Enigo, KeyboardControllable, MouseButton, MouseControllable};
use gdk::keyval_to_unicode;
use gdk::keys::Key;
use gdk::keys::constants as key;
use glib::{IsA, Object, object::Cast};
use gtk::{Inhibit, ToolButton, ToolButtonExt, Widget, WidgetExt};
use gtk_test::{self, focus, mouse_move, run_loop, wait_for_draw};
use relm::StreamHandle;

// TODO: should remove the signal after wait()?
// FIXME: remove when it's in gtk-test.
macro_rules! gtk_observer_new {
    ($widget:expr, $signal_name:ident, |$e1:pat $(,$e:pat)*|) => {{
        let observer = gtk_test::Observer::new();
        let res = (*observer.get_inner()).clone();
        $widget.$signal_name(move |$e1 $(,$e:expr)*| {
            *res.borrow_mut() = true;
        });
        observer
    }};
    ($widget:expr, $signal_name:ident, |$e1:pat $(,$e:pat)*| $block:block) => {{
        let observer = gtk_test::Observer::new();
        let res = (*observer.get_inner()).clone();
        $widget.$signal_name(move |$e1 $(,$e)*| {
            *res.borrow_mut() = true;
            $block
        });
        observer
    }}
}

pub struct Observer<MSG> {
    result: Rc<RefCell<Option<MSG>>>,
}

impl<MSG: Clone + 'static> Observer<MSG> {
    pub fn new<F: Fn(&MSG) -> bool + 'static>(stream: StreamHandle<MSG>, predicate: F) -> Self {
        let result = Rc::new(RefCell::new(None));
        let res = result.clone();
        stream.observe(move |msg| {
            if predicate(msg) {
                *res.borrow_mut() = Some(msg.clone());
            }
        });
        Self {
            result,
        }
    }

    pub fn wait(&self) -> MSG {
        loop {
            if let Ok(ref result) = self.result.try_borrow() {
                if result.is_some() {
                    break;
                }
            }
            gtk_test::run_loop();
        }
        self.result.borrow_mut().take()
            .expect("Message to take")
    }
}

#[macro_export]
macro_rules! relm_observer_new {
    ($component:expr, $pat:pat) => {
        $crate::Observer::new($component.stream(), |msg|
            if let $pat = msg {
                true
            }
            else {
                false
            }
        );
    };
}

#[macro_export]
macro_rules! relm_observer_wait {
    (let $($variant:ident)::*($name1:ident, $name2:ident $(,$rest:ident)*) = $observer:expr) => {
        let ($name1, $name2 $(, $rest)*) = {
            let msg = $observer.wait();
            if let $($variant)::*($name1, $name2 $(, $rest)*) = msg {
                ($name1, $name2 $(, $rest)*)
            }
            else {
                panic!("Wrong message type.");
            }
        };
    };
    (let $($variant:ident)::*($name:ident) = $observer:expr) => {
        let $name = {
            let msg = $observer.wait();
            if let $($variant)::*($name) = msg {
                $name
            }
            else {
                panic!("Wrong message type.");
            }
        };
    };
    (let $($variant:ident)::* = $observer:expr) => {
        let () = {
            let msg = $observer.wait();
            if let $($variant)::* = msg {
                ()
            }
            else {
                panic!("Wrong message type.");
            }
        };
    };
}

// FIXME: remove when it's in gtk-test.
pub fn click<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt + IsA<W>>(widget: &W) {
    wait_for_draw(widget, || {
        let observer =
            if let Ok(tool_button) = widget.clone().dynamic_cast::<ToolButton>() {
                gtk_observer_new!(tool_button, connect_clicked, |_|)
            }
            else {
                gtk_observer_new!(widget, connect_button_press_event, |_, _| {
                    Inhibit(false)
                })
            };
        let allocation = widget.get_allocation();
        mouse_move(widget, allocation.width / 2, allocation.height / 2);
        let mut enigo = Enigo::new();
        println!("Click");
        enigo.mouse_click(MouseButton::Left);
        observer.wait();
        // FIXME: even if the click was registered, that does not mean the relm events were
        // processed.
        // The following could happen:
        // The above observer got the GTK event.
        // The observer wait returns.
        // Relm gets a GTK event.
        // Relm emits a message.
        //
        // Since we don't try to process messages after the wait for as long as is required, we
        // might never see the message being processed.

        gtk_test::wait(0);
        run_loop();
    });
}

pub fn mouse_move_to<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt + IsA<W>>(widget: &W) {
    wait_for_draw(widget, || {
        let allocation = widget.get_allocation();
        mouse_move(widget, allocation.width / 2, allocation.height / 2);
    });
}

pub fn double_click<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt>(widget: &W) {
    wait_for_draw(widget, || {
        let observer = gtk_observer_new!(widget, connect_button_release_event, |_, _| {
            Inhibit(false)
        });
        let allocation = widget.get_allocation();
        mouse_move(widget, allocation.width / 2, allocation.height / 2);
        let mut enigo = Enigo::new();
        // FIXME: seems like it's triggered as two single clicks.
        println!("Click 1");
        enigo.mouse_click(MouseButton::Left);
        run_loop();
        println!("Click 2");
        enigo.mouse_click(MouseButton::Left);
        observer.wait();

        gtk_test::wait(0);
        run_loop();
    });
}

// FIXME: don't wait the observer for modifier keys like shift?
pub fn key_press<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt>(widget: &W, key: Key) {
    wait_for_draw(widget, || {
        let observer = gtk_observer_new!(widget, connect_key_press_event, |_, _| {
            Inhibit(false)
        });
        focus(widget);
        let mut enigo = Enigo::new();
        enigo.key_down(gdk_key_to_enigo_key(key));
        observer.wait();
    });
}

pub fn key_release<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt>(widget: &W, key: Key) {
    wait_for_draw(widget, || {
        let observer = gtk_observer_new!(widget, connect_key_release_event, |_, _| {
            Inhibit(false)
        });
        focus(widget);
        let mut enigo = Enigo::new();
        enigo.key_up(gdk_key_to_enigo_key(key));
        observer.wait();
    });
}

pub fn enter_key<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt>(widget: &W, key: Key) {
    wait_for_draw(widget, || {
        let observer = gtk_observer_new!(widget, connect_key_release_event, |_, _| {
            Inhibit(false)
        });
        focus(widget);
        let mut enigo = Enigo::new();
        enigo.key_click(gdk_key_to_enigo_key(key));
        observer.wait();
    });
}

pub fn enter_keys<W: Clone + IsA<Object> + IsA<Widget> + WidgetExt>(widget: &W, text: &str) {
    wait_for_draw(widget, || {
        focus(widget);
        let mut enigo = Enigo::new();
        for char in text.chars() {
            let observer = gtk_observer_new!(widget, connect_key_release_event, |_, _| {
                Inhibit(false)
            });
            enigo.key_sequence(&char.to_string());
            observer.wait();
        }
    });
}

fn gdk_key_to_enigo_key(key: Key) -> enigo::Key {
    use enigo::Key::*;
    match key {
        key::Return => Return,
        key::Tab => Tab,
        key::space => Space,
        key::BackSpace => Backspace,
        key::Escape => Escape,
        key::Super_L | key::Super_R => Meta,
        key::Control_L | key::Control_R => Control,
        key::Shift_L | key::Shift_R => Shift,
        key::Shift_Lock => CapsLock,
        key::Alt_L | key::Alt_R => Alt,
        key::Option => Option,
        key::End => End,
        key::Home => Home,
        key::Page_Down => PageDown,
        key::Page_Up => PageUp,
        key::leftarrow => LeftArrow,
        key::rightarrow => RightArrow,
        key::downarrow => DownArrow,
        key::uparrow => UpArrow,
        key::F1 => F1,
        key::F2 => F2,
        key::F3 => F3,
        key::F4 => F4,
        key::F5 => F5,
        key::F6 => F6,
        key::F7 => F7,
        key::F8 => F8,
        key::F9 => F9,
        key::F10 => F10,
        key::F11 => F11,
        key::F12 => F12,
        _ => {
            if let Some(char) = keyval_to_unicode(*key) {
                Layout(char)
            }
            else {
                Raw(*key as u16)
            }
        },
    }
}
