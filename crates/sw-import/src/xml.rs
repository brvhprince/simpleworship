//! A tiny view of XML for the song importers: element starts (with their
//! attributes, names in lowercase), ends, and text with entities resolved.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

/// One simplified XML event.
pub enum XmlEvent {
    /// An element starts; `empty` for self-closing ones (no matching end).
    Start {
        name: String,
        attributes: HashMap<String, String>,
        empty: bool,
    },
    /// An element ends.
    End { name: String },
    /// Text (including CDATA).
    Text(String),
}

/// Calls `handle` for every event, in document order.
pub fn for_each_event(xml: &str, mut handle: impl FnMut(XmlEvent)) -> Result<(), String> {
    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader.read_event().map_err(|error| error.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let attributes = element
                    .attributes()
                    .flatten()
                    .map(|attribute| {
                        let key = attribute.key.local_name().as_ref().to_lowercase();
                        let value = attribute
                            .normalized_value(XmlVersion::Implicit1_0)
                            .map(|value| value.into_owned())
                            .unwrap_or_default();
                        (key, value)
                    })
                    .collect();
                handle(XmlEvent::Start {
                    name: element.local_name().as_ref().to_lowercase(),
                    attributes,
                    empty,
                });
            }
            Event::End(element) => handle(XmlEvent::End {
                name: element.local_name().as_ref().to_lowercase(),
            }),
            Event::Text(text) => handle(XmlEvent::Text(text.xml10_content().into_owned())),
            Event::CData(data) => handle(XmlEvent::Text(data.xml10_content().into_owned())),
            Event::GeneralRef(reference) => {
                let name = reference.into_inner();
                if let Some(text) = resolve(&name) {
                    handle(XmlEvent::Text(text));
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn resolve(name: &str) -> Option<String> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code).map(String::from);
    }
    quick_xml::escape::resolve_predefined_entity(name).map(str::to_owned)
}
